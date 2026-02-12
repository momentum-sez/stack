//! # Merkle Mountain Range (MMR)
//!
//! An append-only authenticated data structure used for corridor receipt
//! chains. The MMR provides efficient proofs of inclusion for any historical
//! receipt without requiring the full chain.
//!
//! ## Security Invariant
//!
//! All leaf hashes are computed from `CanonicalBytes` via `sha256_digest()`.
//! Internal nodes are computed by hashing the concatenation of child digests.
//!
//! ## Implements
//!
//! Spec §16 — Receipt chain structure and inclusion proofs.

use msez_core::ContentDigest;

/// A Merkle Mountain Range for append-only commitment chains.
///
/// Placeholder — full implementation will store peaks, compute
/// inclusion proofs, and verify membership.
#[derive(Debug, Clone)]
pub struct MerkleMountainRange {
    /// The peaks of the MMR (one per complete binary tree).
    peaks: Vec<ContentDigest>,
    /// Total number of leaves appended.
    leaf_count: u64,
}

impl MerkleMountainRange {
    /// Create an empty MMR.
    pub fn new() -> Self {
        Self {
            peaks: Vec::new(),
            leaf_count: 0,
        }
    }

    /// Returns the number of leaves in the MMR.
    pub fn leaf_count(&self) -> u64 {
        self.leaf_count
    }

    /// Returns the current peaks of the MMR.
    pub fn peaks(&self) -> &[ContentDigest] {
        &self.peaks
    }
}

impl Default for MerkleMountainRange {
    fn default() -> Self {
        Self::new()
    }
}
