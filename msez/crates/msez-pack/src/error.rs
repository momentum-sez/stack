//! Pack-specific error types.
//!
//! Structured errors for lawpack, regpack, and licensepack operations.
//! All errors carry context (file paths, line numbers where possible)
//! to support production debugging of sovereign infrastructure.

use std::path::PathBuf;

use thiserror::Error;

/// Errors that can occur during pack operations.
#[derive(Debug, Error)]
pub enum PackError {
    /// YAML parsing failed.
    #[error("failed to parse YAML at {path}: {source}")]
    YamlParse {
        path: PathBuf,
        source: serde_yaml::Error,
    },

    /// JSON parsing failed.
    #[error("failed to parse JSON at {path}: {source}")]
    JsonParse {
        path: PathBuf,
        source: serde_json::Error,
    },

    /// A required file was not found.
    #[error("required file not found: {path}")]
    FileNotFound { path: PathBuf },

    /// YAML manifest contains non-JSON-compatible types (floats, timestamps).
    #[error("{context}: {path}: {detail}")]
    JsonIncompatible {
        context: String,
        path: String,
        detail: String,
    },

    /// Invalid lawpack reference format.
    #[error("invalid lawpack ref {input:?}: {reason}")]
    InvalidLawpackRef { input: String, reason: String },

    /// Invalid SHA-256 digest string.
    #[error("invalid SHA-256 digest: {digest:?} (expected 64 lowercase hex chars)")]
    InvalidDigest { digest: String },

    /// Digest verification failed.
    #[error("digest mismatch for {context}: expected {expected}, got {actual}")]
    DigestMismatch {
        context: String,
        expected: String,
        actual: String,
    },

    /// Lock verification failed.
    #[error("lock verification failed: {detail}")]
    LockVerificationFailed { detail: String },

    /// Validation error.
    #[error("validation error: {0}")]
    Validation(String),

    /// Compliance domain not recognized.
    #[error("unknown compliance domain: {domain:?}")]
    UnknownDomain { domain: String },

    /// Zone manifest is invalid.
    #[error("invalid zone manifest at {path}: {detail}")]
    InvalidZoneManifest { path: PathBuf, detail: String },

    /// Canonicalization error (delegated from msez-core).
    #[error("canonicalization error: {0}")]
    Canonicalization(#[from] msez_core::CanonicalizationError),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic serde_json error (not file-specific).
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Generic serde_yaml error (not file-specific).
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

/// Result type alias for pack operations.
pub type PackResult<T> = Result<T, PackError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_not_found_display() {
        let err = PackError::FileNotFound {
            path: PathBuf::from("/tmp/missing.yaml"),
        };
        assert!(format!("{err}").contains("/tmp/missing.yaml"));
    }

    #[test]
    fn json_incompatible_display() {
        let err = PackError::JsonIncompatible {
            context: "lawpack field".to_string(),
            path: "rules[0].rate".to_string(),
            detail: "contains float".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("lawpack field"));
        assert!(msg.contains("contains float"));
    }

    #[test]
    fn invalid_lawpack_ref_display() {
        let err = PackError::InvalidLawpackRef {
            input: "bad-ref".to_string(),
            reason: "wrong format".to_string(),
        };
        assert!(format!("{err}").contains("bad-ref"));
    }

    #[test]
    fn invalid_digest_display() {
        let err = PackError::InvalidDigest {
            digest: "xyz".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("xyz"));
        assert!(msg.contains("64 lowercase hex"));
    }

    #[test]
    fn digest_mismatch_display() {
        let err = PackError::DigestMismatch {
            context: "lawpack".to_string(),
            expected: "aabb".to_string(),
            actual: "ccdd".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("aabb"));
        assert!(msg.contains("ccdd"));
    }

    #[test]
    fn lock_verification_failed_display() {
        let err = PackError::LockVerificationFailed {
            detail: "hash mismatch".to_string(),
        };
        assert!(format!("{err}").contains("hash mismatch"));
    }

    #[test]
    fn validation_display() {
        let err = PackError::Validation("empty name".to_string());
        assert!(format!("{err}").contains("empty name"));
    }

    #[test]
    fn unknown_domain_display() {
        let err = PackError::UnknownDomain {
            domain: "crypto_trading".to_string(),
        };
        assert!(format!("{err}").contains("crypto_trading"));
    }

    #[test]
    fn invalid_zone_manifest_display() {
        let err = PackError::InvalidZoneManifest {
            path: PathBuf::from("zone.yaml"),
            detail: "missing jurisdiction_id".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("zone.yaml"));
        assert!(msg.contains("missing jurisdiction_id"));
    }

    #[test]
    fn io_error_from_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = PackError::from(io_err);
        assert!(format!("{err}").contains("access denied"));
    }

    #[test]
    fn pack_result_alias_works() {
        let ok: PackResult<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: PackResult<i32> = Err(PackError::Validation("bad".to_string()));
        assert!(err.is_err());
    }

    #[test]
    fn all_variants_are_debug() {
        let err = PackError::Validation("test".to_string());
        assert!(!format!("{err:?}").is_empty());
    }
}
