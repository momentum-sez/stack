//! Rust counterpart of tests/perf/test_receipt_chain_verification_perf.py
//! Performance tests for receipt chain operations at scale.

use msez_core::{sha256_digest, CanonicalBytes, CorridorId, Timestamp};
use msez_corridor::{CorridorReceipt, ReceiptChain};
use serde_json::json;

fn make_next_root(i: u64) -> String {
    sha256_digest(&CanonicalBytes::new(&json!({"perf_payload": i})).unwrap()).to_hex()
}

fn make_receipt(chain: &ReceiptChain, i: u64) -> CorridorReceipt {
    CorridorReceipt {
        receipt_type: "MSEZCorridorStateReceipt".to_string(),
        corridor_id: chain.corridor_id().clone(),
        sequence: chain.height(),
        timestamp: Timestamp::now(),
        prev_root: chain.mmr_root().unwrap(),
        next_root: make_next_root(i),
        lawpack_digest_set: vec!["deadbeef".repeat(8)],
        ruleset_digest_set: vec!["cafebabe".repeat(8)],
    }
}

#[test]
fn receipt_chain_100_receipts() {
    let mut chain = ReceiptChain::new(CorridorId::new());
    for i in 0..100 {
        let receipt = make_receipt(&chain, i);
        chain.append(receipt).unwrap();
    }
    assert_eq!(chain.height(), 100);
    let root = chain.mmr_root().unwrap();
    assert_eq!(root.len(), 64);
}

#[test]
fn receipt_chain_inclusion_proofs_at_scale() {
    let mut chain = ReceiptChain::new(CorridorId::new());
    for i in 0..50 {
        let receipt = make_receipt(&chain, i);
        chain.append(receipt).unwrap();
    }
    for idx in [0, 10, 25, 49] {
        let proof = chain.build_inclusion_proof(idx).unwrap();
        assert!(chain.verify_inclusion_proof(&proof).unwrap());
    }
}

#[test]
fn checkpoint_creation_after_batch() {
    let mut chain = ReceiptChain::new(CorridorId::new());
    for i in 0..30 {
        let receipt = make_receipt(&chain, i);
        chain.append(receipt).unwrap();
    }
    let cp = chain.create_checkpoint().unwrap();
    assert_eq!(cp.height, 30);
    assert_eq!(cp.mmr_root.len(), 64);
}
