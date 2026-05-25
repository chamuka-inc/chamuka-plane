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
                let Some(msg) = msg else { break };
                let Ok(msg) = msg else { break };
                if relay_upstream_to_client(msg, &mut client).await.is_err() {
                    break;
                }
            }
            msg = client.recv() => {
                let Some(msg) = msg else { break };
                let Ok(msg) = msg else { break };
                if relay_client_to_upstream(msg, &mut upstream).await.is_err() {
                    break;
                }
            }
        }
    }

    let _ = upstream.close(None).await;
}

async fn relay_upstream_to_client(
    msg: tungstenite::Message,
    client: &mut WebSocket,
) -> Result<(), ()> {
    let translated = match msg {
        tungstenite::Message::Binary(data) => Message::Binary(data),
        tungstenite::Message::Text(text) => Message::Text(text.as_str().into()),
        tungstenite::Message::Ping(data) => Message::Ping(data),
        tungstenite::Message::Pong(data) => Message::Pong(data),
        tungstenite::Message::Close(_) => return Err(()),
        tungstenite::Message::Frame(_) => return Ok(()),
    };
    client.send(translated).await.map_err(|_| ())
}

async fn relay_client_to_upstream<S>(
    msg: Message,
    upstream: &mut tokio_tungstenite::WebSocketStream<S>,
) -> Result<(), ()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let translated = match msg {
        Message::Binary(data) => tungstenite::Message::Binary(data),
        Message::Text(text) => tungstenite::Message::Text(text.as_str().into()),
        Message::Ping(data) => tungstenite::Message::Ping(data),
        Message::Pong(data) => tungstenite::Message::Pong(data),
        Message::Close(_) => return Err(()),
    };
    upstream.send(translated).await.map_err(|_| ())
}
