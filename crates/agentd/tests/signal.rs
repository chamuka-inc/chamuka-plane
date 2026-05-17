#![allow(clippy::unwrap_used, clippy::expect_used)]

use common::ext::agentd::ExecRequest;
use reqwest::StatusCode;
use std::collections::HashMap;

#[tokio::test]
async fn signal_to_nonexistent_process_returns_404() {
    let addr = agentd::serve_on_ephemeral().await;
    let url = format!("http://{addr}/v1/exec/nonexistent-id/signal");

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&serde_json::json!({"signal": "TERM"}))
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn signal_kills_long_running_process() {
    let addr = agentd::serve_on_ephemeral().await;
    let exec_url = format!("http://{addr}/v1/exec");

    // Start a detached long-running process
    let req = ExecRequest {
        cmd: "sleep".to_string(),
        args: vec!["300".to_string()],
        env: HashMap::new(),
        cwd: None,
        stdin: None,
        timeout_ms: None,
        detached: true,
        pty: false,
        pty_size: None,
        user: None,
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(&exec_url)
        .json(&req)
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.expect("json parse failed");
    let cmd_id = body["cmd_id"].as_str().expect("cmd_id");

    // Give process time to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Send KILL signal
    let signal_url = format!("http://{addr}/v1/exec/{cmd_id}/signal");
    let resp = client
        .post(&signal_url)
        .json(&serde_json::json!({"signal": "KILL"}))
        .send()
        .await
        .expect("signal request failed");

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}
