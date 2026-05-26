use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, KeepAlive},
    response::Sse,
    Json,
};
use chamuka_common::ext::api::{
    CreateCredentialBindingRequest, CreateSandboxRequest, CreateSandboxResponse,
    CreateSecretRequest, CreateSecretResponse, CredentialBinding, ExtendTimeoutRequest,
    GetSandboxResponse, ListSandboxesResponse, SandboxConnection, SnapshotRequest,
    SnapshotResponse, UsageRecord, UsageResponse, UsageSummary,
};
use chamuka_common::ext::network::NetworkPolicy;
use chamuka_common::ext::sandbox::{Arch, Sandbox, SandboxResources, TrustLevel};
use chamuka_common::ext::snapshot::Snapshot;
use futures_util::StreamExt;
use plane_common::types::{
    backend_state::BackendStatusStreamEntry, BackendStatus, ConnectRequest, KeyConfig, SpawnConfig,
};
use serde::Deserialize;
use serde_json::json;
use std::convert::Infallible;

use crate::controller::core::Controller;
use crate::database::sandbox::SandboxRow;

use super::{
    error::ApiError,
    types::{ExtensionActionKind, SandboxId},
};

pub(crate) fn resolve_image(
    runtime: &str,
    explicit_image: Option<&str>,
    registry: Option<&str>,
) -> Result<String, ApiError> {
    super::runtime_catalog::resolve_runtime_image(runtime, explicit_image, registry)
}

pub(crate) fn trust_to_pool(trust: &TrustLevel) -> &'static str {
    match trust {
        TrustLevel::Trusted => "trusted",
        TrustLevel::Untrusted => "untrusted",
        TrustLevel::Paranoid => "paranoid",
    }
}

pub(crate) fn extract_tenant(headers: &HeaderMap) -> Result<String, ApiError> {
    headers
        .get("x-auth-tenant")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .ok_or_else(|| ApiError::unauthenticated("missing or invalid X-Auth-Tenant header"))
}

/// Return the row's backend_id, or a 410 Gone error if the backend has
/// been reaped past retention. Use this in any endpoint that needs to
/// dispatch a message to the sandbox's drone or query Plane backend state.
fn require_backend_id(row: &SandboxRow) -> Result<&str, ApiError> {
    row.backend_id
        .as_deref()
        .ok_or_else(|| ApiError::gone("Sandbox backend has been reaped; operation not available"))
}

async fn get_sandbox_row(
    controller: &Controller,
    id: &str,
    tenant: &str,
) -> Result<SandboxRow, ApiError> {
    controller
        .db
        .sandbox()
        .get_by_id(id, tenant)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to query sandbox");
            ApiError::internal("Failed to query sandbox")
        })?
        .ok_or_else(|| ApiError::not_found("Sandbox not found"))
}

