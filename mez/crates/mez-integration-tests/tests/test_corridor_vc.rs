//! # Corridor-Related Verifiable Credentials
//!
//! Tests corridor lifecycle transition VCs, corridor anchor proof VCs,
//! multi-jurisdiction corridor VCs, and tamper detection across all of these.
//! Verifies that the VC signing and verification pipeline correctly handles
//! corridor-specific credential subjects and proof types.

use mez_crypto::SigningKey;
use mez_vc::{ContextValue, CredentialTypeValue, ProofType, ProofValue, VerifiableCredential};
use rand_core::OsRng;
use serde_json::json;

fn make_corridor_lifecycle_vc(
    from_state: &str,
    to_state: &str,
    corridor_id: &str,
) -> VerifiableCredential {
    VerifiableCredential {
        context: ContextValue::Array(vec![json!("https://www.w3.org/2018/credentials/v1")]),
        id: Some(format!(
            "urn:mez:vc:corridor-transition:{corridor_id}:{from_state}-{to_state}"
        )),
        credential_type: CredentialTypeValue::Array(vec![
            "VerifiableCredential".to_string(),
            "CorridorLifecycleTransition".to_string(),
        ]),
        issuer: "did:mez:corridor-authority".to_string(),
        issuance_date: chrono::Utc::now(),
        expiration_date: None,
        credential_subject: json!({
            "corridor_id": corridor_id,
            "from_state": from_state,
            "to_state": to_state,
            "evidence_digest": "aa".repeat(32),
            "jurisdiction_a": "PK-RSEZ",
            "jurisdiction_b": "AE-DIFC"
        }),
        proof: ProofValue::default(),
    }
}

#[test]
fn corridor_vc_lifecycle_transition() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();

    let mut vc = make_corridor_lifecycle_vc("DRAFT", "PENDING", "corr-001");

    vc.sign_ed25519(
        &sk,
        "did:mez:corridor-authority#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    assert!(!vc.proof.is_empty());
    assert_eq!(vc.credential_subject["from_state"], "DRAFT");
    assert_eq!(vc.credential_subject["to_state"], "PENDING");

    let results = vc.verify(|_vm| Ok(vk.clone()));
    assert_eq!(results.len(), 1);
    assert!(
        results[0].ok,
        "transition VC verification failed: {}",
        results[0].error
    );
}

#[test]
fn corridor_vc_anchor_proof() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();

    let mut vc = VerifiableCredential {
        context: ContextValue::Array(vec![json!("https://www.w3.org/2018/credentials/v1")]),
        id: Some("urn:mez:vc:corridor-anchor:corr-002".to_string()),
        credential_type: CredentialTypeValue::Array(vec![
            "VerifiableCredential".to_string(),
            "CorridorAnchorProof".to_string(),
        ]),
        issuer: "did:mez:anchor-service".to_string(),
        issuance_date: chrono::Utc::now(),
        expiration_date: None,
        credential_subject: json!({
            "corridor_id": "corr-002",
            "anchor_target": "ethereum-sepolia",
            "mmr_root": "bb".repeat(32),
            "chain_height": 1000,
            "tx_hash": "cc".repeat(32),
            "block_number": 12345678
        }),
        proof: ProofValue::default(),
    };

    vc.sign_ed25519(
        &sk,
        "did:mez:anchor-service#key-1".to_string(),
        ProofType::MezEd25519Signature2025,
        None,
    )
    .unwrap();

    let results = vc.verify(|_vm| Ok(vk.clone()));
    assert_eq!(results.len(), 1);
    assert!(
        results[0].ok,
        "anchor proof VC verification failed: {}",
        results[0].error
    );

    // Verify credential subject fields
    assert_eq!(vc.credential_subject["anchor_target"], "ethereum-sepolia");
    assert_eq!(vc.credential_subject["chain_height"], 1000);
}

#[test]
fn corridor_vc_multi_jurisdiction() {
    let sk_pk = SigningKey::generate(&mut OsRng);
    let sk_ae = SigningKey::generate(&mut OsRng);
    let sk_sg = SigningKey::generate(&mut OsRng);
    let vk_pk = sk_pk.verifying_key();
    let vk_ae = sk_ae.verifying_key();
    let vk_sg = sk_sg.verifying_key();

    let mut vc = VerifiableCredential {
        context: ContextValue::Array(vec![json!("https://www.w3.org/2018/credentials/v1")]),
        id: Some("urn:mez:vc:multi-corridor:trilateral".to_string()),
        credential_type: CredentialTypeValue::Array(vec![
            "VerifiableCredential".to_string(),
            "CorridorMultiJurisdiction".to_string(),
        ]),
        issuer: "did:mez:governance-council".to_string(),
        issuance_date: chrono::Utc::now(),
        expiration_date: None,
        credential_subject: json!({
            "jurisdictions": ["PK-RSEZ", "AE-DIFC", "SG-JURONG"],
            "agreement_type": "trilateral",
            "compliance_domains": ["aml", "kyc", "sanctions"]
        }),
        proof: ProofValue::default(),
    };

    // Three jurisdictions sign
    vc.sign_ed25519(
        &sk_pk,
        "did:mez:jurisdiction:PK-RSEZ#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    vc.sign_ed25519(
        &sk_ae,
        "did:mez:jurisdiction:AE-DIFC#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    vc.sign_ed25519(
        &sk_sg,
        "did:mez:jurisdiction:SG-JURONG#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    assert_eq!(vc.proof.as_list().len(), 3);

    let results = vc.verify(move |vm: &str| {
        if vm.contains("PK-RSEZ") {
            Ok(vk_pk.clone())
        } else if vm.contains("AE-DIFC") {
            Ok(vk_ae.clone())
        } else if vm.contains("SG-JURONG") {
            Ok(vk_sg.clone())
        } else {
            Err(format!("unknown verification method: {vm}"))
        }
    });

    assert_eq!(results.len(), 3);
    for (i, r) in results.iter().enumerate() {
        assert!(r.ok, "jurisdiction {i} verification failed: {}", r.error);
    }
}

#[test]
fn corridor_vc_tamper_detection() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();

    let mut vc = make_corridor_lifecycle_vc("PENDING", "ACTIVE", "corr-003");

    vc.sign_ed25519(
        &sk,
        "did:mez:authority#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    // Verify succeeds before tampering
    let results_before = vc.verify(|_vm| Ok(vk.clone()));
    assert!(results_before[0].ok);

    // Tamper with the credential subject
    vc.credential_subject = json!({
        "corridor_id": "corr-003",
        "from_state": "PENDING",
        "to_state": "DEPRECATED",
        "evidence_digest": "ff".repeat(32),
        "jurisdiction_a": "PK-RSEZ",
        "jurisdiction_b": "AE-DIFC"
    });

    // Verification must fail after tampering
    let results_after = vc.verify(|_vm| Ok(vk.clone()));
    assert!(!results_after[0].ok, "tampered VC must fail verification");
}

#[test]
fn corridor_vc_signing_input_excludes_proof() {
    let sk = SigningKey::generate(&mut OsRng);

    let mut vc = make_corridor_lifecycle_vc("ACTIVE", "HALTED", "corr-004");
    let input_before = vc.signing_input().unwrap();

    vc.sign_ed25519(
        &sk,
        "did:mez:authority#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    let input_after = vc.signing_input().unwrap();

    // Signing input must be identical regardless of proof presence
    assert_eq!(input_before.as_bytes(), input_after.as_bytes());
}
