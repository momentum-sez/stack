//! # msez-schema — Schema Validation & Code Generation
//!
//! Provides runtime JSON Schema validation for YAML module descriptors
//! and (planned) compile-time Rust type generation from schema definitions.
//!
//! ## Runtime Validation
//!
//! The `validate` module validates JSON/YAML documents against the 116
//! JSON schemas in `schemas/`. This replaces the Python `tools/msez/schema.py`
//! functionality.
//!
//! ## Code Generation (Planned)
//!
//! The `codegen` module (via `build.rs`) will generate Rust types from
//! schema definitions at compile time, ensuring that the API surface
//! and data model cannot diverge.
//!
//! ## Crate Policy
//!
//! - Depends only on `msez-core` internally.
//! - Schema `$id` and `$ref` URIs must never be changed without verifying
//!   all references across the repository.

pub mod validate;

pub use validate::SchemaValidator;
