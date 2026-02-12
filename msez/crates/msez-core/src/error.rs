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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msez_error_canonicalization_display() {
        let inner = CanonicalizationError::FloatRejected(1.5);
        let err = MsezError::Canonicalization(inner);
        let msg = format!("{err}");
        assert!(msg.contains("canonicalization error"));
    }

    #[test]
    fn msez_error_state_transition_display() {
        let inner = StateTransitionError::InvalidTransition {
            from: "DRAFT".to_string(),
            to: "ACTIVE".to_string(),
            reason: "missing evidence".to_string(),
        };
        let err = MsezError::StateTransition(inner);
        let msg = format!("{err}");
        assert!(msg.contains("DRAFT"));
        assert!(msg.contains("ACTIVE"));
    }

    #[test]
    fn msez_error_validation_display() {
        let inner = ValidationError::InvalidDid("bad:did".to_string());
        let err = MsezError::Validation(inner);
        assert!(format!("{err}").contains("bad:did"));
    }

    #[test]
    fn msez_error_schema_validation_display() {
        let err = MsezError::SchemaValidation("missing field".to_string());
        assert!(format!("{err}").contains("missing field"));
    }

    #[test]
    fn msez_error_cryptographic_display() {
        let err = MsezError::Cryptographic("bad key".to_string());
        assert!(format!("{err}").contains("bad key"));
    }

    #[test]
    fn msez_error_integrity_display() {
        let err = MsezError::Integrity("digest mismatch".to_string());
        assert!(format!("{err}").contains("digest mismatch"));
    }

    #[test]
    fn msez_error_security_display() {
        let err = MsezError::Security("injection attempt".to_string());
        assert!(format!("{err}").contains("injection attempt"));
    }

    #[test]
    fn canonicalization_error_float_rejected() {
        let err = CanonicalizationError::FloatRejected(3.14);
        let msg = format!("{err}");
        assert!(msg.contains("float values are not permitted"));
        assert!(msg.contains("3.14"));
    }

    #[test]
    fn state_transition_error_migration_timeout() {
        let err = StateTransitionError::MigrationTimeout {
            migration_id: "mig-001".to_string(),
            state: "IN_TRANSIT".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("mig-001"));
        assert!(msg.contains("IN_TRANSIT"));
    }

    #[test]
    fn state_transition_error_missing_evidence() {
        let err = StateTransitionError::MissingEvidence {
            from: "PENDING".to_string(),
            to: "ACTIVE".to_string(),
            evidence_type: "regulatory_approval".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("regulatory_approval"));
    }

    #[test]
    fn validation_error_invalid_cnic() {
        let err = ValidationError::InvalidCnic("123".to_string());
        assert!(format!("{err}").contains("123"));
        assert!(format!("{err}").contains("13 digits"));
    }

    #[test]
    fn validation_error_invalid_ntn() {
        let err = ValidationError::InvalidNtn("abc".to_string());
        assert!(format!("{err}").contains("abc"));
    }

    #[test]
    fn validation_error_invalid_passport() {
        let err = ValidationError::InvalidPassportNumber("!".to_string());
        assert!(format!("{err}").contains("alphanumeric"));
    }

    #[test]
    fn validation_error_invalid_jurisdiction_id() {
        let err = ValidationError::InvalidJurisdictionId;
        assert!(format!("{err}").contains("non-empty"));
    }

    #[test]
    fn validation_error_invalid_timestamp() {
        let err = ValidationError::InvalidTimestamp {
            value: "not-a-date".to_string(),
            reason: "parse failed".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("not-a-date"));
        assert!(msg.contains("parse failed"));
    }

    #[test]
    fn all_error_types_are_debug() {
        let e1 = MsezError::Security("test".to_string());
        let e2 = CanonicalizationError::FloatRejected(0.0);
        let e3 = StateTransitionError::MigrationTimeout {
            migration_id: "x".to_string(),
            state: "y".to_string(),
        };
        let e4 = ValidationError::InvalidJurisdictionId;
        assert!(!format!("{e1:?}").is_empty());
        assert!(!format!("{e2:?}").is_empty());
        assert!(!format!("{e3:?}").is_empty());
        assert!(!format!("{e4:?}").is_empty());
    }
}
