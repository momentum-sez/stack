//! # Smart Asset Anchoring and Verification Test
//!
//! Tests the L1 anchor protocol using the MockAnchorTarget. Verifies that
//! commitments can be anchored and receipts carry valid fields including
//! chain IDs, block numbers, and deterministic transaction IDs derived
//! from the checkpoint digest.

use msez_core::{sha256_digest, CanonicalBytes, ContentDigest};
use msez_corridor::{AnchorCommitment, AnchorTarget, MockAnchorTarget};
use serde_json::json;

fn test_digest(label: &str) -> ContentDigest {
    let canonical = CanonicalBytes::new(&json!({"anchor_test": label})).unwrap();
    sha256_digest(&canonical)
}

// ---------------------------------------------------------------------------
// 1. Anchor commitment and verify receipt
// ---------------------------------------------------------------------------

#[test]
fn anchor_commitment_and_verify() {
    let target = MockAnchorTarget::new("mock-ethereum");
    let commitment = AnchorCommitment {
        checkpoint_digest: test_digest("checkpoint-001"),
        chain_id: Some("mock-ethereum".to_string()),
        checkpoint_height: 42,
    };

    let receipt = target.anchor(commitment).unwrap();
    assert_eq!(receipt.chain_id, "mock-ethereum");
    assert_eq!(
        receipt.status,
        msez_corridor::anchor::AnchorStatus::Finalized
    );

    // Verify receipt via check_status
    let status = target.check_status(&receipt.transaction_id).unwrap();
    assert_eq!(status, msez_corridor::anchor::AnchorStatus::Finalized);
}

// ---------------------------------------------------------------------------
// 2. Mock anchor produces receipt with correct fields
// ---------------------------------------------------------------------------

#[test]
fn mock_anchor_produces_receipt() {
    let target = MockAnchorTarget::new("mock-polygon");
    let commitment = AnchorCommitment {
        checkpoint_digest: test_digest("asset-registry"),
        chain_id: Some("mock-polygon".to_string()),
        checkpoint_height: 100,
    };

    let receipt = target.anchor(commitment).unwrap();
    assert_eq!(receipt.chain_id, "mock-polygon");
    assert!(receipt.transaction_id.starts_with("mock-tx-"));
    assert_eq!(receipt.block_number, 1);
    assert_eq!(receipt.commitment.checkpoint_height, 100);
}

// ---------------------------------------------------------------------------
// 3. Anchor receipt has valid fields and increments blocks
// ---------------------------------------------------------------------------

#[test]
fn anchor_receipt_has_valid_fields() {
    let target = MockAnchorTarget::new("mock-arbitrum");

    for expected_block in 1..=5 {
        let commitment = AnchorCommitment {
            checkpoint_digest: test_digest(&format!("block-{expected_block}")),
            chain_id: Some("mock-arbitrum".to_string()),
            checkpoint_height: expected_block,
        };
        let receipt = target.anchor(commitment).unwrap();
        assert_eq!(receipt.block_number, expected_block);
        assert_eq!(receipt.chain_id, "mock-arbitrum");
        assert!(!receipt.transaction_id.is_empty());
    }
}

// ---------------------------------------------------------------------------
// 4. Anchor commitment serialization roundtrip
// ---------------------------------------------------------------------------

#[test]
fn anchor_commitment_serde_roundtrip() {
    let commitment = AnchorCommitment {
        checkpoint_digest: test_digest("serde-test"),
        chain_id: Some("ethereum".to_string()),
        checkpoint_height: 999,
    };

    let json_str = serde_json::to_string(&commitment).unwrap();
    let deserialized: AnchorCommitment = serde_json::from_str(&json_str).unwrap();
    assert_eq!(
        deserialized.checkpoint_digest.to_hex(),
        commitment.checkpoint_digest.to_hex()
    );
    assert_eq!(deserialized.checkpoint_height, 999);
}

// ---------------------------------------------------------------------------
// 5. Chain ID is accessible via trait method
// ---------------------------------------------------------------------------

#[test]
fn chain_id_accessible() {
    let target = MockAnchorTarget::new("arbitrum-sepolia");
    assert_eq!(target.chain_id(), "arbitrum-sepolia");
}
