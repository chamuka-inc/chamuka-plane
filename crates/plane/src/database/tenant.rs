use sqlx::PgPool;

pub struct TenantDatabase<'a> {
    pool: &'a PgPool,
}

impl<'a> TenantDatabase<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn ensure_exists(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO tenant (id)
            VALUES ($1)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(id)
        .execute(self.pool)
        .await?;
        Ok(())
    }
}
