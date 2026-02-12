//! # Licensepack — License Lifecycle Management
//!
//! Manages the lifecycle of business licenses within a jurisdiction,
//! supporting 15+ license categories for deployments like Pakistan.
//!
//! ## Implements
//!
//! Spec §15 — Licensepack structure and license lifecycle.

use serde::{Deserialize, Serialize};

/// A license pack containing license type definitions and lifecycle
/// rules for a jurisdiction.
///
/// Placeholder — full implementation will include license category
/// definitions, renewal rules, and integration with the license
/// state machine in `msez-state`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Licensepack {
    /// Human-readable name of the licensepack.
    pub name: String,
    /// Jurisdiction this licensepack applies to.
    pub jurisdiction: String,
    /// Version of the licensepack.
    pub version: String,
}
