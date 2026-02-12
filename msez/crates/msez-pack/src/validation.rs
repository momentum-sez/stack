//! # Pack Validation Rules
//!
//! Validates pack bundles against the pack schema and ensures structural
//! integrity of compiled packs.

/// Result of validating a pack bundle.
#[derive(Debug)]
pub struct PackValidationResult {
    /// Whether the pack is structurally valid.
    pub is_valid: bool,
    /// Validation errors, if any.
    pub errors: Vec<String>,
}
