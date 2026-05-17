#![allow(clippy::unwrap_used, clippy::expect_used)]

use common::ext::agentd::FsEvent;
use futures_util::StreamExt;
use tokio_tungstenite::connect_async;

#[tokio::test]
async fn fs_watch_detects_file_creation() {
    let addr = agentd::serve_on_ephemeral().await;
    let dir = tempfile::tempdir().expect("create tempdir");
    let dir_path = dir.path().to_str().unwrap();

    let ws_url = format!("ws://{addr}/v1/fs/watch?path={dir_path}&recursive=true");
    let (mut ws, _) = connect_async(&ws_url).await.expect("ws connect failed");

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let file_path = dir.path().join("new_file.txt");
    std::fs::write(&file_path, "test data").expect("write file");

    let timeout = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while let Some(Ok(msg)) = ws.next().await {
            if msg.is_text() {
                let text = msg.into_text().unwrap();
                let event: FsEvent = serde_json::from_str(&text).expect("parse event");
                if event.path.contains("new_file.txt") {
                    return Some(event);
                }
            }
        }
        None
    })
    .await;

    assert!(
        timeout.is_ok() && timeout.unwrap().is_some(),
        "should receive a file creation event within timeout"
    );
}
