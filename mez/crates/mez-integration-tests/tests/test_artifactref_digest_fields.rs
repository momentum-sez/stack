//! # ArtifactRef Digest Field Behavior
//!
//! Tests the behavior of ArtifactRef digest fields, verifying that the digest
//! stored in an ArtifactRef matches the content that was stored, that different
//! content produces different ArtifactRef digests, and that ArtifactRef can
//! round-trip through CAS storage and resolution.

use mez_core::{sha256_digest, CanonicalBytes};
use mez_crypto::{ArtifactRef, ContentAddressedStore};
use serde_json::json;

fn make_store() -> (tempfile::TempDir, ContentAddressedStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    (dir, store)
}

#[test]
fn artifact_ref_digest_matches_stored_content() {
    let (_dir, store) = make_store();

    let data = json!({"corridor": "PK-AE", "status": "active"});
    let artifact_ref = store.store("receipt", &data).unwrap();

    // Independently compute the expected digest
    let canonical = CanonicalBytes::new(&data).unwrap();
    let expected_digest = sha256_digest(&canonical);

    assert_eq!(
        artifact_ref.digest, expected_digest,
        "ArtifactRef digest must match independently computed digest"
    );
    assert_eq!(artifact_ref.artifact_type, "receipt");
}

#[test]
fn artifact_ref_from_different_content_differs() {
    let (_dir, store) = make_store();

    let data_a = json!({"key": "alpha", "value": 1});
    let data_b = json!({"key": "beta", "value": 2});

    let ref_a = store.store("test", &data_a).unwrap();
    let ref_b = store.store("test", &data_b).unwrap();

    assert_ne!(
        ref_a.digest, ref_b.digest,
        "different content must produce different ArtifactRef digests"
    );
}

#[test]
fn artifact_ref_roundtrip_through_store() {
    let (_dir, store) = make_store();

    let data = json!({
        "entity": "entity-001",
        "jurisdiction": "PK-RSEZ",
        "compliance": {"aml": "pass", "kyc": "pass"}
    });

    let artifact_ref = store.store("entity", &data).unwrap();

    // Resolve using the ArtifactRef's resolve_ref convenience method
    let resolved = store.resolve_ref(&artifact_ref).unwrap();
    assert!(resolved.is_some(), "stored artifact must be resolvable");

    // Parse the resolved bytes and verify semantic equivalence
    let resolved_value: serde_json::Value = serde_json::from_slice(&resolved.unwrap()).unwrap();
    let resolved_canonical = CanonicalBytes::new(&resolved_value).unwrap();
    let original_canonical = CanonicalBytes::new(&data).unwrap();

    assert_eq!(
        original_canonical.as_bytes(),
        resolved_canonical.as_bytes(),
        "round-tripped content must be canonically identical"
    );
}

#[test]
fn artifact_ref_new_validates_type() {
    let canonical = CanonicalBytes::new(&json!({"test": true})).unwrap();
    let digest = sha256_digest(&canonical);

    // Valid artifact types
    let valid_ref = ArtifactRef::new("lawpack", digest.clone());
    assert!(valid_ref.is_ok());

    let valid_ref2 = ArtifactRef::new("corridor-receipt", digest.clone());
    assert!(valid_ref2.is_ok());

    // Invalid artifact types
    let invalid_ref = ArtifactRef::new("", digest.clone());
    assert!(invalid_ref.is_err());

    let invalid_ref2 = ArtifactRef::new("has_underscore", digest.clone());
    assert!(invalid_ref2.is_err());
}

#[test]
fn artifact_ref_path_construction() {
    let canonical = CanonicalBytes::new(&json!({"path": "test"})).unwrap();
    let digest = sha256_digest(&canonical);
    let hex = digest.to_hex();

    let artifact_ref = ArtifactRef::new("vc", digest).unwrap();
    let path = artifact_ref.path_in(std::path::Path::new("/cas/artifacts"));

    let expected = format!("/cas/artifacts/vc/{hex}.json");
    assert_eq!(path.to_str().unwrap(), expected);
}

#[test]
fn artifact_ref_same_content_different_types() {
    let (_dir, store) = make_store();

    let data = json!({"shared": "content"});
    let ref_receipt = store.store("receipt", &data).unwrap();
    let ref_vc = store.store("vc", &data).unwrap();

    // Same content produces same digest regardless of artifact type
    assert_eq!(ref_receipt.digest, ref_vc.digest);

    // But they are stored in different namespaces
    assert_ne!(ref_receipt.artifact_type, ref_vc.artifact_type);

    // Both should be independently resolvable
    assert!(store.contains("receipt", &ref_receipt.digest).unwrap());
    assert!(store.contains("vc", &ref_vc.digest).unwrap());
}
