//! # msez-pack — Pack Trilogy
//!
//! Implements the three foundational pack types that encode jurisdictional
//! configuration as machine-readable artifacts:
//!
//! - **Lawpack** (`lawpack.rs`): Translates legislative statutes into
//!   machine-readable compliance rules. Includes JCS canonicalization
//!   and content-addressed artifact generation.
//!
//! - **Regpack** (`regpack.rs`): Encodes regulatory requirement sets —
//!   the operational rules that entities must follow within a jurisdiction.
//!
//! - **Licensepack** (`licensepack.rs`): Manages the lifecycle of business
//!   licenses, tracking 15+ license categories for jurisdictions like Pakistan.
//!
//! ## Strengths Preserved from Python
//!
//! The Pack Trilogy is the most complete open-source implementation of
//! machine-readable jurisdictional configuration in existence. This Rust
//! port preserves the exact semantics while gaining compile-time validation
//! of pack structures via `serde` derive.
//!
//! ## Crate Policy
//!
//! - Depends only on `msez-core` internally.
//! - Pack artifacts must produce byte-identical content-addressed digests
//!   to the Python implementation for the same input data.

pub mod lawpack;
pub mod licensepack;
pub mod regpack;
pub mod validation;

pub use lawpack::Lawpack;
pub use licensepack::Licensepack;
pub use regpack::Regpack;
