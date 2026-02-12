//! # Error Hierarchy
//!
//! Structured error types for the entire SEZ Stack, built with `thiserror`.
//! No `Box<dyn Error>`, no `.unwrap()` outside tests.
//!
//! Each subsystem defines specific error variants that carry diagnostic context:
//! the operation that failed, the state at the time of failure, and actionable
//! information for operators.

use thiserror::Error;

/// Top-level error type for the SEZ Stack.
#[derive(Error, Debug)]
pub enum MsezError {
    /// Canonicalization failure during digest computation.
    #[error("canonicalization error: {0}")]
    Canonicalization(#[from] CanonicalizationError),

    /// State machine transition violation.
    #[error("state transition error: {0}")]
    StateTransition(#[from] StateTransitionError),

    /// Domain primitive validation failure.
    #[error("validation error: {0}")]
    Validation(#[from] ValidationError),

    /// Schema validation failure.
    #[error("schema validation error: {0}")]
    SchemaValidation(String),

    /// Cryptographic operation failure.
    #[error("cryptographic error: {0}")]
    Cryptographic(String),

    /// Integrity violation in content-addressed storage.
    #[error("integrity error: {0}")]
    Integrity(String),

    /// Security policy violation.
    #[error("security violation: {0}")]
    Security(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Errors during canonical serialization.
#[derive(Error, Debug)]
pub enum CanonicalizationError {
    /// Float values are not permitted in canonical representations.
    /// Amounts must be strings or integers.
    #[error("float values are not permitted in canonical representations; use string or integer for amounts: {0}")]
    FloatRejected(f64),

    /// JSON serialization failed during canonicalization.
    #[error("serialization failed: {0}")]
    SerializationFailed(#[from] serde_json::Error),
}

/// Errors during state machine transitions.
#[derive(Error, Debug)]
pub enum StateTransitionError {
    /// The attempted transition is not valid from the current state.
    #[error("invalid transition from {from} to {to}: {reason}")]
    InvalidTransition {
        /// The current state name.
        from: String,
        /// The attempted target state name.
        to: String,
        /// Human-readable reason for the rejection.
        reason: String,
    },

    /// A migration exceeded its deadline.
    #[error("migration {migration_id} exceeded deadline at state {state}")]
    MigrationTimeout {
        /// The migration identifier.
        migration_id: String,
        /// The state the migration was in when the deadline expired.
        state: String,
    },

    /// Required evidence was not provided for a transition.
    #[error("missing evidence for transition from {from} to {to}: {evidence_type}")]
    MissingEvidence {
        /// The current state name.
        from: String,
        /// The target state name.
        to: String,
        /// Description of the missing evidence.
        evidence_type: String,
    },
}

/// Validation errors for domain primitive newtypes.
///
/// Each identifier type enforces format constraints at construction time.
/// These errors carry the invalid input and the expected format so that
/// operators can diagnose misconfiguration without guesswork.
#[derive(Error, Debug)]
pub enum ValidationError {
    /// DID does not conform to W3C DID syntax (did:method:identifier).
    #[error("invalid DID format: \"{0}\" (expected did:<method>:<identifier>)")]
    InvalidDid(String),

    /// CNIC does not conform to Pakistan NADRA format (13 digits).
    #[error("invalid CNIC format: \"{0}\" (expected 13 digits, optionally as XXXXX-XXXXXXX-X)")]
    InvalidCnic(String),

    /// NTN does not conform to Pakistan FBR format (7-digit number).
    #[error("invalid NTN format: \"{0}\" (expected 7-digit number)")]
    InvalidNtn(String),

    /// Passport number fails basic format validation.
    #[error("invalid passport number: \"{0}\" (expected 5-20 alphanumeric characters)")]
    InvalidPassportNumber(String),

    /// Jurisdiction identifier is empty.
    #[error("invalid jurisdiction ID: must be non-empty")]
    InvalidJurisdictionId,

    /// Timestamp string is not valid UTC ISO 8601.
    #[error("invalid timestamp: \"{value}\" ({reason})")]
    InvalidTimestamp {
        /// The string that failed to parse.
        value: String,
        /// Why it was rejected.
        reason: String,
    },
}
