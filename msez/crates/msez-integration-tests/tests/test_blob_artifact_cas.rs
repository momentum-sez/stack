//! # Blob-Like Artifact Storage in CAS
//!
//! Tests storing blob-like artifacts (large JSON payloads, base64-encoded data
//! fields, and deeply nested structures) in the content-addressed store.
//! Verifies that large artifacts maintain digest stability and that the CAS
//! correctly handles payloads of varying sizes.

use msez_core::{sha256_digest, CanonicalBytes};
use msez_crypto::ContentAddressedStore;
use serde_json::json;

fn make_store() -> (tempfile::TempDir, ContentAddressedStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    (dir, store)
}

#[test]
fn blob_artifact_store_and_resolve() {
    let (_dir, store) = make_store();

    // Simulate a blob-like artifact with a large base64-encoded payload field
    let payload = "a".repeat(10_000);
    let blob = json!({
        "blob_type": "document-scan",
        "content_type": "application/pdf",
        "payload_b64": payload,
        "metadata": {
            "filename": "incorporation-certificate.pdf",
            "size_bytes": 10000
        }
    });

    let artifact_ref = store.store("blob", &blob).unwrap();
    assert_eq!(artifact_ref.artifact_type, "blob");

    let resolved = store.resolve("blob", &artifact_ref.digest).unwrap();
    assert!(resolved.is_some());

    let bytes = resolved.unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(parsed["blob_type"], "document-scan");
    assert_eq!(parsed["payload_b64"].as_str().unwrap().len(), 10_000);
}

#[test]
fn large_blob_artifact() {
    let (_dir, store) = make_store();

    // Create a moderately large artifact with many entries
    let entries: Vec<serde_json::Value> = (0..500)
        .map(|i| {
            json!({
                "index": i,
                "entity_id": format!("ent-{:04}", i),
                "status": "active"
            })
        })
        .collect();

    let blob = json!({
        "blob_type": "entity-registry-snapshot",
        "entries": entries,
        "total_count": 500
    });

    let artifact_ref = store.store("registry", &blob).unwrap();
    assert_eq!(artifact_ref.artifact_type, "registry");

    // Verify it resolves correctly
    let resolved = store.resolve("registry", &artifact_ref.digest).unwrap();
    assert!(resolved.is_some());

    let parsed: serde_json::Value = serde_json::from_slice(&resolved.unwrap()).unwrap();
    assert_eq!(parsed["total_count"], 500);
    assert_eq!(parsed["entries"].as_array().unwrap().len(), 500);
}

#[test]
fn blob_digest_stability() {
    // The same blob data must always produce the same digest
    let payload = "x".repeat(5_000);
    let blob = json!({
        "blob_type": "evidence",
        "data": payload,
        "version": 1
    });

    let canonical_1 = CanonicalBytes::new(&blob).unwrap();
    let digest_1 = sha256_digest(&canonical_1);

    let canonical_2 = CanonicalBytes::new(&blob).unwrap();
    let digest_2 = sha256_digest(&canonical_2);

    assert_eq!(digest_1, digest_2);

    // Store in CAS and verify the stored digest matches
    let (_dir, store) = make_store();
    let artifact_ref = store.store("evidence", &blob).unwrap();
    assert_eq!(artifact_ref.digest, digest_1);
}

#[test]
fn blob_with_nested_arrays() {
    let (_dir, store) = make_store();

    let blob = json!({
        "matrix": [
            [1, 2, 3, 4, 5],
            [6, 7, 8, 9, 10],
            [11, 12, 13, 14, 15]
        ],
        "labels": ["row-a", "row-b", "row-c"]
    });

    let artifact_ref = store.store("tensor-data", &blob).unwrap();

    let resolved = store.resolve("tensor-data", &artifact_ref.digest).unwrap();
    assert!(resolved.is_some());

    let parsed: serde_json::Value = serde_json::from_slice(&resolved.unwrap()).unwrap();
    let matrix = parsed["matrix"].as_array().unwrap();
    assert_eq!(matrix.len(), 3);
    assert_eq!(matrix[0].as_array().unwrap().len(), 5);
}
