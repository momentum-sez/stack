//! Rust counterpart of tests/scenarios/test_smart_asset_integration_scaffold.py
//! Smart asset integration scaffolding tests.

use msez_core::{CanonicalBytes, sha256_digest, JurisdictionId, ComplianceDomain};
use msez_crypto::ContentAddressedStore;
use msez_vc::{VerifiableCredential, ContextValue, CredentialTypeValue, ProofValue};
use msez_tensor::tensor::{ComplianceTensor, DefaultJurisdiction};
use serde_json::json;

#[test]
fn smart_asset_registration_vc() {
    let vc = VerifiableCredential {
        context: ContextValue::Array(vec![json!("https://www.w3.org/2018/credentials/v1")]),
        id: Some("urn:msez:vc:smart-asset:scaffold:001".to_string()),
        credential_type: CredentialTypeValue::Array(vec![
            "VerifiableCredential".to_string(),
            "SmartAssetRegistryVC".to_string(),
        ]),
        issuer: "did:key:z6MkScaffoldIssuer".to_string(),
        issuance_date: chrono::Utc::now(),
        expiration_date: None,
        credential_subject: json!({"asset_id": "a".repeat(64), "name": "Scaffold Asset"}),
        proof: ProofValue::default(),
    };
    let input = vc.signing_input().unwrap();
    assert!(!input.as_bytes().is_empty());
}

#[test]
fn smart_asset_compliance_evaluation() {
    let jid = JurisdictionId::new("PK-RSEZ").unwrap();
    let config = DefaultJurisdiction::new(jid);
    let _tensor = ComplianceTensor::new(config);
    let domains = ComplianceDomain::all();
    assert_eq!(domains.len(), 20);
}

#[test]
fn smart_asset_cas_storage() {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    let asset_data = json!({"asset_id": "test", "type": "mining_license", "jurisdiction": "PK-RSEZ"});
    let aref = store.store("smart-asset", &asset_data).unwrap();
    let resolved = store.resolve("smart-asset", &aref.digest).unwrap();
    assert!(resolved.is_some());
}

#[test]
fn smart_asset_digest_chain() {
    let d1 = sha256_digest(&CanonicalBytes::new(&json!({"step": 1})).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&json!({"step": 2, "prev": d1.to_hex()})).unwrap());
    assert_ne!(d1, d2);
    assert_eq!(d1.to_hex().len(), 64);
    assert_eq!(d2.to_hex().len(), 64);
}
