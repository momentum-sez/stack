//! # Schema Validation
//!
//! Runtime validation of JSON/YAML documents against JSON Schema
//! definitions (Draft 2020-12).
//!
//! ## Security Invariant
//!
//! Schema validation is a trust boundary. Documents that fail validation
//! must be rejected with structured error information including the schema
//! path, the violating field, and the expected vs actual value.
//!
//! ## Implements
//!
//! Spec §6 — Schema contract validation rules.

use std::path::Path;
use thiserror::Error;

/// Error during schema validation.
#[derive(Error, Debug)]
pub enum SchemaValidationError {
    /// The document did not conform to the schema.
    #[error("validation failed: {message}")]
    ValidationFailed {
        /// Human-readable error message.
        message: String,
    },

    /// The schema file could not be loaded.
    #[error("schema load error: {0}")]
    SchemaLoadError(String),

    /// IO error reading schema or document.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// A schema validator backed by the `jsonschema` crate.
///
/// Placeholder — full implementation will load schemas from the
/// `schemas/` directory and validate documents against them.
#[derive(Debug)]
pub struct SchemaValidator {
    /// Root directory containing JSON schema files.
    schema_dir: std::path::PathBuf,
}

impl SchemaValidator {
    /// Create a new validator with schemas from the given directory.
    pub fn new(schema_dir: impl AsRef<Path>) -> Self {
        Self {
            schema_dir: schema_dir.as_ref().to_path_buf(),
        }
    }

    /// Returns the schema directory path.
    pub fn schema_dir(&self) -> &Path {
        &self.schema_dir
    }
}
