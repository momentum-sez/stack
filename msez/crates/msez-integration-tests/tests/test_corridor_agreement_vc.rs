//! # Corridor Bilateral Agreement Verifiable Credentials
//!
//! Tests the creation, signing, and verification of corridor bilateral
//! agreement VCs. These VCs encode the bilateral agreement between two
//! jurisdictions establishing a trade corridor, and must carry Ed25519
//! proofs from both parties.

use msez_core::{sha256_digest, JurisdictionId};
use msez_crypto::SigningKey;
use msez_vc::{
    ContextValue, CredentialTypeValue, ProofType, ProofValue, VerifiableCredential,
};
use rand_core::OsRng;
use serde_json::json;

fn make_corridor_agreement_vc(
    jurisdiction_a: &str,
    jurisdiction_b: &str,
) -> VerifiableCredential {
    VerifiableCredential {
        context: ContextValue::Array(vec![json!("https://www.w3.org/2018/credentials/v1")]),
        id: Some(format!(
            "urn:msez:vc:corridor-agreement:{}-{}",
            jurisdiction_a, jurisdiction_b
        )),
        credential_type: CredentialTypeValue::Array(vec![
            "VerifiableCredential".to_string(),
            "CorridorBilateralAgreement".to_string(),
        ]),
        issuer: format!("did:msez:jurisdiction:{jurisdiction_a}"),
        issuance_date: chrono::Utc::now(),
        expiration_date: None,
        credential_subject: json!({
            "agreement_type": "bilateral-corridor",
            "jurisdiction_a": jurisdiction_a,
            "jurisdiction_b": jurisdiction_b,
            "effective_date": "2026-01-01T00:00:00Z",
            "compliance_domains": ["aml", "kyc", "sanctions", "tax"],
            "settlement_currency": "USD",
            "watcher_quorum": 3
        }),
        proof: ProofValue::default(),
    }
}

#[test]
fn corridor_agreement_vc_creation() {
    let ja = JurisdictionId::new("PK-RSEZ").unwrap();
    let jb = JurisdictionId::new("AE-DIFC").unwrap();

    let vc = make_corridor_agreement_vc(ja.as_str(), jb.as_str());

    assert!(vc.credential_type.contains_vc_type());
    assert_eq!(vc.issuer, "did:msez:jurisdiction:PK-RSEZ");
    assert_eq!(
        vc.credential_subject["jurisdiction_a"],
        "PK-RSEZ"
    );
    assert_eq!(
        vc.credential_subject["jurisdiction_b"],
        "AE-DIFC"
    );
    assert!(vc.proof.is_empty());
}

#[test]
fn corridor_agreement_vc_sign_verify() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();

    let mut vc = make_corridor_agreement_vc("PK-RSEZ", "AE-DIFC");
    vc.sign_ed25519(
        &sk,
        "did:key:z6MkPKRSEZ#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    assert!(!vc.proof.is_empty());
    assert_eq!(vc.proof.as_list().len(), 1);

    // Verify the signature
    let results = vc.verify(|_vm| Ok(vk.clone()));
    assert_eq!(results.len(), 1);
    assert!(results[0].ok, "verification failed: {}", results[0].error);
}

#[test]
fn corridor_agreement_vc_bilateral_fields() {
    let sk_a = SigningKey::generate(&mut OsRng);
    let sk_b = SigningKey::generate(&mut OsRng);
    let vk_a = sk_a.verifying_key();
    let vk_b = sk_b.verifying_key();

    let mut vc = make_corridor_agreement_vc("PK-RSEZ", "AE-DIFC");

    // Both jurisdictions sign the agreement
    vc.sign_ed25519(
        &sk_a,
        "did:msez:jurisdiction:PK-RSEZ#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    vc.sign_ed25519(
        &sk_b,
        "did:msez:jurisdiction:AE-DIFC#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    assert_eq!(vc.proof.as_list().len(), 2);

    // Both signatures must verify with the correct key resolver
    let results = vc.verify(move |vm: &str| {
        if vm.contains("PK-RSEZ") {
            Ok(vk_a.clone())
        } else if vm.contains("AE-DIFC") {
            Ok(vk_b.clone())
        } else {
            Err(format!("unknown verification method: {vm}"))
        }
    });

    assert_eq!(results.len(), 2);
    assert!(results[0].ok, "jurisdiction A verification failed: {}", results[0].error);
    assert!(results[1].ok, "jurisdiction B verification failed: {}", results[1].error);
}

#[test]
fn corridor_agreement_vc_signing_input_deterministic() {
    let vc = make_corridor_agreement_vc("PK-RSEZ", "AE-DIFC");

    let input_1 = vc.signing_input().unwrap();
    let input_2 = vc.signing_input().unwrap();

    assert_eq!(input_1.as_bytes(), input_2.as_bytes());

    // The signing input digest must be a valid 64-char hex string
    let digest = sha256_digest(&input_1);
    assert_eq!(digest.to_hex().len(), 64);
}
