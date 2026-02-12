//! # Merkle Mountain Range (MMR)
//!
//! An append-only authenticated data structure for corridor receipt chains.
//! The MMR supports efficient inclusion proofs and is used to construct
//! checkpoints that can be anchored to L1 chains.
//!
//! ## Design
//!
//! The MMR is a forest of perfect binary Merkle trees. Appending a leaf
//! may merge existing peaks. The root commitment is the hash of all peak
//! digests concatenated.

use msez_core::ContentDigest;

/// A Merkle Mountain Range for append-only receipt chain commitment.
///
/// Each leaf is a [`ContentDigest`] of a corridor receipt. The MMR
/// root can be anchored to an L1 chain for finality.
#[derive(Debug, Clone)]
pub struct MerkleMountainRange {
    /// The leaf nodes (content digests of receipts).
    _leaves: Vec<ContentDigest>,
    /// Peak digests at each tree height.
    _peaks: Vec<ContentDigest>,
}

impl MerkleMountainRange {
    /// Create an empty MMR.
    pub fn new() -> Self {
        Self {
            _leaves: Vec::new(),
            _peaks: Vec::new(),
        }
    }
}

impl Default for MerkleMountainRange {
    fn default() -> Self {
        Self::new()
    }
}

/// An inclusion proof for a leaf in the MMR.
#[derive(Debug, Clone)]
pub struct MmrInclusionProof {
    /// The index of the leaf being proved.
    pub leaf_index: u64,
    /// The sibling hashes along the path to the peak.
    pub siblings: Vec<ContentDigest>,
}