pub async fn create_sandbox(
    State(controller): State<Controller>,
    headers: HeaderMap,
    Json(request): Json<CreateSandboxRequest>,
) -> Result<(StatusCode, Json<CreateSandboxResponse>), ApiError> {
    let tenant = extract_tenant(&headers)?;
    let image = resolve_image(
        &request.runtime,
        request.image.as_deref(),
        controller.sandbox_runtime_registry.as_deref(),
    )?;

    let pool = trust_to_pool(&request.trust);
    let resources = &request.resources;

    let key_config = request
        .key
        .as_ref()
        .map_or_else(KeyConfig::new_random, |k| KeyConfig {
            name: k.name.clone(),
            namespace: k.namespace.clone().unwrap_or_default(),
            tag: k.tag.clone().unwrap_or_default(),
        });

    let env = super::runtime_catalog::merge_runtime_env(&request.runtime, &request.env);

    let executable = json!({
        "image": image,
        "env": env,
        "resource_limits": {
            "memory_limit_bytes": i64::from(resources.memory_mb()) * 1024 * 1024
        }
    });

    let timeout_ms = request.timeout_ms.unwrap_or(300_000);
    let lifetime_limit_seconds = Some(timeout_ms.div_ceil(1000) as i32);

    let spawn_config = SpawnConfig {
        id: None,
        cluster: None,
        pool: pool.into(),
        executable,
        lifetime_limit_seconds,
        max_idle_seconds: None,
        use_static_token: request.use_static_token,
        subdomain: None,
    };

    let connect_request = ConnectRequest {
        key: Some(key_config),
        spawn_config: Some(spawn_config),
        user: None,
        auth: serde_json::Map::new(),
    };

    controller
        .db
        .tenant()
        .ensure_exists(&tenant)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to ensure tenant exists");
            ApiError::internal("Failed to initialize tenant")
        })?;

    let connect_response = controller
        .connect(&connect_request)
        .await
        .map_err(ApiError::from)?;

    let sandbox_id = SandboxId::new();
    let now = chrono::Utc::now();
    let expires_at = now + chrono::Duration::milliseconds(timeout_ms as i64);

    let cluster = controller
        .default_cluster
        .as_ref()
        .map(|c| c.to_string())
        .unwrap_or_default();

    let trust_str = pool;
    let arch_str = request.arch.to_string();
    let ports_i32: Vec<i32> = request.ports.iter().map(|&p| i32::from(p)).collect();
    let tags_json = serde_json::to_value(&request.tags).unwrap_or_default();

    controller
        .db
        .sandbox()
        .insert(
            sandbox_id.as_ref(),
            &connect_response.backend_id.to_string(),
            &tenant,
            &request.runtime,
            trust_str,
            &arch_str,
            &image,
            resources.vcpus as i32,
            resources.memory_mb() as i32,
            &ports_i32,
            &tags_json,
            request.persistent,
            None,
            expires_at,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to insert sandbox row");
            ApiError::internal("Failed to persist sandbox")
        })?;

    let sandbox = Sandbox {
        id: sandbox_id.to_string(),
        backend_id: Some(connect_response.backend_id.to_string()),
        tenant,
        status: BackendStatus::Scheduled,
        runtime: request.runtime.clone(),
        trust: request.trust,
        arch: request.arch,
        image: image.clone(),
        resources: SandboxResources {
            vcpus: resources.vcpus,
            memory_mb: resources.memory_mb(),
        },
        ports: request.ports.clone(),
        tags: request.tags.clone(),
        persistent: request.persistent,
        snapshot_source: None,
        created_at: now,
        updated_at: now,
        expires_at,
        ready_at: None,
        terminated_at: None,
        drone_name: connect_response.drone.map(|d| d.to_string()),
        cluster,
    };

    let connection = SandboxConnection {
        token: connect_response.token.to_string(),
        url: connect_response.url.clone(),
        secret_token: connect_response.secret_token.map(|t| t.to_string()),
        status_url: connect_response.status_url.clone(),
    };

    let response = CreateSandboxResponse {
        sandbox,
        connection,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_sandbox(
    State(controller): State<Controller>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<GetSandboxResponse>, ApiError> {
    let tenant = extract_tenant(&headers)?;
    let row = get_sandbox_row(&controller, &id, &tenant).await?;
    let backend_id = row.backend_id.clone();
    let token = row.bearer_token.clone();
    let now = chrono::Utc::now();
    let sandbox = sandbox_from_row(row, &controller, now);

    let connection = token.map(|token| {
        let cluster = controller
            .default_cluster
            .as_ref()
            .map(|c| c.to_string())
            .unwrap_or_default();
        let url = format!("http://{}/{}/", cluster, token);
        let status_url = backend_id
            .clone()
            .and_then(|b| plane_common::names::BackendName::try_from(b).ok())
            .map(|bn| controller.client.backend_status_url(&bn).to_string())
            .unwrap_or_default();
        SandboxConnection {
            token,
            url,
            secret_token: None,
            status_url,
        }
    });

    Ok(Json(GetSandboxResponse {
        sandbox,
        connection,
    }))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    pub cursor: Option<String>,
}

fn default_limit() -> i64 {
    100
}

pub async fn list_sandboxes(
    State(controller): State<Controller>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Result<Json<ListSandboxesResponse>, ApiError> {
    let tenant = extract_tenant(&headers)?;

    let limit = query.limit.clamp(1, 500);
    let cursor = query
        .cursor
        .and_then(|c| c.parse::<chrono::DateTime<chrono::Utc>>().ok());

    let rows = controller
        .db
        .sandbox()
        .list_by_tenant(&tenant, limit + 1, cursor)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to list sandboxes");
            ApiError::internal("Failed to list sandboxes")
        })?;

    let now = chrono::Utc::now();
    let has_more = rows.len() as i64 > limit;
    let items: Vec<Sandbox> = rows
        .into_iter()
        .take(limit as usize)
        .map(|r| sandbox_from_row(r, &controller, now))
        .collect();

    let next_cursor = if has_more {
        items.last().map(|s| s.created_at.to_rfc3339())
    } else {
        None
    };

    Ok(Json(ListSandboxesResponse { items, next_cursor }))
}

pub async fn delete_sandbox(
    State(controller): State<Controller>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<DeleteQuery>,
) -> Result<StatusCode, ApiError> {
    let tenant = extract_tenant(&headers)?;
    let row = get_sandbox_row(&controller, &id, &tenant).await?;

    let backend_id_str = require_backend_id(&row)?.to_string();
    let backend_id: plane_common::names::BackendName = backend_id_str
        .try_into()
        .map_err(|_| ApiError::internal("Invalid backend_id stored in sandbox row"))?;

    let result = if query.force.unwrap_or(false) {
        controller.client.hard_terminate(&backend_id).await
    } else {
        controller.client.soft_terminate(&backend_id).await
    };

    result.map_err(|e| {
        tracing::error!(error = ?e, "Failed to terminate backend");
        ApiError::internal("Failed to terminate sandbox")
    })?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct DeleteQuery {
    pub force: Option<bool>,
}

pub async fn extend_sandbox(
    State(controller): State<Controller>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<ExtendTimeoutRequest>,
) -> Result<Json<Sandbox>, ApiError> {
    let tenant = extract_tenant(&headers)?;

    // Reject extension on a reaped-backend sandbox. Without this guard
    // expires_at keeps moving forward and cleanup_terminated never reclaims
    // the orphaned row.
    let existing = get_sandbox_row(&controller, &id, &tenant).await?;
    require_backend_id(&existing)?;

    let additional = chrono::Duration::milliseconds(request.additional_ms as i64);
    let row = controller
        .db
        .sandbox()
        .extend_expires_at(&id, &tenant, additional)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to extend sandbox");
            ApiError::internal("Failed to extend sandbox")
        })?
        .ok_or_else(|| ApiError::not_found("Sandbox not found"))?;

    Ok(Json(sandbox_from_row(row, &controller, chrono::Utc::now())))
}

