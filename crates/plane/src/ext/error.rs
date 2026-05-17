use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use plane_common::util::random_string;
use serde::Serialize;

use crate::database::connect::ConnectError;

#[derive(Debug, Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
    request_id: String,
}

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "INVALID_REQUEST",
            message: message.into(),
            details: None,
        }
    }

    pub fn unauthenticated(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "UNAUTHENTICATED",
            message: message.into(),
            details: None,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "NOT_FOUND",
            message: message.into(),
            details: None,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "INTERNAL",
            message: message.into(),
            details: None,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let request_id = random_string();

        if self.status.is_server_error() {
            tracing::error!(
                request_id = %request_id,
                code = %self.code,
                message = %self.message,
                "Server error in /pub/v1 handler"
            );
        }

        let body = ErrorEnvelope {
            error: ErrorBody {
                code: self.code,
                message: self.message,
                details: self.details,
                request_id,
            },
        };

        (self.status, Json(body)).into_response()
    }
}

impl From<ConnectError> for ApiError {
    fn from(err: ConnectError) -> Self {
        match &err {
            ConnectError::FailedToAcquireKey
            | ConnectError::KeyHeld { .. }
            | ConnectError::KeyHeldUnhealthy
            | ConnectError::KeyUnheldNoSpawnConfig => Self {
                status: StatusCode::CONFLICT,
                code: "KEY_HELD",
                message: err.to_string(),
                details: None,
            },
            ConnectError::NoDroneAvailable => Self {
                status: StatusCode::SERVICE_UNAVAILABLE,
                code: "NO_DRONE_AVAILABLE",
                message: "No active drone available for the requested pool.".to_string(),
                details: None,
            },
            ConnectError::NoClusterProvided => Self {
                status: StatusCode::BAD_REQUEST,
                code: "INVALID_REQUEST",
                message: "No cluster provided and no default cluster configured.".to_string(),
                details: None,
            },
            ConnectError::DatabaseError(_)
            | ConnectError::Serialization(_)
            | ConnectError::FailedToRemoveKey
            | ConnectError::Other(_) => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "INTERNAL",
                message: "Internal error.".to_string(),
                details: None,
            },
        }
    }
}
