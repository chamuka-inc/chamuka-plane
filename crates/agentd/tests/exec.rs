#![allow(clippy::unwrap_used, clippy::expect_used)]

use common::ext::agentd::{ExecRequest, ExecSyncResponse};
use reqwest::StatusCode;
use std::collections::HashMap;
use std::time::Instant;

#[tokio::test]
async fn exec_sync_echo() {
    let addr = agentd::serve_on_ephemeral().await;
    let url = format!("http://{addr}/v1/exec");

    let req = ExecRequest {
        cmd: "echo".to_string(),
        args: vec!["hello".to_string()],
        env: HashMap::new(),
        cwd: None,
        stdin: None,
        timeout_ms: Some(5000),
        detached: false,
        pty: false,
        pty_size: None,
        user: None,
    };

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
    assert!(body.stderr.is_empty());
    assert!(!body.cmd_id.is_empty());
}

#[tokio::test]
async fn exec_sync_nonzero_exit() {
    let addr = agentd::serve_on_ephemeral().await;
    let url = format!("http://{addr}/v1/exec");

    let req = ExecRequest {
        cmd: "sh".to_string(),
        args: vec!["-c".to_string(), "exit 42".to_string()],
        env: HashMap::new(),
        cwd: None,
        stdin: None,
        timeout_ms: Some(5000),
        detached: false,
        pty: false,
        pty_size: None,
        user: None,
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&req)
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::OK);
    let body: ExecSyncResponse = resp.json().await.expect("json parse failed");
    assert_eq!(body.exit_code, 42);
}

#[tokio::test]
async fn exec_sync_with_env() {
    let addr = agentd::serve_on_ephemeral().await;
    let url = format!("http://{addr}/v1/exec");

    let mut env = HashMap::new();
    env.insert("MY_VAR".to_string(), "test_value".to_string());

    let req = ExecRequest {
        cmd: "sh".to_string(),
        args: vec!["-c".to_string(), "echo $MY_VAR".to_string()],
        env,
        cwd: None,
        stdin: None,
        timeout_ms: Some(5000),
        detached: false,
        pty: false,
        pty_size: None,
        user: None,
    };

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
    assert_eq!(body.stdout.trim(), "test_value");
}

#[tokio::test]
async fn exec_sync_with_stdin() {
    let addr = agentd::serve_on_ephemeral().await;
    let url = format!("http://{addr}/v1/exec");

    let req = ExecRequest {
        cmd: "cat".to_string(),
        args: vec![],
        env: HashMap::new(),
        cwd: None,
        stdin: Some("piped input".to_string()),
        timeout_ms: Some(5000),
        detached: false,
        pty: false,
        pty_size: None,
        user: None,
    };

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
    assert_eq!(body.stdout, "piped input");
}

#[tokio::test]
async fn exec_sync_timeout_kills_process() {
    let addr = agentd::serve_on_ephemeral().await;
    let url = format!("http://{addr}/v1/exec");

    let req = ExecRequest {
        cmd: "sleep".to_string(),
        args: vec!["60".to_string()],
        env: HashMap::new(),
        cwd: None,
        stdin: None,
        timeout_ms: Some(200),
        detached: false,
        pty: false,
        pty_size: None,
        user: None,
    };

    let start = Instant::now();
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&req)
        .send()
        .await
        .expect("request failed");

    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 2000,
        "timeout took too long: {elapsed:?}"
    );
    assert_eq!(resp.status(), StatusCode::OK);
    let body: ExecSyncResponse = resp.json().await.expect("json parse failed");
    assert_ne!(body.exit_code, 0, "process should have been killed");
}
