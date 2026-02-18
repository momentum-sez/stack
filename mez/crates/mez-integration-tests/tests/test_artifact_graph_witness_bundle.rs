//! # Witness Bundle Artifacts
//!
//! Tests witness bundle artifacts where multiple independent attestation
//! witnesses contribute to a single bundle. Verifies that witness ordering
//! is deterministic through canonicalization, that all witnesses are included
//! in the bundle digest, and that the bundle stores correctly in CAS.

use mez_core::{sha256_digest, CanonicalBytes};
use mez_crypto::ContentAddressedStore;
use serde_json::json;

fn make_store() -> (tempfile::TempDir, ContentAddressedStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    (dir, store)
}

fn make_witness(id: &str, attestation: &str) -> serde_json::Value {
    json!({
        "witness_id": id,
        "attestation": attestation,
        "timestamp": "2026-01-15T12:00:00Z",
        "jurisdiction": "PK-RSEZ"
    })
}

#[test]
fn witness_bundle_stores_correctly() {
    let (_dir, store) = make_store();

    let bundle = json!({
        "bundle_type": "witness-attestation",
        "witnesses": [
            make_witness("w1", "fork-branch-a-valid"),
            make_witness("w2", "fork-branch-a-valid"),
            make_witness("w3", "fork-branch-a-valid")
        ],
        "quorum_reached": true,
        "resolution": "branch-a"
    });

    let artifact_ref = store.store("witness-bundle", &bundle).unwrap();
    assert_eq!(artifact_ref.artifact_type, "witness-bundle");

    let resolved = store
        .resolve("witness-bundle", &artifact_ref.digest)
        .unwrap();
    assert!(resolved.is_some());

    let parsed: serde_json::Value = serde_json::from_slice(&resolved.unwrap()).unwrap();
    assert_eq!(parsed["witnesses"].as_array().unwrap().len(), 3);
    assert_eq!(parsed["quorum_reached"], true);
}

#[test]
fn witness_bundle_digest_includes_all_witnesses() {
    // Changing any single witness must change the bundle digest
    let bundle_3 = json!({
        "bundle_type": "witness-attestation",
        "witnesses": [
            make_witness("w1", "valid"),
            make_witness("w2", "valid"),
            make_witness("w3", "valid")
        ]
    });

    let bundle_modified = json!({
        "bundle_type": "witness-attestation",
        "witnesses": [
            make_witness("w1", "valid"),
            make_witness("w2", "CHANGED"),
            make_witness("w3", "valid")
        ]
    });

    let digest_original = sha256_digest(&CanonicalBytes::new(&bundle_3).unwrap());
    let digest_modified = sha256_digest(&CanonicalBytes::new(&bundle_modified).unwrap());

    assert_ne!(
        digest_original, digest_modified,
        "changing a witness must change the bundle digest"
    );
}

#[test]
fn witness_bundle_ordering_deterministic() {
    // Since canonicalization sorts object keys, the same bundle data
    // must always produce the same canonical bytes regardless of
    // construction order.
    let bundle_a = json!({
        "witnesses": [make_witness("w1", "ok"), make_witness("w2", "ok")],
        "bundle_type": "witness-attestation"
    });

    let bundle_b = json!({
        "bundle_type": "witness-attestation",
        "witnesses": [make_witness("w1", "ok"), make_witness("w2", "ok")]
    });

    let canonical_a = CanonicalBytes::new(&bundle_a).unwrap();
    let canonical_b = CanonicalBytes::new(&bundle_b).unwrap();

    assert_eq!(
        canonical_a.as_bytes(),
        canonical_b.as_bytes(),
        "object key order must not affect canonical representation"
    );

    let digest_a = sha256_digest(&canonical_a);
    let digest_b = sha256_digest(&canonical_b);
    assert_eq!(digest_a, digest_b);
}

#[test]
fn witness_count_affects_digest() {
    let bundle_2 = json!({
        "bundle_type": "witness-attestation",
        "witnesses": [make_witness("w1", "ok"), make_witness("w2", "ok")]
    });

    let bundle_3 = json!({
        "bundle_type": "witness-attestation",
        "witnesses": [make_witness("w1", "ok"), make_witness("w2", "ok"), make_witness("w3", "ok")]
    });

    let digest_2 = sha256_digest(&CanonicalBytes::new(&bundle_2).unwrap());
    let digest_3 = sha256_digest(&CanonicalBytes::new(&bundle_3).unwrap());

    assert_ne!(
        digest_2, digest_3,
        "different witness counts must produce different digests"
    );
}
