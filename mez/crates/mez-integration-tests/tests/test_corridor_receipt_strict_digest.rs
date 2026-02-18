//! # Strict Digest Computation on Corridor Receipts
//!
//! Tests that corridor receipt digests are computed strictly via the JCS
//! canonicalization pipeline, producing deterministic results. Verifies that
//! receipt content changes alter the digest, that receipt chain roots are
//! deterministic, and that sequence ordering is enforced.

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

#[test]
fn receipt_digest_is_deterministic() {
    let chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
    let receipt = make_receipt(&chain, 42);

    let digest_1 = receipt.content_digest().unwrap();
    let digest_2 = receipt.content_digest().unwrap();

    assert_eq!(digest_1, digest_2);
    assert_eq!(digest_1.to_hex().len(), 64);
}

#[test]
fn receipt_digest_changes_with_content() {
    let chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());

    let mut receipt_a = CorridorReceipt {
        receipt_type: "MEZCorridorStateReceipt".to_string(),
        corridor_id: chain.corridor_id().clone(),
        sequence: 0,
        timestamp: Timestamp::now(),
        prev_root: chain.final_state_root_hex(),
        next_root: String::new(),
        lawpack_digest_set: vec!["aa".repeat(32).into()],
        ruleset_digest_set: vec!["bb".repeat(32).into()],
        proof: None,
        transition: None,
        transition_type_registry_digest_sha256: None,
        zk: None,
        anchor: None,
    };
    receipt_a.seal_next_root().unwrap();

    let mut receipt_b = CorridorReceipt {
        receipt_type: "MEZCorridorStateReceipt".to_string(),
        corridor_id: chain.corridor_id().clone(),
        sequence: 0,
        timestamp: Timestamp::now(),
        prev_root: chain.final_state_root_hex(),
        next_root: String::new(),
        lawpack_digest_set: vec!["cc".repeat(32).into()],
        ruleset_digest_set: vec!["dd".repeat(32).into()],
        proof: None,
        transition: None,
        transition_type_registry_digest_sha256: None,
        zk: None,
        anchor: None,
    };
    receipt_b.seal_next_root().unwrap();

    let digest_a = receipt_a.content_digest().unwrap();
    let digest_b = receipt_b.content_digest().unwrap();

    assert_ne!(
        digest_a, digest_b,
        "receipts with different content must have different digests"
    );
}

#[test]
fn receipt_chain_root_deterministic() {
    // Build two identical chains and verify they produce the same MMR root
    let corridor_id = CorridorId::new();

    let mut chain_1 = ReceiptChain::new(corridor_id.clone(), test_genesis_root());
    let mut chain_2 = ReceiptChain::new(corridor_id, test_genesis_root());

    for i in 0..5 {
        let receipt_1 = make_receipt(&chain_1, i);
        let receipt_2 = make_receipt(&chain_2, i);
        chain_1.append(receipt_1).unwrap();
        chain_2.append(receipt_2).unwrap();
    }

    assert_eq!(chain_1.mmr_root().unwrap(), chain_2.mmr_root().unwrap());
}

#[test]
fn receipt_sequence_enforced() {
    let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());

    // Append first receipt (sequence 0)
    let receipt_0 = make_receipt(&chain, 0);
    chain.append(receipt_0).unwrap();
    assert_eq!(chain.height(), 1);

    // Try to append with wrong sequence
    let mut bad_receipt = make_receipt(&chain, 1);
    bad_receipt.sequence = 5; // Expected: 1
    let result = chain.append(bad_receipt);
    assert!(result.is_err());

    // Chain height must not have changed
    assert_eq!(chain.height(), 1);

    // Correct sequence should succeed
    let correct_receipt = make_receipt(&chain, 1);
    chain.append(correct_receipt).unwrap();
    assert_eq!(chain.height(), 2);
}

#[test]
fn receipt_prev_root_chain_integrity() {
    let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());

    // Append several receipts
    for i in 0..3 {
        let receipt = make_receipt(&chain, i);
        chain.append(receipt).unwrap();
    }

    // Each receipt in the chain should have a prev_root that was the
    // final_state_root at the time of its insertion.
    let receipts = chain.receipts();
    assert_eq!(receipts.len(), 3);

    // The first receipt's prev_root should be the genesis root
    assert_eq!(receipts[0].prev_root, "00".repeat(32));

    // All receipts should have 64-char hex prev_roots
    for receipt in receipts {
        assert!(!receipt.prev_root.is_empty());
        assert_eq!(receipt.prev_root.len(), 64);
    }
}
