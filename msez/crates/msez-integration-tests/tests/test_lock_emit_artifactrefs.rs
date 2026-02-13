//! # Lock File Emission of Artifact References Integration Tests
//!
//! Python counterpart: `tests/test_lock_emit_artifactrefs.py`
//!
//! Tests lock file emission behavior:
//! - Emitting artifact references from lock data
//! - Emitted references are resolvable from CAS
//! - Multiple emissions from the same data are deterministic

use msez_core::{sha256_digest, CanonicalBytes};
use msez_crypto::ContentAddressedStore;
use serde_json::json;

fn make_store() -> (tempfile::TempDir, ContentAddressedStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    (dir, store)
}

// ---------------------------------------------------------------------------
// 1. Emit artifact ref from lock file data
// ---------------------------------------------------------------------------

#[test]
fn emit_artifact_ref_from_lock() {
    let (_dir, store) = make_store();

    let lock = json!({
        "schema_version": "1.0",
        "jurisdiction_id": "PK-RSEZ",
        "lawpack_digest": "aa".repeat(32),
        "regpack_digest": "bb".repeat(32),
        "licensepack_digest": "cc".repeat(32),
        "locked_at": "2026-02-12T00:00:00Z"
    });

    let artifact_ref = store.store("lockfile", &lock).unwrap();
    assert_eq!(artifact_ref.artifact_type, "lockfile");
    assert_eq!(artifact_ref.digest.to_hex().len(), 64);
}

// ---------------------------------------------------------------------------
// 2. Emitted ref is resolvable
// ---------------------------------------------------------------------------

#[test]
fn emitted_ref_resolvable() {
    let (_dir, store) = make_store();

    let lock = json!({
        "jurisdiction_id": "AE-DIFC",
        "modules": ["corporate/formation", "tax/withholding"],
        "pack_version": 3
    });

    let artifact_ref = store.store("lockfile", &lock).unwrap();
    let resolved = store.resolve("lockfile", &artifact_ref.digest).unwrap();
    assert!(
        resolved.is_some(),
        "emitted lockfile ref must be resolvable"
    );

    let resolved_bytes = resolved.unwrap();
    let reparsed: serde_json::Value = serde_json::from_slice(&resolved_bytes).unwrap();
    assert_eq!(reparsed["jurisdiction_id"], "AE-DIFC");
}

// ---------------------------------------------------------------------------
// 3. Multiple emissions are deterministic
// ---------------------------------------------------------------------------

#[test]
fn multiple_emissions_deterministic() {
    let (_dir, store) = make_store();

    let lock = json!({
        "schema_version": "1.0",
        "jurisdiction_id": "PK-RSEZ",
        "content_hash": "dd".repeat(32)
    });

    let ref1 = store.store("lockfile", &lock).unwrap();
    let ref2 = store.store("lockfile", &lock).unwrap();
    let ref3 = store.store("lockfile", &lock).unwrap();

    assert_eq!(ref1.digest, ref2.digest, "first and second must match");
    assert_eq!(ref2.digest, ref3.digest, "second and third must match");
}

// ---------------------------------------------------------------------------
// 4. Emission includes correct artifact type
// ---------------------------------------------------------------------------

#[test]
fn emission_preserves_artifact_type() {
    let (_dir, store) = make_store();

    let types = ["lockfile", "lawpack", "regpack", "licensepack"];
    let data = json!({"test": true});

    for artifact_type in &types {
        let artifact_ref = store.store(artifact_type, &data).unwrap();
        assert_eq!(
            artifact_ref.artifact_type, *artifact_type,
            "artifact type must be preserved in ref"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. Different lock content produces different refs
// ---------------------------------------------------------------------------

#[test]
fn different_content_different_refs() {
    let (_dir, store) = make_store();

    let lock_a = json!({"jurisdiction_id": "PK-RSEZ", "version": 1});
    let lock_b = json!({"jurisdiction_id": "AE-DIFC", "version": 1});

    let ref_a = store.store("lockfile", &lock_a).unwrap();
    let ref_b = store.store("lockfile", &lock_b).unwrap();
    assert_ne!(
        ref_a.digest, ref_b.digest,
        "different lock content must produce different digests"
    );
}

// ---------------------------------------------------------------------------
// 6. Manual digest matches CAS digest
// ---------------------------------------------------------------------------

#[test]
fn manual_digest_matches_cas_digest() {
    let (_dir, store) = make_store();

    let data = json!({"test_field": "value", "number": 42});
    let artifact_ref = store.store("test", &data).unwrap();
    let manual_digest = sha256_digest(&CanonicalBytes::new(&data).unwrap());

    assert_eq!(artifact_ref.digest, manual_digest);
}
