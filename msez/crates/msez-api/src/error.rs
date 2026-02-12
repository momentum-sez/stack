//! # API Error Types
//!
//! Maps domain errors to HTTP responses with structured error bodies.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

/// Application-level error type that implements [`IntoResponse`] for Axum.
#[derive(Error, Debug)]
pub enum AppError {
    /// Resource not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Request validation failed.
    #[error("validation error: {0}")]
    Validation(String),

    /// Authentication/authorization failure.
    #[error("unauthorized: {0}")]
    Unauthorized(String),

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
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, self.to_string()).into_response()
    }
}
