//! # Receipt & Checkpoint Schema Conformance Tests
//!
//! Validates that `CorridorReceipt` and `Checkpoint` structs produce JSON
//! that conforms to the normative schemas:
//! - `schemas/corridor.receipt.schema.json`
//! - `schemas/corridor.checkpoint.schema.json`
//!
//! Also verifies negative cases: missing required fields must be rejected.

use mez_core::{ContentDigest, CorridorId, Timestamp};
use mez_corridor::{CorridorReceipt, ProofObject, ReceiptChain, ReceiptProof};
use mez_schema::SchemaValidator;

/// Locate the repo-root `schemas/` directory from the integration test crate.
fn schema_dir() -> std::path::PathBuf {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .find(|p| p.join("schemas").is_dir())
        .expect("could not find repo root with schemas/ directory")
        .join("schemas")
}

/// Build a SchemaValidator with all schemas loaded.
fn validator() -> SchemaValidator {
    SchemaValidator::new(schema_dir()).expect("failed to load schemas")
}

/// Create a deterministic genesis root for testing.
fn test_genesis_root() -> ContentDigest {
    ContentDigest::from_hex(&"00".repeat(32)).unwrap()
}

/// Create a dummy proof for schema conformance (receipts MUST have proof).
fn dummy_proof() -> ReceiptProof {
    ReceiptProof::Single(ProofObject {
        proof_type: "MezEd25519Signature2025".to_string(),
        created: "2026-01-15T12:00:00Z".to_string(),
        verification_method: "did:key:z6MkTest#key-1".to_string(),
        proof_purpose: "assertionMethod".to_string(),
        jws: "eyJ0eXAiOiJKV1MiLCJhbGciOiJFZERTQSJ9..test-signature".to_string(),
    })
}

/// Build a receipt with properly computed next_root.
fn make_receipt(chain: &ReceiptChain) -> CorridorReceipt {
    let mut receipt = CorridorReceipt {
        receipt_type: "MEZCorridorStateReceipt".to_string(),
        corridor_id: chain.corridor_id().clone(),
        sequence: chain.height(),
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
    receipt.seal_next_root().unwrap();
    receipt
}

// ---------------------------------------------------------------------------
// Receipt schema conformance
// ---------------------------------------------------------------------------

#[test]
fn receipt_with_proof_validates_against_schema() {
    let chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
    let mut receipt = make_receipt(&chain);
    receipt.proof = Some(dummy_proof());

    let json = serde_json::to_value(&receipt).unwrap();
    let v = validator();
    let result = v.validate_value_by_filename(&json, "corridor.receipt.schema.json");
    assert!(
        result.is_ok(),
        "receipt with proof must validate against schema: {:?}",
        result.err()
    );
}

#[test]
fn receipt_without_proof_fails_schema_validation() {
    let chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
    let receipt = make_receipt(&chain);
    assert!(receipt.proof.is_none());

    let json = serde_json::to_value(&receipt).unwrap();
    let v = validator();
    let result = v.validate_value_by_filename(&json, "corridor.receipt.schema.json");
    assert!(
        result.is_err(),
        "receipt without proof must fail schema validation"
    );
}

#[test]
fn receipt_roundtrip_preserves_schema_conformance() {
    let chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
    let mut receipt = make_receipt(&chain);
    receipt.proof = Some(dummy_proof());

    // Serialize → deserialize → reserialize → validate
    let json_str = serde_json::to_string(&receipt).unwrap();
    let deserialized: CorridorReceipt = serde_json::from_str(&json_str).unwrap();
    let reserialize = serde_json::to_value(&deserialized).unwrap();

    let v = validator();
    let result = v.validate_value_by_filename(&reserialize, "corridor.receipt.schema.json");
    assert!(
        result.is_ok(),
        "round-tripped receipt must still validate: {:?}",
        result.err()
    );
}

#[test]
fn receipt_with_optional_fields_validates() {
    let chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
    let mut receipt = CorridorReceipt {
        receipt_type: "MEZCorridorStateReceipt".to_string(),
        corridor_id: chain.corridor_id().clone(),
        sequence: 0,
        timestamp: Timestamp::now(),
        prev_root: chain.final_state_root_hex(),
        next_root: String::new(),
        lawpack_digest_set: vec!["aa".repeat(32).into()],
        ruleset_digest_set: vec!["bb".repeat(32).into()],
        proof: None,
        transition: Some(serde_json::json!({
            "type": "MEZTransitionEnvelope",
            "kind": "test.transfer.v1",
            "payload_sha256": "cc".repeat(32)
        })),
        transition_type_registry_digest_sha256: Some("dd".repeat(32)),
        zk: Some(serde_json::json!({
            "proof_system": "groth16"
        })),
        anchor: Some(serde_json::json!({
            "chain_id": "ethereum",
            "method": "calldata"
        })),
    };
    receipt.seal_next_root().unwrap();
    receipt.proof = Some(dummy_proof());

    let json = serde_json::to_value(&receipt).unwrap();
    let v = validator();
    let result = v.validate_value_by_filename(&json, "corridor.receipt.schema.json");
    assert!(
        result.is_ok(),
        "receipt with optional fields must validate: {:?}",
        result.err()
    );
}

// ---------------------------------------------------------------------------
// Checkpoint schema conformance
// ---------------------------------------------------------------------------

#[test]
fn checkpoint_with_proof_validates_against_schema() {
    let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
    let receipt = make_receipt(&chain);
    chain.append(receipt).unwrap();

    let mut checkpoint = chain.create_checkpoint().unwrap();
    checkpoint.proof = Some(dummy_proof());

    let json = serde_json::to_value(&checkpoint).unwrap();
    let v = validator();
    let result = v.validate_value_by_filename(&json, "corridor.checkpoint.schema.json");
    assert!(
        result.is_ok(),
        "checkpoint with proof must validate against schema: {:?}",
        result.err()
    );
}

#[test]
fn checkpoint_without_proof_fails_schema_validation() {
    let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());
    let receipt = make_receipt(&chain);
    chain.append(receipt).unwrap();

    let checkpoint = chain.create_checkpoint().unwrap();
    assert!(checkpoint.proof.is_none());

    let json = serde_json::to_value(&checkpoint).unwrap();
    let v = validator();
    let result = v.validate_value_by_filename(&json, "corridor.checkpoint.schema.json");
    assert!(
        result.is_err(),
        "checkpoint without proof must fail schema validation"
    );
}

#[test]
fn checkpoint_fields_match_chain_state() {
    let mut chain = ReceiptChain::new(CorridorId::new(), test_genesis_root());

    for _ in 0..5 {
        let receipt = make_receipt(&chain);
        chain.append(receipt).unwrap();
    }

    let checkpoint = chain.create_checkpoint().unwrap();

    assert_eq!(checkpoint.genesis_root, test_genesis_root().to_hex());
    assert_eq!(checkpoint.final_state_root, chain.final_state_root_hex());
    assert_eq!(checkpoint.receipt_count, 5);
    assert_eq!(checkpoint.mmr.mmr_type, "MEZReceiptMMR");
    assert_eq!(checkpoint.mmr.algorithm, "sha256");
    assert_eq!(checkpoint.mmr.size, 5);
    assert_eq!(checkpoint.mmr.root, chain.mmr_root().unwrap());
}
