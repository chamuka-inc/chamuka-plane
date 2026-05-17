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
    use common::ext::agentd::{ComputerAction, ComputerActionResponse, ComputerOutput};

    let output = match req.action {
        ComputerAction::Screenshot => ComputerOutput::Image {
            base64: String::new(),
            media_type: "image/png".to_string(),
        },
        ComputerAction::CursorPosition => ComputerOutput::Text {
            text: "0,0".to_string(),
        },
        _ => ComputerOutput::Ok,
    };

    let resp = ComputerActionResponse { output };
    (StatusCode::OK, Json(resp)).into_response()
}
