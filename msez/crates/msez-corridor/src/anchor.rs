//! # L1 Anchoring
//!
//! Anchors corridor checkpoints to L1 chains for settlement finality.
//!
//! ## Design Decision: L1 is Optional
//!
//! The SEZ Stack operates as a self-sovereign system. L1 anchoring provides
//! additional finality guarantees but is not required for corridor operation.
//! The system functions without any blockchain dependencies — L1 anchoring
//! is an optional enhancement for environments that require on-chain
//! settlement finality.
//!
//! ## Architecture
//!
//! The [`AnchorTarget`] trait defines the interface for L1 chain adapters.
//! The trait is **sealed** — only implementations within this crate are
//! permitted. This prevents external code from creating unaudited anchor
//! targets that could compromise settlement finality assumptions.
//!
//! ## Spec Reference
//!
//! Implements the anchoring protocol from `spec/40-corridors.md` Part IV.
//! Port of `tools/phoenix/anchor.py` `ChainAdapter` protocol.

use msez_core::ContentDigest;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors from L1 anchoring operations.
#[derive(Error, Debug)]
pub enum AnchorError {
    /// The anchor target rejected the commitment.
    #[error("anchor rejected: {0}")]
    Rejected(String),

    /// The L1 chain is unavailable.
    #[error("chain unavailable: {chain_id}")]
    ChainUnavailable {
        /// The L1 chain identifier.
        chain_id: String,
    },

    /// The anchor transaction failed.
    #[error("anchor transaction failed on chain {chain_id}: {reason}")]
    TransactionFailed {
        /// The L1 chain identifier.
        chain_id: String,
        /// Failure reason.
        reason: String,
    },

    /// The anchor receipt could not be verified.
    #[error("anchor verification failed: {0}")]
    VerificationFailed(String),
}

/// A commitment to anchor a corridor checkpoint on L1.
///
/// Contains the checkpoint digest and the target L1 chain. The commitment
/// is submitted to an [`AnchorTarget`] implementation, which returns an
/// [`AnchorReceipt`] upon successful anchoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorCommitment {
    /// The checkpoint digest being anchored.
    pub checkpoint_digest: ContentDigest,
    /// The L1 chain identifier (optional — `None` means "mock/local").
    pub chain_id: Option<String>,
    /// Corridor checkpoint height at anchor time.
    pub checkpoint_height: u64,
}

/// Receipt of a successful L1 anchor operation.
///
/// Proves that a corridor checkpoint was anchored on a specific L1 chain
/// at a specific block height/transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorReceipt {
    /// The commitment that was anchored.
    pub commitment: AnchorCommitment,
    /// The L1 chain identifier where the anchor was placed.
    pub chain_id: String,
    /// The L1 transaction identifier (hash or reference).
    pub transaction_id: String,
    /// The L1 block number containing the anchor transaction.
    pub block_number: u64,
    /// Status of the anchor (pending confirmation, confirmed, finalized).
    pub status: AnchorStatus,
}

/// Status of an L1 anchor transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnchorStatus {
    /// Transaction submitted but not yet confirmed.
    Pending,
    /// Transaction confirmed but not yet finalized.
    Confirmed,
    /// Transaction finalized — irreversible.
    Finalized,
    /// Transaction failed.
    Failed,
}

/// Trait for L1 chain anchor targets.
///
/// Sealed — only implementations within this crate are permitted.
/// This prevents unaudited anchor targets from being used in
/// production, which could compromise settlement finality assumptions.
///
/// ## Security Invariant
///
/// Implementations must ensure that `anchor()` only returns `Ok` when
/// the commitment has been durably recorded on the target chain.
/// Returning `Ok` for an unanchored commitment could cause the system
/// to assume finality that does not exist.
///
/// ## Audit Reference
///
/// Per audit §5.5: anchor target trait is sealed to prevent external
/// implementations that could weaken finality guarantees.
pub trait AnchorTarget: private::Sealed {
    /// Anchor a corridor checkpoint digest to the L1 chain.
    ///
    /// Returns an [`AnchorReceipt`] on success, proving the checkpoint
    /// was durably recorded on L1.
    fn anchor(&self, commitment: AnchorCommitment) -> Result<AnchorReceipt, AnchorError>;

    /// Check the status of a previously submitted anchor.
    fn check_status(&self, transaction_id: &str) -> Result<AnchorStatus, AnchorError>;

    /// Return the chain identifier for this anchor target.
    fn chain_id(&self) -> &str;
}

mod private {
    pub trait Sealed {}
    impl Sealed for super::MockAnchorTarget {}
}

