#![allow(clippy::expect_used)]

use super::types::*;
use chamuka_common::ext::api::{
    CreateCredentialBindingRequest, CreateSecretRequest, ExtendTimeoutRequest, SnapshotRequest,
};
use std::path::Path;

#[test]
fn sandbox_id_has_prefix() {
    let id = SandboxId::new();
    assert!(
        id.0.starts_with("sb-"),
        "SandboxId should start with sb- prefix, got: {}",
        id.0
    );
}

#[test]
fn resources_spec_memory_calculation() {
    let spec = ResourcesSpec { vcpus: 2 };
    assert_eq!(spec.memory_mb(), 4096);

    let spec = ResourcesSpec { vcpus: 4 };
    assert_eq!(spec.memory_mb(), 8192);
}

#[test]
fn trust_to_pool_mapping() {
    use super::api_v1::trust_to_pool;
    assert_eq!(trust_to_pool(&TrustLevel::Trusted), "trusted");
    assert_eq!(trust_to_pool(&TrustLevel::Untrusted), "untrusted");
    assert_eq!(trust_to_pool(&TrustLevel::Paranoid), "paranoid");
}

#[test]
fn resolve_image_known_runtime() {
    use super::api_v1::resolve_image;
    assert_eq!(
        resolve_image("node22", None).expect("resolve"),
        "node:22-slim"
    );
    assert_eq!(
        resolve_image("python313", None).expect("resolve"),
        "python:3.13-slim"
    );
    assert_eq!(resolve_image("bun1", None).expect("resolve"), "oven/bun:1");
}

#[test]
fn resolve_image_explicit_overrides_runtime() {
    use super::api_v1::resolve_image;
    assert_eq!(
        resolve_image("node22", Some("my-org/custom:v1")).expect("resolve"),
        "my-org/custom:v1"
    );
}

#[test]
fn resolve_image_custom_without_explicit_fails() {
    use super::api_v1::resolve_image;
    let result = resolve_image("custom", None);
    assert!(result.is_err());
}

#[test]
fn create_request_deserialization_minimal() {
    let json = r#"{"runtime": "node22"}"#;
    let req: CreateSandboxRequest = serde_json::from_str(json).expect("deserialize");
    assert_eq!(req.runtime, "node22");
    assert_eq!(req.trust, TrustLevel::Untrusted);
    assert_eq!(req.resources.vcpus, 2);
    assert_eq!(req.resources.memory_mb(), 4096);
}

#[test]
fn create_request_deserialization_full() {
    let json = r#"{
        "runtime": "custom",
        "image": "my-org/my-image:latest",
        "trust": "paranoid",
        "arch": "arm64",
        "resources": {"vcpus": 4},
        "timeout_ms": 600000,
        "ports": [3000, 8080],
        "env": {"NODE_ENV": "production"},
        "tags": {"team": "platform"},
        "persistent": true,
        "use_static_token": true,
        "key": {"name": "my-key", "namespace": "ns", "tag": "v1"}
    }"#;
    let req: CreateSandboxRequest = serde_json::from_str(json).expect("deserialize");
    assert_eq!(req.runtime, "custom");
    assert_eq!(req.trust, TrustLevel::Paranoid);
    assert_eq!(req.arch, Arch::Arm64);
    assert_eq!(req.resources.vcpus, 4);
    assert_eq!(req.resources.memory_mb(), 8192);
    assert_eq!(req.image, Some("my-org/my-image:latest".to_string()));
    assert_eq!(req.timeout_ms, Some(600000));
    assert_eq!(req.ports, vec![3000, 8080]);
    assert!(req.persistent);
    assert!(req.use_static_token);
    let key = req.key.expect("key");
    assert_eq!(key.name, "my-key");
}

#[test]
fn error_response_from_connect_error_no_drone() {
    use super::error::ApiError;
    use crate::database::connect::ConnectError;

    let err = ConnectError::NoDroneAvailable;
    let api_err = ApiError::from(err);
    assert_eq!(api_err.code, "NO_DRONE_AVAILABLE");
    assert_eq!(api_err.status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
}

#[test]
fn error_response_from_connect_error_key_held() {
    use super::error::ApiError;
    use crate::database::connect::ConnectError;

    let err = ConnectError::KeyHeld {
        request_tag: "v1".to_string(),
        key_tag: "v2".to_string(),
    };
    let api_err = ApiError::from(err);
    assert_eq!(api_err.code, "KEY_HELD");
    assert_eq!(api_err.status, axum::http::StatusCode::CONFLICT);
}

#[test]
fn error_response_from_connect_error_database() {
    use super::error::ApiError;
    use crate::database::connect::ConnectError;

    let err = ConnectError::Other("something broke".to_string());
    let api_err = ApiError::from(err);
    assert_eq!(api_err.code, "INTERNAL");
    assert_eq!(
        api_err.status,
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    );
}

fn fixtures_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/plane")
        .parent()
        .expect("crates")
        .parent()
        .expect("project root")
        .join("contracts/fixtures")
}

