//! # Artifact Bundle Attestation
//!
//! Tests that bundles of artifacts can be stored in CAS and have attestation
//! digests computed deterministically. Verifies that bundle-level digests are
//! stable across invocations and that attestation metadata is preserved through
//! the content-addressed storage pipeline.

use mez_core::{sha256_digest, CanonicalBytes};
use mez_crypto::ContentAddressedStore;
use serde_json::json;

fn make_store() -> (tempfile::TempDir, ContentAddressedStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    (dir, store)
}

#[test]
fn bundle_digest_is_deterministic() {
    let bundle = json!({
        "bundle_type": "lawpack-trilogy",
        "artifacts": [
            {"type": "lawpack", "digest": "aa".repeat(32)},
            {"type": "regpack", "digest": "bb".repeat(32)},
            {"type": "licensepack", "digest": "cc".repeat(32)}
        ],
        "version": 1
    });

    let canonical1 = CanonicalBytes::new(&bundle).unwrap();
    let digest1 = sha256_digest(&canonical1);

    let canonical2 = CanonicalBytes::new(&bundle).unwrap();
    let digest2 = sha256_digest(&canonical2);

    assert_eq!(digest1, digest2);
    assert_eq!(digest1.to_hex().len(), 64);
}

#[test]
fn bundle_with_attestation_fields() {
    let bundle = json!({
        "bundle_type": "attested-evidence",
        "artifacts": [
            {"type": "receipt", "digest": "dd".repeat(32)},
            {"type": "vc", "digest": "ee".repeat(32)}
        ],
        "attestation": {
            "attester": "did:key:z6MkAttester",
            "timestamp": "2026-01-15T12:00:00Z",
            "jurisdiction": "PK-RSEZ"
        }
    });

    let canonical = CanonicalBytes::new(&bundle).unwrap();
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);

    // Verify the canonical bytes parse back to the same structure
    let reparsed: serde_json::Value = serde_json::from_slice(canonical.as_bytes()).unwrap();
    assert_eq!(reparsed["attestation"]["attester"], "did:key:z6MkAttester");
    assert_eq!(reparsed["attestation"]["jurisdiction"], "PK-RSEZ");
}

#[test]
fn bundle_content_addressed_storage() {
    let (_dir, store) = make_store();

    let bundle = json!({
        "bundle_type": "corridor-evidence",
        "components": [
            {"artifact": "bilateral-agreement", "digest": "11".repeat(32)},
            {"artifact": "regulatory-approval-a", "digest": "22".repeat(32)},
            {"artifact": "regulatory-approval-b", "digest": "33".repeat(32)}
        ]
    });

    let artifact_ref = store.store("bundle", &bundle).unwrap();
    assert_eq!(artifact_ref.artifact_type, "bundle");

    // Verify the stored bundle can be resolved
    let resolved = store.resolve("bundle", &artifact_ref.digest).unwrap();
    assert!(resolved.is_some());

    // Verify the resolved content matches the original
    let resolved_bytes = resolved.unwrap();
    let resolved_value: serde_json::Value = serde_json::from_slice(&resolved_bytes).unwrap();
    let resolved_canonical = CanonicalBytes::new(&resolved_value).unwrap();
    let original_canonical = CanonicalBytes::new(&bundle).unwrap();
    assert_eq!(original_canonical, resolved_canonical);
}
