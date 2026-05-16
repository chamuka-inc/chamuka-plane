#![allow(clippy::unwrap_used, clippy::expect_used)]

use reqwest::StatusCode;

#[tokio::test]
async fn health_returns_ready() {
    let addr = agentd::serve_on_ephemeral().await;
    let url = format!("http://{addr}/health");
    let resp = reqwest::get(&url).await.expect("request failed");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.expect("json parse failed");
    assert_eq!(body["status"], "ready");
}
