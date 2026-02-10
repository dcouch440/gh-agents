//! Centralized API error type for structured JSON error responses.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// Structured JSON error body returned to API clients.
#[derive(Serialize)]
struct ErrorBody {
    error: String,
    status: u16,
}

/// Centralized error type for all API handlers.
///
/// Each variant maps to an HTTP status code and carries a human-readable message.
/// Implements `IntoResponse` to produce structured JSON error responses.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// 400 Bad Request — validation failures, malformed input
    #[error("{0}")]
    BadRequest(String),

    /// 401 Unauthorized — missing or invalid credentials
    #[error("{0}")]
    Unauthorized(String),

    /// 403 Forbidden — authenticated but not allowed
    #[error("{0}")]
    Forbidden(String),

    /// 404 Not Found — resource does not exist or ownership mismatch
    #[error("{0}")]
    NotFound(String),

    /// 409 Conflict — resource already exists or invalid state transition
    #[error("{0}")]
    Conflict(String),

    /// 503 Service Unavailable — downstream service unreachable
    #[error("{0}")]
    ServiceUnavailable(String),

    /// 500 Internal Server Error — unexpected failures, database errors
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    /// Resource not found by type name.
    pub fn not_found(resource: &str) -> Self {
        AppError::NotFound(format!("{resource} not found"))
    }

    /// Validation failure.
    pub fn bad_request(msg: impl Into<String>) -> Self {
        AppError::BadRequest(msg.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        match &self {
            AppError::Internal(msg) => tracing::error!("Internal error: {}", msg),
            AppError::ServiceUnavailable(msg) => tracing::warn!("Service unavailable: {}", msg),
            _ => {}
        }

        let body = ErrorBody {
            error: self.to_string(),
            status: status.as_u16(),
        };

        (status, Json(body)).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}

#[cfg(test)]
mod tests;
