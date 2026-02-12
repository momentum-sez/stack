//! # Licensepack — License Lifecycle Management
//!
//! Manages 15+ license categories required for Pakistan deployment,
//! including manufacturing, trading, and professional certifications.

use serde::{Deserialize, Serialize};

use msez_core::{ContentDigest, JurisdictionId};

/// A compiled licensepack containing license category definitions
/// and lifecycle rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Licensepack {
    /// The jurisdiction this licensepack applies to.
    pub jurisdiction: JurisdictionId,
    /// Human-readable name of the licensepack.
    pub name: String,
    /// Version string (semver).
    pub version: String,
    /// Content digest of the compiled licensepack.
    pub digest: Option<ContentDigest>,
}
