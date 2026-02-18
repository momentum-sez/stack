//! Rust counterpart of tests/perf/test_receipt_chain_verification_perf.py
//! Performance tests for receipt chain operations at scale.

use msez_core::{ContentDigest, CorridorId, Timestamp};
use msez_corridor::{CorridorReceipt, ReceiptChain};

fn test_genesis_root() -> ContentDigest {
    ContentDigest::from_hex(&"00".repeat(32)).unwrap()
}

fn make_receipt(chain: &ReceiptChain, _i: u64) -> CorridorReceipt {
    let mut receipt = CorridorReceipt {
        receipt_type: "MSEZCorridorStateReceipt".to_string(),
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
fn receipt_chain_100_receipts() {
    let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
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
    let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
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
    let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
    for i in 0..30 {
        let receipt = make_receipt(&chain, i);
        chain.append(receipt).unwrap();
    }
    let cp = chain.create_checkpoint().unwrap();
    assert_eq!(cp.height(), 30);
    assert_eq!(cp.mmr_root().to_string().len(), 64);
}
