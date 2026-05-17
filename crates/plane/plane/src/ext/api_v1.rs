use axum::{extract::State, http::StatusCode, Json};
use chamuka_common::ext::api::{CreateSandboxRequest, CreateSandboxResponse, SandboxConnection};
use chamuka_common::ext::sandbox::{Sandbox, SandboxResources, TrustLevel};
use plane_common::types::{BackendStatus, ConnectRequest, KeyConfig, SpawnConfig};
use serde_json::json;

use crate::controller::core::Controller;

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

pub async fn create_sandbox(
    State(controller): State<Controller>,
    Json(request): Json<CreateSandboxRequest>,
) -> Result<(StatusCode, Json<CreateSandboxResponse>), ApiError> {
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

    let sandbox = Sandbox {
        id: sandbox_id.to_string(),
        backend_id: connect_response.backend_id.to_string(),
        tenant: "default".to_string(),
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
