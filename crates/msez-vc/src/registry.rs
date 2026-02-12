//! # Smart Asset Registry VC
//!
//! The credential type used to assert compliance evaluation results
//! for smart assets.
//!
//! ## Implements
//!
//! Spec §14 — Smart Asset Registry credential structure.

use serde::{Deserialize, Serialize};

/// A Smart Asset Registry Verifiable Credential.
///
/// Placeholder — full implementation will include asset metadata,
/// compliance evaluation results, and anchor verification data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartAssetRegistryVc {
    /// The asset type being registered.
    pub asset_type: String,
    /// The jurisdiction of registration.
    pub jurisdiction: String,
}
