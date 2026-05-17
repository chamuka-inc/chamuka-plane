use sqlx::PgPool;

pub struct ExtensionActionDatabase<'a> {
    pool: &'a PgPool,
}

impl<'a> ExtensionActionDatabase<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(
        &self,
        id: &str,
        kind: &str,
        backend_id: &str,
        sandbox_id: &str,
        drone_id: i32,
        payload: &serde_json::Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO extension_action (id, kind, backend_id, sandbox_id, drone_id, payload)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(id)
        .bind(kind)
        .bind(backend_id)
        .bind(sandbox_id)
        .bind(drone_id)
        .bind(payload)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn ack(&self, action_id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE extension_action
            SET acked_at = now()
            WHERE id = $1 AND acked_at IS NULL
            "#,
        )
        .bind(action_id)
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn pending_for_drone(
        &self,
        drone_id: i32,
        limit: i64,
    ) -> Result<Vec<ExtensionActionRow>, sqlx::Error> {
        let rows = sqlx::query_as::<_, ExtensionActionRow>(
            r#"
            SELECT id, kind, backend_id, sandbox_id, drone_id, payload
            FROM extension_action
            WHERE drone_id = $1 AND acked_at IS NULL
            ORDER BY created_at ASC
            LIMIT $2
            "#,
        )
        .bind(drone_id)
        .bind(limit)
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn cleanup_acked(&self, batch_size: i32) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM extension_action
            WHERE id IN (
                SELECT id FROM extension_action
                WHERE acked_at IS NOT NULL
                  AND acked_at < now() - interval '1 day'
                LIMIT $1
            )
            "#,
        )
        .bind(batch_size)
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected())
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ExtensionActionRow {
    pub id: String,
    pub kind: String,
    pub backend_id: String,
    pub sandbox_id: String,
    pub drone_id: i32,
    pub payload: serde_json::Value,
}
