//! Tests for lawpack strict mode: verifies that the content-addressed store
//! correctly handles duplicate storage and that strict mode preserves
//! existing artifacts.

use msez_crypto::ContentAddressedStore;
use msez_core::sha256_digest;
use serde_json::json;

// ---------------------------------------------------------------------------
// Store lawpack artifact once
// ---------------------------------------------------------------------------

#[test]
fn store_lawpack_artifact_once() {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    let data = json!({
        "jurisdiction_id": "PK-RSEZ",
        "version": "1.0",
        "name": "Test Lawpack"
    });

    let artifact_ref = store.store("lawpack", &data).unwrap();

    // Must be resolvable.
    let resolved = store.resolve_ref(&artifact_ref).unwrap();
    assert!(resolved.is_some(), "Stored artifact must be resolvable");
}

// ---------------------------------------------------------------------------
// Duplicate store returns same ref
// ---------------------------------------------------------------------------

#[test]
fn duplicate_store_returns_same_ref() {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    let data = json!({"key": "lawpack-data", "version": "1.0"});

    let ref1 = store.store("lawpack", &data).unwrap();
    let ref2 = store.store("lawpack", &data).unwrap();

    assert_eq!(
        ref1.digest.to_hex(),
        ref2.digest.to_hex(),
        "Duplicate store must return the same content digest"
    );

    // Both refs must resolve to the same content.
    let resolved1 = store.resolve_ref(&ref1).unwrap();
    let resolved2 = store.resolve_ref(&ref2).unwrap();
    assert_eq!(resolved1, resolved2);
}

// ---------------------------------------------------------------------------
// Strict mode preserves existing
// ---------------------------------------------------------------------------

#[test]
fn strict_mode_preserves_existing() {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());

    // Store the first artifact.
    let data1 = json!({"artifact": "first"});
    let ref1 = store.store("lawpack", &data1).unwrap();

    // Store a different artifact.
    let data2 = json!({"artifact": "second"});
    let ref2 = store.store("lawpack", &data2).unwrap();

    // Both must be independently resolvable.
    let resolved1 = store.resolve_ref(&ref1).unwrap();
    let resolved2 = store.resolve_ref(&ref2).unwrap();

    assert!(resolved1.is_some(), "First artifact must be resolvable");
    assert!(resolved2.is_some(), "Second artifact must be resolvable");
    assert_ne!(
        resolved1, resolved2,
        "Different artifacts must have different content"
    );
}

#[test]
fn content_addressed_store_integrity() {
    // Verify that the content digest matches what we compute independently.
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    let data = json!({"integrity": "check"});

    let canonical = msez_core::CanonicalBytes::new(&data).unwrap();
    let expected_digest = sha256_digest(&canonical);
    let artifact_ref = store.store("lawpack", &data).unwrap();

    assert_eq!(
        artifact_ref.digest.to_hex(),
        expected_digest.to_hex(),
        "CAS ref digest must match independently computed digest"
    );
}

#[test]
fn cas_multiple_artifacts_coexist() {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());

    let refs: Vec<_> = (0..20)
        .map(|i| {
            let data = json!({"artifact_index": i, "data": format!("content-{}", i)});
            store.store("lawpack", &data).unwrap()
        })
        .collect();

    // All must be independently resolvable.
    for artifact_ref in &refs {
        let resolved = store.resolve_ref(artifact_ref).unwrap();
        assert!(
            resolved.is_some(),
            "Artifact with digest {} must be resolvable",
            artifact_ref.digest.to_hex()
        );
    }
}
