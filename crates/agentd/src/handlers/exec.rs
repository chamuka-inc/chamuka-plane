use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use common::ext::agentd::{DetachedStatus, ExecDetachedResponse, ExecRequest, ExecSyncResponse};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use uuid::Uuid;

use crate::process_registry::ProcessRegistry;

const DEFAULT_TIMEOUT_MS: u64 = 30_000;
const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024;

pub async fn post(
    State(registry): State<ProcessRegistry>,
    Json(req): Json<ExecRequest>,
) -> Response {
    let cmd_id = Uuid::new_v4().to_string();

    let mut command = Command::new(&req.cmd);
    command.args(&req.args);

    for (key, val) in &req.env {
        command.env(key, val);
    }

    if let Some(ref cwd) = req.cwd {
        command.current_dir(cwd);
    }

    if req.stdin.is_some() {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    command.kill_on_drop(true);

    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("spawn failed: {e}")})),
            )
                .into_response();
        }
    };

    if let Some(ref stdin_data) = req.stdin {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(stdin_data.as_bytes()).await;
        }
    }

    if req.detached {
        registry.insert(cmd_id.clone(), child).await;
        let resp = ExecDetachedResponse {
            cmd_id,
            status: DetachedStatus::Running,
        };
        return (StatusCode::OK, Json(resp)).into_response();
    }

    let start = Instant::now();
    let timeout = Duration::from_millis(req.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS));

    let mut stdout_handle = child.stdout.take();
    let mut stderr_handle = child.stderr.take();

    let stdout_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        if let Some(ref mut out) = stdout_handle {
            let _ = out
                .take(MAX_OUTPUT_BYTES as u64)
                .read_to_end(&mut buf)
                .await;
        }
        buf
    });

    let stderr_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        if let Some(ref mut err) = stderr_handle {
            let _ = err
                .take(MAX_OUTPUT_BYTES as u64)
                .read_to_end(&mut buf)
                .await;
        }
        buf
    });

    match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(status)) => {
            let stdout_buf = stdout_task.await.unwrap_or_default();
            let stderr_buf = stderr_task.await.unwrap_or_default();
            let duration_ms = start.elapsed().as_millis() as u64;
            let truncated = if stdout_buf.len() > MAX_OUTPUT_BYTES {
                Some(true)
            } else {
                None
            };

            let resp = ExecSyncResponse {
                cmd_id,
                exit_code: status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&stdout_buf).into_owned(),
                stderr: String::from_utf8_lossy(&stderr_buf).into_owned(),
                duration_ms,
                truncated,
            };
            (StatusCode::OK, Json(resp)).into_response()
        }
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("wait failed: {e}")})),
        )
            .into_response(),
        Err(_elapsed) => {
            let _ = child.kill().await;
            stdout_task.abort();
            stderr_task.abort();
            let duration_ms = start.elapsed().as_millis() as u64;
            let resp = ExecSyncResponse {
                cmd_id,
                exit_code: -1,
                stdout: String::new(),
                stderr: "process killed: timeout exceeded".to_string(),
                duration_ms,
                truncated: None,
            };
            (StatusCode::OK, Json(resp)).into_response()
        }
    }
}
