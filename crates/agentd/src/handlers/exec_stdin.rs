use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::ext::agentd::StdinRequest;

use crate::process_registry::{ProcessRegistry, StdinError};

pub async fn post(
    State(registry): State<ProcessRegistry>,
    Path(cmd_id): Path<String>,
    Json(req): Json<StdinRequest>,
) -> impl IntoResponse {
    match registry.write_stdin(&cmd_id, req.data.as_bytes()).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(StdinError::NotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(StdinError::NoStdin) => (StatusCode::GONE, "process has no stdin pipe").into_response(),
        Err(StdinError::IoError(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("stdin write failed: {e}"),
        )
            .into_response(),
    }
}
