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
