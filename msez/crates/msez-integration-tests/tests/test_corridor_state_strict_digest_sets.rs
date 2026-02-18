//! # Corridor State Digest Set Computation
//!
//! Tests that corridor state digest sets (lawpack and ruleset digests) are
//! correctly included in receipt chain computations. Verifies that digest
//! sets are deterministic, that empty digest sets are handled gracefully,
//! and that the inclusion of lawpack and ruleset digests affects the receipt
//! content digest.

use msez_core::{ContentDigest, CorridorId, Timestamp};
use msez_corridor::{CorridorReceipt, DigestEntry};
use msez_corridor::ReceiptChain;

fn test_genesis_root() -> ContentDigest {
    ContentDigest::from_hex(&"00".repeat(32)).unwrap()
}

fn make_receipt_with_digests(
    chain: &ReceiptChain,
    _i: u64,
    lawpack_digests: Vec<DigestEntry>,
    ruleset_digests: Vec<DigestEntry>,
) -> CorridorReceipt {
    let mut receipt = CorridorReceipt {
        receipt_type: "MSEZCorridorStateReceipt".to_string(),
        corridor_id: chain.corridor_id().clone(),
        sequence: chain.height(),
        timestamp: Timestamp::now(),
        prev_root: chain.final_state_root_hex(),
        next_root: String::new(),
        lawpack_digest_set: lawpack_digests,
        ruleset_digest_set: ruleset_digests,
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
fn digest_set_includes_lawpack_and_ruleset() {
    let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());

    let lawpack_digests: Vec<DigestEntry> = vec!["aa".repeat(32).into(), "bb".repeat(32).into()];
    let ruleset_digests: Vec<DigestEntry> = vec!["cc".repeat(32).into()];

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
    let chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());

    let receipt_a = make_receipt_with_digests(
        &chain,
        0,
        vec!["deadbeef".repeat(8).into()],
        vec!["cafebabe".repeat(8).into()],
    );
    let receipt_b = make_receipt_with_digests(
        &chain,
        0,
        vec!["deadbeef".repeat(8).into()],
        vec!["cafebabe".repeat(8).into()],
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
    let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());

    // A receipt with empty digest sets should still be valid
    let receipt = make_receipt_with_digests(&chain, 0, Vec::new(), Vec::new());

    let content_digest = receipt.content_digest().unwrap();
    assert_eq!(content_digest.to_hex().len(), 64);

    // It should append to the chain successfully
    chain.append(receipt).unwrap();
    assert_eq!(chain.height(), 1);
}

#[test]
fn different_digest_sets_produce_different_receipts() {
    let chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());

    let receipt_with_lawpack = make_receipt_with_digests(&chain, 0, vec!["aa".repeat(32).into()], Vec::new());
    let receipt_with_ruleset = make_receipt_with_digests(&chain, 0, Vec::new(), vec!["bb".repeat(32).into()]);
    let receipt_with_both =
        make_receipt_with_digests(&chain, 0, vec!["aa".repeat(32).into()], vec!["bb".repeat(32).into()]);

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
    let chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());

    // Same digests but in different order
    let receipt_ab =
        make_receipt_with_digests(&chain, 0, vec!["aa".repeat(32).into(), "bb".repeat(32).into()], Vec::new());
    let receipt_ba =
        make_receipt_with_digests(&chain, 0, vec!["bb".repeat(32).into(), "aa".repeat(32).into()], Vec::new());

    let digest_ab = receipt_ab.content_digest().unwrap();
    let digest_ba = receipt_ba.content_digest().unwrap();

    // Array order is preserved in JSON canonicalization, so different order
    // means different digest (arrays are not sorted, only object keys are)
    assert_ne!(
        digest_ab, digest_ba,
        "lawpack digest order must affect the content digest"
    );
}
