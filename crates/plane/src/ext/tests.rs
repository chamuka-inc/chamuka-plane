#![allow(clippy::expect_used)]

use super::types::*;
use chamuka_common::ext::api::{
    CreateCredentialBindingRequest, CreateSecretRequest, ExtendTimeoutRequest, SnapshotRequest,
};
use plane_common::protocol::{
    CredentialPayloadWire, PushCredentialWire, PushNetworkPolicyWire, SnapshotCommandWire,
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
fn resolve_image_known_runtime_bare() {
    use super::api_v1::resolve_image;
    assert_eq!(
        resolve_image("node24", None, None).expect("resolve"),
        "sandbox-runtime:node24-latest"
    );
    assert_eq!(
        resolve_image("python313", None, None).expect("resolve"),
        "sandbox-runtime:python313-latest"
    );
    assert_eq!(
        resolve_image("agent", None, None).expect("resolve"),
        "sandbox-runtime:agent-latest"
    );
}

#[test]
fn resolve_image_known_runtime_with_registry() {
    use super::api_v1::resolve_image;
    assert_eq!(
        resolve_image("node24", None, Some("192.168.56.1:5000")).expect("resolve"),
        "192.168.56.1:5000/sandbox-runtime:node24-latest"
    );
}

#[test]
fn resolve_image_rejects_explicit_for_known_runtime() {
    use super::api_v1::resolve_image;
    let result = resolve_image("node24", Some("my-org/custom:v1"), None);
    assert!(
        result.is_err(),
        "expected error when explicit image is supplied for a known runtime"
    );
}

#[test]
fn resolve_image_custom_requires_explicit() {
    use super::api_v1::resolve_image;
    let result = resolve_image("custom", None, None);
    assert!(result.is_err());
}

#[test]
fn resolve_image_custom_uses_explicit() {
    use super::api_v1::resolve_image;
    assert_eq!(
        resolve_image("custom", Some("my-org/custom:v1"), None).expect("resolve"),
        "my-org/custom:v1"
    );
}

#[test]
fn resolve_image_unknown_runtime_requires_explicit() {
    use super::api_v1::resolve_image;
    assert!(resolve_image("not-a-runtime", None, None).is_err());
    assert_eq!(
        resolve_image("not-a-runtime", Some("my-org/custom:v1"), None).expect("resolve"),
        "my-org/custom:v1"
    );
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
    assert_eq!(resp.sandbox.backend_id.as_deref(), Some("bk-gen789"));
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

// --- Phase 3.3: Extension action protocol and database tests ---

#[test]
fn message_to_drone_push_network_policy_roundtrip() {
    use plane_common::protocol::MessageToDrone;

    let policy = serde_json::json!({"mode": "allow-all"});
    let msg = MessageToDrone::PushNetworkPolicy(PushNetworkPolicyWire {
        action_id: "xak-policy-001".to_string(),
        backend_id: "bk-abc123".to_string(),
        policy,
        version: 1,
    });
    let json = serde_json::to_string(&msg).expect("serialize");
    let deserialized: MessageToDrone = serde_json::from_str(&json).expect("deserialize");
    match deserialized {
        MessageToDrone::PushNetworkPolicy(m) => {
            assert_eq!(m.backend_id, "bk-abc123");
            assert_eq!(m.version, 1);
        }
        _ => panic!("expected PushNetworkPolicy variant"),
    }
}

#[test]
fn message_to_drone_push_credential_roundtrip() {
    use plane_common::protocol::MessageToDrone;

    let msg = MessageToDrone::PushCredential(PushCredentialWire {
        action_id: "xak-cred-001".to_string(),
        backend_id: "bk-abc123".to_string(),
        binding: CredentialPayloadWire {
            id: "cred-001".to_string(),
            destination: "api.openai.com".to_string(),
            header_name: "Authorization".to_string(),
            header_format: "Bearer {value}".to_string(),
            value: "sk-secret".to_string(),
        },
    });
    let json = serde_json::to_string(&msg).expect("serialize");
    let deserialized: MessageToDrone = serde_json::from_str(&json).expect("deserialize");
    match deserialized {
        MessageToDrone::PushCredential(m) => {
            assert_eq!(m.backend_id, "bk-abc123");
            assert_eq!(m.binding.id, "cred-001");
        }
        _ => panic!("expected PushCredential variant"),
    }
}

#[test]
fn message_to_drone_snapshot_roundtrip() {
    use plane_common::protocol::MessageToDrone;

    let msg = MessageToDrone::Snapshot(SnapshotCommandWire {
        action_id: "xak-001".to_string(),
        backend_id: "bk-abc123".to_string(),
        ttl_days: Some(30),
        name: Some("my-snapshot".to_string()),
    });
    let json = serde_json::to_string(&msg).expect("serialize");
    let deserialized: MessageToDrone = serde_json::from_str(&json).expect("deserialize");
    match deserialized {
        MessageToDrone::Snapshot(m) => {
            assert_eq!(m.action_id, "xak-001");
            assert_eq!(m.ttl_days, Some(30));
        }
        _ => panic!("expected Snapshot variant"),
    }
}

#[test]
fn message_to_drone_revoke_credential_roundtrip() {
    use plane_common::protocol::MessageToDrone;

    let msg = MessageToDrone::RevokeCredential {
        action_id: "xak-revoke-001".to_string(),
        binding_id: "cred-001".to_string(),
    };
    let json = serde_json::to_string(&msg).expect("serialize");
    let deserialized: MessageToDrone = serde_json::from_str(&json).expect("deserialize");
    match deserialized {
        MessageToDrone::RevokeCredential {
            action_id,
            binding_id,
        } => {
            assert_eq!(action_id, "xak-revoke-001");
            assert_eq!(binding_id, "cred-001");
        }
        _ => panic!("expected RevokeCredential variant"),
    }
}

#[test]
fn message_from_drone_ack_extension_action_roundtrip() {
    use plane_common::protocol::MessageFromDrone;

    let msg = MessageFromDrone::AckExtensionAction {
        action_id: "xak-001".to_string(),
    };
    let json = serde_json::to_string(&msg).expect("serialize");
    let deserialized: MessageFromDrone = serde_json::from_str(&json).expect("deserialize");
    match deserialized {
        MessageFromDrone::AckExtensionAction { action_id } => {
            assert_eq!(action_id, "xak-001");
        }
        _ => panic!("expected AckExtensionAction variant"),
    }
}

#[test]
fn extension_action_id_has_prefix() {
    let id = super::types::ExtensionActionId::new();
    assert!(
        id.0.starts_with("xak-"),
        "ExtensionActionId should start with xak- prefix, got: {}",
        id.0
    );
}

#[test]
fn build_push_network_policy_wire() {
    use super::dispatch::build_push_network_policy;
    use chamuka_common::ext::network::{NetworkPolicy, NetworkPolicyMode};

    let policy = NetworkPolicy {
        mode: NetworkPolicyMode::AllowAll,
        rules: None,
    };
    let wire = build_push_network_policy("bk-test123", &policy, 5);
    assert_eq!(wire.backend_id, "bk-test123");
    assert_eq!(wire.version, 5);
    let mode: String =
        serde_json::from_value(wire.policy.get("mode").expect("mode").clone()).expect("str");
    assert_eq!(mode, "allow-all");
}

#[test]
fn build_push_credential_wire() {
    use super::dispatch::build_push_credential;

    let wire = build_push_credential(
        "bk-test123",
        "cred-001",
        "api.openai.com",
        "Authorization",
        "Bearer {value}",
        "sk-secret",
    );
    assert_eq!(wire.backend_id, "bk-test123");
    assert_eq!(wire.binding.id, "cred-001");
    assert_eq!(wire.binding.destination, "api.openai.com");
    assert_eq!(wire.binding.value, "sk-secret");
}

#[test]
fn build_snapshot_command_wire() {
    use super::dispatch::build_snapshot_command;

    let wire = build_snapshot_command("xak-001", "bk-test123", Some(30), Some("my-snap"));
    assert_eq!(wire.action_id, "xak-001");
    assert_eq!(wire.backend_id, "bk-test123");
    assert_eq!(wire.ttl_days, Some(30));
    assert_eq!(wire.name.as_deref(), Some("my-snap"));
}
