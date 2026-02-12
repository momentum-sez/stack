//! # Smart Asset Registry VC
//!
//! A Verifiable Credential that attests to a smart asset's registration,
//! compliance evaluation, and ownership within a jurisdiction.

use serde::{Deserialize, Serialize};

use msez_core::{ContentDigest, EntityId, JurisdictionId};

/// A Smart Asset Registry Verifiable Credential.
///
/// Attests that a smart asset has been registered within a jurisdiction
/// and has passed compliance evaluation across the required domains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartAssetRegistryVc {
    /// The entity that owns the smart asset.
    pub owner: EntityId,
    /// The jurisdiction where the asset is registered.
    pub jurisdiction: JurisdictionId,
    /// The content digest of the asset's compliance tensor commitment.
    pub compliance_commitment: ContentDigest,
    /// The asset classification type.
    pub asset_type: String,
}
