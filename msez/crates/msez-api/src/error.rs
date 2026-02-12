//! # API Error Types
//!
//! Structured error type implementing `axum::response::IntoResponse`.
//! Maps domain errors from msez-state, msez-core, etc. to HTTP status codes.
//! Returns JSON error response bodies with error code, message, and details.
//! Never exposes internal error details in production responses.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;
use utoipa::ToSchema;

/// Structured JSON error response body.
///
/// All error responses use this format for consistency across the API surface.
/// The `details` field carries additional context for 422 validation errors
/// but is omitted for 500-class errors to prevent information leakage.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub error: ErrorDetail,
}

/// Inner error detail.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorDetail {
    /// Machine-readable error code (e.g., "NOT_FOUND", "VALIDATION_ERROR").
    pub code: String,
    /// Human-readable error message.
    pub message: String,
    /// Additional details, present only for client errors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Application-level error type that implements [`IntoResponse`] for Axum.
///
/// Maps domain errors to appropriate HTTP status codes and structured
/// JSON error bodies. Internal error details are never exposed to clients.
#[derive(Error, Debug)]
pub enum AppError {
    /// Resource not found (404).
    #[error("not found: {0}")]
    NotFound(String),

    /// Request validation failed (422).
    #[error("validation error: {0}")]
    Validation(String),

    /// Request body could not be parsed (400).
    #[error("bad request: {0}")]
    BadRequest(String),

    /// Authentication failure — missing or invalid token (401).
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    /// Authorization failure — insufficient permissions (403).
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// Conflict with current resource state (409).
    #[error("conflict: {0}")]
    Conflict(String),

    /// Internal server error (500). Message is logged but not returned to client.
    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// Return the HTTP status code and machine-readable error code for this error.
    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            Self::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            Self::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_ERROR"),
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            Self::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            Self::Forbidden(_) => (StatusCode::FORBIDDEN, "FORBIDDEN"),
            Self::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT"),
            Self::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = self.status_and_code();

        // Never expose internal error messages to clients.
        let message = match &self {
            Self::Internal(_) => "An internal error occurred".to_string(),
            other => other.to_string(),
        };

        // Log internal errors for operator visibility.
        if matches!(&self, Self::Internal(_)) {
            tracing::error!(error = %self, "internal server error");
        }

        let body = ErrorBody {
            error: ErrorDetail {
                code: code.to_string(),
                message,
                details: None,
            },
        };

        (status, Json(body)).into_response()
    }
}

/// Convert msez-core validation errors to API errors.
impl From<msez_core::ValidationError> for AppError {
    fn from(err: msez_core::ValidationError) -> Self {
        Self::Validation(err.to_string())
    }
}

/// Convert msez-state entity errors to API errors.
impl From<msez_state::entity::EntityError> for AppError {
    fn from(err: msez_state::entity::EntityError) -> Self {
        match &err {
            msez_state::entity::EntityError::AlreadyTerminal { .. }
            | msez_state::entity::EntityError::InvalidTransition { .. }
            | msez_state::entity::EntityError::InvalidDissolutionAdvance { .. }
            | msez_state::entity::EntityError::DissolutionComplete => {
                Self::Conflict(err.to_string())
            }
        }
    }
}
