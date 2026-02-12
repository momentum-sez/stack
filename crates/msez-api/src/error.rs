//! # Application Error
//!
//! Maps domain errors to structured HTTP responses with proper
//! status codes and error bodies.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

/// Application-level error type that maps to HTTP responses.
#[derive(Error, Debug)]
pub enum AppError {
    /// Resource not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Request validation failed.
    #[error("validation error: {0}")]
    Validation(String),

    /// Authentication required.
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    /// Insufficient permissions.
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// Internal server error.
    #[error("internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = serde_json::json!({
            "error": {
                "code": status.as_u16(),
                "message": self.to_string(),
            }
        });
        (status, axum::Json(body)).into_response()
    }
}
