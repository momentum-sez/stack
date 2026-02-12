//! # Regpack — Regulatory Requirement Sets
//!
//! Maps statutory provisions to operational compliance checks.

use serde::{Deserialize, Serialize};

use msez_core::{ContentDigest, JurisdictionId};

/// A compiled regpack containing regulatory requirement mappings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Regpack {
    /// The jurisdiction this regpack applies to.
    pub jurisdiction: JurisdictionId,
    /// Human-readable name of the regpack.
    pub name: String,
    /// Version string (semver).
    pub version: String,
    /// Content digest of the compiled regpack.
    pub digest: Option<ContentDigest>,
}
