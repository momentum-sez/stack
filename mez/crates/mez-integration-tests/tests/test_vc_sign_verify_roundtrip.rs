//! # VC Sign/Verify Roundtrip Test
//!
//! Tests the complete lifecycle of a Verifiable Credential:
//! 1. Generate Ed25519 keypair
//! 2. Create a SmartAssetRegistryVC
//! 3. Sign it
//! 4. Verify the signature
//! 5. Tamper with one field, verify signature fails
//! 6. Verify the VC body was canonicalized (not raw-serialized) during signing

use mez_crypto::{SigningKey, VerifyingKey};
use mez_vc::{
    ContextValue, CredentialTypeValue, ProofType, ProofValue, VcError, VerifiableCredential,
};
use rand_core::OsRng;
use serde_json::json;

fn make_test_vc() -> VerifiableCredential {
    VerifiableCredential {
        context: ContextValue::Array(vec![json!("https://www.w3.org/2018/credentials/v1")]),
        id: Some("urn:mez:vc:smart-asset:001".to_string()),
        credential_type: CredentialTypeValue::Array(vec![
            "VerifiableCredential".to_string(),
            "SmartAssetRegistryVC".to_string(),
        ]),
        issuer: "did:key:z6MkTestIssuer".to_string(),
        issuance_date: chrono::Utc::now(),
        expiration_date: None,
        credential_subject: json!({
            "asset_id": "a".repeat(64),
            "name": "Reko Diq Mining License",
            "jurisdiction_bindings": [
                {
                    "jurisdiction_id": "PK-RSEZ",
                    "binding_status": "active",
                    "lawpack_ref": format!("PK-RSEZ:financial:{}", "ab".repeat(32))
                }
            ],
            "compliance_status": {
                "aml": "compliant",
                "kyc": "compliant",
                "sanctions": "compliant"
            }
        }),
        proof: ProofValue::default(),
    }
}

fn make_key_resolver(vk: VerifyingKey) -> impl Fn(&str) -> Result<VerifyingKey, String> {
    move |_vm: &str| Ok(vk.clone())
}

// ---------------------------------------------------------------------------
// 1. Generate keypair, sign, verify
// ---------------------------------------------------------------------------

#[test]
fn sign_and_verify_roundtrip() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();

    let mut vc = make_test_vc();
    vc.sign_ed25519(
        &sk,
        "did:key:z6MkTestIssuer#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    // VC should now have a proof
    assert!(!vc.proof.is_empty());

    // Verify
    let results = vc.verify(make_key_resolver(vk));
    assert_eq!(results.len(), 1);
    assert!(results[0].ok, "verification failed: {}", results[0].error);
}

#[test]
fn sign_with_mez_proof_type() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();

    let mut vc = make_test_vc();
    vc.sign_ed25519(
        &sk,
        "did:key:z6MkTestIssuer#key-1".to_string(),
        ProofType::MezEd25519Signature2025,
        None,
    )
    .unwrap();

    let results = vc.verify(make_key_resolver(vk));
    assert_eq!(results.len(), 1);
    assert!(results[0].ok);
}

// ---------------------------------------------------------------------------
// 2. Tamper with credential subject → verification fails
// ---------------------------------------------------------------------------

#[test]
fn tampered_subject_fails_verification() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();

    let mut vc = make_test_vc();
    vc.sign_ed25519(
        &sk,
        "did:key:z6MkTestIssuer#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    // Tamper with the credential subject
    vc.credential_subject = json!({
        "asset_id": "b".repeat(64),
        "name": "Tampered Asset",
        "jurisdiction_bindings": []
    });

    let results = vc.verify(make_key_resolver(vk));
    assert_eq!(results.len(), 1);
    assert!(!results[0].ok, "tampered VC must fail verification");
}

#[test]
fn tampered_issuer_fails_verification() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();

    let mut vc = make_test_vc();
    vc.sign_ed25519(
        &sk,
        "did:key:z6MkTestIssuer#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    // Tamper with the issuer
    vc.issuer = "did:key:z6MkMaliciousIssuer".to_string();

    let results = vc.verify(make_key_resolver(vk));
    assert_eq!(results.len(), 1);
    assert!(!results[0].ok, "tampered issuer must fail verification");
}

// ---------------------------------------------------------------------------
// 3. Wrong key fails verification
// ---------------------------------------------------------------------------

