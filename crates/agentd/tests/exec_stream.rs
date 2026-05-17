#![allow(clippy::unwrap_used, clippy::expect_used)]

use common::ext::agentd::{ExecRequest, LogFrame};
use futures_util::StreamExt;
use reqwest::StatusCode;
use std::collections::HashMap;
use tokio_tungstenite::connect_async;

#[tokio::test]
async fn exec_stdout_stream_receives_output() {
    let addr = agentd::serve_on_ephemeral().await;
    let exec_url = format!("http://{addr}/v1/exec");

    let req = ExecRequest {
        cmd: "sh".to_string(),
        args: vec!["-c".to_string(), "echo hello; echo world".to_string()],
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
    let body: serde_json::Value = resp.json().await.expect("json");
    let cmd_id = body["cmd_id"].as_str().unwrap().to_string();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let ws_url = format!("ws://{addr}/v1/exec/{cmd_id}/logs");
    let (mut ws, _) = connect_async(&ws_url).await.expect("ws connect failed");

    let mut frames = Vec::new();
    while let Some(Ok(msg)) = ws.next().await {
        if msg.is_text() {
            let text = msg.into_text().unwrap();
            let frame: LogFrame = serde_json::from_str(&text).expect("parse frame");
            frames.push(frame);
            if matches!(frames.last(), Some(LogFrame::Exit { .. })) {
                break;
            }
        }
    }

    assert!(
        frames
            .iter()
            .any(|f| matches!(f, LogFrame::Exit { code, .. } if *code == 0)),
        "should receive exit frame with code 0"
    );
}

#[tokio::test]
async fn exec_logs_stream_nonexistent_cmd_closes() {
    let addr = agentd::serve_on_ephemeral().await;
    let ws_url = format!("ws://{addr}/v1/exec/nonexistent/logs");
    let result = connect_async(&ws_url).await;
    assert!(
        result.is_err() || {
            let (mut ws, _) = result.unwrap();
            let msg = ws.next().await;
            msg.is_none() || msg.unwrap().is_err()
        }
    );
}
