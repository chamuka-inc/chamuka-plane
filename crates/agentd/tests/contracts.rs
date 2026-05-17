#![allow(clippy::unwrap_used, clippy::expect_used)]

use common::ext::agentd::{ExecRequest, ExecSyncResponse};
use common::test_helpers::load_fixture_as;
use reqwest::StatusCode;

#[tokio::test]
async fn contract_exec_minimal_fixture() {
    let addr = agentd::serve_on_ephemeral().await;
    let url = format!("http://{addr}/v1/exec");

    let req: ExecRequest = load_fixture_as("agentd/exec-request.minimal.json");
    assert_eq!(req.cmd, "echo");
    assert_eq!(req.args, vec!["hello"]);

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&req)
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::OK);
    let body: ExecSyncResponse = resp.json().await.expect("json parse failed");
    assert_eq!(body.exit_code, 0);
    assert_eq!(body.stdout.trim(), "hello");
}

#[tokio::test]
async fn contract_exec_full_fixture_deserializes() {
    let req: ExecRequest = load_fixture_as("agentd/exec-request.full.json");
    assert_eq!(req.cmd, "python3");
    assert!(req.pty);
    assert!(req.pty_size.is_some());
    assert_eq!(req.timeout_ms, Some(30000));
    assert_eq!(req.user, Some("sandbox".to_string()));
}

#[tokio::test]
async fn contract_exec_response_fixture_deserializes() {
    let resp: ExecSyncResponse = load_fixture_as("agentd/exec-sync-response.json");
    assert_eq!(resp.cmd_id, "cmd-abc123");
    assert_eq!(resp.exit_code, 0);
    assert!(!resp.stdout.is_empty());
}
