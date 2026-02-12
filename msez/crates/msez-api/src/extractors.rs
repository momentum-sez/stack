//! # Custom Extractors & Validation
//!
//! Provides the [`Validate`] trait for request DTOs and a helper
//! to extract + validate JSON bodies in handlers.

use axum::extract::rejection::JsonRejection;
use axum::Json;

use crate::error::AppError;

/// Trait for request types that can validate their business rules
/// beyond what serde deserialization checks.
pub trait Validate {
    /// Validate business rules. Returns an error message on failure.
    fn validate(&self) -> Result<(), String>;
}

/// Extract a JSON body, mapping deserialization errors to [`AppError::BadRequest`].
///
/// This is the primary extraction helper. Handlers should use:
/// ```ignore
/// async fn handler(body: Result<Json<T>, JsonRejection>) -> Result<..., AppError> {
///     let req = extract_json(body)?;
///     // use req...
/// }
/// ```
pub fn extract_json<T>(result: Result<Json<T>, JsonRejection>) -> Result<T, AppError> {
    result
        .map(|Json(v)| v)
        .map_err(|err| AppError::BadRequest(err.body_text()))
}

/// Extract a JSON body and validate it using the [`Validate`] trait.
///
/// Combines deserialization error mapping with business rule validation.
pub fn extract_validated_json<T: Validate>(
    result: Result<Json<T>, JsonRejection>,
) -> Result<T, AppError> {
    let value = extract_json(result)?;
    value.validate().map_err(AppError::Validation)?;
    Ok(value)
}
