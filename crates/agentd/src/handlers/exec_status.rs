use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::ext::agentd::{ExecStatus, ExecStatusResponse};

use crate::process_registry::{ProcessRegistry, ProcessStatus, SignalError};

pub async fn get(
    State(registry): State<ProcessRegistry>,
    Path(cmd_id): Path<String>,
) -> impl IntoResponse {
    match registry.status(&cmd_id).await {
        Ok(ProcessStatus::Running) => {
            let resp = ExecStatusResponse {
                cmd_id,
                status: ExecStatus::Running,
                exit_code: None,
            };
            (StatusCode::OK, Json(resp)).into_response()
        }
        Ok(ProcessStatus::Exited(code)) => {
            let resp = ExecStatusResponse {
                cmd_id,
                status: ExecStatus::Exited,
                exit_code: Some(code),
            };
            (StatusCode::OK, Json(resp)).into_response()
        }
        Err(SignalError::NotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("status check failed: {e}"),
        )
            .into_response(),
    }
}
