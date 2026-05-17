use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::ext::agentd::ComputerActionRequest;

#[cfg(not(feature = "desktop"))]
pub async fn post(Json(_req): Json<ComputerActionRequest>) -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        "computer actions require desktop variant",
    )
        .into_response()
}

#[cfg(feature = "desktop")]
pub async fn post(Json(req): Json<ComputerActionRequest>) -> impl IntoResponse {
    use crate::computer;

    match computer::execute(&req).await {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(computer::ComputerError::MissingField(field)) => (
            StatusCode::BAD_REQUEST,
            format!("missing required field: {field}"),
        )
            .into_response(),
        Err(computer::ComputerError::InvalidCoordinate(x, y)) => (
            StatusCode::BAD_REQUEST,
            format!("invalid coordinate: ({x}, {y})"),
        )
            .into_response(),
        Err(computer::ComputerError::ExecutionFailed(msg)) => {
            (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
        }
    }
}
