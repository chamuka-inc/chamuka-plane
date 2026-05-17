use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chamuka_common::ext::api::{
    CreateSandboxRequest, CreateSandboxResponse, ListSandboxesResponse, SandboxConnection,
};
use chamuka_common::ext::sandbox::{Arch, Sandbox, SandboxResources, TrustLevel};
use plane_common::types::{BackendStatus, ConnectRequest, KeyConfig, SpawnConfig};
use serde::Deserialize;
use serde_json::json;

use crate::controller::core::Controller;
use crate::database::sandbox::SandboxRow;

use super::{error::ApiError, types::SandboxId};

const RUNTIME_IMAGES: &[(&str, &str)] = &[
    ("node22", "node:22-slim"),
    ("node24", "node:24-slim"),
    ("python313", "python:3.13-slim"),
    ("bun1", "oven/bun:1"),
    ("desktop", "sandbox-platform/desktop:latest"),
];

pub(crate) fn resolve_image(
    runtime: &str,
    explicit_image: Option<&str>,
) -> Result<String, ApiError> {
    if let Some(img) = explicit_image {
        return Ok(img.to_string());
    }
    RUNTIME_IMAGES
        .iter()
        .find(|(r, _)| *r == runtime)
        .map(|(_, img)| img.to_string())
        .ok_or_else(|| ApiError::invalid_request("image is required when runtime is 'custom'"))
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

pub async fn create_sandbox(
    State(controller): State<Controller>,
    headers: HeaderMap,
    Json(request): Json<CreateSandboxRequest>,
) -> Result<(StatusCode, Json<CreateSandboxResponse>), ApiError> {
    let tenant = extract_tenant(&headers)?;
    let image = resolve_image(&request.runtime, request.image.as_deref())?;

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

    let executable = json!({
        "image": image,
        "env": request.env,
        "resource_limits": {
            "memory_limit_bytes": i64::from(resources.memory_mb()) * 1024 * 1024
        }
    });

    let lifetime_limit_seconds = request.timeout_ms.map(|ms| ms.div_ceil(1000) as i32);

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

    let connect_response = controller
        .connect(&connect_request)
        .await
        .map_err(ApiError::from)?;

    let sandbox_id = SandboxId::new();
    let now = chrono::Utc::now();
    let timeout_ms = request.timeout_ms.unwrap_or(300_000);
    let expires_at = now + chrono::Duration::milliseconds(timeout_ms as i64);

    let cluster = controller
        .default_cluster
        .as_ref()
        .map(|c| c.to_string())
        .unwrap_or_default();

    let trust_str = pool;
    let arch_str = format!("{:?}", request.arch).to_lowercase();
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
        backend_id: connect_response.backend_id.to_string(),
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
) -> Result<Json<Sandbox>, ApiError> {
    let tenant = extract_tenant(&headers)?;

    let row = controller
        .db
        .sandbox()
        .get_by_id(&id, &tenant)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to query sandbox");
            ApiError::internal("Failed to query sandbox")
        })?
        .ok_or_else(|| ApiError::not_found("Sandbox not found"))?;

    Ok(Json(sandbox_from_row(row, &controller)))
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

    let has_more = rows.len() as i64 > limit;
    let items: Vec<Sandbox> = rows
        .into_iter()
        .take(limit as usize)
        .map(|r| sandbox_from_row(r, &controller))
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

    let row = controller
        .db
        .sandbox()
        .get_by_id(&id, &tenant)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to query sandbox");
            ApiError::internal("Failed to query sandbox")
        })?
        .ok_or_else(|| ApiError::not_found("Sandbox not found"))?;

    let backend_id: plane_common::names::BackendName = row
        .backend_id
        .clone()
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

fn sandbox_from_row(row: SandboxRow, controller: &Controller) -> Sandbox {
    let trust = match row.trust_tier.as_str() {
        "trusted" => TrustLevel::Trusted,
        "paranoid" => TrustLevel::Paranoid,
        _ => TrustLevel::Untrusted,
    };
    let arch = match row.arch.as_str() {
        "arm64" => Arch::Arm64,
        _ => Arch::Amd64,
    };
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

    Sandbox {
        id: row.id,
        backend_id: row.backend_id,
        tenant: row.tenant,
        status: BackendStatus::Scheduled,
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
        drone_name: None,
        cluster,
    }
}
