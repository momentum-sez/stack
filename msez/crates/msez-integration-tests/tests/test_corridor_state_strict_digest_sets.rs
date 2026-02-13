//! # Corridor State Digest Set Computation
//!
//! Tests that corridor state digest sets (lawpack and ruleset digests) are
//! correctly included in receipt chain computations. Verifies that digest
//! sets are deterministic, that empty digest sets are handled gracefully,
//! and that the inclusion of lawpack and ruleset digests affects the receipt
//! content digest.

use msez_core::{sha256_digest, CanonicalBytes, CorridorId, Timestamp};
use msez_corridor::CorridorReceipt;
use msez_corridor::ReceiptChain;
use serde_json::json;

fn make_next_root(i: u64) -> String {
    let data = json!({"payload": i, "test": "digest-sets"});
    let canonical = CanonicalBytes::new(&data).unwrap();
    sha256_digest(&canonical).to_hex()
}

fn make_receipt_with_digests(
    chain: &ReceiptChain,
    i: u64,
    lawpack_digests: Vec<String>,
    ruleset_digests: Vec<String>,
) -> CorridorReceipt {
    CorridorReceipt {
        receipt_type: "MSEZCorridorStateReceipt".to_string(),
        corridor_id: chain.corridor_id().clone(),
        sequence: chain.height(),
        timestamp: Timestamp::now(),
        prev_root: chain.mmr_root().unwrap(),
        next_root: make_next_root(i),
        lawpack_digest_set: lawpack_digests,
        ruleset_digest_set: ruleset_digests,
    }
}

#[test]
fn digest_set_includes_lawpack_and_ruleset() {
    let mut chain = ReceiptChain::new(CorridorId::new());

    let lawpack_digests = vec!["aa".repeat(32), "bb".repeat(32)];
    let ruleset_digests = vec!["cc".repeat(32)];

    let receipt =
        make_receipt_with_digests(&chain, 0, lawpack_digests.clone(), ruleset_digests.clone());

    // Verify the receipt preserves digest sets
    assert_eq!(receipt.lawpack_digest_set, lawpack_digests);
    assert_eq!(receipt.ruleset_digest_set, ruleset_digests);

    // The receipt content digest must include these sets
    let content_digest = receipt.content_digest().unwrap();
    assert_eq!(content_digest.to_hex().len(), 64);

    chain.append(receipt).unwrap();
    assert_eq!(chain.height(), 1);
}

#[test]
fn digest_set_deterministic() {
    let chain = ReceiptChain::new(CorridorId::new());

    let receipt_a = make_receipt_with_digests(
        &chain,
        0,
        vec!["deadbeef".repeat(8)],
        vec!["cafebabe".repeat(8)],
    );
    let receipt_b = make_receipt_with_digests(
        &chain,
        0,
        vec!["deadbeef".repeat(8)],
        vec!["cafebabe".repeat(8)],
    );

    let digest_a = receipt_a.content_digest().unwrap();
    let digest_b = receipt_b.content_digest().unwrap();

    assert_eq!(
        digest_a, digest_b,
        "identical digest sets must produce identical content digests"
    );
}

#[test]
fn empty_digest_set_handled() {
    let mut chain = ReceiptChain::new(CorridorId::new());

    // A receipt with empty digest sets should still be valid
    let receipt = make_receipt_with_digests(&chain, 0, vec![], vec![]);

    let content_digest = receipt.content_digest().unwrap();
    assert_eq!(content_digest.to_hex().len(), 64);

    // It should append to the chain successfully
    chain.append(receipt).unwrap();
    assert_eq!(chain.height(), 1);
}

#[test]
fn different_digest_sets_produce_different_receipts() {
    let chain = ReceiptChain::new(CorridorId::new());

    let receipt_with_lawpack = make_receipt_with_digests(&chain, 0, vec!["aa".repeat(32)], vec![]);
    let receipt_with_ruleset = make_receipt_with_digests(&chain, 0, vec![], vec!["bb".repeat(32)]);
    let receipt_with_both =
        make_receipt_with_digests(&chain, 0, vec!["aa".repeat(32)], vec!["bb".repeat(32)]);

    let digest_lp = receipt_with_lawpack.content_digest().unwrap();
    let digest_rs = receipt_with_ruleset.content_digest().unwrap();
    let digest_both = receipt_with_both.content_digest().unwrap();

    // All three must differ
    assert_ne!(
        digest_lp, digest_rs,
        "lawpack-only vs ruleset-only must differ"
    );
    assert_ne!(digest_lp, digest_both, "lawpack-only vs both must differ");
    assert_ne!(digest_rs, digest_both, "ruleset-only vs both must differ");
}

#[test]
fn digest_set_order_matters() {
    let chain = ReceiptChain::new(CorridorId::new());

    // Same digests but in different order
    let receipt_ab =
        make_receipt_with_digests(&chain, 0, vec!["aa".repeat(32), "bb".repeat(32)], vec![]);
    let receipt_ba =
        make_receipt_with_digests(&chain, 0, vec!["bb".repeat(32), "aa".repeat(32)], vec![]);

    let digest_ab = receipt_ab.content_digest().unwrap();
    let digest_ba = receipt_ba.content_digest().unwrap();

    // Array order is preserved in JSON canonicalization, so different order
    // means different digest (arrays are not sorted, only object keys are)
    assert_ne!(
        digest_ab, digest_ba,
        "lawpack digest order must affect the content digest"
    );
}
