#![allow(clippy::unwrap_used, clippy::expect_used)]

use common::ext::agentd::PortsResponse;
use reqwest::StatusCode;

#[tokio::test]
async fn ports_returns_listening() {
    let addr = agentd::serve_on_ephemeral().await;
    let url = format!("http://{addr}/v1/ports");
    let resp = reqwest::get(&url).await.expect("request failed");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: PortsResponse = resp.json().await.expect("json parse failed");
    // agentd itself is listening on the ephemeral port
    let has_our_port = body.listening.iter().any(|p| p.port == addr.port());
    assert!(has_our_port, "should see agentd's own port in listing");
}
