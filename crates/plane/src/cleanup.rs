use crate::database::{subscribe::EventSubscriptionManager, PlaneDatabase};
use anyhow::Result;

const CLEANUP_LOOP_INTERVAL_SECONDS: u64 = 60 * 3;
const DEFAULT_BATCH_SIZE: i32 = 100;
const SECRET_RETENTION_DAYS: i32 = 30;
const ACKED_ACTION_RETENTION_DAYS: i32 = 1;

pub async fn run_cleanup(
    db: &PlaneDatabase,
    min_age_days: Option<i32>,
    cleanup_batch_size: Option<i32>,
) -> Result<()> {
    tracing::info!("Running cleanup");

    let batch_size = cleanup_batch_size.unwrap_or(DEFAULT_BATCH_SIZE);

    if let Some(min_age_days) = min_age_days {
        db.backend().cleanup(min_age_days, batch_size).await?;
        EventSubscriptionManager::clean_up_events(&db.pool, min_age_days).await?;
    }

    db.clean_up_tokens().await?;

    run_extension_cleanup(db, batch_size).await?;

    tracing::info!("Done running cleanup");

    Ok(())
}

async fn run_extension_cleanup(db: &PlaneDatabase, batch_size: i32) -> Result<()> {
    let snapshots_deleted = db.snapshot().cleanup_expired(batch_size).await?;
    if snapshots_deleted > 0 {
        tracing::info!(snapshots_deleted, "Cleaned up expired snapshots");
    }

    let secrets_deleted = db
        .secret()
        .cleanup_soft_deleted(SECRET_RETENTION_DAYS, batch_size)
        .await?;
    if secrets_deleted > 0 {
        tracing::info!(secrets_deleted, "Cleaned up soft-deleted secrets");
    }

    let actions_deleted = db
        .extension_action()
        .cleanup_acked(ACKED_ACTION_RETENTION_DAYS, batch_size)
        .await?;
    if actions_deleted > 0 {
        tracing::info!(actions_deleted, "Cleaned up acked extension actions");
    }

    Ok(())
}

