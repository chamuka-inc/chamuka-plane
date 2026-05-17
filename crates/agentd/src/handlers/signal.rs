use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::ext::agentd::{Signal, SignalRequest};

use crate::process_registry::{ProcessRegistry, SignalError};

fn signal_to_libc(sig: Signal) -> i32 {
    match sig {
        Signal::Term => libc::SIGTERM,
        Signal::Kill => libc::SIGKILL,
        Signal::Int => libc::SIGINT,
        Signal::Hup => libc::SIGHUP,
        Signal::Usr1 => libc::SIGUSR1,
        Signal::Usr2 => libc::SIGUSR2,
    }
}

pub async fn post(
    State(registry): State<ProcessRegistry>,
    Path(cmd_id): Path<String>,
    Json(req): Json<SignalRequest>,
) -> impl IntoResponse {
    let sig = signal_to_libc(req.signal);
    match registry.signal(&cmd_id, sig).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(SignalError::NotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(SignalError::ProcessExited) => {
            (StatusCode::GONE, "process already exited").into_response()
        }
        Err(SignalError::OsError(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("signal failed: {e}"),
        )
            .into_response(),
    }
}
