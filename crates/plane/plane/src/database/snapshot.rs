use std::collections::HashMap;

use chamuka_common::ext::sandbox::Arch;
use chamuka_common::ext::snapshot::Snapshot;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

pub struct SnapshotDatabase<'a> {
    pool: &'a PgPool,
}

impl<'a> SnapshotDatabase<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert(
        &self,
        id: &str,
        sandbox_id: &str,
        tenant: &str,
        base_image: &str,
        arch: &str,
        runtime: &str,
        name: Option<&str>,
        tags: &HashMap<String, String>,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        let tags_json = serde_json::to_value(tags).unwrap_or_default();
        sqlx::query(
            r#"
            INSERT INTO snapshot (
                id, sandbox_id, tenant, image_ref, digest, base_image,
                size_bytes, arch, runtime, name, tags, expires_at
            ) VALUES ($1, $2, $3, $4, '', $5, 0, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(id)
        .bind(sandbox_id)
        .bind(tenant)
        .bind(format!("{base_image}:snapshot"))
        .bind(base_image)
        .bind(arch)
        .bind(runtime)
        .bind(name)
        .bind(&tags_json)
        .bind(expires_at)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_by_id(&self, id: &str, tenant: &str) -> Result<Option<Snapshot>, sqlx::Error> {
        let row = sqlx::query_as::<_, SnapshotRow>(
            r#"
            SELECT
                id, sandbox_id, tenant, image_ref, digest, base_image,
                size_bytes, arch, runtime, name, tags, created_at, expires_at
            FROM snapshot
            WHERE id = $1 AND tenant = $2
            "#,
        )
        .bind(id)
        .bind(tenant)
        .fetch_optional(self.pool)
        .await?;
        Ok(row.map(snapshot_from_row))
    }

    pub async fn delete(&self, id: &str, tenant: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM snapshot WHERE id = $1 AND tenant = $2
            "#,
        )
        .bind(id)
        .bind(tenant)
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct SnapshotRow {
    pub id: String,
    pub sandbox_id: String,
    pub tenant: String,
    pub image_ref: String,
    pub digest: String,
    pub base_image: String,
    pub size_bytes: i64,
    pub arch: String,
    pub runtime: String,
    pub name: Option<String>,
    pub tags: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

fn snapshot_from_row(row: SnapshotRow) -> Snapshot {
    let arch = match row.arch.as_str() {
        "arm64" => Arch::Arm64,
        _ => Arch::Amd64,
    };
    let tags: HashMap<String, String> = serde_json::from_value(row.tags).unwrap_or_default();

    Snapshot {
        id: row.id,
        sandbox_id: row.sandbox_id,
        tenant: row.tenant,
        image_ref: row.image_ref,
        digest: row.digest,
        base_image: row.base_image,
        size_bytes: row.size_bytes as u64,
        arch,
        runtime: row.runtime,
        name: row.name,
        tags,
        created_at: row.created_at,
        expires_at: row.expires_at,
    }
}
