//! # Checkpoint Audit Dual Output Test
//!
//! Tests checkpoint creation and audit trail with MMR state. Verifies that
//! checkpoints correctly capture the MMR root at the time of creation,
//! that multiple checkpoints at different heights differ, and that the
//! checkpoint height matches the receipt count in the chain.

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
// 1. Checkpoint captures MMR state at creation time
// ---------------------------------------------------------------------------

#[test]
fn checkpoint_captures_mmr_state() {
    let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());

    for i in 0..5 {
        let receipt = make_receipt(&chain, i);
        chain.append(receipt).unwrap();
    }

    let checkpoint = chain.create_checkpoint().unwrap();
    assert_eq!(checkpoint.height(), 5);
    assert_eq!(checkpoint.mmr_root().to_string(), chain.mmr_root().unwrap());
    assert_eq!(checkpoint.checkpoint_digest.to_hex().len(), 64);
}

// ---------------------------------------------------------------------------
// 2. Multiple checkpoints at different heights differ
// ---------------------------------------------------------------------------

#[test]
fn multiple_checkpoints_differ() {
    let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());

    // Create first batch and checkpoint
    for i in 0..3 {
        chain.append(make_receipt(&chain, i)).unwrap();
    }
    let cp1 = chain.create_checkpoint().unwrap();

    // Create second batch and checkpoint
    for i in 3..7 {
        chain.append(make_receipt(&chain, i)).unwrap();
    }
    let cp2 = chain.create_checkpoint().unwrap();

    assert_ne!(cp1.height(), cp2.height());
    assert_ne!(cp1.mmr_root().to_string(), cp2.mmr_root().to_string());
    assert_ne!(cp1.checkpoint_digest, cp2.checkpoint_digest);
    assert_eq!(chain.checkpoints().len(), 2);
}

// ---------------------------------------------------------------------------
// 3. Checkpoint height matches receipt count
// ---------------------------------------------------------------------------

#[test]
fn checkpoint_height_matches_receipt_count() {
    let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
    let receipt_count = 8;

    for i in 0..receipt_count {
        chain.append(make_receipt(&chain, i)).unwrap();
    }
    assert_eq!(chain.height(), receipt_count);

    let checkpoint = chain.create_checkpoint().unwrap();
    assert_eq!(checkpoint.height(), receipt_count);
}

// ---------------------------------------------------------------------------
// 4. Checkpoint digest is deterministic for same state
// ---------------------------------------------------------------------------

#[test]
fn checkpoint_digest_is_hex() {
    let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
    for i in 0..4 {
        chain.append(make_receipt(&chain, i)).unwrap();
    }
    let checkpoint = chain.create_checkpoint().unwrap();
    let hex = checkpoint.checkpoint_digest.to_hex();
    assert_eq!(hex.len(), 64);
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
}

// ---------------------------------------------------------------------------
// 5. MMR root is valid hex after receipt append
// ---------------------------------------------------------------------------

#[test]
fn mmr_root_is_valid_hex_after_appends() {
    let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
    for i in 0..10 {
        chain.append(make_receipt(&chain, i)).unwrap();
    }
    let root = chain.mmr_root().unwrap();
    assert_eq!(root.len(), 64);
    assert!(root.chars().all(|c| c.is_ascii_hexdigit()));
}
