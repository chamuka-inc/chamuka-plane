use sqlx::PgPool;

pub struct CredentialDatabase<'a> {
    pool: &'a PgPool,
}

impl<'a> CredentialDatabase<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    // Maps 1:1 to the INSERT column list
    #[allow(clippy::too_many_arguments)]
    pub async fn insert(
        &self,
        id: &str,
        sandbox_id: &str,
        secret_id: &str,
        destination: &str,
        header_name: &str,
        header_format: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO credential_binding (
                id, sandbox_id, secret_id, destination, header_name, header_format
            ) VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(id)
        .bind(sandbox_id)
        .bind(secret_id)
        .bind(destination)
        .bind(header_name)
        .bind(header_format)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn resolve_secret_value(
        &self,
        credential_id: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT s.encrypted_value
            FROM credential_binding cb
            JOIN secret s ON s.id = cb.secret_id
            WHERE cb.id = $1 AND s.deleted_at IS NULL
            "#,
        )
        .bind(credential_id)
        .fetch_optional(self.pool)
        .await
    }

    pub async fn delete(&self, id: &str, sandbox_id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM credential_binding WHERE id = $1 AND sandbox_id = $2
            "#,
        )
        .bind(id)
        .bind(sandbox_id)
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}
