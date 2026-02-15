//! # Corridor Receipt Chain
//!
//! Append-only corridor receipts backed by MMR for efficient inclusion proofs.
//!
//! ## Design
//!
//! Each corridor maintains a receipt chain — an ordered sequence of
//! [`CorridorReceipt`] objects, each representing a state transition event.
//! Receipts are content-addressed: each receipt contains a `prev_root` (the
//! chain's MMR root before this receipt) and a `next_root` (the payload's
//! canonical digest). The MMR accumulator provides O(log n) inclusion proofs
//! without disclosing the full receipt set.
//!
//! ## Integrity Model
//!
//! 1. Each receipt is canonicalized via [`CanonicalBytes`] and digested via
//!    [`sha256_digest`] to produce the `next_root`.
//! 2. The `next_root` is appended to the MMR.
//! 3. Checkpoints snapshot the MMR root at periodic intervals for anchoring.
//! 4. Inclusion proofs verify that a specific receipt is part of the chain.
//!
//! ## Spec Reference
//!
//! Implements receipt chain per `spec/40-corridors.md` Part IV.
//! Matches `schemas/corridor.receipt.schema.json`.
//! Port of `tools/phoenix/bridge.py` `BridgeReceiptChain` class.

use msez_core::{sha256_digest, CanonicalBytes, ContentDigest, CorridorId, Timestamp};
use msez_crypto::mmr::{
    build_inclusion_proof, verify_inclusion_proof, MerkleMountainRange, MmrInclusionProof,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors specific to receipt chain operations.
#[derive(Error, Debug)]
pub enum ReceiptError {
    /// Receipt sequence number does not follow the chain.
    #[error("sequence mismatch: expected {expected}, got {actual} for corridor {corridor_id}")]
    SequenceMismatch {
        /// The expected next sequence number.
        expected: u64,
        /// The actual sequence number provided.
        actual: u64,
        /// The corridor this receipt belongs to.
        corridor_id: String,
    },

    /// Receipt prev_root does not match the current chain root.
    #[error("prev_root mismatch for receipt #{sequence}: expected {expected}, got {actual}")]
    PrevRootMismatch {
        /// The expected prev_root (current MMR root).
        expected: String,
        /// The actual prev_root in the receipt.
        actual: String,
        /// The receipt sequence number.
        sequence: u64,
    },

    /// MMR operation failed.
    #[error("MMR error: {0}")]
    Mmr(#[from] msez_crypto::CryptoError),

    /// Canonicalization failed.
    #[error("canonicalization error: {0}")]
    Canonicalization(#[from] msez_core::CanonicalizationError),

    /// Inclusion proof verification failed.
    #[error("inclusion proof verification failed for receipt #{leaf_index}")]
    InclusionProofFailed {
        /// The leaf index that failed verification.
        leaf_index: usize,
    },

    /// Attempted to build proof for empty chain.
    #[error("cannot build inclusion proof for empty receipt chain")]
    EmptyChain,
}

/// A corridor receipt recording a cross-border transaction event.
///
/// Matches the structure defined in `schemas/corridor.receipt.schema.json`.
/// Each receipt forms a link in the append-only receipt chain, with
/// `prev_root` and `next_root` providing hash chain integrity.
///
/// ## Security Invariant
///
/// The `next_root` is the canonical digest of the receipt payload,
/// computed via `CanonicalBytes::new()` → `sha256_digest()`. This ensures
/// all digests use JCS-compatible canonicalization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CorridorReceipt {
    /// Receipt type discriminator.
    #[serde(rename = "type")]
    pub receipt_type: String,
    /// The corridor this receipt belongs to.
    pub corridor_id: CorridorId,
    /// Sequence number within the corridor (0-indexed).
    pub sequence: u64,
    /// When the receipt was created.
    pub timestamp: Timestamp,
    /// MMR root before this receipt was appended.
    pub prev_root: String,
    /// Canonical digest of this receipt's payload (64 hex chars).
    pub next_root: String,
    /// Lawpack digest set governing this receipt.
    pub lawpack_digest_set: Vec<String>,
    /// Ruleset digest set governing this receipt.
    pub ruleset_digest_set: Vec<String>,
}

impl CorridorReceipt {
    /// Compute the canonical content digest of this receipt.
    ///
    /// Uses the `CanonicalBytes` → `sha256_digest` pipeline to ensure
    /// JCS-compatible canonicalization.
    pub fn content_digest(&self) -> Result<ContentDigest, ReceiptError> {
        let canonical = CanonicalBytes::new(self)?;
        Ok(sha256_digest(&canonical))
    }
}

/// A checkpoint capturing the MMR root at a specific height.
///
/// Checkpoints are periodic snapshots of the receipt chain's MMR root,
/// used for L1 anchoring and cross-corridor verification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    /// The corridor this checkpoint belongs to.
    pub corridor_id: CorridorId,
    /// The receipt chain height at checkpoint time.
    pub height: u64,
    /// The MMR root hash at checkpoint time (64 hex chars).
    pub mmr_root: String,
    /// When the checkpoint was created.
    pub timestamp: Timestamp,
    /// Digest of the checkpoint content for anchoring.
    pub checkpoint_digest: ContentDigest,
}

/// An append-only receipt chain for a single corridor, backed by MMR.
///
/// Maintains the ordered sequence of receipts and the MMR accumulator
/// for efficient inclusion proofs.
///
/// ## Security Invariant
///
/// Receipts can only be appended (not modified or removed). Each receipt's
/// `prev_root` must match the current MMR root, and its sequence number
/// must be exactly `chain.height()`. This ensures fork detection: any
/// deviation from these invariants indicates a fork.
#[derive(Debug)]
pub struct ReceiptChain {
    /// The corridor this chain belongs to.
    corridor_id: CorridorId,
    /// Ordered receipts.
    receipts: Vec<CorridorReceipt>,
    /// The MMR accumulator tracking next_root digests.
    mmr: MerkleMountainRange,
    /// Hex-encoded next_roots for inclusion proof building.
    next_roots: Vec<String>,
    /// Periodic checkpoints.
    checkpoints: Vec<Checkpoint>,
}

impl ReceiptChain {
    /// Create a new empty receipt chain for a corridor.
    pub fn new(corridor_id: CorridorId) -> Self {
        Self {
            corridor_id,
            receipts: Vec::new(),
            mmr: MerkleMountainRange::new(),
            next_roots: Vec::new(),
            checkpoints: Vec::new(),
        }
    }

    /// Return the corridor ID this chain belongs to.
    pub fn corridor_id(&self) -> &CorridorId {
        &self.corridor_id
    }

    /// Return the current chain height (number of receipts).
    pub fn height(&self) -> u64 {
        self.receipts.len() as u64
    }

    /// Return the current MMR root hash (64 hex chars, or empty if no receipts).
    pub fn mmr_root(&self) -> Result<String, ReceiptError> {
        Ok(self.mmr.root()?)
    }

    /// Access the receipts in the chain.
    pub fn receipts(&self) -> &[CorridorReceipt] {
        &self.receipts
    }

    /// Access the checkpoints.
    pub fn checkpoints(&self) -> &[Checkpoint] {
        &self.checkpoints
    }

    /// Append a receipt to the chain.
    ///
    /// Validates:
    /// 1. Sequence number matches expected (chain height).
    /// 2. `prev_root` matches current MMR root.
    ///
    /// After validation, the receipt's `next_root` is appended to the MMR.
    ///
    /// ## Security Invariant
    ///
    /// Enforces append-only semantics and chain integrity. Any violation
    /// of sequence or prev_root indicates a potential fork.
    pub fn append(&mut self, receipt: CorridorReceipt) -> Result<(), ReceiptError> {
        let expected_seq = self.height();
        if receipt.sequence != expected_seq {
            return Err(ReceiptError::SequenceMismatch {
                expected: expected_seq,
                actual: receipt.sequence,
                corridor_id: self.corridor_id.to_string(),
            });
        }

        let current_root = self.mmr_root()?;
        if receipt.prev_root != current_root {
            return Err(ReceiptError::PrevRootMismatch {
                expected: current_root,
                actual: receipt.prev_root.clone(),
                sequence: receipt.sequence,
            });
        }

        self.mmr.append(&receipt.next_root)?;
        self.next_roots.push(receipt.next_root.clone());
        self.receipts.push(receipt);
        Ok(())
    }

    /// Create a checkpoint at the current chain height.
    ///
    /// The checkpoint captures the MMR root and chain height, producing
    /// a content-addressed digest suitable for L1 anchoring.
    pub fn create_checkpoint(&mut self) -> Result<Checkpoint, ReceiptError> {
        let root = self.mmr_root()?;
        let height = self.height();

        let checkpoint_data = serde_json::json!({
            "corridor_id": self.corridor_id,
            "height": height,
            "mmr_root": root,
        });
        let canonical = CanonicalBytes::new(&checkpoint_data)?;
        let digest = sha256_digest(&canonical);

        let checkpoint = Checkpoint {
            corridor_id: self.corridor_id.clone(),
            height,
            mmr_root: root,
            timestamp: Timestamp::now(),
            checkpoint_digest: digest,
        };

        self.checkpoints.push(checkpoint.clone());
        Ok(checkpoint)
    }

    /// Build an MMR inclusion proof for a receipt at the given index.
    ///
    /// The proof demonstrates that the receipt's `next_root` is included
    /// in the MMR at the claimed root, without revealing other receipts.
    pub fn build_inclusion_proof(
        &self,
        leaf_index: usize,
    ) -> Result<MmrInclusionProof, ReceiptError> {
        if self.next_roots.is_empty() {
            return Err(ReceiptError::EmptyChain);
        }
        Ok(build_inclusion_proof(&self.next_roots, leaf_index)?)
    }

    /// Verify an MMR inclusion proof against the current chain state.
    ///
    /// Returns `true` if the proof is valid and the root matches the
    /// current MMR root.
    pub fn verify_inclusion_proof(&self, proof: &MmrInclusionProof) -> Result<bool, ReceiptError> {
        let current_root = self.mmr_root()?;
        if proof.root != current_root {
            return Ok(false);
        }
        Ok(verify_inclusion_proof(proof))
    }
}

/// Standalone receipt verification: given a proof, verify its internal
/// consistency without requiring the full chain.
///
/// This is used by light clients and regulators who receive a proof
/// and root commitment but do not have access to the full receipt chain.
pub fn verify_receipt_proof(proof: &MmrInclusionProof) -> bool {
    verify_inclusion_proof(proof)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_next_root(i: u64) -> String {
        let data = serde_json::json!({"payload": i});
        let canonical = CanonicalBytes::new(&data).unwrap();
        sha256_digest(&canonical).to_hex()
    }

    fn make_receipt(chain: &ReceiptChain, i: u64) -> CorridorReceipt {
        let next_root = make_next_root(i);
        let prev_root = chain.mmr_root().unwrap();
        CorridorReceipt {
            receipt_type: "MSEZCorridorStateReceipt".to_string(),
            corridor_id: chain.corridor_id().clone(),
            sequence: chain.height(),
            timestamp: Timestamp::now(),
            prev_root,
            next_root,
            lawpack_digest_set: vec!["deadbeef".repeat(8)],
            ruleset_digest_set: vec!["cafebabe".repeat(8)],
        }
    }

    #[test]
    fn empty_chain() {
        let chain = ReceiptChain::new(CorridorId::new());
        assert_eq!(chain.height(), 0);
        assert_eq!(chain.mmr_root().unwrap(), "");
    }

    #[test]
    fn append_single_receipt() {
        let mut chain = ReceiptChain::new(CorridorId::new());
        let receipt = make_receipt(&chain, 1);
        chain.append(receipt).unwrap();
        assert_eq!(chain.height(), 1);
        assert!(!chain.mmr_root().unwrap().is_empty());
    }

    #[test]
    fn append_multiple_receipts() {
        let mut chain = ReceiptChain::new(CorridorId::new());
        for i in 0..10 {
            let receipt = make_receipt(&chain, i);
            chain.append(receipt).unwrap();
        }
        assert_eq!(chain.height(), 10);
        assert_eq!(chain.mmr_root().unwrap().len(), 64);
    }

    #[test]
    fn sequence_mismatch_rejected() {
        let mut chain = ReceiptChain::new(CorridorId::new());
        let receipt = make_receipt(&chain, 1);
        chain.append(receipt).unwrap();

        let mut bad_receipt = make_receipt(&chain, 2);
        bad_receipt.sequence = 5;
        let result = chain.append(bad_receipt);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ReceiptError::SequenceMismatch {
                expected: 1,
                actual: 5,
                ..
            }
        ));
    }

    #[test]
    fn prev_root_mismatch_rejected() {
        let mut chain = ReceiptChain::new(CorridorId::new());
        let receipt = make_receipt(&chain, 1);
        chain.append(receipt).unwrap();

        let mut bad_receipt = make_receipt(&chain, 2);
        bad_receipt.prev_root = "00".repeat(32);
        let result = chain.append(bad_receipt);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ReceiptError::PrevRootMismatch { .. }
        ));
    }

    #[test]
    fn checkpoint_captures_state() {
        let mut chain = ReceiptChain::new(CorridorId::new());
        for i in 0..5 {
            let receipt = make_receipt(&chain, i);
            chain.append(receipt).unwrap();
        }

        let checkpoint = chain.create_checkpoint().unwrap();
        assert_eq!(checkpoint.height, 5);
        assert_eq!(checkpoint.mmr_root, chain.mmr_root().unwrap());
        assert_eq!(checkpoint.checkpoint_digest.to_hex().len(), 64);
    }

    #[test]
    fn inclusion_proof_roundtrip() {
        let mut chain = ReceiptChain::new(CorridorId::new());
        for i in 0..10 {
            let receipt = make_receipt(&chain, i);
            chain.append(receipt).unwrap();
        }

        for idx in [0, 1, 4, 7, 9] {
            let proof = chain.build_inclusion_proof(idx).unwrap();
            assert!(chain.verify_inclusion_proof(&proof).unwrap());
            assert!(verify_receipt_proof(&proof));
        }
    }

    #[test]
    fn tampered_proof_fails() {
        let mut chain = ReceiptChain::new(CorridorId::new());
        for i in 0..5 {
            let receipt = make_receipt(&chain, i);
            chain.append(receipt).unwrap();
        }

        let mut proof = chain.build_inclusion_proof(2).unwrap();
        if !proof.path.is_empty() {
            proof.path[0].hash = "00".repeat(32);
        }
        assert!(!verify_receipt_proof(&proof));
    }

    #[test]
    fn empty_chain_proof_fails() {
        let chain = ReceiptChain::new(CorridorId::new());
        assert!(matches!(
            chain.build_inclusion_proof(0),
            Err(ReceiptError::EmptyChain)
        ));
    }

    #[test]
    fn receipt_content_digest_deterministic() {
        let chain = ReceiptChain::new(CorridorId::new());
        let receipt = make_receipt(&chain, 1);
        let d1 = receipt.content_digest().unwrap();
        let d2 = receipt.content_digest().unwrap();
        assert_eq!(d1, d2);
        assert_eq!(d1.to_hex().len(), 64);
    }

    #[test]
    fn multiple_checkpoints() {
        let mut chain = ReceiptChain::new(CorridorId::new());

        for i in 0..3 {
            let receipt = make_receipt(&chain, i);
            chain.append(receipt).unwrap();
        }
        let cp1 = chain.create_checkpoint().unwrap();

        for i in 3..7 {
            let receipt = make_receipt(&chain, i);
            chain.append(receipt).unwrap();
        }
        let cp2 = chain.create_checkpoint().unwrap();

        assert_eq!(cp1.height, 3);
        assert_eq!(cp2.height, 7);
        assert_ne!(cp1.mmr_root, cp2.mmr_root);
        assert_eq!(chain.checkpoints().len(), 2);
    }
}
