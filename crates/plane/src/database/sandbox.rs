use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

const SANDBOX_SELECT: &str = r#"
    SELECT
        s.id, s.backend_id, s.tenant, s.runtime, s.trust_tier, s.arch, s.image,
        s.vcpus, s.memory_mb, s.ports, s.tags,
        s.persistent, s.snapshot_source,
        s.created_at, s.updated_at, s.expires_at, s.ready_at, s.terminated_at,
        b.last_status AS "live_status",
        n.name AS "drone_name",
        t.token AS "bearer_token"
    FROM sandbox s
    LEFT JOIN backend b ON b.id = s.backend_id
    LEFT JOIN node n ON n.id = b.drone_id
    LEFT JOIN token t ON t.backend_id = s.backend_id
"#;

pub struct SandboxDatabase<'a> {
    pool: &'a PgPool,
}

impl<'a> SandboxDatabase<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert(
        &self,
        id: &str,
        backend_id: &str,
        tenant: &str,
        runtime: &str,
        trust_tier: &str,
        arch: &str,
        image: &str,
        vcpus: i32,
        memory_mb: i32,
        ports: &[i32],
        tags: &serde_json::Value,
        persistent: bool,
        snapshot_source: Option<&str>,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO sandbox (
                id, backend_id, tenant, runtime, trust_tier, arch, image,
                vcpus, memory_mb, ports, tags, persistent, snapshot_source, expires_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#,
        )
        .bind(id)
        .bind(backend_id)
        .bind(tenant)
        .bind(runtime)
        .bind(trust_tier)
        .bind(arch)
        .bind(image)
        .bind(vcpus)
        .bind(memory_mb)
        .bind(ports)
        .bind(tags)
        .bind(persistent)
        .bind(snapshot_source)
        .bind(expires_at)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_by_id(
        &self,
        id: &str,
        tenant: &str,
    ) -> Result<Option<SandboxRow>, sqlx::Error> {
        let sql = format!("{SANDBOX_SELECT} WHERE s.id = $1 AND s.tenant = $2");
        let row = sqlx::query_as::<_, SandboxRow>(&sql)
            .bind(id)
            .bind(tenant)
            .fetch_optional(self.pool)
            .await?;
        Ok(row)
    }

    pub async fn extend_expires_at(
        &self,
        id: &str,
        tenant: &str,
        additional: chrono::Duration,
    ) -> Result<Option<SandboxRow>, sqlx::Error> {
        let updated = sqlx::query(
            r#"
            UPDATE sandbox
            SET expires_at = expires_at + $3, updated_at = NOW()
            WHERE id = $1 AND tenant = $2
            "#,
        )
        .bind(id)
        .bind(tenant)
        .bind(additional)
        .execute(self.pool)
        .await?;

        if updated.rows_affected() == 0 {
            return Ok(None);
        }
        self.get_by_id(id, tenant).await
    }

    pub async fn update_network_policy(
        &self,
        id: &str,
        tenant: &str,
        policy: &serde_json::Value,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE sandbox
            SET network_policy = $2, updated_at = NOW()
            WHERE id = $1 AND tenant = $3
            "#,
        )
        .bind(id)
        .bind(policy)
        .bind(tenant)
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn get_drone_id_for_backend(
        &self,
        backend_id: &str,
    ) -> Result<Option<i32>, sqlx::Error> {
        let row: Option<(i32,)> = sqlx::query_as(r#"SELECT drone_id FROM backend WHERE id = $1"#)
            .bind(backend_id)
            .fetch_optional(self.pool)
            .await?;
        Ok(row.map(|r| r.0))
    }

    pub async fn list_by_tenant(
        &self,
        tenant: &str,
        limit: i64,
        cursor: Option<DateTime<Utc>>,
    ) -> Result<Vec<SandboxRow>, sqlx::Error> {
        let rows = if let Some(before) = cursor {
            let sql = format!(
                "{SANDBOX_SELECT} WHERE s.tenant = $1 AND s.created_at < $2 ORDER BY s.created_at DESC LIMIT $3"
            );
            sqlx::query_as::<_, SandboxRow>(&sql)
                .bind(tenant)
                .bind(before)
                .bind(limit)
                .fetch_all(self.pool)
                .await?
        } else {
            let sql =
                format!("{SANDBOX_SELECT} WHERE s.tenant = $1 ORDER BY s.created_at DESC LIMIT $2");
            sqlx::query_as::<_, SandboxRow>(&sql)
                .bind(tenant)
                .bind(limit)
                .fetch_all(self.pool)
                .await?
        };
        Ok(rows)
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct SandboxRow {
    pub id: String,
    pub backend_id: Option<String>,
    pub tenant: String,
    pub runtime: String,
    pub trust_tier: String,
    pub arch: String,
    pub image: String,
    pub vcpus: i32,
    pub memory_mb: i32,
    pub ports: Vec<i32>,
    pub tags: serde_json::Value,
    pub persistent: bool,
    pub snapshot_source: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub ready_at: Option<DateTime<Utc>>,
    pub terminated_at: Option<DateTime<Utc>>,
    pub live_status: Option<String>,
    pub drone_name: Option<String>,
    pub bearer_token: Option<String>,
}
