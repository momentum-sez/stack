//! # msez-schema — Schema Validation & Code Generation
//!
//! Provides runtime JSON Schema validation for YAML module descriptors
//! and security-critical schema analysis for the SEZ Stack.
//!
//! ## Runtime Validation (`validate`)
//!
//! The [`validate`] module loads all 113 JSON schemas from the `schemas/`
//! directory, registers them for cross-schema `$ref` resolution, and
//! validates JSON/YAML documents against them. Key function:
//!
//! - [`SchemaValidator::validate_module`] — validates a YAML module
//!   descriptor against `module.schema.json`, matching the behavior
//!   of `msez validate --all-modules` from the Python CLI.
//!
//! ## Security Schema Analysis (`codegen`)
//!
//! The [`codegen`] module identifies security-critical schemas (VCs,
//! receipts, attestations, proofs) and audits their `additionalProperties`
//! settings per audit finding §3.1. Phase 1 is runtime analysis;
//! Phase 2 will add compile-time Rust type generation via `build.rs`.
//!
//! ## Crate Policy
//!
//! - Depends only on `msez-core` internally.
//! - Schema `$id` and `$ref` URIs must never be changed without verifying
//!   all references across the repository.
//! - Schema validation is a trust boundary: invalid documents are rejected
//!   with structured errors including path, field, and expected-vs-actual.

pub mod codegen;
pub mod validate;

pub use codegen::{audit_additional_properties, SecuritySchemaSpec, SECURITY_CRITICAL_SCHEMAS};
pub use validate::{SchemaValidationError, SchemaValidator, ValidationViolations, Violation};