#[derive(Debug, Deserialize)]
pub struct StatusQuery {
    #[serde(default = "default_stream")]
    pub stream: bool,
}

fn default_stream() -> bool {
    true
}

pub async fn sandbox_status(
    State(controller): State<Controller>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<StatusQuery>,
) -> Result<axum::response::Response, ApiError> {
    use axum::response::IntoResponse;

    let tenant = extract_tenant(&headers)?;
    let row = get_sandbox_row(&controller, &id, &tenant).await?;

    let backend_id_str = require_backend_id(&row)?.to_string();
    let backend_id: plane_common::names::BackendName = backend_id_str
        .try_into()
        .map_err(|_| ApiError::internal("Invalid backend_id stored in sandbox row"))?;

    if !query.stream {
        let entry = controller
            .db
            .backend()
            .backend(&backend_id)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to query backend status");
                ApiError::internal("Failed to query backend status")
            })?
            .map(|row| BackendStatusStreamEntry::from_state(row.state, row.last_status_time))
            .ok_or_else(|| ApiError::not_found("Backend not found"))?;

        return Ok(Json(entry).into_response());
    }

    let last_status: Option<BackendStatus> = headers
        .get("Last-Event-ID")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| BackendStatus::try_from(v.to_owned()).ok());

    let mut st = Box::pin(
        controller
            .db
            .backend()
            .status_stream(&backend_id)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to open status stream");
                ApiError::internal("Failed to open status stream")
            })?,
    );

    let stream = async_stream::stream! {
        while let Some(status) = st.next().await {
            if let Some(last) = last_status {
                if status.status <= last {
                    continue;
                }
            }
            let id = status.status.to_string();
            let event: Result<Event, Infallible> = Ok(Event::default()
                .json_data(&status)
                .expect("always serializable")
                .id(id));
            yield event;
        }
    };

    Ok(Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(15)))
        .into_response())
}

