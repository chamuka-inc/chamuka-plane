#![allow(clippy::unwrap_used, clippy::expect_used)]

use common::ext::agentd::{ExecRequest, ExecSyncResponse, LogFrame};
use common::ext::api::{CreateSandboxRequest, CreateSandboxResponse};
use common::ext::network::NetworkPolicy;
use common::ext::sandbox::Sandbox;
use common::ext::snapshot::Snapshot;
use common::ext::usage::UsageRecord;
use std::fs;
use std::path::Path;

fn fixture_path(rel: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("contracts")
        .join("fixtures")
        .join(rel)
}

fn load_fixture<T: serde::de::DeserializeOwned>(rel: &str) -> T {
    let path = fixture_path(rel);
    let data = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {}: {e}", path.display()));
    serde_json::from_str(&data)
        .unwrap_or_else(|e| panic!("Failed to parse fixture {}: {e}", path.display()))
}

#[test]
fn sandbox_minimal() {
    let s: Sandbox = load_fixture("sandbox/minimal.json");
    assert_eq!(s.id, "sb-min001");
    assert_eq!(s.status, common::types::BackendStatus::Scheduled);
}

#[test]
fn sandbox_full() {
    let s: Sandbox = load_fixture("sandbox/full.json");
    assert_eq!(s.id, "sb-full789");
    assert!(s.snapshot_source.is_some());
    assert!(s.ready_at.is_some());
    assert!(s.drone_name.is_some());
}

#[test]
fn sandbox_terminated() {
    let s: Sandbox = load_fixture("sandbox/terminated.json");
    assert_eq!(s.status, common::types::BackendStatus::Terminated);
    assert!(s.terminated_at.is_some());
}

#[test]
fn network_allow_all() {
    let p: NetworkPolicy = load_fixture("network/allow-all.json");
    assert_eq!(p.mode, common::ext::network::NetworkPolicyMode::AllowAll);
    assert!(p.rules.is_none());
}

#[test]
fn network_deny_all() {
    let p: NetworkPolicy = load_fixture("network/deny-all.json");
    assert_eq!(p.mode, common::ext::network::NetworkPolicyMode::DenyAll);
}

#[test]
fn network_rules() {
    let p: NetworkPolicy = load_fixture("network/rules.json");
    assert_eq!(p.mode, common::ext::network::NetworkPolicyMode::Rules);
    assert_eq!(p.rules.as_ref().unwrap().len(), 3);
}

#[test]
fn api_create_sandbox_request_minimal() {
    let r: CreateSandboxRequest = load_fixture("api/create-sandbox-request.minimal.json");
    assert_eq!(r.runtime, "node24");
    assert_eq!(r.trust, common::ext::sandbox::TrustLevel::Untrusted);
}

#[test]
fn api_create_sandbox_request_full() {
    let r: CreateSandboxRequest = load_fixture("api/create-sandbox-request.full.json");
    assert_eq!(r.runtime, "python313");
    assert!(r.source.is_some());
    assert!(r.initial_network_policy.is_some());
}

#[test]
fn api_create_sandbox_response() {
    let r: CreateSandboxResponse = load_fixture("api/create-sandbox-response.json");
    assert_eq!(r.sandbox.id, "sb-new001");
    assert!(!r.connection.token.is_empty());
}

#[test]
fn agentd_exec_request_minimal() {
    let r: ExecRequest = load_fixture("agentd/exec-request.minimal.json");
    assert_eq!(r.cmd, "echo");
}

#[test]
fn agentd_exec_request_full() {
    let r: ExecRequest = load_fixture("agentd/exec-request.full.json");
    assert!(r.pty);
    assert!(r.pty_size.is_some());
}

#[test]
fn agentd_exec_sync_response() {
    let r: ExecSyncResponse = load_fixture("agentd/exec-sync-response.json");
    assert_eq!(r.exit_code, 0);
    assert_eq!(r.cmd_id, "cmd-abc123");
}

#[test]
fn agentd_log_frames() {
    let frames: Vec<LogFrame> = load_fixture("agentd/log-frames.json");
    assert_eq!(frames.len(), 4);
}

#[test]
fn snapshot_full() {
    let s: Snapshot = load_fixture("snapshot/full.json");
    assert_eq!(s.id, "snap-abc123");
    assert_eq!(s.size_bytes, 524288000);
}

#[test]
fn usage_record() {
    let u: UsageRecord = load_fixture("usage/record.json");
    assert_eq!(u.sandbox_id, "sb-full789");
    assert_eq!(u.cpu_nsec, 45000000000);
}
