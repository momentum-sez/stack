//! # Receipt Chain
//!
//! Append-only corridor receipt chain backed by a Merkle Mountain Range.
//!
//! ## Security Invariant
//!
//! All receipt hashes are computed from `CanonicalBytes`. The MMR provides
//! efficient inclusion proofs for any historical receipt.
//!
//! ## Implements
//!
//! Spec §16 — Receipt chain structure.

use msez_crypto::MerkleMountainRange;

/// A corridor receipt chain backed by an MMR.
///
/// Placeholder — full implementation will append receipts, generate
/// inclusion proofs, and verify receipt membership.
#[derive(Debug)]
pub struct ReceiptChain {
    /// The underlying Merkle Mountain Range.
    pub mmr: MerkleMountainRange,
}

impl ReceiptChain {
    /// Create an empty receipt chain.
    pub fn new() -> Self {
        Self {
            mmr: MerkleMountainRange::new(),
        }
    }
}

impl Default for ReceiptChain {
    fn default() -> Self {
        Self::new()
    }
}
