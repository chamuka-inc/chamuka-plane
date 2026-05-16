use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::ext::agentd::{ExecRequest, ExecSyncResponse};
use std::process::Stdio;
use std::time::Instant;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use uuid::Uuid;

pub async fn post(Json(req): Json<ExecRequest>) -> impl IntoResponse {
    let start = Instant::now();
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

    let output = match child.wait_with_output().await {
        Ok(o) => o,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("wait failed: {e}")})),
            )
                .into_response();
        }
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    let truncated = if stdout.len() > 10 * 1024 * 1024 {
        Some(true)
    } else {
        None
    };

    let resp = ExecSyncResponse {
        cmd_id,
        exit_code: output.status.code().unwrap_or(-1),
        stdout,
        stderr,
        duration_ms,
        truncated,
    };

    (StatusCode::OK, Json(resp)).into_response()
}