pub async fn snapshot_sandbox(
    State(controller): State<Controller>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<SnapshotRequest>,
) -> Result<(StatusCode, Json<SnapshotResponse>), ApiError> {
    let tenant = extract_tenant(&headers)?;
    let row = get_sandbox_row(&controller, &id, &tenant).await?;
    let backend_id = require_backend_id(&row)?.to_string();

    let snapshot_id = format!("snap-{}", plane_common::util::random_string());
    let action_id = super::types::ExtensionActionId::new();
    let now = chrono::Utc::now();
    let ttl_days = request.ttl_days.unwrap_or(7);
    let expires_at = now + chrono::Duration::days(i64::from(ttl_days));

    controller
        .db
        .snapshot()
        .insert(
            &snapshot_id,
            &row.id,
            &tenant,
            &row.image,
            &row.arch,
            &row.runtime,
            request.name.as_deref(),
            &request.tags,
            expires_at,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to insert snapshot");
            ApiError::internal("Failed to create snapshot")
        })?;

    let wire = super::dispatch::build_snapshot_command(
        action_id.as_ref(),
        &backend_id,
        request.ttl_days,
        request.name.as_deref(),
    );
    let payload = serde_json::to_value(&wire).unwrap_or_default();

    enqueue_extension_action(
        &controller,
        ExtensionActionKind::Snapshot,
        &backend_id,
        &row.id,
        &payload,
    )
    .await;

    let snapshot = Snapshot {
        id: snapshot_id,
        sandbox_id: Some(row.id),
        tenant,
        image_ref: format!("{}:snapshot", row.image),
        digest: String::new(),
        base_image: row.image,
        size_bytes: 0,
        arch: row.arch.parse().unwrap_or(Arch::Amd64),
        runtime: row.runtime,
        name: request.name,
        tags: request.tags,
        created_at: now,
        expires_at,
    };

    Ok((StatusCode::CREATED, Json(SnapshotResponse { snapshot })))
}

pub async fn update_network_policy(
    State(controller): State<Controller>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(policy): Json<NetworkPolicy>,
) -> Result<Json<NetworkPolicy>, ApiError> {
    let tenant = extract_tenant(&headers)?;
    let row = get_sandbox_row(&controller, &id, &tenant).await?;
    let backend_id = require_backend_id(&row)?.to_string();

    let policy_json = serde_json::to_value(&policy).unwrap_or_default();
    let updated = controller
        .db
        .sandbox()
        .update_network_policy(&id, &tenant, &policy_json)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to update network policy");
            ApiError::internal("Failed to update network policy")
        })?;

    if !updated {
        return Err(ApiError::not_found("Sandbox not found"));
    }

    enqueue_extension_action(
        &controller,
        ExtensionActionKind::PushPolicy,
        &backend_id,
        &row.id,
        &policy_json,
    )
    .await;

    Ok(Json(policy))
}

pub async fn create_credential_binding(
    State(controller): State<Controller>,
    headers: HeaderMap,
    Path(sandbox_id): Path<String>,
    Json(request): Json<CreateCredentialBindingRequest>,
) -> Result<(StatusCode, Json<CredentialBinding>), ApiError> {
    let tenant = extract_tenant(&headers)?;
    let row = get_sandbox_row(&controller, &sandbox_id, &tenant).await?;
    let backend_id = require_backend_id(&row)?.to_string();

    // Reject binding to a missing or soft-deleted secret up-front. Without this,
    // the FK only checks row existence and a tombstoned secret would silently
    // resolve to an empty value at dispatch time.
    let secret_active = controller
        .db
        .secret()
        .exists_active(&request.secret_id, &tenant)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to check secret state");
            ApiError::internal("Failed to validate secret")
        })?;
    if !secret_active {
        return Err(ApiError::not_found("Secret not found"));
    }

    let cred_id = format!("cred-{}", plane_common::util::random_string());
    let now = chrono::Utc::now();

    controller
        .db
        .credential()
        .insert(
            &cred_id,
            &sandbox_id,
            &request.secret_id,
            &request.destination,
            &request.header_name,
            request.header_format.as_deref(),
        )
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to insert credential binding");
            ApiError::internal("Failed to create credential binding")
        })?;

    let payload = serde_json::to_value(super::dispatch::build_push_credential(
        &backend_id,
        &cred_id,
        &request.destination,
        &request.header_name,
        request.header_format.as_deref().unwrap_or("{value}"),
        "", // secret value resolved at dispatch time
    ))
    .unwrap_or_default();

    enqueue_extension_action(
        &controller,
        ExtensionActionKind::PushCredential,
        &backend_id,
        &sandbox_id,
        &payload,
    )
    .await;

    let binding = CredentialBinding {
        id: cred_id,
        sandbox_id,
        destination: request.destination,
        header_name: request.header_name,
        header_format: request
            .header_format
            .unwrap_or_else(|| "{value}".to_owned()),
        secret_id: request.secret_id,
        created_at: now.to_rfc3339(),
    };

    Ok((StatusCode::CREATED, Json(binding)))
}