pub async fn run_cleanup_loop(
    db: PlaneDatabase,
    min_age_days: Option<i32>,
    cleanup_batch_size: Option<i32>,
) {
    let random_offset_seconds = rand::random::<u64>() % CLEANUP_LOOP_INTERVAL_SECONDS;
    tokio::time::sleep(tokio::time::Duration::from_secs(random_offset_seconds)).await;

    loop {
        if let Err(e) = run_cleanup(&db, min_age_days, cleanup_batch_size).await {
            tracing::error!("Error running cleanup: {:?}", e);
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(
            CLEANUP_LOOP_INTERVAL_SECONDS,
        ))
        .await;
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::needless_borrows_for_generic_args
    )]

    use crate::database::PlaneDatabase;
    use sqlx::PgPool;

    async fn test_pool() -> Option<PgPool> {
        let url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres@127.0.0.1:5432/postgres".to_string());
        sqlx::PgPool::connect(&url).await.ok()
    }

    struct TestFixture {
        suffix: String,
        drone_id: i32,
    }

    impl TestFixture {
        fn backend_id(&self) -> String {
            id("bk", &self.suffix)
        }
        fn tenant_id(&self) -> String {
            id("tenant", &self.suffix)
        }
        fn sandbox_id(&self) -> String {
            id("sb", &self.suffix)
        }
    }

    fn id(prefix: &str, suffix: &str) -> String {
        format!("{prefix}-gc-{suffix}")
    }

    async fn setup(pool: &PgPool, suffix: &str) -> TestFixture {
        teardown_by_suffix(pool, suffix).await;

        let controller_id = id("ctrl", suffix);
        let node_name = id("node", suffix);
        let backend_id = id("bk", suffix);
        let tenant_id = id("tenant", suffix);
        let sandbox_id = id("sb", suffix);

        sqlx::query(
            "INSERT INTO controller (id, last_heartbeat, is_online, plane_version, plane_hash, ip)
             VALUES ($1, now(), true, '0.0.0', 'test', '127.0.0.1')",
        )
        .bind(&controller_id)
        .execute(pool)
        .await
        .expect("insert controller");

        sqlx::query(
            "INSERT INTO node (kind, name, plane_version, plane_hash, ip, controller)
             VALUES ('drone', $1, '0.0.0', 'test', '127.0.0.1', $2)",
        )
        .bind(&node_name)
        .bind(&controller_id)
        .execute(pool)
        .await
        .expect("insert node");

        let drone_id: i32 = sqlx::query_scalar("SELECT id FROM node WHERE name = $1")
            .bind(&node_name)
            .fetch_one(pool)
            .await
            .expect("get node id");

        sqlx::query(
            "INSERT INTO drone (id, ready, draining, pool)
             VALUES ($1, true, false, 'default')",
        )
        .bind(drone_id)
        .execute(pool)
        .await
        .expect("insert drone");

        sqlx::query(
            "INSERT INTO backend (id, cluster, last_status, last_status_time, drone_id,
                                  last_keepalive, state)
             VALUES ($1, 'default', 'running', now(), $2, now(), '{}'::jsonb)",
        )
        .bind(&backend_id)
        .bind(drone_id)
        .execute(pool)
        .await
        .expect("insert backend");

        sqlx::query("INSERT INTO tenant (id) VALUES ($1)")
            .bind(&tenant_id)
            .execute(pool)
            .await
            .expect("insert tenant");

        sqlx::query(
            "INSERT INTO sandbox (id, backend_id, tenant, runtime, trust_tier, arch, image,
                                  vcpus, memory_mb, expires_at)
             VALUES ($1, $2, $3, 'node24', 'untrusted', 'amd64', 'node:24', 2, 4096,
                     now() + interval '1 hour')",
        )
        .bind(&sandbox_id)
        .bind(&backend_id)
        .bind(&tenant_id)
        .execute(pool)
        .await
        .expect("insert sandbox");

        TestFixture {
            suffix: suffix.to_string(),
            drone_id,
        }
    }

    // Intentionally ignores errors: teardown is best-effort; rows may not exist
    async fn teardown_by_suffix(pool: &PgPool, suffix: &str) {
        let backend_id = id("bk", suffix);
        let sandbox_id = id("sb", suffix);
        let tenant_id = id("tenant", suffix);
        let node_name = id("node", suffix);
        let controller_id = id("ctrl", suffix);

        sqlx::query("DELETE FROM extension_action WHERE backend_id = $1")
            .bind(&backend_id)
            .execute(pool)
            .await
            .ok();
        sqlx::query("DELETE FROM credential_binding WHERE sandbox_id = $1")
            .bind(&sandbox_id)
            .execute(pool)
            .await
            .ok();
        sqlx::query("DELETE FROM snapshot WHERE sandbox_id = $1")
            .bind(&sandbox_id)
            .execute(pool)
            .await
            .ok();
        sqlx::query("DELETE FROM secret WHERE tenant = $1")
            .bind(&tenant_id)
            .execute(pool)
            .await
            .ok();
        sqlx::query("DELETE FROM sandbox WHERE id = $1")
            .bind(&sandbox_id)
            .execute(pool)
            .await
            .ok();
        sqlx::query("DELETE FROM backend WHERE id = $1")
            .bind(&backend_id)
            .execute(pool)
            .await
            .ok();
        let did: Option<i32> = sqlx::query_scalar("SELECT id FROM node WHERE name = $1")
            .bind(&node_name)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
        if let Some(did) = did {
            sqlx::query("DELETE FROM drone WHERE id = $1")
                .bind(did)
                .execute(pool)
                .await
                .ok();
            sqlx::query("DELETE FROM node WHERE id = $1")
                .bind(did)
                .execute(pool)
                .await
                .ok();
        }
        sqlx::query("DELETE FROM tenant WHERE id = $1")
            .bind(&tenant_id)
            .execute(pool)
            .await
            .ok();
        sqlx::query("DELETE FROM controller WHERE id = $1")
            .bind(&controller_id)
            .execute(pool)
            .await
            .ok();
    }

    #[tokio::test]
    async fn snapshot_cleanup_deletes_expired() {
        let pool = match test_pool().await {
            Some(p) => p,
            None => return,
        };
        let f = setup(&pool, "snap-exp").await;

        sqlx::query(
            "INSERT INTO snapshot (id, sandbox_id, tenant, image_ref, digest, base_image,
                                   size_bytes, arch, runtime, expires_at)
             VALUES ($1, $2, $3, 'img:snap', '', 'node:24', 0, 'amd64', 'node24',
                     now() - interval '1 hour')",
        )
        .bind(format!("snap-old-{}", f.suffix))
        .bind(&f.sandbox_id())
        .bind(&f.tenant_id())
        .execute(&pool)
        .await
        .expect("insert expired snapshot");

        sqlx::query(
            "INSERT INTO snapshot (id, sandbox_id, tenant, image_ref, digest, base_image,
                                   size_bytes, arch, runtime, expires_at)
             VALUES ($1, $2, $3, 'img:snap2', '', 'node:24', 0, 'amd64', 'node24',
                     now() + interval '1 hour')",
        )
        .bind(format!("snap-fresh-{}", f.suffix))
        .bind(&f.sandbox_id())
        .bind(&f.tenant_id())
        .execute(&pool)
        .await
        .expect("insert fresh snapshot");

        let db = PlaneDatabase::new(pool.clone());
        let deleted = db.snapshot().cleanup_expired(100).await.expect("cleanup");
        assert!(deleted >= 1);

        let remaining: i64 =
            sqlx::query_scalar("SELECT count(*) FROM snapshot WHERE sandbox_id = $1")
                .bind(&f.sandbox_id())
                .fetch_one(&pool)
                .await
                .expect("count");
        assert_eq!(remaining, 1);

        teardown_by_suffix(&pool, &f.suffix).await;
    }

    #[tokio::test]
    async fn snapshot_cleanup_respects_batch_size() {
        let pool = match test_pool().await {
            Some(p) => p,
            None => return,
        };
        let f = setup(&pool, "snap-bat").await;

        for i in 0..5 {
            sqlx::query(
                "INSERT INTO snapshot (id, sandbox_id, tenant, image_ref, digest, base_image,
                                       size_bytes, arch, runtime, expires_at)
                 VALUES ($1, $2, $3, 'img:snap', '', 'node:24', 0, 'amd64', 'node24',
                         now() - interval '1 hour')",
            )
            .bind(format!("snap-bat-{i}-{}", f.suffix))
            .bind(&f.sandbox_id())
            .bind(&f.tenant_id())
            .execute(&pool)
            .await
            .expect("insert snapshot");
        }

        let db = PlaneDatabase::new(pool.clone());
        let deleted = db.snapshot().cleanup_expired(2).await.expect("cleanup");
        assert!(deleted >= 2);

        let remaining: i64 =
            sqlx::query_scalar("SELECT count(*) FROM snapshot WHERE sandbox_id = $1")
                .bind(&f.sandbox_id())
                .fetch_one(&pool)
                .await
                .expect("count");
        assert!(remaining <= 3);

        teardown_by_suffix(&pool, &f.suffix).await;
    }

    #[tokio::test]
    async fn secret_cleanup_deletes_old_soft_deleted() {
        let pool = match test_pool().await {
            Some(p) => p,
            None => return,
        };
        let f = setup(&pool, "sec-del").await;

        sqlx::query(
            "INSERT INTO secret (id, tenant, kms_key_id, wrapped_data_key, nonce,
                                 ciphertext, auth_tag, deleted_at)
             VALUES ($1, $2, 'kms-1', '\\x00', '\\x00', '\\x00', '\\x00',
                     now() - interval '31 days')",
        )
        .bind(format!("sec-old-{}", f.suffix))
        .bind(&f.tenant_id())
        .execute(&pool)
        .await
        .expect("insert old soft-deleted secret");

        sqlx::query(
            "INSERT INTO secret (id, tenant, kms_key_id, wrapped_data_key, nonce,
                                 ciphertext, auth_tag, deleted_at)
             VALUES ($1, $2, 'kms-1', '\\x00', '\\x00', '\\x00', '\\x00',
                     now() - interval '1 day')",
        )
        .bind(format!("sec-recent-{}", f.suffix))
        .bind(&f.tenant_id())
        .execute(&pool)
        .await
        .expect("insert recent soft-deleted secret");

        sqlx::query(
            "INSERT INTO secret (id, tenant, kms_key_id, wrapped_data_key, nonce,
                                 ciphertext, auth_tag)
             VALUES ($1, $2, 'kms-1', '\\x00', '\\x00', '\\x00', '\\x00')",
        )
        .bind(format!("sec-active-{}", f.suffix))
        .bind(&f.tenant_id())
        .execute(&pool)
        .await
        .expect("insert active secret");

        let db = PlaneDatabase::new(pool.clone());
        db.secret()
            .cleanup_soft_deleted(30, 100)
            .await
            .expect("cleanup");

        let old_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM secret WHERE id = $1)")
                .bind(format!("sec-old-{}", f.suffix))
                .fetch_one(&pool)
                .await
                .expect("check old");
        assert!(!old_exists, "old soft-deleted secret should be cleaned up");

        let recent_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM secret WHERE id = $1)")
                .bind(format!("sec-recent-{}", f.suffix))
                .fetch_one(&pool)
                .await
                .expect("check recent");
        assert!(recent_exists, "recently deleted secret should survive");

        let active_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM secret WHERE id = $1)")
                .bind(format!("sec-active-{}", f.suffix))
                .fetch_one(&pool)
                .await
                .expect("check active");
        assert!(active_exists, "active secret should survive");

        teardown_by_suffix(&pool, &f.suffix).await;
    }

    #[tokio::test]
    async fn secret_cleanup_skips_with_active_bindings() {
        let pool = match test_pool().await {
            Some(p) => p,
            None => return,
        };
        let f = setup(&pool, "sec-bnd").await;

        let secret_id = format!("sec-bound-{}", f.suffix);
        sqlx::query(
            "INSERT INTO secret (id, tenant, kms_key_id, wrapped_data_key, nonce,
                                 ciphertext, auth_tag, deleted_at)
             VALUES ($1, $2, 'kms-1', '\\x00', '\\x00', '\\x00', '\\x00',
                     now() - interval '60 days')",
        )
        .bind(&secret_id)
        .bind(&f.tenant_id())
        .execute(&pool)
        .await
        .expect("insert bound secret");

        sqlx::query(
            "INSERT INTO credential_binding (id, sandbox_id, secret_id, destination,
                                             header_name, header_format)
             VALUES ($1, $2, $3, 'api.example.com', 'Authorization', '{value}')",
        )
        .bind(format!("cb-{}", f.suffix))
        .bind(&f.sandbox_id())
        .bind(&secret_id)
        .execute(&pool)
        .await
        .expect("insert credential binding");

        let db = PlaneDatabase::new(pool.clone());
        let deleted = db
            .secret()
            .cleanup_soft_deleted(30, 100)
            .await
            .expect("cleanup");

        let still_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM secret WHERE id = $1)")
                .bind(&secret_id)
                .fetch_one(&pool)
                .await
                .expect("check");
        assert!(
            still_exists,
            "secret with active binding should not be deleted (got deleted={deleted})"
        );

        teardown_by_suffix(&pool, &f.suffix).await;
    }

    #[tokio::test]
    async fn extension_action_cleanup_deletes_old_acked() {
        let pool = match test_pool().await {
            Some(p) => p,
            None => return,
        };
        let f = setup(&pool, "xak-del").await;

        sqlx::query(
            "INSERT INTO extension_action (id, kind, backend_id, sandbox_id, drone_id, payload,
                                           created_at, acked_at)
             VALUES ($1, 'policy', $2, $3, $4, '{}'::jsonb,
                     now() - interval '3 days', now() - interval '2 days')",
        )
        .bind(format!("xak-old-{}", f.suffix))
        .bind(&f.backend_id())
        .bind(&f.sandbox_id())
        .bind(f.drone_id)
        .execute(&pool)
        .await
        .expect("insert old acked action");

        sqlx::query(
            "INSERT INTO extension_action (id, kind, backend_id, sandbox_id, drone_id, payload,
                                           created_at, acked_at)
             VALUES ($1, 'policy', $2, $3, $4, '{}'::jsonb,
                     now() - interval '1 hour', now() - interval '30 minutes')",
        )
        .bind(format!("xak-recent-{}", f.suffix))
        .bind(&f.backend_id())
        .bind(&f.sandbox_id())
        .bind(f.drone_id)
        .execute(&pool)
        .await
        .expect("insert recently acked action");

        sqlx::query(
            "INSERT INTO extension_action (id, kind, backend_id, sandbox_id, drone_id, payload)
             VALUES ($1, 'policy', $2, $3, $4, '{}'::jsonb)",
        )
        .bind(format!("xak-pending-{}", f.suffix))
        .bind(&f.backend_id())
        .bind(&f.sandbox_id())
        .bind(f.drone_id)
        .execute(&pool)
        .await
        .expect("insert pending action");

        let db = PlaneDatabase::new(pool.clone());
        let deleted = db
            .extension_action()
            .cleanup_acked(1, 100)
            .await
            .expect("cleanup");
        assert!(deleted >= 1);

        let old_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM extension_action WHERE id = $1)")
                .bind(format!("xak-old-{}", f.suffix))
                .fetch_one(&pool)
                .await
                .expect("check old");
        assert!(!old_exists);

        let recent_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM extension_action WHERE id = $1)")
                .bind(format!("xak-recent-{}", f.suffix))
                .fetch_one(&pool)
                .await
                .expect("check recent");
        assert!(recent_exists);

        let pending_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM extension_action WHERE id = $1)")
                .bind(format!("xak-pending-{}", f.suffix))
                .fetch_one(&pool)
                .await
                .expect("check pending");
        assert!(pending_exists);

        teardown_by_suffix(&pool, &f.suffix).await;
    }

    #[tokio::test]
    async fn run_extension_cleanup_integration() {
        let pool = match test_pool().await {
            Some(p) => p,
            None => return,
        };
        let f = setup(&pool, "full-gc").await;

        sqlx::query(
            "INSERT INTO snapshot (id, sandbox_id, tenant, image_ref, digest, base_image,
                                   size_bytes, arch, runtime, expires_at)
             VALUES ($1, $2, $3, 'img:snap', '', 'node:24', 0, 'amd64', 'node24',
                     now() - interval '1 hour')",
        )
        .bind(format!("snap-gc-{}", f.suffix))
        .bind(&f.sandbox_id())
        .bind(&f.tenant_id())
        .execute(&pool)
        .await
        .expect("insert expired snapshot");

        sqlx::query(
            "INSERT INTO secret (id, tenant, kms_key_id, wrapped_data_key, nonce,
                                 ciphertext, auth_tag, deleted_at)
             VALUES ($1, $2, 'kms-1', '\\x00', '\\x00', '\\x00', '\\x00',
                     now() - interval '60 days')",
        )
        .bind(format!("sec-gc-{}", f.suffix))
        .bind(&f.tenant_id())
        .execute(&pool)
        .await
        .expect("insert old secret");

        sqlx::query(
            "INSERT INTO extension_action (id, kind, backend_id, sandbox_id, drone_id, payload,
                                           acked_at)
             VALUES ($1, 'policy', $2, $3, $4, '{}'::jsonb, now() - interval '2 days')",
        )
        .bind(format!("xak-gc-{}", f.suffix))
        .bind(&f.backend_id())
        .bind(&f.sandbox_id())
        .bind(f.drone_id)
        .execute(&pool)
        .await
        .expect("insert old acked action");

        let db = PlaneDatabase::new(pool.clone());
        super::run_extension_cleanup(&db, 100)
            .await
            .expect("run_extension_cleanup");

        let snap_gone: bool =
            sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM snapshot WHERE id = $1)")
                .bind(format!("snap-gc-{}", f.suffix))
                .fetch_one(&pool)
                .await
                .expect("check");
        assert!(snap_gone, "expired snapshot should be cleaned up");

        let sec_gone: bool =
            sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM secret WHERE id = $1)")
                .bind(format!("sec-gc-{}", f.suffix))
                .fetch_one(&pool)
                .await
                .expect("check");
        assert!(sec_gone, "old soft-deleted secret should be cleaned up");

        let act_gone: bool =
            sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM extension_action WHERE id = $1)")
                .bind(format!("xak-gc-{}", f.suffix))
                .fetch_one(&pool)
                .await
                .expect("check");
        assert!(act_gone, "old acked action should be cleaned up");

        teardown_by_suffix(&pool, &f.suffix).await;
    }
}
