//! # Smart Asset Receipt Chain Operations Test
//!
//! Tests the core receipt chain operations: appending receipts, building
//! and verifying MMR inclusion proofs, rejecting duplicate sequences, and
//! verifying prev_root linkage across the chain.

use mez_core::{ContentDigest, CorridorId, Timestamp};
use mez_corridor::{CorridorReceipt, ReceiptChain};

fn test_genesis_root() -> ContentDigest {
    ContentDigest::from_hex(&"00".repeat(32)).unwrap()
}

fn make_receipt(chain: &ReceiptChain, _i: u64) -> CorridorReceipt {
    let mut receipt = CorridorReceipt {
        receipt_type: "MEZCorridorStateReceipt".to_string(),
        corridor_id: chain.corridor_id().clone(),
        sequence: chain.height(),
        timestamp: Timestamp::now(),
        prev_root: chain.final_state_root_hex(),
        next_root: String::new(),
        lawpack_digest_set: vec!["deadbeef".repeat(8).into()],
        ruleset_digest_set: vec!["cafebabe".repeat(8).into()],
        proof: None,
        transition: None,
        transition_type_registry_digest_sha256: None,
        zk: None,
        anchor: None,
    };
    receipt.seal_next_root().unwrap();
    receipt
}

// ---------------------------------------------------------------------------
// 1. Append receipts and verify chain height
// ---------------------------------------------------------------------------

#[test]
fn receipt_chain_append_and_verify() {
    let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
    assert_eq!(chain.height(), 0);

    for i in 0..10 {
        let receipt = make_receipt(&chain, i);
        chain.append(receipt).unwrap();
    }
    assert_eq!(chain.height(), 10);
    assert_eq!(chain.receipts().len(), 10);

    // MMR root should be valid hex
    let root = chain.mmr_root().unwrap();
    assert_eq!(root.len(), 64);
    assert!(root.chars().all(|c| c.is_ascii_hexdigit()));
}

// ---------------------------------------------------------------------------
// 2. Build and verify MMR inclusion proofs
// ---------------------------------------------------------------------------

#[test]
fn receipt_chain_inclusion_proofs() {
    let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
    for i in 0..8 {
        chain.append(make_receipt(&chain, i)).unwrap();
    }

    // Verify inclusion proof for every receipt
    for idx in 0..8 {
        let proof = chain.build_inclusion_proof(idx).unwrap();
        assert!(
            chain.verify_inclusion_proof(&proof).unwrap(),
            "inclusion proof for receipt {idx} must verify"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. Reject duplicate sequence numbers
// ---------------------------------------------------------------------------

#[test]
fn receipt_chain_reject_duplicate_sequence() {
    let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
    let receipt = make_receipt(&chain, 0);
    chain.append(receipt).unwrap();

    // Create receipt with wrong sequence (should be 1, use 5)
    let mut bad_receipt = make_receipt(&chain, 1);
    bad_receipt.sequence = 5;
    assert!(chain.append(bad_receipt).is_err());
}

// ---------------------------------------------------------------------------
// 4. Prev root linkage verification
// ---------------------------------------------------------------------------

#[test]
fn receipt_chain_prev_root_linkage() {
    let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
    let receipt = make_receipt(&chain, 0);
    chain.append(receipt).unwrap();

    // Create receipt with wrong prev_root
    let mut bad_receipt = make_receipt(&chain, 1);
    bad_receipt.prev_root = "ff".repeat(32);
    assert!(chain.append(bad_receipt).is_err());
}

// ---------------------------------------------------------------------------
// 5. Receipt content digest is deterministic
// ---------------------------------------------------------------------------

#[test]
fn receipt_content_digest_deterministic() {
    let chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
    let receipt = make_receipt(&chain, 42);
    let d1 = receipt.content_digest().unwrap();
    let d2 = receipt.content_digest().unwrap();
    assert_eq!(d1, d2);
    assert_eq!(d1.to_hex().len(), 64);
}

// ---------------------------------------------------------------------------
// 6. Corridor ID is preserved across the chain
// ---------------------------------------------------------------------------

#[test]
fn receipt_chain_corridor_id_preserved() {
    let corridor_id = CorridorId::new();
    let chain = ReceiptChain::new(corridor_id.clone(), test_genesis_root());
    assert_eq!(*chain.corridor_id(), corridor_id);
}