pub async fn delete_credential_binding(
    State(controller): State<Controller>,
    headers: HeaderMap,
    Path((sandbox_id, cred_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let tenant = extract_tenant(&headers)?;
    let row = get_sandbox_row(&controller, &sandbox_id, &tenant).await?;
    let backend_id = require_backend_id(&row)?.to_string();

    let deleted = controller
        .db
        .credential()
        .delete(&cred_id, &sandbox_id)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to delete credential binding");
            ApiError::internal("Failed to delete credential binding")
        })?;

    if !deleted {
        return Err(ApiError::not_found("Credential binding not found"));
    }

    let payload = serde_json::json!({"binding_id": cred_id});
    enqueue_extension_action(
        &controller,
        ExtensionActionKind::RevokeCredential,
        &backend_id,
        &sandbox_id,
        &payload,
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_snapshot(
    State(controller): State<Controller>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Snapshot>, ApiError> {
    let tenant = extract_tenant(&headers)?;

    let snapshot = controller
        .db
        .snapshot()
        .get_by_id(&id, &tenant)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to query snapshot");
            ApiError::internal("Failed to query snapshot")
        })?
        .ok_or_else(|| ApiError::not_found("Snapshot not found"))?;

    Ok(Json(snapshot))
}

pub async fn delete_snapshot(
    State(controller): State<Controller>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let tenant = extract_tenant(&headers)?;

    let deleted = controller
        .db
        .snapshot()
        .delete(&id, &tenant)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to delete snapshot");
            ApiError::internal("Failed to delete snapshot")
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("Snapshot not found"))
    }
}

pub async fn create_secret(
    State(controller): State<Controller>,
    headers: HeaderMap,
    Json(request): Json<CreateSecretRequest>,
) -> Result<(StatusCode, Json<CreateSecretResponse>), ApiError> {
    let tenant = extract_tenant(&headers)?;

    let secret_id = format!("secret-{}", plane_common::util::random_string());
    let now = chrono::Utc::now();

    controller
        .db
        .secret()
        .insert(&secret_id, &tenant, &request.value, request.name.as_deref())
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to insert secret");
            ApiError::internal("Failed to create secret")
        })?;

    let response = CreateSecretResponse {
        id: secret_id,
        name: request.name,
        created_at: now.to_rfc3339(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn delete_secret(
    State(controller): State<Controller>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let tenant = extract_tenant(&headers)?;

    let outcome = controller
        .db
        .secret()
        .delete(&id, &tenant)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to delete secret");
            ApiError::internal("Failed to delete secret")
        })?;

    match outcome {
        crate::database::secret::DeleteOutcome::Deleted => Ok(StatusCode::NO_CONTENT),
        crate::database::secret::DeleteOutcome::NotFound => {
            Err(ApiError::not_found("Secret not found"))
        }
    }
}

const COST_PER_CPU_HOUR: f64 = 0.20;
const NSEC_PER_HOUR: f64 = 3_600_000_000_000.0;
const MB_SEC_PER_GB_HOUR: f64 = 3_600.0 * 1024.0;

#[derive(Deserialize)]
pub struct UsageQuery {
    period: Option<String>,
    sandbox_id: Option<String>,
}

