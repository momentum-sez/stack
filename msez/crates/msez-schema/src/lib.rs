//! # msez-schema — Schema Validation & Codegen
//!
//! This crate provides runtime JSON Schema validation for YAML module
//! descriptors and (in the future) compile-time Rust type generation
//! from schema definitions via `build.rs`.
//!
//! ## Responsibilities
//!
//! - **Runtime validation:** Validate YAML module descriptors against
//!   the 116 JSON schemas in `schemas/`.
//! - **Codegen (planned):** Generate strongly-typed Rust structs from
//!   JSON Schema definitions at compile time via a `build.rs` script.
//!
//! ## Design
//!
//! The validator loads schemas from the filesystem and caches compiled
//! validators. Validation errors include the schema path, the violating
//! field, and the expected vs actual value.

pub mod validate;

// Re-export primary types.
pub use validate::{SchemaValidator, ValidationError};
