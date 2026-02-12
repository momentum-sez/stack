//! # API Error Types
//!
//! Structured error type implementing `axum::response::IntoResponse`.
//! Maps domain errors from msez-state, msez-core, etc. to HTTP status codes.
//! Returns JSON error response bodies with error code, message, and details.
//! Never exposes internal error details in production responses.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

/// Structured JSON error response body.
///
/// All error responses use this format for consistency across the API surface.
/// The `details` field carries additional context for 422 validation errors
/// but is omitted for 500-class errors to prevent information leakage.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorBody {
    pub error: ErrorDetail,
}

/// Inner error detail.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_status_code() {
        let err = AppError::NotFound("missing entity".to_string());
        let (status, code) = err.status_and_code();
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(code, "NOT_FOUND");
    }

    #[test]
    fn validation_status_code() {
        let err = AppError::Validation("bad field".to_string());
        let (status, code) = err.status_and_code();
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(code, "VALIDATION_ERROR");
    }

    #[test]
    fn bad_request_status_code() {
        let err = AppError::BadRequest("malformed JSON".to_string());
        let (status, code) = err.status_and_code();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(code, "BAD_REQUEST");
    }

    #[test]
    fn unauthorized_status_code() {
        let err = AppError::Unauthorized("no token".to_string());
        let (status, code) = err.status_and_code();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(code, "UNAUTHORIZED");
    }

    #[test]
    fn forbidden_status_code() {
        let err = AppError::Forbidden("insufficient scope".to_string());
        let (status, code) = err.status_and_code();
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(code, "FORBIDDEN");
    }

    #[test]
    fn conflict_status_code() {
        let err = AppError::Conflict("entity already exists".to_string());
        let (status, code) = err.status_and_code();
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(code, "CONFLICT");
    }

    #[test]
    fn internal_status_code() {
        let err = AppError::Internal("db connection failed".to_string());
        let (status, code) = err.status_and_code();
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(code, "INTERNAL_ERROR");
    }

    #[test]
    fn error_display_messages() {
        assert!(format!("{}", AppError::NotFound("x".into())).contains("x"));
        assert!(format!("{}", AppError::Validation("y".into())).contains("y"));
        assert!(format!("{}", AppError::BadRequest("z".into())).contains("z"));
        assert!(format!("{}", AppError::Unauthorized("a".into())).contains("a"));
        assert!(format!("{}", AppError::Forbidden("b".into())).contains("b"));
        assert!(format!("{}", AppError::Conflict("c".into())).contains("c"));
        assert!(format!("{}", AppError::Internal("d".into())).contains("d"));
    }

    #[test]
    fn entity_error_converts_to_conflict() {
        let entity_err = msez_state::entity::EntityError::DissolutionComplete;
        let app_err = AppError::from(entity_err);
        let (status, _) = app_err.status_and_code();
        assert_eq!(status, StatusCode::CONFLICT);
    }

    #[test]
    fn error_body_serializes() {
        let body = ErrorBody {
            error: ErrorDetail {
                code: "TEST".to_string(),
                message: "test message".to_string(),
                details: None,
            },
        };
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("TEST"));
        assert!(json.contains("test message"));
        assert!(!json.contains("details")); // skipped when None
    }

    #[test]
    fn error_body_with_details_serializes() {
        let body = ErrorBody {
            error: ErrorDetail {
                code: "VALIDATION_ERROR".to_string(),
                message: "bad input".to_string(),
                details: Some(serde_json::json!({"field": "name"})),
            },
        };
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("details"));
        assert!(json.contains("name"));
    }

    // ── into_response tests ──────────────────────────────────────

    use http_body_util::BodyExt;

    /// Helper to extract status and body from a Response.
    async fn response_parts(
        err: AppError,
    ) -> (StatusCode, ErrorBody) {
        let response = err.into_response();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body: ErrorBody = serde_json::from_slice(&bytes).unwrap();
        (status, body)
    }

    #[tokio::test]
    async fn into_response_not_found() {
        let (status, body) = response_parts(AppError::NotFound("entity 123".into())).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.error.code, "NOT_FOUND");
        assert!(body.error.message.contains("entity 123"));
        assert!(body.error.details.is_none());
    }

    #[tokio::test]
    async fn into_response_validation() {
        let (status, body) = response_parts(AppError::Validation("bad field".into())).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body.error.code, "VALIDATION_ERROR");
        assert!(body.error.message.contains("bad field"));
    }

    #[tokio::test]
    async fn into_response_bad_request() {
        let (status, body) = response_parts(AppError::BadRequest("malformed".into())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.error.code, "BAD_REQUEST");
        assert!(body.error.message.contains("malformed"));
    }

    #[tokio::test]
    async fn into_response_unauthorized() {
        let (status, body) = response_parts(AppError::Unauthorized("no token".into())).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.error.code, "UNAUTHORIZED");
        assert!(body.error.message.contains("no token"));
    }

    #[tokio::test]
    async fn into_response_forbidden() {
        let (status, body) = response_parts(AppError::Forbidden("nope".into())).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body.error.code, "FORBIDDEN");
        assert!(body.error.message.contains("nope"));
    }

    #[tokio::test]
    async fn into_response_conflict() {
        let (status, body) = response_parts(AppError::Conflict("already exists".into())).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.error.code, "CONFLICT");
        assert!(body.error.message.contains("already exists"));
    }

    #[tokio::test]
    async fn into_response_internal_hides_details() {
        let (status, body) =
            response_parts(AppError::Internal("db connection failed".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.error.code, "INTERNAL_ERROR");
        // The internal error message must NOT appear in the response body.
        assert!(
            !body.error.message.contains("db connection"),
            "internal error details must not leak: {}",
            body.error.message
        );
        assert_eq!(body.error.message, "An internal error occurred");
        assert!(body.error.details.is_none());
    }

    #[test]
    fn validation_error_from_msez_core() {
        // Verify the From<msez_core::ValidationError> impl produces Validation variant.
        let core_err = msez_core::ValidationError::InvalidDid("bad:did".to_string());
        let app_err = AppError::from(core_err);
        match &app_err {
            AppError::Validation(msg) => {
                assert!(msg.contains("bad:did"), "got: {msg}");
            }
            other => panic!("expected Validation, got: {other:?}"),
        }
    }

    #[test]
    fn entity_error_already_terminal_converts_to_conflict() {
        let entity_err = msez_state::entity::EntityError::AlreadyTerminal {
            id: msez_core::EntityId::new(),
            state: msez_state::entity::EntityLifecycleState::Dissolved,
        };
        let app_err = AppError::from(entity_err);
        let (status, _) = app_err.status_and_code();
        assert_eq!(status, StatusCode::CONFLICT);
    }

    #[test]
    fn entity_error_invalid_transition_converts_to_conflict() {
        let entity_err = msez_state::entity::EntityError::InvalidTransition {
            from: msez_state::entity::EntityLifecycleState::Active,
            to: msez_state::entity::EntityLifecycleState::Applied,
            reason: "cannot return to Applied".to_string(),
        };
        let app_err = AppError::from(entity_err);
        let (status, code) = app_err.status_and_code();
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(code, "CONFLICT");
    }
}
