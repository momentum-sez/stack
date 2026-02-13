//! # Lock File and Node Artifact References Integration Tests
//!
//! Python counterpart: `tests/test_lock_and_node_artifactrefs.py`
//!
//! Tests that artifact references from lock files and node operations
//! are deterministic, content-addressable, and differ for different content.

use msez_core::{sha256_digest, CanonicalBytes};
use msez_crypto::ContentAddressedStore;
use serde_json::json;

fn make_store() -> (tempfile::TempDir, ContentAddressedStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    (dir, store)
}

// ---------------------------------------------------------------------------
// 1. Lock artifact ref matches content digest
// ---------------------------------------------------------------------------

#[test]
fn lock_artifact_ref_matches_content() {
    let (_dir, store) = make_store();

    let lock_data = json!({
        "schema_version": "1.0",
        "jurisdiction_id": "PK-RSEZ",
        "locked_at": "2026-02-12T00:00:00Z",
        "packs": {
            "lawpack": {"digest": "aa".repeat(32)},
            "regpack": {"digest": "bb".repeat(32)},
            "licensepack": {"digest": "cc".repeat(32)}
        }
    });

    let artifact_ref = store.store("lockfile", &lock_data).unwrap();

    // The digest computed by CAS must match manual computation
    let manual_digest = sha256_digest(&CanonicalBytes::new(&lock_data).unwrap());
    assert_eq!(artifact_ref.digest, manual_digest);
}

// ---------------------------------------------------------------------------
// 2. Node artifact ref is deterministic
// ---------------------------------------------------------------------------

#[test]
fn node_artifact_ref_deterministic() {
    let node_data = json!({
        "node_type": "zone",
        "jurisdiction_id": "PK-RSEZ",
        "modules": ["tax/withholding", "aml/screening", "kyc/identity"],
        "version": 1
    });

    let d1 = sha256_digest(&CanonicalBytes::new(&node_data).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&node_data).unwrap());
    assert_eq!(d1, d2, "node artifact digest must be deterministic");
}

// ---------------------------------------------------------------------------
// 3. Lock and node refs differ for different content
// ---------------------------------------------------------------------------

#[test]
fn lock_and_node_refs_differ() {
    let lock_data = json!({
        "type": "lockfile",
        "jurisdiction_id": "PK-RSEZ",
        "digest": "aa".repeat(32)
    });

    let node_data = json!({
        "type": "node",
        "jurisdiction_id": "PK-RSEZ",
        "digest": "aa".repeat(32)
    });

    let d_lock = sha256_digest(&CanonicalBytes::new(&lock_data).unwrap());
    let d_node = sha256_digest(&CanonicalBytes::new(&node_data).unwrap());
    assert_ne!(
        d_lock, d_node,
        "lock and node with different types must have different digests"
    );
}

// ---------------------------------------------------------------------------
// 4. CAS store and resolve roundtrip for lock data
// ---------------------------------------------------------------------------

#[test]
fn cas_roundtrip_lock_data() {
    let (_dir, store) = make_store();

    let lock_data = json!({
        "schema_version": "1.0",
        "jurisdiction_id": "AE-DIFC",
        "packs": {
            "lawpack": {"digest": "dd".repeat(32)}
        }
    });

    let artifact_ref = store.store("lockfile", &lock_data).unwrap();
    let resolved = store.resolve("lockfile", &artifact_ref.digest).unwrap();
    assert!(resolved.is_some(), "stored lock data must be resolvable");

    let resolved_bytes = resolved.unwrap();
    let reparsed: serde_json::Value = serde_json::from_slice(&resolved_bytes).unwrap();
    assert_eq!(reparsed["jurisdiction_id"], "AE-DIFC");
}

// ---------------------------------------------------------------------------
// 5. Multiple lock files produce different digests
// ---------------------------------------------------------------------------

#[test]
fn multiple_lock_files_different_digests() {
    let lock_v1 = json!({"version": "1.0", "packs": {"lawpack": "aa".repeat(32)}});
    let lock_v2 = json!({"version": "2.0", "packs": {"lawpack": "bb".repeat(32)}});

    let d1 = sha256_digest(&CanonicalBytes::new(&lock_v1).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&lock_v2).unwrap());
    assert_ne!(
        d1, d2,
        "different lock file versions must produce different digests"
    );
}

// ---------------------------------------------------------------------------
// 6. Identical content in different stores yields same digest
// ---------------------------------------------------------------------------

#[test]
fn identical_content_cross_store_same_digest() {
    let (_dir1, store1) = make_store();
    let (_dir2, store2) = make_store();

    let data = json!({"canonical": "test", "value": 42});

    let ref1 = store1.store("test", &data).unwrap();
    let ref2 = store2.store("test", &data).unwrap();
    assert_eq!(ref1.digest, ref2.digest);
}