#[test]
fn fixture_create_sandbox_request_minimal_deserializes() {
    let path = fixtures_dir().join("api/create-sandbox-request.minimal.json");
    let json = std::fs::read_to_string(&path).expect("read fixture");
    let req: CreateSandboxRequest =
        serde_json::from_str(&json).expect("deserialize minimal request fixture");
    assert_eq!(req.runtime, "node24");
    assert_eq!(req.trust, TrustLevel::Untrusted);
}

#[test]
fn fixture_create_sandbox_request_full_deserializes() {
    let path = fixtures_dir().join("api/create-sandbox-request.full.json");
    let json = std::fs::read_to_string(&path).expect("read fixture");
    let req: CreateSandboxRequest =
        serde_json::from_str(&json).expect("deserialize full request fixture");
    assert_eq!(req.runtime, "python313");
    assert_eq!(req.trust, TrustLevel::Paranoid);
    assert_eq!(req.arch, Arch::Arm64);
    assert_eq!(req.resources.vcpus, 4);
    assert_eq!(req.resources.memory_mb(), 8192);
    assert_eq!(req.ports, vec![8080, 9090]);
    assert!(req.use_static_token);
    assert!(req.key.is_some());
    let key = req.key.expect("key");
    assert_eq!(key.name, "ml-worker-prod");
}

#[test]
fn fixture_create_sandbox_response_deserializes() {
    let path = fixtures_dir().join("api/create-sandbox-response.json");
    let json = std::fs::read_to_string(&path).expect("read fixture");
    let resp: CreateSandboxResponse =
        serde_json::from_str(&json).expect("deserialize response fixture");
    assert_eq!(resp.sandbox.id, "sb-new001");
    assert_eq!(resp.sandbox.backend_id, "bk-gen789");
    assert_eq!(resp.sandbox.cluster, "us-east-1");
    assert_eq!(resp.connection.token, "tk-a1b2c3d4e5f6");
}

#[test]
fn extract_tenant_from_header() {
    use super::api_v1::extract_tenant;
    use axum::http::HeaderMap;

    let mut headers = HeaderMap::new();
    headers.insert("x-auth-tenant", "tenant-abc".parse().expect("header"));
    assert_eq!(extract_tenant(&headers).expect("tenant"), "tenant-abc");
}

#[test]
fn extract_tenant_missing_header_fails() {
    use super::api_v1::extract_tenant;
    use axum::http::HeaderMap;

    let headers = HeaderMap::new();
    let err = extract_tenant(&headers).unwrap_err();
    assert_eq!(err.code, "UNAUTHENTICATED");
}

#[test]
fn extract_tenant_empty_header_fails() {
    use super::api_v1::extract_tenant;
    use axum::http::HeaderMap;

    let mut headers = HeaderMap::new();
    headers.insert("x-auth-tenant", "".parse().expect("header"));
    let err = extract_tenant(&headers).unwrap_err();
    assert_eq!(err.code, "UNAUTHENTICATED");
}

#[test]
fn extend_timeout_request_deserialization() {
    let json = r#"{"additional_ms": 60000}"#;
    let req: ExtendTimeoutRequest = serde_json::from_str(json).expect("deserialize");
    assert_eq!(req.additional_ms, 60000);
}

#[test]
fn snapshot_request_deserialization_minimal() {
    let json = r#"{}"#;
    let req: SnapshotRequest = serde_json::from_str(json).expect("deserialize");
    assert!(req.name.is_none());
    assert!(req.ttl_days.is_none());
    assert!(req.tags.is_empty());
}

#[test]
fn snapshot_request_deserialization_full() {
    let json = r#"{"name": "before-deploy", "ttl_days": 30, "tags": {"env": "staging"}}"#;
    let req: SnapshotRequest = serde_json::from_str(json).expect("deserialize");
    assert_eq!(req.name.as_deref(), Some("before-deploy"));
    assert_eq!(req.ttl_days, Some(30));
    assert_eq!(req.tags.get("env").map(|v| v.as_str()), Some("staging"));
}

#[test]
fn create_secret_request_deserialization() {
    let json = r#"{"value": "sk-secret123", "name": "openai-key"}"#;
    let req: CreateSecretRequest = serde_json::from_str(json).expect("deserialize");
    assert_eq!(req.value, "sk-secret123");
    assert_eq!(req.name.as_deref(), Some("openai-key"));
}

#[test]
fn create_credential_binding_request_deserialization() {
    let json = r#"{
        "secret_id": "secret-abc123",
        "destination": "api.openai.com",
        "header_name": "Authorization",
        "header_format": "Bearer {value}"
    }"#;
    let req: CreateCredentialBindingRequest = serde_json::from_str(json).expect("deserialize");
    assert_eq!(req.secret_id, "secret-abc123");
    assert_eq!(req.destination, "api.openai.com");
    assert_eq!(req.header_name, "Authorization");
    assert_eq!(req.header_format.as_deref(), Some("Bearer {value}"));
}

#[test]
fn credential_binding_request_without_format() {
    let json = r#"{
        "secret_id": "secret-abc123",
        "destination": "*.stripe.com",
        "header_name": "X-API-Key"
    }"#;
    let req: CreateCredentialBindingRequest = serde_json::from_str(json).expect("deserialize");
    assert!(req.header_format.is_none());
}
