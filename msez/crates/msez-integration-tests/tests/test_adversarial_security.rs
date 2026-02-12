//! # Adversarial Security Scenarios Test
//!
//! Tests adversarial security scenarios: float injection rejection in
//! canonical serialization, tampered VC detection, wrong-key verification
//! failure, replay attacks with different context, and oversized payload
//! handling.

use msez_core::CanonicalBytes;
use msez_crypto::SigningKey;
use msez_vc::{
    ContextValue, CredentialTypeValue, ProofType, ProofValue, VerifiableCredential,
};
use rand_core::OsRng;
use serde_json::json;

fn make_test_vc(subject: serde_json::Value) -> VerifiableCredential {
    VerifiableCredential {
        context: ContextValue::Array(vec![json!("https://www.w3.org/2018/credentials/v1")]),
        id: Some("urn:msez:vc:adversarial:001".to_string()),
        credential_type: CredentialTypeValue::Array(vec![
            "VerifiableCredential".to_string(),
            "MsezTestCredential".to_string(),
        ]),
        issuer: "did:key:z6MkAdversarial".to_string(),
        issuance_date: chrono::Utc::now(),
        expiration_date: None,
        credential_subject: subject,
        proof: ProofValue::default(),
    }
}

// ---------------------------------------------------------------------------
// 1. Float injection rejected by canonical serialization
// ---------------------------------------------------------------------------

#[test]
fn float_injection_rejected() {
    // CanonicalBytes rejects floats to prevent canonicalization ambiguity
    let data = json!({"amount": 3.14159});
    let result = CanonicalBytes::new(&data);
    assert!(result.is_err());

    // Integers are accepted
    let int_data = json!({"amount": 314159});
    assert!(CanonicalBytes::new(&int_data).is_ok());

    // String amounts are accepted
    let str_data = json!({"amount": "3.14159"});
    assert!(CanonicalBytes::new(&str_data).is_ok());
}

// ---------------------------------------------------------------------------
// 2. Tampered VC detected during verification
// ---------------------------------------------------------------------------

#[test]
fn tampered_vc_detected() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();

    let mut vc = make_test_vc(json!({"asset_id": "a".repeat(64), "name": "Legitimate"}));
    vc.sign_ed25519(
        &sk,
        "did:key:z6MkTest#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    // Tamper with the credential subject
    vc.credential_subject = json!({"asset_id": "b".repeat(64), "name": "Tampered"});

    // Verification must fail
    let results = vc.verify(move |_| Ok(vk.clone()));
    assert_eq!(results.len(), 1);
    assert!(!results[0].ok, "tampered VC must not verify");
}

// ---------------------------------------------------------------------------
// 3. Wrong key verification fails
// ---------------------------------------------------------------------------

#[test]
fn wrong_key_verification_fails() {
    let sk1 = SigningKey::generate(&mut OsRng);
    let sk2 = SigningKey::generate(&mut OsRng);
    let vk2 = sk2.verifying_key();

    let mut vc = make_test_vc(json!({"entity": "test"}));
    vc.sign_ed25519(
        &sk1,
        "did:key:z6MkSigner1#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    // Verify with wrong key
    let results = vc.verify(move |_| Ok(vk2.clone()));
    assert_eq!(results.len(), 1);
    assert!(!results[0].ok, "wrong key must not verify");
}

// ---------------------------------------------------------------------------
// 4. Replay attack with different context
// ---------------------------------------------------------------------------

#[test]
fn replay_attack_different_context() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();

    let mut vc1 = make_test_vc(json!({"entity": "test", "context": "original"}));
    vc1.sign_ed25519(
        &sk,
        "did:key:z6MkSigner#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    // Create a different VC with the same proof (replay)
    let mut vc2 = make_test_vc(json!({"entity": "test", "context": "replay-target"}));
    vc2.proof = vc1.proof.clone();

    // Verification with correct key fails because subject differs
    let results = vc2.verify(move |_| Ok(vk.clone()));
    assert_eq!(results.len(), 1);
    assert!(!results[0].ok, "replayed proof must not verify in different context");
}

// ---------------------------------------------------------------------------
// 5. Oversized payload handled gracefully
// ---------------------------------------------------------------------------

#[test]
fn oversized_payload_handled() {
    // Large payload should still canonicalize (no stack overflow)
    let large_array: Vec<i64> = (0..10_000).collect();
    let data = json!({"large_array": large_array});
    let result = CanonicalBytes::new(&data);
    assert!(result.is_ok());

    // Deeply nested but valid structure
    let nested = json!({"a": {"b": {"c": {"d": {"e": "deep"}}}}});
    let result = CanonicalBytes::new(&nested);
    assert!(result.is_ok());
}
