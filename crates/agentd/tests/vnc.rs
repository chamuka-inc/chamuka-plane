#![allow(clippy::unwrap_used, clippy::expect_used)]

use common::test_helpers::load_fixture;
use futures_util::{SinkExt, StreamExt};
use reqwest::StatusCode;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn contract_vnc_unavailable_matches_fixture() {
    let addr = agentd::serve_on_ephemeral().await;
    let url = format!("http://{addr}/v1/vnc");
    let resp = reqwest::get(&url).await.expect("request failed");
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body: serde_json::Value = resp.json().await.expect("json parse failed");
    let fixture: serde_json::Value = load_fixture("agentd/vnc-unavailable.json");
    assert_eq!(body, fixture);
}

#[tokio::test]
async fn vnc_relays_binary_frames() {
    let listener = TcpListener::bind("127.0.0.1:6080")
        .await
        .expect("bind 6080");

    let mock_handle = tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.expect("accept");
            let mut ws = match accept_async(stream).await {
                Ok(ws) => ws,
                Err(_) => continue,
            };
            while let Some(Ok(msg)) = ws.next().await {
                if let Message::Binary(data) = msg {
                    let mut reply = b"echo:".to_vec();
                    reply.extend_from_slice(&data);
                    if ws.send(Message::Binary(reply.into())).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let addr = agentd::serve_on_ephemeral().await;
    let ws_url = format!("ws://{addr}/v1/vnc");

    let (mut client, _) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .expect("connect to agentd /v1/vnc");

    client
        .send(Message::Binary(b"hello".to_vec().into()))
        .await
        .expect("send");

    let reply = client.next().await.expect("reply").expect("reply ok");
    match reply {
        Message::Binary(data) => assert_eq!(&data[..], b"echo:hello"),
        other => panic!("expected binary, got {other:?}"),
    }

    client.close(None).await.expect("close");
    mock_handle.abort();
}
