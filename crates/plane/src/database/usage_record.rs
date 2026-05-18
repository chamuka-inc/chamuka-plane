use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct UsageRow {
    pub sandbox_id: String,
    pub bucket_start: DateTime<Utc>,
    pub cpu_nsec: i64,
    pub memory_mb_seconds: i64,
    pub rx_bytes: i64,
    pub tx_bytes: i64,
}

pub struct UsageRecordDatabase<'a> {
    pool: &'a PgPool,
}

impl<'a> UsageRecordDatabase<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    const MAX_USAGE_ROWS: i64 = 10_000;

    pub async fn query_by_tenant(
        &self,
        tenant: &str,
        since: DateTime<Utc>,
        sandbox_id: Option<&str>,
    ) -> Result<Vec<UsageRow>, sqlx::Error> {
        let (filter, limit) = if sandbox_id.is_some() {
            ("AND u.sandbox_id = $3", "$4")
        } else {
            ("", "$3")
        };
        let sql = format!(
            r#"SELECT u.sandbox_id, u.bucket_start, u.cpu_nsec, u.memory_mb_seconds, u.rx_bytes, u.tx_bytes
               FROM usage_record u
               JOIN sandbox s ON s.id = u.sandbox_id
               WHERE s.tenant = $1 AND u.bucket_start >= $2 {filter}
               ORDER BY u.bucket_start DESC
               LIMIT {limit}"#,
        );
        let mut q = sqlx::query_as::<_, UsageRow>(&sql).bind(tenant).bind(since);
        if let Some(sid) = sandbox_id {
            q = q.bind(sid);
        }
        q.bind(Self::MAX_USAGE_ROWS).fetch_all(self.pool).await
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
