#![allow(clippy::unwrap_used, clippy::expect_used)]

use reqwest::StatusCode;

#[tokio::test]
async fn info_returns_metadata() {
    let addr = agentd::serve_on_ephemeral().await;
    let url = format!("http://{addr}/info");
    let resp = reqwest::get(&url).await.expect("request failed");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.expect("json parse failed");
    assert_eq!(body["runtime"], "agentd-headless");
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
    assert!(!body["arch"].as_str().unwrap().is_empty());
    assert!(!body["os"].as_str().unwrap().is_empty());
}
