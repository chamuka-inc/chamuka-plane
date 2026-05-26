use sqlx::PgPool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteOutcome {
    /// The secret existed (active or already soft-deleted) and is now soft-deleted.
    Deleted,
    /// No secret with this (id, tenant) exists.
    NotFound,
}

pub struct SecretDatabase<'a> {
    pool: &'a PgPool,
}

impl<'a> SecretDatabase<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(
        &self,
        id: &str,
        tenant: &str,
        value: &str,
        name: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO secret (id, tenant, encrypted_value, name)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(id)
        .bind(tenant)
        .bind(value)
        .bind(name)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Soft-delete a secret. Idempotent: a second delete on an already-tombstoned
    /// row still returns `DeleteOutcome::Deleted` (not `NotFound`) so SDK
    /// retries map to 204 (no content) instead of 404. `NotFound` is reserved
    /// for the case where no row with this (id, tenant) ever existed.
    /// Implementation: COALESCE preserves the original deleted_at on a repeat
    /// call, and RETURNING distinguishes "row matched" from "no match".
    pub async fn delete(&self, id: &str, tenant: &str) -> Result<DeleteOutcome, sqlx::Error> {
        let deleted_at: Option<Option<chrono::DateTime<chrono::Utc>>> = sqlx::query_scalar(
            r#"
            UPDATE secret
            SET deleted_at = COALESCE(deleted_at, now())
            WHERE id = $1 AND tenant = $2
            RETURNING deleted_at
            "#,
        )
        .bind(id)
        .bind(tenant)
        .fetch_optional(self.pool)
        .await?;

        Ok(match deleted_at {
            None => DeleteOutcome::NotFound,
            Some(_) => DeleteOutcome::Deleted,
        })
    }

    /// Returns true iff a secret with this (id, tenant) exists and is not soft-deleted.
    pub async fn exists_active(&self, id: &str, tenant: &str) -> Result<bool, sqlx::Error> {
        let exists: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT TRUE FROM secret
            WHERE id = $1 AND tenant = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant)
        .fetch_optional(self.pool)
        .await?;
        Ok(exists.is_some())
    }

    pub async fn cleanup_soft_deleted(
        &self,
        min_age_days: i32,
        batch_size: i32,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM secret
            WHERE id IN (
                SELECT s.id FROM secret s
                WHERE s.deleted_at IS NOT NULL
                  AND s.deleted_at < now() - make_interval(days => $1)
                  AND NOT EXISTS (
                      SELECT 1 FROM credential_binding cb WHERE cb.secret_id = s.id
                  )
                LIMIT $2
            )
            "#,
        )
        .bind(min_age_days)
        .bind(batch_size)
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected())
    }
}
