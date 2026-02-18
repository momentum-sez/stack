//! # mez-schema — Schema Validation & Codegen
//!
//! This crate provides runtime JSON Schema validation for YAML module
//! descriptors and (in the future) compile-time Rust type generation
//! from schema definitions via `build.rs`.
//!
//! ## Responsibilities
//!
//! - **Runtime validation:** Validate YAML module descriptors against
//!   the 116 JSON schemas in `schemas/` using Draft 2020-12. Internal
//!   `$ref` URIs (`https://schemas.momentum-ez.org/mez/...`) are
//!   resolved against a pre-loaded schema registry.
//!
//! - **Security policy checks:** Verify that security-critical schemas
//!   have `additionalProperties: false` at envelope levels per audit §3.1.
//!
//! - **Codegen (Phase 1 — runtime only):** The [`codegen`] module
//!   documents which schemas require `additionalProperties: false` and
//!   provides runtime policy checks. A future phase will add compile-time
//!   Rust type generation via a `build.rs` script.
//!
//! ## Design
//!
//! The [`SchemaValidator`] loads all `*.schema.json` files at construction
//! time, builds a URI → schema map for `$ref` resolution via the
//! `jsonschema` crate's `Retrieve` trait, and provides typed validation
//! methods for modules, zones, and profiles.
//!
//! Validation errors are structured via [`SchemaValidationError`] with
//! the schema `$id`, the JSON Pointer path to the violating field, and
//! a human-readable message.

pub mod codegen;
pub mod validate;

// Re-export primary types for ergonomic imports.
pub use validate::{
    ModuleFailure, ModuleValidationReport, SchemaValidationDetail, SchemaValidationError,
    SchemaValidator, ValidationError,
};

pub use codegen::{
    check_additional_properties_policy, AdditionalPropertiesViolation, EXTENSIBLE_PATHS,
    SECURITY_CRITICAL_SCHEMAS,
};
