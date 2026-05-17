use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use common::ext::agentd::LogFrame;
use tokio::io::AsyncReadExt;

use crate::process_registry::ProcessRegistry;

pub async fn logs(
    State(registry): State<ProcessRegistry>,
    Path(cmd_id): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let streams = match registry.take_streams(&cmd_id).await {
        Ok(s) => s,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    ws.on_upgrade(move |socket| handle_logs(socket, registry, cmd_id, streams))
        .into_response()
}

async fn handle_logs(
    mut socket: WebSocket,
    registry: ProcessRegistry,
    cmd_id: String,
    streams: crate::process_registry::ProcessStreams,
) {
    let (stdout_tx, mut stdout_rx) = tokio::sync::mpsc::channel::<LogFrame>(64);
    let stderr_tx = stdout_tx.clone();

    if let Some(mut stdout) = streams.stdout {
        let tx = stdout_tx.clone();
        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            loop {
                match stdout.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = base64_encode(&buf[..n]);
                        if tx.send(LogFrame::Stdout { data }).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }

    if let Some(mut stderr) = streams.stderr {
        let tx = stderr_tx;
        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            loop {
                match stderr.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = base64_encode(&buf[..n]);
                        if tx.send(LogFrame::Stderr { data }).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }

    drop(stdout_tx);

    let exit_task = {
        let registry = registry.clone();
        let cmd_id = cmd_id.clone();
        tokio::spawn(async move { registry.wait_exit(&cmd_id).await })
    };

    while let Some(frame) = stdout_rx.recv().await {
        let json = match serde_json::to_string(&frame) {
            Ok(j) => j,
            Err(_) => break,
        };
        if socket.send(Message::Text(json.into())).await.is_err() {
            return;
        }
    }

    if let Ok(Ok(code)) = exit_task.await {
        let exit_frame = LogFrame::Exit { code, signal: None };
        if let Ok(json) = serde_json::to_string(&exit_frame) {
            let _ = socket.send(Message::Text(json.into())).await;
        }
    }
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}
