//! # Corridor Security Artifacts
//!
//! Tests corridor security artifacts including signed attestations and proofs.
//! Verifies that security artifacts can be signed with Ed25519, that tampered
//! artifacts fail verification, and that security artifacts store correctly
//! in the content-addressed store.

use mez_core::{sha256_digest, CanonicalBytes, CorridorId};
use mez_crypto::{ContentAddressedStore, SigningKey, VerifyingKey};
use rand_core::OsRng;
use serde_json::json;

fn make_store() -> (tempfile::TempDir, ContentAddressedStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    (dir, store)
}

#[test]
fn security_artifact_signed_and_verified() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();

    let corridor_id = CorridorId::new();
    let attestation = json!({
        "type": "corridor-attestation",
        "corridor_id": corridor_id.to_string(),
        "attester": "watcher-pk-001",
        "statement": "receipt-chain-valid",
        "chain_height": 100,
        "mmr_root": "aa".repeat(32)
    });

    let canonical = CanonicalBytes::new(&attestation).unwrap();
    let signature = sk.sign(&canonical);

    // Verify the signature
    let verify_result = vk.verify(&canonical, &signature);
    assert!(
        verify_result.is_ok(),
        "signature verification must succeed: {:?}",
        verify_result.err()
    );
}

#[test]
fn tampered_security_artifact_fails() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();

    let corridor_id = CorridorId::new();
    let original = json!({
        "type": "corridor-attestation",
        "corridor_id": corridor_id.to_string(),
        "chain_height": 50,
        "status": "valid"
    });

    let canonical = CanonicalBytes::new(&original).unwrap();
    let signature = sk.sign(&canonical);

    // Tamper with the attestation
    let tampered = json!({
        "type": "corridor-attestation",
        "corridor_id": corridor_id.to_string(),
        "chain_height": 999,
        "status": "valid"
    });

    let tampered_canonical = CanonicalBytes::new(&tampered).unwrap();

    // Signature must not verify against tampered data
    let verify_result = vk.verify(&tampered_canonical, &signature);
    assert!(
        verify_result.is_err(),
        "tampered artifact must fail verification"
    );
}

#[test]
fn security_artifact_cas_storage() {
    let (_dir, store) = make_store();
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();

    let corridor_id = CorridorId::new();
    let attestation = json!({
        "type": "corridor-security-attestation",
        "corridor_id": corridor_id.to_string(),
        "watcher": "watcher-ae-001",
        "evidence_digest": "bb".repeat(32),
        "verdict": "no-fork-detected"
    });

    // Sign the attestation
    let canonical = CanonicalBytes::new(&attestation).unwrap();
    let signature = sk.sign(&canonical);

    // Store the attestation with its signature as a security artifact
    let security_artifact = json!({
        "attestation": attestation,
        "signature": signature.to_hex(),
        "verifying_key": vk.to_hex()
    });

    let artifact_ref = store
        .store("security-attestation", &security_artifact)
        .unwrap();
    assert_eq!(artifact_ref.artifact_type, "security-attestation");

    // Resolve and verify the stored artifact
    let resolved = store
        .resolve("security-attestation", &artifact_ref.digest)
        .unwrap();
    assert!(resolved.is_some());

    let bytes = resolved.unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    // Reconstruct and verify the signature from the stored artifact
    let stored_attestation = &parsed["attestation"];
    let stored_sig_hex = parsed["signature"].as_str().unwrap();
    let stored_vk_hex = parsed["verifying_key"].as_str().unwrap();

    let recovered_canonical = CanonicalBytes::new(stored_attestation).unwrap();
    let recovered_sig = mez_crypto::Ed25519Signature::from_hex(stored_sig_hex).unwrap();
    let recovered_vk = VerifyingKey::from_hex(stored_vk_hex).unwrap();

    assert!(
        recovered_vk
            .verify(&recovered_canonical, &recovered_sig)
            .is_ok(),
        "signature from CAS-stored security artifact must verify"
    );
}

#[test]
fn wrong_key_fails_verification() {
    let sk_1 = SigningKey::generate(&mut OsRng);
    let sk_2 = SigningKey::generate(&mut OsRng);
    let vk_2 = sk_2.verifying_key();

    let data = json!({"type": "security-evidence", "corridor": "test"});
    let canonical = CanonicalBytes::new(&data).unwrap();
    let signature = sk_1.sign(&canonical);

    // Verifying with a different key must fail
    let result = vk_2.verify(&canonical, &signature);
    assert!(result.is_err());
}

#[test]
fn security_artifact_digest_is_deterministic() {
    let attestation = json!({
        "type": "corridor-security-attestation",
        "corridor_id": "fixed-corridor",
        "watcher": "watcher-001",
        "evidence_digest": "cc".repeat(32)
    });

    let digest_1 = sha256_digest(&CanonicalBytes::new(&attestation).unwrap());
    let digest_2 = sha256_digest(&CanonicalBytes::new(&attestation).unwrap());

    assert_eq!(digest_1, digest_2);
}
