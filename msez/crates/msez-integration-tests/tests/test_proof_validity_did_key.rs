//! # Proof Validity with DID Key Method Tests
//!
//! Tests proof generation and verification using the `did:key` method.
//! Verifies the `z6Mk` prefix format, invalid DID rejection, and that
//! proof verification methods correctly include the DID identifier.

use msez_core::Did;
use msez_crypto::SigningKey;
use msez_vc::{ContextValue, CredentialTypeValue, ProofType, ProofValue, VerifiableCredential};
use rand_core::OsRng;
use serde_json::json;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_vc_with_issuer(issuer: &str) -> VerifiableCredential {
    VerifiableCredential {
        context: ContextValue::Array(vec![json!("https://www.w3.org/2018/credentials/v1")]),
        id: Some("urn:msez:vc:did-key-test:001".to_string()),
        credential_type: CredentialTypeValue::Array(vec![
            "VerifiableCredential".to_string(),
            "SmartAssetRegistryVC".to_string(),
        ]),
        issuer: issuer.to_string(),
        issuance_date: chrono::Utc::now(),
        expiration_date: None,
        credential_subject: json!({
            "asset_id": "d".repeat(64),
            "name": "DID Key Test Asset"
        }),
        proof: ProofValue::default(),
    }
}

// ---------------------------------------------------------------------------
// 1. DID key proof sign and verify
// ---------------------------------------------------------------------------

#[test]
fn did_key_proof_sign_verify() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();

    let did_key = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
    let mut vc = make_vc_with_issuer(did_key);

    vc.sign_ed25519(
        &sk,
        format!("{did_key}#key-1"),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    let results = vc.verify(move |_vm: &str| Ok(vk.clone()));
    assert_eq!(results.len(), 1);
    assert!(
        results[0].ok,
        "DID key proof should verify: {}",
        results[0].error
    );
}

#[test]
fn did_key_multiple_signatures() {
    let sk1 = SigningKey::generate(&mut OsRng);
    let sk2 = SigningKey::generate(&mut OsRng);
    let vk1 = sk1.verifying_key();
    let vk2 = sk2.verifying_key();

    let did1 = "did:key:z6MkSigner1TestKey";
    let did2 = "did:key:z6MkSigner2TestKey";

    let mut vc = make_vc_with_issuer(did1);

    vc.sign_ed25519(
        &sk1,
        format!("{did1}#key-1"),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    vc.sign_ed25519(
        &sk2,
        format!("{did2}#key-1"),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    assert_eq!(vc.proof.as_list().len(), 2);

    let results = vc.verify(move |vm: &str| {
        if vm.contains("Signer1") {
            Ok(vk1.clone())
        } else if vm.contains("Signer2") {
            Ok(vk2.clone())
        } else {
            Err(format!("unknown verification method: {vm}"))
        }
    });

    assert_eq!(results.len(), 2);
    assert!(results[0].ok);
    assert!(results[1].ok);
}

// ---------------------------------------------------------------------------
// 2. DID key format z6Mk prefix
// ---------------------------------------------------------------------------

#[test]
fn did_key_format_z6mk_prefix() {
    // Valid DID key format with z6Mk prefix
    let did = Did::new("did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK").unwrap();
    assert_eq!(did.method(), "key");
    assert!(
        did.method_specific_id().starts_with("z6Mk"),
        "Ed25519 DID key should start with z6Mk prefix"
    );
}

#[test]
fn did_key_parts_accessible() {
    let did = Did::new("did:key:z6MkTestKey123").unwrap();
    assert_eq!(did.method(), "key");
    assert_eq!(did.method_specific_id(), "z6MkTestKey123");
    assert_eq!(did.as_str(), "did:key:z6MkTestKey123");
}

// ---------------------------------------------------------------------------
// 3. Invalid DID key rejected
// ---------------------------------------------------------------------------

#[test]
fn invalid_did_key_rejected() {
    // Missing "did:" prefix
    assert!(Did::new("key:z6MkTestKey").is_err());

    // Missing method separator
    assert!(Did::new("did:").is_err());

    // No method-specific identifier
    assert!(Did::new("did:key:").is_err());

    // Missing method
    assert!(Did::new("did::z6MkTestKey").is_err());

    // Completely empty
    assert!(Did::new("").is_err());
}

#[test]
fn did_method_must_be_lowercase() {
    // Method with uppercase should be rejected
    assert!(Did::new("did:KEY:z6MkTestKey").is_err());
    assert!(Did::new("did:Key:z6MkTestKey").is_err());
}

#[test]
fn did_valid_methods_accepted() {
    // Various valid DID methods
    assert!(Did::new("did:key:z6MkTest").is_ok());
    assert!(Did::new("did:web:example.com").is_ok());
    assert!(Did::new("did:ethr:0x1234").is_ok());
    assert!(Did::new("did:ion:EiDk2RpPVuC4wNANUTn_4YXJczjzi10zLG1XE4AjkcGOLA").is_ok());
}

// ---------------------------------------------------------------------------
// 4. Proof verification method includes DID
// ---------------------------------------------------------------------------

#[test]
fn proof_verification_method_includes_did() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();

    let did_key = "did:key:z6MkVerMethodTest";
    let verification_method = format!("{did_key}#key-1");

    let mut vc = make_vc_with_issuer(did_key);
    vc.sign_ed25519(
        &sk,
        verification_method.clone(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    // The proof's verification method should contain the DID
    let proofs = vc.proof.as_list();
    assert_eq!(proofs.len(), 1);

    // Verify that the key resolver receives the correct verification method
    let results = vc.verify(move |vm: &str| {
        assert!(
            vm.contains("z6MkVerMethodTest"),
            "verification method must include DID key identifier"
        );
        Ok(vk.clone())
    });

    assert_eq!(results.len(), 1);
    assert!(results[0].ok);
}

// ---------------------------------------------------------------------------
// 5. DID display and serialization
// ---------------------------------------------------------------------------

#[test]
fn did_display_format() {
    let did = Did::new("did:key:z6MkDisplayTest").unwrap();
    assert_eq!(format!("{did}"), "did:key:z6MkDisplayTest");
}

#[test]
fn did_serde_roundtrip() {
    let did = Did::new("did:key:z6MkSerdeTest").unwrap();
    let serialized = serde_json::to_string(&did).unwrap();
    let deserialized: Did = serde_json::from_str(&serialized).unwrap();
    assert_eq!(did, deserialized);
}
