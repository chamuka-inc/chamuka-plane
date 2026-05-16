#![allow(clippy::unwrap_used, clippy::expect_used)]

use common::ext::agentd::{ExecRequest, ExecSyncResponse, LogFrame, Signal};
use common::ext::api::{CreateSandboxRequest, CreateSandboxResponse, ListSandboxesResponse};
use common::ext::network::{NetworkPolicy, NetworkPolicyMode, NetworkRule};
use common::ext::sandbox::{Arch, Sandbox, TrustLevel};
use common::ext::snapshot::Snapshot;
use common::ext::usage::UsageRecord;
use common::protocol::MessageFromDrone;
use common::types::{BackendStatus, ConnectRequest, ConnectResponse};

#[test]
fn plane_types_reexported() {
    let _status = BackendStatus::Ready;
    let _req = ConnectRequest {
        key: None,
        spawn_config: None,
        user: None,
        auth: Default::default(),
    };
    let _ = std::mem::size_of::<ConnectResponse>();
    let _ = std::mem::size_of::<MessageFromDrone>();
}

#[test]
fn ext_sandbox_constructible() {
    let json = serde_json::json!({
        "id": "sb-abc123",
        "backend_id": "bk-xyz",
        "tenant": "tenant-1",
        "status": "ready",
        "runtime": "node24",
        "trust": "untrusted",
        "arch": "amd64",
        "image": "registry.example.com/node:24",
        "resources": { "vcpus": 2, "memory_mb": 512 },
        "ports": [3000, 8080],
        "tags": { "env": "dev" },
        "persistent": false,
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:01:00Z",
        "expires_at": "2026-01-02T00:00:00Z",
        "cluster": "us-east-1"
    });
    let sandbox: Sandbox = serde_json::from_value(json).expect("sandbox deserialization");
    assert_eq!(sandbox.id, "sb-abc123");
    assert_eq!(sandbox.trust, TrustLevel::Untrusted);
    assert_eq!(sandbox.arch, Arch::Amd64);
    assert_eq!(sandbox.resources.vcpus, 2);
}

#[test]
fn ext_network_policy_roundtrip() {
    let policy = NetworkPolicy {
        mode: NetworkPolicyMode::Rules,
        rules: Some(vec![NetworkRule {
            domains: Some(vec!["api.example.com".to_string()]),
            cidrs: None,
            ports: Some(vec![443]),
            action: None,
            inject: None,
        }]),
    };
    let json = serde_json::to_string(&policy).expect("serialize");
    let parsed: NetworkPolicy = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.mode, NetworkPolicyMode::Rules);
    assert_eq!(parsed.rules.as_ref().expect("rules").len(), 1);
}

#[test]
fn ext_exec_request_roundtrip() {
    let req = ExecRequest {
        cmd: "node".to_string(),
        args: vec!["index.js".to_string()],
        env: Default::default(),
        cwd: Some("/app".to_string()),
        stdin: None,
        timeout_ms: Some(30000),
        detached: false,
        pty: false,
        pty_size: None,
        user: None,
    };
    let json = serde_json::to_value(&req).expect("serialize");
    assert_eq!(json["cmd"], "node");
    assert_eq!(json["cwd"], "/app");
}

#[test]
fn ext_usage_record_schema() {
    let schema = schemars::schema_for!(UsageRecord);
    let props = schema
        .schema
        .object
        .as_ref()
        .expect("object schema")
        .properties
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    assert!(props.contains(&"sandbox_id".to_string()));
    assert!(props.contains(&"cpu_nsec".to_string()));
    assert!(props.contains(&"rx_bytes".to_string()));
}

#[test]
fn signal_variants_serialize() {
    assert_eq!(
        serde_json::to_string(&Signal::Term).expect("ser"),
        "\"TERM\""
    );
    assert_eq!(
        serde_json::to_string(&Signal::Kill).expect("ser"),
        "\"KILL\""
    );
}

#[test]
fn log_frame_tagged_union() {
    let frame = LogFrame::Stdout {
        data: "hello\n".to_string(),
    };
    let json = serde_json::to_value(&frame).expect("serialize");
    assert_eq!(json["stream"], "stdout");
    assert_eq!(json["data"], "hello\n");
}

#[test]
fn create_sandbox_request_defaults() {
    let json = serde_json::json!({
        "runtime": "node24",
        "resources": { "vcpus": 1, "memory_mb": 256 }
    });
    let req: CreateSandboxRequest = serde_json::from_value(json).expect("deser");
    assert_eq!(req.trust, TrustLevel::Untrusted);
    assert_eq!(req.arch, Arch::Amd64);
    assert!(!req.persistent);
    assert!(!req.use_static_token);
}

#[test]
fn snapshot_schema_has_required_fields() {
    let schema = schemars::schema_for!(Snapshot);
    let props = schema
        .schema
        .object
        .as_ref()
        .expect("object")
        .properties
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    assert!(props.contains(&"digest".to_string()));
    assert!(props.contains(&"image_ref".to_string()));
    assert!(props.contains(&"arch".to_string()));
}

// Compile-time assertion: these types are accessible from the public API.
#[test]
fn assert_types_exist() {
    let _ = std::mem::size_of::<ExecSyncResponse>();
    let _ = std::mem::size_of::<CreateSandboxResponse>();
    let _ = std::mem::size_of::<ListSandboxesResponse>();
}
