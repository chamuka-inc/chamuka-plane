use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use common::ext::agentd::LogFrame;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;

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

pub async fn stdout_stream(
    State(registry): State<ProcessRegistry>,
    Path(cmd_id): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let streams = match registry.take_streams(&cmd_id).await {
        Ok(s) => s,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    ws.on_upgrade(move |socket| {
        handle_single_stream(socket, registry, cmd_id, streams.stdout, make_stdout_frame)
    })
    .into_response()
}

pub async fn stderr_stream(
    State(registry): State<ProcessRegistry>,
    Path(cmd_id): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let streams = match registry.take_streams(&cmd_id).await {
        Ok(s) => s,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    ws.on_upgrade(move |socket| {
        handle_single_stream(socket, registry, cmd_id, streams.stderr, make_stderr_frame)
    })
    .into_response()
}

fn make_stdout_frame(data: String) -> LogFrame {
    LogFrame::Stdout { data }
}

fn make_stderr_frame(data: String) -> LogFrame {
    LogFrame::Stderr { data }
}

async fn handle_single_stream<R: AsyncReadExt + Unpin + Send + 'static>(
    mut socket: WebSocket,
    registry: ProcessRegistry,
    cmd_id: String,
    reader: Option<R>,
    make_frame: fn(String) -> LogFrame,
) {
    if let Some(mut reader) = reader {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let frame = make_frame(base64_encode(&buf[..n]));
                    let json = match serde_json::to_string(&frame) {
                        Ok(j) => j,
                        Err(_) => break,
                    };
                    if socket.send(Message::Text(json.into())).await.is_err() {
                        return;
                    }
                }
                Err(_) => break,
            }
        }
    }

    if let Ok(code) = registry.wait_exit(&cmd_id).await {
        let exit_frame = LogFrame::Exit { code, signal: None };
        if let Ok(json) = serde_json::to_string(&exit_frame) {
            let _ = socket.send(Message::Text(json.into())).await;
        }
    }
}

fn pump_reader<R: AsyncReadExt + Unpin + Send + 'static>(
    mut reader: R,
    make_frame: fn(String) -> LogFrame,
    tx: mpsc::Sender<LogFrame>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let frame = make_frame(base64_encode(&buf[..n]));
                    if tx.send(frame).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    })
}

async fn handle_logs(
    mut socket: WebSocket,
    registry: ProcessRegistry,
    cmd_id: String,
    streams: crate::process_registry::ProcessStreams,
) {
    let (tx, mut rx) = mpsc::channel::<LogFrame>(64);

    if let Some(stdout) = streams.stdout {
        pump_reader(stdout, make_stdout_frame, tx.clone());
    }
    if let Some(stderr) = streams.stderr {
        pump_reader(stderr, make_stderr_frame, tx.clone());
    }
    drop(tx);

    while let Some(frame) = rx.recv().await {
        let json = match serde_json::to_string(&frame) {
            Ok(j) => j,
            Err(_) => break,
        };
        if socket.send(Message::Text(json.into())).await.is_err() {
            return;
        }
    }

    if let Ok(code) = registry.wait_exit(&cmd_id).await {
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
