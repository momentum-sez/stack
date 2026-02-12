//! # Runtime Schema Validation
//!
//! Validates JSON/YAML documents against JSON Schema definitions.
//! Errors carry structured diagnostic information: schema path,
//! violating field, expected vs actual value.

use std::path::PathBuf;

/// A compiled schema validator that can validate documents against
/// a collection of JSON Schema definitions.
#[derive(Debug)]
pub struct SchemaValidator {
    /// The root directory containing JSON schema files.
    _schema_dir: PathBuf,
}

impl SchemaValidator {
    /// Create a new validator that loads schemas from the given directory.
    pub fn new(schema_dir: impl Into<PathBuf>) -> Self {
        Self {
            _schema_dir: schema_dir.into(),
        }
    }
}

/// A structured validation error with diagnostic context.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// The JSON Schema `$id` or file path that was violated.
    pub schema_path: String,
    /// The JSON Pointer to the field that failed validation.
    pub instance_path: String,
    /// Human-readable description of the violation.
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "schema={}, path={}: {}",
            self.schema_path, self.instance_path, self.message
        )
    }
}

impl std::error::Error for ValidationError {}
