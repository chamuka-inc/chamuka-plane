use sqlx::PgPool;

pub struct UsageRecordDatabase<'a> {
    pool: &'a PgPool,
}

impl<'a> UsageRecordDatabase<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn accumulate_for_backend(
        &self,
        backend_id: &str,
        cpu_nsec_delta: i64,
        memory_mb_seconds_delta: i64,
        rx_bytes_delta: i64,
        tx_bytes_delta: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO usage_record (sandbox_id, bucket_start, cpu_nsec, memory_mb_seconds, rx_bytes, tx_bytes)
            SELECT s.id, date_trunc('hour', NOW()), $2, $3, $4, $5
            FROM sandbox s WHERE s.backend_id = $1
            ON CONFLICT (sandbox_id, bucket_start)
            DO UPDATE SET
                cpu_nsec = usage_record.cpu_nsec + EXCLUDED.cpu_nsec,
                memory_mb_seconds = usage_record.memory_mb_seconds + EXCLUDED.memory_mb_seconds,
                rx_bytes = usage_record.rx_bytes + EXCLUDED.rx_bytes,
                tx_bytes = usage_record.tx_bytes + EXCLUDED.tx_bytes
            "#,
        )
        .bind(backend_id)
        .bind(cpu_nsec_delta)
        .bind(memory_mb_seconds_delta)
        .bind(rx_bytes_delta)
        .bind(tx_bytes_delta)
        .execute(self.pool)
        .await?;
        Ok(())
    }
}
