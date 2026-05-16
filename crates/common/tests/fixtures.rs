#![allow(clippy::unwrap_used, clippy::expect_used)]

use common::ext::agentd::{ExecRequest, ExecSyncResponse, LogFrame};
use common::ext::api::{CreateSandboxRequest, CreateSandboxResponse};
use common::ext::network::NetworkPolicy;
use common::ext::sandbox::Sandbox;
use common::ext::snapshot::Snapshot;
use common::ext::usage::UsageRecord;
use common::test_helpers::load_fixture_as;

#[test]
fn sandbox_minimal() {
    let s: Sandbox = load_fixture_as("sandbox/minimal.json");
    assert_eq!(s.id, "sb-min001");
    assert_eq!(s.status, common::types::BackendStatus::Scheduled);
}

#[test]
fn sandbox_full() {
    let s: Sandbox = load_fixture_as("sandbox/full.json");
    assert_eq!(s.id, "sb-full789");
    assert!(s.snapshot_source.is_some());
    assert!(s.ready_at.is_some());
    assert!(s.drone_name.is_some());
}

#[test]
fn sandbox_terminated() {
    let s: Sandbox = load_fixture_as("sandbox/terminated.json");
    assert_eq!(s.status, common::types::BackendStatus::Terminated);
    assert!(s.terminated_at.is_some());
}

#[test]
fn network_allow_all() {
    let p: NetworkPolicy = load_fixture_as("network/allow-all.json");
    assert_eq!(p.mode, common::ext::network::NetworkPolicyMode::AllowAll);
    assert!(p.rules.is_none());
}

#[test]
fn network_deny_all() {
    let p: NetworkPolicy = load_fixture_as("network/deny-all.json");
    assert_eq!(p.mode, common::ext::network::NetworkPolicyMode::DenyAll);
}

#[test]
fn network_rules() {
    let p: NetworkPolicy = load_fixture_as("network/rules.json");
    assert_eq!(p.mode, common::ext::network::NetworkPolicyMode::Rules);
    assert_eq!(p.rules.as_ref().unwrap().len(), 3);
}

#[test]
fn api_create_sandbox_request_minimal() {
    let r: CreateSandboxRequest = load_fixture_as("api/create-sandbox-request.minimal.json");
    assert_eq!(r.runtime, "node24");
    assert_eq!(r.trust, common::ext::sandbox::TrustLevel::Untrusted);
}

#[test]
fn api_create_sandbox_request_full() {
    let r: CreateSandboxRequest = load_fixture_as("api/create-sandbox-request.full.json");
    assert_eq!(r.runtime, "python313");
    assert!(r.source.is_some());
    assert!(r.initial_network_policy.is_some());
}

#[test]
fn api_create_sandbox_response() {
    let r: CreateSandboxResponse = load_fixture_as("api/create-sandbox-response.json");
    assert_eq!(r.sandbox.id, "sb-new001");
    assert!(!r.connection.token.is_empty());
}

#[test]
fn agentd_exec_request_minimal() {
    let r: ExecRequest = load_fixture_as("agentd/exec-request.minimal.json");
    assert_eq!(r.cmd, "echo");
}

#[test]
fn agentd_exec_request_full() {
    let r: ExecRequest = load_fixture_as("agentd/exec-request.full.json");
    assert!(r.pty);
    assert!(r.pty_size.is_some());
}

#[test]
fn agentd_exec_sync_response() {
    let r: ExecSyncResponse = load_fixture_as("agentd/exec-sync-response.json");
    assert_eq!(r.exit_code, 0);
    assert_eq!(r.cmd_id, "cmd-abc123");
}

#[test]
fn agentd_log_frames() {
    let frames: Vec<LogFrame> = load_fixture_as("agentd/log-frames.json");
    assert_eq!(frames.len(), 4);
}

#[test]
fn snapshot_full() {
    let s: Snapshot = load_fixture_as("snapshot/full.json");
    assert_eq!(s.id, "snap-abc123");
    assert_eq!(s.size_bytes, 524288000);
}

#[test]
fn usage_record() {
    let u: UsageRecord = load_fixture_as("usage/record.json");
    assert_eq!(u.sandbox_id, "sb-full789");
    assert_eq!(u.cpu_nsec, 45000000000);
}
