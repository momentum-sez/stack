//! # CLI Signing Command Tests
//!
//! Tests the Rust equivalents of CLI signing operations: key generation,
//! VC signing, VC verification, and unsigned VC rejection. These test
//! the same code paths that the CLI would invoke for `msez sign` and
//! `msez verify` subcommands.

use msez_core::{sha256_digest, CanonicalBytes};
use msez_crypto::{SigningKey, VerifyingKey};
use msez_vc::{
    ContextValue, CredentialTypeValue, ProofType, ProofValue, VcError, VerifiableCredential,
};
use rand_core::OsRng;
use serde_json::json;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_vc() -> VerifiableCredential {
    VerifiableCredential {
        context: ContextValue::Array(vec![json!("https://www.w3.org/2018/credentials/v1")]),
        id: Some("urn:msez:vc:cli-test:001".to_string()),
        credential_type: CredentialTypeValue::Array(vec![
            "VerifiableCredential".to_string(),
            "SmartAssetRegistryVC".to_string(),
        ]),
        issuer: "did:key:z6MkCliTest".to_string(),
        issuance_date: chrono::Utc::now(),
        expiration_date: None,
        credential_subject: json!({
            "asset_id": "c".repeat(64),
            "name": "CLI Test Asset",
            "jurisdiction_bindings": [
                {
                    "jurisdiction_id": "PK-RSEZ",
                    "binding_status": "active"
                }
            ]
        }),
        proof: ProofValue::default(),
    }
}

// ---------------------------------------------------------------------------
// 1. Key generation produces valid keypair
// ---------------------------------------------------------------------------

#[test]
fn keygen_produces_valid_keypair() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();

    // Sign some test data (must use CanonicalBytes, not raw bytes)
    let data = CanonicalBytes::new(&json!({"msg": "test message for key validation"})).unwrap();
    let signature = sk.sign(&data);

    // Verify the signature
    assert!(
        vk.verify(&data, &signature).is_ok(),
        "generated keypair must produce verifiable signatures"
    );
}

#[test]
fn keygen_produces_unique_keys() {
    let sk1 = SigningKey::generate(&mut OsRng);
    let sk2 = SigningKey::generate(&mut OsRng);

    let vk1 = sk1.verifying_key();
    let vk2 = sk2.verifying_key();

    // Two generated keys should be different
    assert_ne!(
        vk1.as_bytes(),
        vk2.as_bytes(),
        "two independently generated keys must differ"
    );
}

// ---------------------------------------------------------------------------
// 2. Sign VC produces proof
// ---------------------------------------------------------------------------

#[test]
fn sign_vc_produces_proof() {
    let sk = SigningKey::generate(&mut OsRng);
    let mut vc = make_vc();

    assert!(vc.proof.is_empty(), "VC should have no proof before signing");

    vc.sign_ed25519(
        &sk,
        "did:key:z6MkCliTest#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    assert!(!vc.proof.is_empty(), "VC should have proof after signing");

    let proofs = vc.proof.as_list();
    assert_eq!(proofs.len(), 1);
}

#[test]
fn sign_vc_with_msez_proof_type() {
    let sk = SigningKey::generate(&mut OsRng);
    let mut vc = make_vc();

    vc.sign_ed25519(
        &sk,
        "did:key:z6MkCliTest#key-1".to_string(),
        ProofType::MsezEd25519Signature2025,
        None,
    )
    .unwrap();

    assert!(!vc.proof.is_empty());
}

// ---------------------------------------------------------------------------
// 3. Verify signed VC succeeds
// ---------------------------------------------------------------------------

#[test]
fn verify_signed_vc_succeeds() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();
    let mut vc = make_vc();

    vc.sign_ed25519(
        &sk,
        "did:key:z6MkCliTest#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    let results = vc.verify(move |_vm: &str| Ok(vk.clone()));
    assert_eq!(results.len(), 1);
    assert!(results[0].ok, "valid signature should verify: {}", results[0].error);
}

#[test]
fn verify_all_signed_vc_succeeds() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();
    let mut vc = make_vc();

    vc.sign_ed25519(
        &sk,
        "did:key:z6MkCliTest#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    assert!(vc.verify_all(move |_vm: &str| Ok(vk.clone())).is_ok());
}

// ---------------------------------------------------------------------------
// 4. Verify unsigned VC fails
// ---------------------------------------------------------------------------

#[test]
fn verify_unsigned_vc_fails() {
    let vc = make_vc();

    // verify_all on unsigned VC should fail with NoProofs
    let result = vc.verify_all(|_| Err("no key".to_string()));
    assert!(matches!(result, Err(VcError::NoProofs)));
}

#[test]
fn verify_with_wrong_key_fails() {
    let sk1 = SigningKey::generate(&mut OsRng);
    let sk2 = SigningKey::generate(&mut OsRng);
    let vk2 = sk2.verifying_key();

    let mut vc = make_vc();
    vc.sign_ed25519(
        &sk1,
        "did:key:z6MkCliTest#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    // Verify with wrong key
    let results = vc.verify(move |_vm: &str| Ok(vk2.clone()));
    assert_eq!(results.len(), 1);
    assert!(!results[0].ok, "wrong key must fail verification");
}

// ---------------------------------------------------------------------------
// 5. Signing input determinism
// ---------------------------------------------------------------------------

#[test]
fn signing_input_is_deterministic() {
    let vc = make_vc();
    let input1 = vc.signing_input().unwrap();
    let input2 = vc.signing_input().unwrap();
    assert_eq!(
        input1.as_bytes(),
        input2.as_bytes(),
        "signing input must be deterministic"
    );
}

#[test]
fn signing_input_unchanged_after_signing() {
    let mut vc = make_vc();
    let input_before = vc.signing_input().unwrap();

    let sk = SigningKey::generate(&mut OsRng);
    vc.sign_ed25519(
        &sk,
        "did:key:z6MkCliTest#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    let input_after = vc.signing_input().unwrap();
    assert_eq!(
        input_before.as_bytes(),
        input_after.as_bytes(),
        "signing input must be identical before and after signing"
    );
}
