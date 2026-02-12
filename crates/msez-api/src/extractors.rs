//! # Custom Extractors
//!
//! Type-safe request extractors for validated JSON bodies,
//! query parameters, and path parameters.

/// Placeholder for custom extractors.
///
/// Full implementation will include:
/// - `ValidatedJson<T>` — JSON body extraction with validation
/// - `ValidatedQuery<T>` — Query parameter extraction with validation
pub struct ValidatedJson<T>(pub T);