pub async fn get_usage(
    State(controller): State<Controller>,
    headers: HeaderMap,
    Query(query): Query<UsageQuery>,
) -> Result<Json<UsageResponse>, ApiError> {
    let tenant = extract_tenant(&headers)?;

    let hours = match query.period.as_deref() {
        Some("1h") => 1,
        Some("6h") => 6,
        Some("7d") => 168,
        Some("30d") => 720,
        _ => 24,
    };
    let since = chrono::Utc::now() - chrono::Duration::hours(hours);

    let rows = controller
        .db
        .usage_record()
        .query_by_tenant(&tenant, since, query.sandbox_id.as_deref())
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to query usage records");
            ApiError::internal("Failed to query usage")
        })?;

    let records: Vec<UsageRecord> = rows
        .into_iter()
        .map(|r| UsageRecord {
            tenant: r.tenant,
            sandbox_id: r.sandbox_id,
            bucket_start: r.bucket_start,
            cpu_nsec: r.cpu_nsec,
            memory_mb_seconds: r.memory_mb_seconds,
            rx_bytes: r.rx_bytes,
            tx_bytes: r.tx_bytes,
        })
        .collect();

    let total_cpu_nsec: i64 = records.iter().map(|r| r.cpu_nsec).sum();
    let total_mem_mb_sec: i64 = records.iter().map(|r| r.memory_mb_seconds).sum();
    let total_rx: i64 = records.iter().map(|r| r.rx_bytes).sum();
    let total_tx: i64 = records.iter().map(|r| r.tx_bytes).sum();

    let total_cpu_hours = total_cpu_nsec as f64 / NSEC_PER_HOUR;
    let total_memory_gb_hours = total_mem_mb_sec as f64 / MB_SEC_PER_GB_HOUR;

    Ok(Json(UsageResponse {
        records,
        summary: UsageSummary {
            total_cpu_hours,
            total_memory_gb_hours,
            total_rx_bytes: total_rx,
            total_tx_bytes: total_tx,
            estimated_cost: total_cpu_hours * COST_PER_CPU_HOUR,
        },
    }))
}

async fn enqueue_extension_action(
    controller: &Controller,
    kind: super::types::ExtensionActionKind,
    backend_id: &str,
    sandbox_id: &str,
    payload: &serde_json::Value,
) {
    let drone_id = match controller
        .db
        .sandbox()
        .get_drone_id_for_backend(backend_id)
        .await
    {
        Ok(Some(id)) => id,
        Ok(None) => {
            tracing::warn!(
                backend_id,
                "No drone found for backend; skipping extension action"
            );
            return;
        }
        Err(e) => {
            tracing::error!(error = ?e, "Failed to look up drone for extension action");
            return;
        }
    };

    let action_id = super::types::ExtensionActionId::new();
    let kind_str = kind.to_string();
    if let Err(e) = controller
        .db
        .extension_action()
        .insert(
            action_id.as_ref(),
            &kind_str,
            backend_id,
            sandbox_id,
            drone_id,
            payload,
        )
        .await
    {
        tracing::error!(error = ?e, "Failed to enqueue extension action");
    }
}

fn sandbox_from_row(
    row: SandboxRow,
    controller: &Controller,
    now: chrono::DateTime<chrono::Utc>,
) -> Sandbox {
    let trust = row.trust_tier.parse().unwrap_or(TrustLevel::Untrusted);
    let arch = row.arch.parse().unwrap_or(Arch::Amd64);
    let cluster = controller
        .default_cluster
        .as_ref()
        .map(|c| c.to_string())
        .unwrap_or_default();
    let tags: std::collections::HashMap<String, String> =
        serde_json::from_value(row.tags).unwrap_or_default();
    let ports: Vec<u16> = row
        .ports
        .iter()
        .filter_map(|&p| u16::try_from(p).ok())
        .collect();

    let status = row
        .live_status
        .as_deref()
        .and_then(|s| {
            use serde::de::{value::Error as DeError, IntoDeserializer};
            BackendStatus::deserialize::<serde::de::value::StrDeserializer<'_, DeError>>(
                s.into_deserializer(),
            )
            .ok()
        })
        .unwrap_or(BackendStatus::Scheduled);

    let status = if row.expires_at < now && status != BackendStatus::Terminated {
        BackendStatus::Terminated
    } else {
        status
    };

    let drone_name = row.drone_name;

    Sandbox {
        id: row.id,
        backend_id: row.backend_id,
        tenant: row.tenant,
        status,
        runtime: row.runtime,
        trust,
        arch,
        image: row.image,
        resources: SandboxResources {
            vcpus: row.vcpus as u32,
            memory_mb: row.memory_mb as u32,
        },
        ports,
        tags,
        persistent: row.persistent,
        snapshot_source: row.snapshot_source,
        created_at: row.created_at,
        updated_at: row.updated_at,
        expires_at: row.expires_at,
        ready_at: row.ready_at,
        terminated_at: row.terminated_at,
        drone_name,
        cluster,
    }
}
