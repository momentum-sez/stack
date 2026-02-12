//! # Error Types — Structured Error Hierarchy
//!
//! Defines the error types used throughout the SEZ Stack. All errors use
//! `thiserror` for derive-based `Display` and `Error` implementations.
//!
//! ## Design
//!
//! - Cryptographic errors fail loudly with full context.
//! - Schema validation errors include the schema path, violating field,
//!   and expected vs actual values.
//! - State machine errors include the current state, attempted transition,
//!   and rejection reason.

use thiserror::Error;

/// Top-level error type for the SEZ Stack.
#[derive(Error, Debug)]
pub enum MsezError {
    /// Canonicalization failed.
    #[error("canonicalization error: {0}")]
    Canonicalization(#[from] CanonicalizationError),

    /// Content integrity violation.
    #[error("integrity error: {0}")]
    Integrity(String),

    /// Security policy violation.
    #[error("security violation: {0}")]
    Security(String),

    /// State machine transition rejected.
    #[error("invalid state transition: {0}")]
    InvalidTransition(String),

    /// Schema validation failure.
    #[error("schema validation error: {0}")]
    SchemaValidation(String),

    /// Serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// IO error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Error during canonical serialization.
#[derive(Error, Debug)]
pub enum CanonicalizationError {
    /// Float values are not permitted in canonical representations.
    /// Amounts must be strings or integers.
    #[error("float values are not permitted in canonical representations; use string or integer for amount: {0}")]
    FloatRejected(f64),

    /// JSON serialization failed.
    #[error("serialization failed: {0}")]
    SerializationFailed(#[from] serde_json::Error),
}

/// Error in cryptographic operations.
#[derive(Error, Debug)]
pub enum CryptoError {
    /// Signature verification failed.
    #[error("signature verification failed: {0}")]
    VerificationFailed(String),

    /// Key generation or parsing failed.
    #[error("key error: {0}")]
    KeyError(String),

    /// Digest computation failed.
    #[error("digest error: {0}")]
    DigestError(String),
}

/// Error in state machine transitions.
#[derive(Error, Debug)]
pub enum StateError {
    /// Attempted an invalid state transition.
    #[error("invalid transition from {from} to {to}: {reason}")]
    InvalidTransition {
        /// Current state name.
        from: String,
        /// Attempted target state name.
        to: String,
        /// Reason the transition was rejected.
        reason: String,
    },

    /// Migration deadline exceeded.
    #[error("migration {migration_id} exceeded deadline at state {state}")]
    MigrationTimeout {
        /// The migration that timed out.
        migration_id: String,
        /// The state the migration was in when it timed out.
        state: String,
    },
}
