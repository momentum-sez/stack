//! # Lawpack — Statute to Machine-Readable Rules
//!
//! Compiles legislative statutes into structured compliance rules that
//! can be evaluated by the Compliance Tensor.

use serde::{Deserialize, Serialize};

use msez_core::{ContentDigest, JurisdictionId};

/// A compiled lawpack bundle containing machine-readable compliance rules
/// derived from legislative statutes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lawpack {
    /// The jurisdiction this lawpack applies to.
    pub jurisdiction: JurisdictionId,
    /// Human-readable name of the lawpack.
    pub name: String,
    /// Version string (semver).
    pub version: String,
    /// Content digest of the compiled lawpack.
    pub digest: Option<ContentDigest>,
}
