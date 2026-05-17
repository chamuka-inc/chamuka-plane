use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

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
        let row = sqlx::query_as::<_, SandboxRow>(
            r#"
            SELECT
                id, backend_id, tenant, runtime, trust_tier, arch, image,
                vcpus, memory_mb, ports, tags,
                persistent, snapshot_source,
                created_at, updated_at, expires_at, ready_at, terminated_at
            FROM sandbox
            WHERE id = $1 AND tenant = $2
            "#,
        )
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
        let row = sqlx::query_as::<_, SandboxRow>(
            r#"
            UPDATE sandbox
            SET expires_at = expires_at + $3, updated_at = NOW()
            WHERE id = $1 AND tenant = $2
            RETURNING
                id, backend_id, tenant, runtime, trust_tier, arch, image,
                vcpus, memory_mb, ports, tags,
                persistent, snapshot_source,
                created_at, updated_at, expires_at, ready_at, terminated_at
            "#,
        )
        .bind(id)
        .bind(tenant)
        .bind(additional)
        .fetch_optional(self.pool)
        .await?;
        Ok(row)
    }

    pub async fn update_network_policy(
        &self,
        id: &str,
        policy: &serde_json::Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE sandbox
            SET network_policy = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(policy)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_by_tenant(
        &self,
        tenant: &str,
        limit: i64,
        cursor: Option<DateTime<Utc>>,
    ) -> Result<Vec<SandboxRow>, sqlx::Error> {
        let rows = if let Some(before) = cursor {
            sqlx::query_as::<_, SandboxRow>(
                r#"
                SELECT
                    id, backend_id, tenant, runtime, trust_tier, arch, image,
                    vcpus, memory_mb, ports, tags,
                    persistent, snapshot_source,
                    created_at, updated_at, expires_at, ready_at, terminated_at
                FROM sandbox
                WHERE tenant = $1 AND created_at < $2
                ORDER BY created_at DESC
                LIMIT $3
                "#,
            )
            .bind(tenant)
            .bind(before)
            .bind(limit)
            .fetch_all(self.pool)
            .await?
        } else {
            sqlx::query_as::<_, SandboxRow>(
                r#"
                SELECT
                    id, backend_id, tenant, runtime, trust_tier, arch, image,
                    vcpus, memory_mb, ports, tags,
                    persistent, snapshot_source,
                    created_at, updated_at, expires_at, ready_at, terminated_at
                FROM sandbox
                WHERE tenant = $1
                ORDER BY created_at DESC
                LIMIT $2
                "#,
            )
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
    pub backend_id: String,
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
}
