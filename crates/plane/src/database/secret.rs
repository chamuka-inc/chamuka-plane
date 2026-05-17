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
}
