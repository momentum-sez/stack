//! # msez-pack — The Pack Trilogy
//!
//! The Pack Trilogy is the most complete open-source implementation of
//! machine-readable jurisdictional configuration in existence. It provides:
//!
//! - **Lawpack** ([`lawpack`]): Compiles statutes (e.g., Income Tax Ordinance
//!   2001, Sales Tax Act 1990) into machine-readable compliance rules.
//!
//! - **Regpack** ([`regpack`]): Defines regulatory requirement sets that
//!   map statutory provisions to operational compliance checks.
//!
//! - **Licensepack** ([`licensepack`]): Manages the full lifecycle of
//!   business licenses, professional certifications, and regulatory
//!   authorizations (15+ categories for Pakistan deployment).
//!
//! ## Data Format
//!
//! Packs are stored as YAML files with content-addressed digests computed
//! via [`CanonicalBytes`](msez_core::CanonicalBytes). The parser validates
//! YAML structure against the pack schema and produces strongly-typed
//! Rust structs.

pub mod lawpack;
pub mod licensepack;
pub mod regpack;
pub mod validation;

// Re-export primary types.
pub use lawpack::Lawpack;
pub use licensepack::Licensepack;
pub use regpack::Regpack;
