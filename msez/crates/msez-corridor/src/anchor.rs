//! # L1 Anchoring
//!
//! Anchors corridor checkpoints to L1 chains for finality.
//! L1 is optional — the system operates without blockchain dependencies.

use msez_core::ContentDigest;

/// A commitment to anchor a corridor checkpoint on L1.
#[derive(Debug, Clone)]
pub struct AnchorCommitment {
    /// The checkpoint digest being anchored.
    pub checkpoint_digest: ContentDigest,
    /// The L1 chain identifier (optional).
    pub chain_id: Option<String>,
}
