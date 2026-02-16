//! # Additional Artifact Types in Content-Addressed Storage
//!
//! Tests that the CAS store correctly handles various artifact types including
//! JSON objects, arrays, nested structures, and empty objects. Verifies that
//! different content produces different digests and that empty objects produce
//! stable, deterministic digests across invocations.

use msez_crypto::ContentAddressedStore;
use serde_json::json;

fn make_store() -> (tempfile::TempDir, ContentAddressedStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    (dir, store)
}

#[test]
fn store_json_object_artifact() {
    let (_dir, store) = make_store();
    let data = json!({
        "entity_id": "ent-001",
        "jurisdiction": "PK-RSEZ",
        "status": "active"
    });
    let artifact_ref = store.store("entity", &data).unwrap();
    assert_eq!(artifact_ref.artifact_type, "entity");
    assert_eq!(artifact_ref.digest.to_hex().len(), 64);

    let resolved = store.resolve("entity", &artifact_ref.digest).unwrap();
    assert!(resolved.is_some(), "CAS resolve should return stored data");
    // Verify the resolved content matches what was stored — not just presence.
    let bytes = resolved.unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(parsed["entity_id"], "ent-001");
    assert_eq!(parsed["jurisdiction"], "PK-RSEZ");
    assert_eq!(parsed["status"], "active");
}

#[test]
fn store_array_artifact() {
    let (_dir, store) = make_store();
    let data = json!([
        {"id": "item-1", "value": 100},
        {"id": "item-2", "value": 200},
        {"id": "item-3", "value": 300}
    ]);
    let artifact_ref = store.store("batch", &data).unwrap();
    assert_eq!(artifact_ref.artifact_type, "batch");

    let resolved = store.resolve("batch", &artifact_ref.digest).unwrap();
    assert!(resolved.is_some());
    let bytes = resolved.unwrap();
    let reparsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(reparsed.is_array());
    assert_eq!(reparsed.as_array().unwrap().len(), 3);
}

#[test]
fn store_nested_artifact() {
    let (_dir, store) = make_store();
    let data = json!({
        "corridor": {
            "id": "corr-001",
            "jurisdictions": {
                "a": {"code": "PK-RSEZ", "zone": "rashakai"},
                "b": {"code": "AE-DIFC", "zone": "difc"}
            },
            "receipts": [
                {"seq": 0, "hash": "aa".repeat(32)},
                {"seq": 1, "hash": "bb".repeat(32)}
            ]
        },
        "version": 1
    });
    let artifact_ref = store.store("corridor-snapshot", &data).unwrap();
    assert_eq!(artifact_ref.artifact_type, "corridor-snapshot");

    let resolved = store
        .resolve("corridor-snapshot", &artifact_ref.digest)
        .unwrap();
    assert!(resolved.is_some());
}

#[test]
fn store_empty_object_produces_stable_digest() {
    let (_dir, store) = make_store();
    let empty = json!({});

    let ref1 = store.store("meta", &empty).unwrap();
    let ref2 = store.store("meta", &empty).unwrap();

    // Same content must produce same digest (idempotent)
    assert_eq!(ref1.digest, ref2.digest);

    // Verify across a fresh store with the same content
    let (_dir2, store2) = make_store();
    let ref3 = store2.store("meta", &empty).unwrap();
    assert_eq!(ref1.digest, ref3.digest);
}

#[test]
fn different_types_produce_different_digests() {
    let (_dir, store) = make_store();
    let obj = json!({"type": "object", "key": "value"});
    let arr = json!([1, 2, 3]);
    let nested = json!({"outer": {"inner": true}});
    let simple = json!({"simple": 42});

    let ref_obj = store.store("artifact", &obj).unwrap();
    let ref_arr = store.store("artifact", &arr).unwrap();
    let ref_nested = store.store("artifact", &nested).unwrap();
    let ref_simple = store.store("artifact", &simple).unwrap();

    // All four different inputs must produce different digests
    let digests = [
        ref_obj.digest.to_hex(),
        ref_arr.digest.to_hex(),
        ref_nested.digest.to_hex(),
        ref_simple.digest.to_hex(),
    ];
    for i in 0..digests.len() {
        for j in (i + 1)..digests.len() {
            assert_ne!(
                digests[i], digests[j],
                "artifacts {i} and {j} should have different digests"
            );
        }
    }
}
