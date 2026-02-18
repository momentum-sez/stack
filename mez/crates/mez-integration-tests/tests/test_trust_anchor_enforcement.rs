//! # Trust Anchor Enforcement and DID Validation Test
//!
//! Tests DID format validation for did:key and did:web methods,
//! rejection of invalid DID formats, and VC issuer validation
//! requirements.

use mez_core::Did;
use mez_crypto::SigningKey;
use mez_vc::{ContextValue, CredentialTypeValue, ProofType, ProofValue, VerifiableCredential};
use rand_core::OsRng;
use serde_json::json;

// ---------------------------------------------------------------------------
// 1. DID key format validation
// ---------------------------------------------------------------------------

#[test]
fn did_key_format_validation() {
    // Valid did:key
    let did = Did::new("did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK");
    assert!(did.is_ok());
    let did = did.unwrap();
    assert_eq!(did.method(), "key");
    assert_eq!(
        did.method_specific_id(),
        "z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK"
    );

    // Another valid did:key
    let did2 = Did::new("did:key:z6MkTest12345");
    assert!(did2.is_ok());
    assert_eq!(did2.unwrap().method(), "key");
}

// ---------------------------------------------------------------------------
// 2. DID web format validation
// ---------------------------------------------------------------------------

#[test]
fn did_web_format_validation() {
    // Valid did:web
    let did = Did::new("did:web:example.com");
    assert!(did.is_ok());
    let did = did.unwrap();
    assert_eq!(did.method(), "web");
    assert_eq!(did.method_specific_id(), "example.com");

    // did:web with path
    let did_path = Did::new("did:web:example.com:users:alice");
    assert!(did_path.is_ok());
    assert_eq!(
        did_path.unwrap().method_specific_id(),
        "example.com:users:alice"
    );
}

// ---------------------------------------------------------------------------
// 3. Invalid DID formats rejected
// ---------------------------------------------------------------------------

#[test]
fn invalid_did_rejected() {
    // Missing prefix
    assert!(Did::new("key:z6MkTest").is_err());
    assert!(Did::new("notadid").is_err());

    // Empty parts
    assert!(Did::new("did:").is_err());
    assert!(Did::new("did::identifier").is_err());
    assert!(Did::new("did:method:").is_err());

    // Uppercase method (W3C DID syntax requires lowercase method)
    assert!(Did::new("did:Key:z6MkTest").is_err());
    assert!(Did::new("did:WEB:example.com").is_err());

    // Empty string
    assert!(Did::new("").is_err());

    // Only did: prefix
    assert!(Did::new("did:a").is_err()); // no second colon
}

// ---------------------------------------------------------------------------
// 4. VC issuer must be a valid DID (structural test)
// ---------------------------------------------------------------------------

#[test]
fn vc_issuer_must_be_valid_did() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();

    // VC with valid DID issuer
    let mut vc = VerifiableCredential {
        context: ContextValue::Array(vec![json!("https://www.w3.org/2018/credentials/v1")]),
        id: Some("urn:mez:vc:trust:001".to_string()),
        credential_type: CredentialTypeValue::Array(vec![
            "VerifiableCredential".to_string(),
            "MezTrustAnchorTest".to_string(),
        ]),
        issuer: "did:key:z6MkTrustAnchor".to_string(),
        issuance_date: chrono::Utc::now(),
        expiration_date: None,
        credential_subject: json!({"entity": "test-entity"}),
        proof: ProofValue::default(),
    };

    vc.sign_ed25519(
        &sk,
        "did:key:z6MkTrustAnchor#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    // Verify the issuer DID is parseable
    let issuer_did = Did::new(&vc.issuer);
    assert!(issuer_did.is_ok());

    // Verify the proof
    let results = vc.verify(move |_| Ok(vk.clone()));
    assert_eq!(results.len(), 1);
    assert!(results[0].ok);
}

// ---------------------------------------------------------------------------
// 5. DID ethr format validation
// ---------------------------------------------------------------------------

#[test]
fn did_ethr_format_validation() {
    let did = Did::new("did:ethr:0xb9c5714089478a327f09197987f16f9e5d936e8a");
    assert!(did.is_ok());
    assert_eq!(did.unwrap().method(), "ethr");
}

// ---------------------------------------------------------------------------
// 6. DID display format
// ---------------------------------------------------------------------------

#[test]
fn did_display_format() {
    let did = Did::new("did:web:example.com").unwrap();
    assert_eq!(format!("{did}"), "did:web:example.com");
    assert_eq!(did.as_str(), "did:web:example.com");
}