/// Mock L1 anchor target for Phase 1 development and testing.
///
/// Simulates successful anchoring without any L1 chain dependency.
/// All anchors are immediately "finalized" with deterministic
/// transaction IDs derived from the commitment digest.
///
/// ## Warning
///
/// This implementation provides NO actual L1 finality guarantees.
/// It is suitable only for development, testing, and Phase 1 deployment
/// where L1 anchoring is not yet required.
#[derive(Debug, Default)]
pub struct MockAnchorTarget {
    chain_id: String,
    next_block: std::sync::atomic::AtomicU64,
}

impl MockAnchorTarget {
    /// Create a new mock anchor target.
    pub fn new(chain_id: impl Into<String>) -> Self {
        Self {
            chain_id: chain_id.into(),
            next_block: std::sync::atomic::AtomicU64::new(1),
        }
    }
}

impl AnchorTarget for MockAnchorTarget {
    fn anchor(&self, commitment: AnchorCommitment) -> Result<AnchorReceipt, AnchorError> {
        let block = self
            .next_block
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let tx_id = format!(
            "mock-tx-{}",
            commitment.checkpoint_digest.to_hex().get(..16).unwrap_or("unknown")
        );

        Ok(AnchorReceipt {
            commitment,
            chain_id: self.chain_id.clone(),
            transaction_id: tx_id,
            block_number: block,
            status: AnchorStatus::Finalized,
        })
    }

    fn check_status(&self, _transaction_id: &str) -> Result<AnchorStatus, AnchorError> {
        Ok(AnchorStatus::Finalized)
    }

    fn chain_id(&self) -> &str {
        &self.chain_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use msez_core::{sha256_digest, CanonicalBytes};

    fn test_digest() -> ContentDigest {
        let canonical =
            CanonicalBytes::new(&serde_json::json!({"checkpoint": "test"})).unwrap();
        sha256_digest(&canonical)
    }

    #[test]
    fn mock_anchor_succeeds() {
        let target = MockAnchorTarget::new("mock-ethereum");
        let commitment = AnchorCommitment {
            checkpoint_digest: test_digest(),
            chain_id: Some("mock-ethereum".to_string()),
            checkpoint_height: 42,
        };

        let receipt = target.anchor(commitment).unwrap();
        assert_eq!(receipt.chain_id, "mock-ethereum");
        assert_eq!(receipt.status, AnchorStatus::Finalized);
        assert_eq!(receipt.block_number, 1);
        assert!(receipt.transaction_id.starts_with("mock-tx-"));
    }

    #[test]
    fn mock_anchor_increments_blocks() {
        let target = MockAnchorTarget::new("mock-eth");

        for expected_block in 1..=5 {
            let commitment = AnchorCommitment {
                checkpoint_digest: test_digest(),
                chain_id: Some("mock-eth".to_string()),
                checkpoint_height: expected_block,
            };
            let receipt = target.anchor(commitment).unwrap();
            assert_eq!(receipt.block_number, expected_block);
        }
    }

    #[test]
    fn mock_check_status_always_finalized() {
        let target = MockAnchorTarget::new("mock-eth");
        let status = target.check_status("mock-tx-abc123").unwrap();
        assert_eq!(status, AnchorStatus::Finalized);
    }

    #[test]
    fn mock_chain_id() {
        let target = MockAnchorTarget::new("arbitrum-sepolia");
        assert_eq!(target.chain_id(), "arbitrum-sepolia");
    }

    #[test]
    fn anchor_commitment_serialization() {
        let commitment = AnchorCommitment {
            checkpoint_digest: test_digest(),
            chain_id: Some("ethereum".to_string()),
            checkpoint_height: 100,
        };

        let json = serde_json::to_string(&commitment).unwrap();
        let deserialized: AnchorCommitment = serde_json::from_str(&json).unwrap();
        assert_eq!(
            deserialized.checkpoint_digest.to_hex(),
            commitment.checkpoint_digest.to_hex()
        );
        assert_eq!(deserialized.checkpoint_height, 100);
    }

    #[test]
    fn anchor_receipt_serialization() {
        let target = MockAnchorTarget::new("mock-eth");
        let commitment = AnchorCommitment {
            checkpoint_digest: test_digest(),
            chain_id: Some("mock-eth".to_string()),
            checkpoint_height: 1,
        };
        let receipt = target.anchor(commitment).unwrap();

        let json = serde_json::to_string(&receipt).unwrap();
        let deserialized: AnchorReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.block_number, receipt.block_number);
        assert_eq!(deserialized.status, AnchorStatus::Finalized);
    }
}
