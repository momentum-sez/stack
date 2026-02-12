//! # Regpack — Regulatory Requirement Sets
//!
//! Encodes regulatory requirements that entities must satisfy within
//! a jurisdiction, independent of the underlying statutes.
//!
//! ## Implements
//!
//! Spec §11 — Regpack structure and regulatory mapping.

use serde::{Deserialize, Serialize};

/// A regulatory requirement pack containing operational rules
/// for entity compliance within a jurisdiction.
///
/// Placeholder — full implementation will include requirement
/// definitions, applicability rules, and compliance evaluation hooks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Regpack {
    /// Human-readable name of the regpack.
    pub name: String,
    /// Jurisdiction this regpack applies to.
    pub jurisdiction: String,
    /// Version of the regpack.
    pub version: String,
}
