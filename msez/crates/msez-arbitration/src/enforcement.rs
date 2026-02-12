//! # Award Enforcement
//!
//! Enforces arbitration awards with corridor receipt generation for
//! cross-border dispute resolution.

use msez_core::ContentDigest;
use serde::{Deserialize, Serialize};

/// An enforcement order for an arbitration award.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcementOrder {
    /// Digest of the arbitration award being enforced.
    pub award_digest: ContentDigest,
    /// The enforcement action to take.
    pub action: EnforcementAction,
}

/// The type of enforcement action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnforcementAction {
    /// Transfer funds from escrow to the prevailing party.
    EscrowRelease { escrow_id: String },
    /// Suspend the respondent's operating license.
    LicenseSuspension { license_id: String },
    /// Generate a corridor receipt recording the enforcement.
    CorridorReceipt { corridor_id: String },
}
