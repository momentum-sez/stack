//! # Evidence Package Management
//!
//! Content-addressed evidence packages with chain-of-custody tracking.

use msez_core::ContentDigest;
use serde::{Deserialize, Serialize};

/// An evidence package submitted in a dispute proceeding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidencePackage {
    /// Content digest of the evidence bundle.
    pub digest: ContentDigest,
    /// Description of the evidence.
    pub description: String,
    /// The party that submitted this evidence.
    pub submitted_by: String,
}
