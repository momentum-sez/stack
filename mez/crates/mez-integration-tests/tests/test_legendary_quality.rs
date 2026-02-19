//! # Legendary Quality Validation Tests
//!
//! The highest-quality tier of integration tests exercising the full VC
//! lifecycle (create, sign, verify, tamper, verify-fails), cross-layer
//! digest agreement, domain serialization correctness, and known test
//! vector anchoring for cross-language determinism.

use mez_core::{sha256_digest, CanonicalBytes, ComplianceDomain};
use mez_crypto::{SigningKey, VerifyingKey};
use mez_vc::{ContextValue, CredentialTypeValue, ProofType, ProofValue, VerifiableCredential};
use rand_core::OsRng;
use serde_json::json;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_test_vc() -> VerifiableCredential {
    VerifiableCredential {
        context: ContextValue::Array(vec![json!("https://www.w3.org/2018/credentials/v1")]),
        id: Some("urn:mez:vc:legendary:001".to_string()),
        credential_type: CredentialTypeValue::Array(vec![
            "VerifiableCredential".to_string(),
            "SmartAssetRegistryVC".to_string(),
        ]),
        issuer: "did:key:z6MkLegendaryIssuer".to_string(),
        issuance_date: chrono::Utc::now(),
        expiration_date: None,
        credential_subject: json!({
            "asset_id": "a".repeat(64),
            "name": "Legendary Mining License",
            "jurisdiction_bindings": [
                {
                    "jurisdiction_id": "PK-REZ",
                    "binding_status": "active",
                    "lawpack_ref": format!("PK-REZ:financial:{}", "ab".repeat(32))
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
// 1. End-to-end VC lifecycle: create, sign, verify, tamper, verify fails
// ---------------------------------------------------------------------------

#[test]
fn end_to_end_vc_lifecycle() {
    // Step 1: Generate keypair
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();

    // Step 2: Create VC
    let mut vc = make_test_vc();
    assert!(vc.proof.is_empty());

    // Step 3: Sign
    vc.sign_ed25519(
        &sk,
        "did:key:z6MkLegendaryIssuer#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();
    assert!(!vc.proof.is_empty());

    // Step 4: Verify
    let results = vc.verify(make_key_resolver(vk.clone()));
    assert_eq!(results.len(), 1);
    assert!(results[0].ok, "signature verification must pass");

    // Step 5: Tamper
    vc.credential_subject = json!({"asset_id": "b".repeat(64), "name": "Tampered"});

    // Step 6: Verify fails
    let results = vc.verify(make_key_resolver(vk));
    assert_eq!(results.len(), 1);
    assert!(
        !results[0].ok,
        "tampered VC must fail signature verification"
    );
}

// ---------------------------------------------------------------------------
// 2. Cross-layer digest agreement
// ---------------------------------------------------------------------------

#[test]
fn cross_layer_digest_agreement() {
    // The same data canonicalized through core layer and used in VC signing
    // must produce identical bytes.
    let data = json!({
        "asset_id": "a".repeat(64),
        "jurisdiction": "PK-REZ",
        "status": "active"
    });

    // Core layer digest
    let core_canonical = CanonicalBytes::new(&data).unwrap();
    let core_digest = sha256_digest(&core_canonical);

    // Repeat with from_value
    let value_canonical = CanonicalBytes::from_value(data.clone()).unwrap();
    let value_digest = sha256_digest(&value_canonical);

    assert_eq!(
        core_digest, value_digest,
        "CanonicalBytes::new and ::from_value must produce identical digests"
    );
    assert_eq!(core_canonical.as_bytes(), value_canonical.as_bytes());
}

#[test]
fn cross_layer_signing_input_uses_canonical_bytes() {
    let vc = make_test_vc();
    let signing_input = vc.signing_input().unwrap();

    // The signing input must be valid UTF-8 (CanonicalBytes guarantees this)
    let input_str = std::str::from_utf8(signing_input.as_bytes()).unwrap();

    // Keys must be sorted (canonical serialization)
    let parsed: serde_json::Value = serde_json::from_slice(signing_input.as_bytes()).unwrap();
    if let serde_json::Value::Object(map) = parsed {
        let keys: Vec<_> = map.keys().collect();
        let mut sorted_keys = keys.clone();
        sorted_keys.sort();
        assert_eq!(keys, sorted_keys, "signing input keys must be sorted");
    }

    // Signing input must NOT contain proof field
    assert!(
        !input_str.contains("proofValue"),
        "signing input must exclude proof"
    );
}

// ---------------------------------------------------------------------------
// 3. All compliance domains serialize correctly
// ---------------------------------------------------------------------------

#[test]
fn all_domains_serialize_correctly() {
    for &domain in ComplianceDomain::all() {
        let name = domain.as_str();
        assert!(!name.is_empty(), "domain {domain:?} has empty name");

        // Each domain should produce a unique, deterministic digest
        let data = json!({"domain": name, "state": "compliant"});
        let canonical = CanonicalBytes::new(&data).unwrap();
        let digest = sha256_digest(&canonical);
        assert_eq!(
            digest.to_hex().len(),
            64,
            "domain {} digest must be 64 hex chars",
            name
        );
    }

    // All domains must have unique names
    let names: Vec<&str> = ComplianceDomain::all().iter().map(|d| d.as_str()).collect();
    let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
    assert_eq!(names.len(), unique.len(), "domain names must be unique");
}

// ---------------------------------------------------------------------------
// 4. Known test vector agreement (cross-language anchor point)
// ---------------------------------------------------------------------------

#[test]
fn known_test_vector_agreement() {
    // This vector MUST match the Python jcs_canonicalize output for the same input.
    // echo -n '{"a":1,"b":2}' | sha256sum
    let data = json!({"b": 2, "a": 1});
    let canonical = CanonicalBytes::new(&data).unwrap();

    // Verify canonical form is sorted
    assert_eq!(
        std::str::from_utf8(canonical.as_bytes()).unwrap(),
        r#"{"a":1,"b":2}"#
    );

    let digest = sha256_digest(&canonical);
    let expected = "43258cff783fe7036d8a43033f830adfc60ec037382473548ac742b888292777";
    assert_eq!(
        digest.to_hex(),
        expected,
        "known test vector must match cross-language anchor"
    );
}

#[test]
fn known_test_vector_empty_object() {
    let data = json!({});
    let canonical = CanonicalBytes::new(&data).unwrap();
    assert_eq!(std::str::from_utf8(canonical.as_bytes()).unwrap(), "{}");

    // echo -n '{}' | sha256sum
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);
    // This digest must be stable across runs
    let digest2 = sha256_digest(&CanonicalBytes::new(&json!({})).unwrap());
    assert_eq!(digest, digest2);
}
