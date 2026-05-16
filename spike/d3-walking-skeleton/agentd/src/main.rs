use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::{net::TcpListener, process::Command, sync::Mutex};

#[derive(Clone)]
struct AppState {
    exec_count: Arc<Mutex<u64>>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

#[derive(Deserialize)]
struct ExecRequest {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[derive(Serialize)]
struct ExecResponse {
    exit_code: i32,
    stdout: String,
    stderr: String,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: "0.0.1-spike",
    })
}

async fn exec_command(
    State(state): State<AppState>,
    Json(req): Json<ExecRequest>,
) -> Result<Json<ExecResponse>, (StatusCode, String)> {
    let mut count = state.exec_count.lock().await;
    *count += 1;
    drop(count);

    let timeout = req.timeout_ms.unwrap_or(30_000);

    let result = tokio::time::timeout(
        std::time::Duration::from_millis(timeout),
        Command::new(&req.command).args(&req.args).output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => Ok(Json(ExecResponse {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })),
        Ok(Err(e)) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("exec failed: {e}"),
        )),
        Err(_) => Err((
            StatusCode::GATEWAY_TIMEOUT,
            format!("command timed out after {timeout}ms"),
        )),
    }
}

#[tokio::main]
async fn main() {
    let state = AppState {
        exec_count: Arc::new(Mutex::new(0)),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/exec", post(exec_command))
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{port}");
    eprintln!("agentd-spike listening on {addr}");

    let listener = TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