#[test]
fn wrong_key_fails_verification() {
    let sk1 = SigningKey::generate(&mut OsRng);
    let sk2 = SigningKey::generate(&mut OsRng);
    let vk2 = sk2.verifying_key();

    let mut vc = make_test_vc();
    vc.sign_ed25519(
        &sk1,
        "did:key:z6MkTestIssuer#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    let results = vc.verify(make_key_resolver(vk2));
    assert_eq!(results.len(), 1);
    assert!(!results[0].ok, "wrong key must fail verification");
}

// ---------------------------------------------------------------------------
// 4. Signing input uses CanonicalBytes (not raw serialization)
// ---------------------------------------------------------------------------

#[test]
fn signing_input_uses_canonical_bytes() {
    let vc = make_test_vc();
    let signing_input = vc.signing_input().unwrap();

    // The signing input should be valid UTF-8 (CanonicalBytes guarantees this)
    assert!(std::str::from_utf8(signing_input.as_bytes()).is_ok());

    // The signing input should NOT contain the proof field
    let input_str = std::str::from_utf8(signing_input.as_bytes()).unwrap();
    assert!(
        !input_str.contains("proofValue"),
        "signing input must exclude proof"
    );

    // Keys should be sorted (canonical)
    let parsed: serde_json::Value = serde_json::from_slice(signing_input.as_bytes()).unwrap();
    if let serde_json::Value::Object(map) = parsed {
        let keys: Vec<_> = map.keys().collect();
        let mut sorted_keys = keys.clone();
        sorted_keys.sort();
        assert_eq!(keys, sorted_keys, "signing input keys must be sorted");
    }
}

#[test]
fn signing_input_excludes_proof() {
    let mut vc = make_test_vc();
    let input_before = vc.signing_input().unwrap();

    let sk = SigningKey::generate(&mut OsRng);
    vc.sign_ed25519(
        &sk,
        "did:key:z6MkTestIssuer#key-1".to_string(),
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

#[test]
fn signing_input_is_deterministic() {
    let vc = make_test_vc();
    let input1 = vc.signing_input().unwrap();
    let input2 = vc.signing_input().unwrap();
    assert_eq!(input1.as_bytes(), input2.as_bytes());
}

// ---------------------------------------------------------------------------
// 5. Multi-party signing
// ---------------------------------------------------------------------------

#[test]
fn multi_party_signing_and_verification() {
    let sk1 = SigningKey::generate(&mut OsRng);
    let sk2 = SigningKey::generate(&mut OsRng);
    let vk1 = sk1.verifying_key();
    let vk2 = sk2.verifying_key();

    let mut vc = make_test_vc();

    vc.sign_ed25519(
        &sk1,
        "did:key:z6MkSigner1#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    vc.sign_ed25519(
        &sk2,
        "did:key:z6MkSigner2#key-1".to_string(),
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
    assert!(results[0].ok, "signer1 failed: {}", results[0].error);
    assert!(results[1].ok, "signer2 failed: {}", results[1].error);
}

// ---------------------------------------------------------------------------
// 6. verify_all
// ---------------------------------------------------------------------------

#[test]
fn verify_all_succeeds_with_valid_signature() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();

    let mut vc = make_test_vc();
    vc.sign_ed25519(
        &sk,
        "did:key:z6MkTest#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    assert!(vc.verify_all(make_key_resolver(vk)).is_ok());
}

#[test]
fn verify_all_fails_with_no_proofs() {
    let vc = make_test_vc();
    let result = vc.verify_all(|_| Err("no key".to_string()));
    assert!(matches!(result, Err(VcError::NoProofs)));
}

#[test]
fn verify_all_fails_with_wrong_key() {
    let sk1 = SigningKey::generate(&mut OsRng);
    let sk2 = SigningKey::generate(&mut OsRng);
    let vk2 = sk2.verifying_key();

    let mut vc = make_test_vc();
    vc.sign_ed25519(
        &sk1,
        "did:key:z6MkTest#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    assert!(vc.verify_all(make_key_resolver(vk2)).is_err());
}

// ---------------------------------------------------------------------------
// 7. Float rejection in VC subject
// ---------------------------------------------------------------------------

#[test]
fn float_in_subject_rejects_signing() {
    let mut vc = make_test_vc();
    vc.credential_subject = json!({"amount": 1.5});
    let result = vc.signing_input();
    assert!(result.is_err(), "float in VC subject must be rejected");
}

// ---------------------------------------------------------------------------
// 8. VC serde roundtrip preserves structure
// ---------------------------------------------------------------------------

#[test]
fn vc_serde_roundtrip() {
    let sk = SigningKey::generate(&mut OsRng);
    let mut vc = make_test_vc();
    vc.sign_ed25519(
        &sk,
        "did:key:z6MkTest#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    let json_str = serde_json::to_string_pretty(&vc).unwrap();
    let deserialized: VerifiableCredential = serde_json::from_str(&json_str).unwrap();

    assert_eq!(vc.issuer, deserialized.issuer);
    assert_eq!(vc.proof.as_list().len(), deserialized.proof.as_list().len());
}
