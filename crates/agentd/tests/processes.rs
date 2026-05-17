#![allow(clippy::unwrap_used, clippy::expect_used)]

use common::ext::agentd::ProcessesResponse;
use reqwest::StatusCode;

#[tokio::test]
async fn processes_returns_list() {
    let addr = agentd::serve_on_ephemeral().await;
    let url = format!("http://{addr}/v1/processes");
    let resp = reqwest::get(&url).await.expect("request failed");
    assert_eq!(resp.status(), StatusCode::OK);

    let body: ProcessesResponse = resp.json().await.expect("json");
    assert!(
        !body.processes.is_empty(),
        "should list at least one process"
    );
    assert!(
        body.processes
            .iter()
            .map(|p| p.pid)
            .any(|pid| pid == std::process::id()),
        "should contain the current process"
    );
}
