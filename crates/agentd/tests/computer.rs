#![allow(clippy::unwrap_used, clippy::expect_used)]

use common::ext::agentd::{ComputerAction, ComputerActionRequest};
use reqwest::StatusCode;

#[tokio::test]
async fn computer_endpoint_responds() {
    let addr = agentd::serve_on_ephemeral().await;
    let url = format!("http://{addr}/v1/computer");
    let client = reqwest::Client::new();
    let req = ComputerActionRequest {
        action: ComputerAction::Screenshot,
        coordinate: None,
        start_coordinate: None,
        text: None,
        duration: None,
        region: None,
        scroll_direction: None,
        scroll_amount: None,
    };

    let resp = client
        .post(&url)
        .json(&req)
        .send()
        .await
        .expect("request failed");

    #[cfg(not(feature = "desktop"))]
    assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);

    #[cfg(feature = "desktop")]
    assert_eq!(resp.status(), StatusCode::OK);
}
