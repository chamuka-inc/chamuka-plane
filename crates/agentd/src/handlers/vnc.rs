use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{FromRequestParts, Request};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::{client_async, tungstenite};

const WEBSOCKIFY_ADDR: &str = "127.0.0.1:6080";

fn unavailable_response() -> impl IntoResponse {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({
            "error": "vnc_not_available",
            "message": "VNC is not enabled on this sandbox"
        })),
    )
}

async fn websockify_available() -> bool {
    TcpStream::connect(WEBSOCKIFY_ADDR).await.is_ok()
}

pub async fn stream(req: Request) -> impl IntoResponse {
    if !websockify_available().await {
        return unavailable_response().into_response();
    }

    let (mut parts, _body) = req.into_parts();
    match WebSocketUpgrade::from_request_parts(&mut parts, &()).await {
        Ok(ws) => ws.on_upgrade(relay).into_response(),
        Err(_) => unavailable_response().into_response(),
    }
}

async fn relay(mut client: WebSocket) {
    let tcp = match TcpStream::connect(WEBSOCKIFY_ADDR).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(addr = WEBSOCKIFY_ADDR, error = %e, "websockify tcp connect failed");
            return;
        }
    };

    let ws_url = format!("ws://{WEBSOCKIFY_ADDR}/websockify");
    let (mut upstream, _) = match client_async(&ws_url, tcp).await {
        Ok(pair) => pair,
        Err(e) => {
            tracing::warn!(url = %ws_url, error = %e, "websockify ws handshake failed");
            return;
        }
    };

    loop {
        tokio::select! {
            msg = StreamExt::next(&mut upstream) => {
                match msg {
                    Some(Ok(tungstenite::Message::Binary(data))) => {
                        if client.send(Message::Binary(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(tungstenite::Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
            msg = client.recv() => {
                match msg {
                    Some(Ok(Message::Binary(data))) => {
                        if upstream.send(tungstenite::Message::Binary(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }

    let _ = upstream.close(None).await;
}
