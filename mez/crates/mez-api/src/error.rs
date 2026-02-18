//! # API Error Types
//!
//! Structured error type implementing `axum::response::IntoResponse`.
//! Maps domain errors from mez-state, mez-core, etc. to HTTP status codes.
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

    /// Request body could not be parsed or contains invalid values (422).
    ///
    /// Normalized with `Validation` to 422 Unprocessable Entity (BUG-038):
    /// the client sent syntactically valid HTTP but semantically invalid
    /// content. Both JSON deserialization failures and business-rule
    /// violations are 422 — only malformed HTTP framing is 400.
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

    /// Mass API returned an error or is unreachable (502).
    #[error("upstream Mass API error: {0}")]
    UpstreamError(String),

    /// Service dependency not configured (503).
    #[error("service unavailable: {0}")]
    ServiceUnavailable(String),

    /// Feature not yet implemented in this proxy layer (501).
    #[error("not implemented: {0}")]
    NotImplemented(String),
}

impl AppError {
    /// Return the HTTP status code and machine-readable error code for this error.
    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            Self::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            Self::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_ERROR"),
            Self::BadRequest(_) => (StatusCode::UNPROCESSABLE_ENTITY, "BAD_REQUEST"),
            Self::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            Self::Forbidden(_) => (StatusCode::FORBIDDEN, "FORBIDDEN"),
            Self::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT"),
            Self::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
            Self::UpstreamError(_) => (StatusCode::BAD_GATEWAY, "UPSTREAM_ERROR"),
            Self::ServiceUnavailable(_) => (StatusCode::SERVICE_UNAVAILABLE, "SERVICE_UNAVAILABLE"),
            Self::NotImplemented(_) => (StatusCode::NOT_IMPLEMENTED, "NOT_IMPLEMENTED"),
        }
    }
}

impl AppError {
    /// Construct an upstream error (502 Bad Gateway).
    pub fn upstream(msg: String) -> Self {
        Self::UpstreamError(msg)
    }

    /// Construct a service unavailable error (503).
    pub fn service_unavailable(msg: &str) -> Self {
        Self::ServiceUnavailable(msg.to_string())
    }

    /// Construct a not-found error (404).
    pub fn not_found(msg: String) -> Self {
        Self::NotFound(msg)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = self.status_and_code();

        // Never expose internal/upstream error messages to clients.
        let message = match &self {
            Self::Internal(_) => "An internal error occurred".to_string(),
            Self::UpstreamError(_) => "An upstream service error occurred".to_string(),
            other => other.to_string(),
        };

        // Log server-side errors for operator visibility.
        match &self {
            Self::Internal(_) => tracing::error!(error = %self, "internal server error"),
            Self::UpstreamError(_) => tracing::error!(error = %self, "upstream API error"),
            Self::ServiceUnavailable(_) => tracing::warn!(error = %self, "service unavailable"),
            Self::NotImplemented(_) => tracing::info!(error = %self, "not implemented"),
            _ => {}
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

/// Convert mez-core validation errors to API errors.
impl From<mez_core::ValidationError> for AppError {
    fn from(err: mez_core::ValidationError) -> Self {
        Self::Validation(err.to_string())
    }
}

/// Convert mez-state entity errors to API errors.
impl From<mez_state::entity::EntityError> for AppError {
    fn from(err: mez_state::entity::EntityError) -> Self {
        match &err {
            mez_state::entity::EntityError::AlreadyTerminal { .. }
            | mez_state::entity::EntityError::InvalidTransition { .. }
            | mez_state::entity::EntityError::InvalidDissolutionAdvance { .. }
            | mez_state::entity::EntityError::DissolutionComplete => {
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
        // BUG-038: BadRequest now returns 422 (same as Validation) since the
        // client sent syntactically valid HTTP but semantically invalid content.
        let err = AppError::BadRequest("malformed JSON".to_string());
        let (status, code) = err.status_and_code();
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
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
    fn upstream_error_status_code() {
        let err = AppError::UpstreamError("Mass API timeout".to_string());
        let (status, code) = err.status_and_code();
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(code, "UPSTREAM_ERROR");
    }

    #[test]
    fn service_unavailable_status_code() {
        let err = AppError::ServiceUnavailable("Mass client not configured".to_string());
        let (status, code) = err.status_and_code();
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(code, "SERVICE_UNAVAILABLE");
    }

    #[test]
    fn not_implemented_status_code() {
        let err = AppError::NotImplemented("update entity".to_string());
        let (status, code) = err.status_and_code();
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
        assert_eq!(code, "NOT_IMPLEMENTED");
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
        let entity_err = mez_state::entity::EntityError::DissolutionComplete;
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
    async fn response_parts(err: AppError) -> (StatusCode, ErrorBody) {
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
        // BUG-038: BadRequest now returns 422 Unprocessable Entity.
        let (status, body) = response_parts(AppError::BadRequest("malformed".into())).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
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
    fn validation_error_from_mez_core() {
        // Verify the From<mez_core::ValidationError> impl produces Validation variant.
        let core_err = mez_core::ValidationError::InvalidDid("bad:did".to_string());
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
        let entity_err = mez_state::entity::EntityError::AlreadyTerminal {
            id: mez_core::EntityId::new(),
            state: mez_state::entity::EntityLifecycleState::Dissolved,
        };
        let app_err = AppError::from(entity_err);
        let (status, _) = app_err.status_and_code();
        assert_eq!(status, StatusCode::CONFLICT);
    }

    #[test]
    fn entity_error_invalid_transition_converts_to_conflict() {
        let entity_err = mez_state::entity::EntityError::InvalidTransition {
            from: mez_state::entity::EntityLifecycleState::Active,
            to: mez_state::entity::EntityLifecycleState::Applied,
            reason: "cannot return to Applied".to_string(),
        };
        let app_err = AppError::from(entity_err);
        let (status, code) = app_err.status_and_code();
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(code, "CONFLICT");
    }
}
