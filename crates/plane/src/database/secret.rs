use sqlx::PgPool;

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

    pub async fn delete(&self, id: &str, tenant: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM secret WHERE id = $1 AND tenant = $2
            "#,
        )
        .bind(id)
        .bind(tenant)
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
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
