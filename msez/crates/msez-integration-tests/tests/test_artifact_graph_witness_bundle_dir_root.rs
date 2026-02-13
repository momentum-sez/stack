//! # Directory-Rooted Witness Bundle Artifacts
//!
//! Tests witness bundle artifacts organized under a directory root in the CAS.
//! Verifies that witness bundles stored under different artifact type namespaces
//! maintain independent digest spaces, that directory-rooted bundles produce
//! deterministic digests, and that the root digest captures all nested content.

use msez_core::{sha256_digest, CanonicalBytes};
use msez_crypto::ContentAddressedStore;
use serde_json::json;

fn make_store() -> (tempfile::TempDir, ContentAddressedStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    (dir, store)
}

#[test]
fn dir_root_witness_bundle_digest() {
    let (_dir, store) = make_store();

    // Store individual witness attestations
    let w1 = json!({"witness": "watcher-pk-001", "branch": "A", "valid": true});
    let w2 = json!({"witness": "watcher-ae-001", "branch": "A", "valid": true});
    let ref1 = store.store("witness", &w1).unwrap();
    let ref2 = store.store("witness", &w2).unwrap();

    // Create a directory root that references all witness artifacts
    let dir_root = json!({
        "root_type": "witness-bundle-root",
        "witness_digests": [
            ref1.digest.to_hex(),
            ref2.digest.to_hex()
        ],
        "corridor_id": "test-corridor",
        "fork_height": 42
    });

    let root_ref = store.store("witness-root", &dir_root).unwrap();
    assert_eq!(root_ref.artifact_type, "witness-root");

    // Compute the root digest independently
    let canonical = CanonicalBytes::new(&dir_root).unwrap();
    let expected = sha256_digest(&canonical);
    assert_eq!(root_ref.digest, expected);
}

#[test]
fn dir_root_contains_expected_artifacts() {
    let (_dir, store) = make_store();

    // Store three witnesses under the "witness" type
    let witnesses: Vec<_> = (0..3)
        .map(|i| {
            let w = json!({"witness_id": format!("w-{i}"), "data": i});
            store.store("witness", &w).unwrap()
        })
        .collect();

    // List all digests under the "witness" type
    let digests = store.list_digests("witness").unwrap();
    assert_eq!(digests.len(), 3);

    // Each witness digest should be in the listing
    for w_ref in &witnesses {
        assert!(
            digests.contains(&w_ref.digest.to_hex()),
            "witness digest {} not found in listing",
            w_ref.digest.to_hex()
        );
    }

    // Create a dir root referencing all of them
    let root = json!({
        "root_type": "witness-dir",
        "entries": witnesses.iter().map(|r| r.digest.to_hex()).collect::<Vec<_>>()
    });
    let root_ref = store.store("witness-root", &root).unwrap();

    // The root itself should be resolvable in its own namespace
    assert!(store.contains("witness-root", &root_ref.digest).unwrap());
}

#[test]
fn dir_root_deterministic_across_invocations() {
    // Build the same directory root structure twice and verify identical digests
    let witness_data: Vec<serde_json::Value> = (0..4)
        .map(|i| json!({"witness_id": format!("w-{i}"), "attestation": "valid"}))
        .collect();

    // First computation
    let digests_1: Vec<String> = witness_data
        .iter()
        .map(|w| {
            let canonical = CanonicalBytes::new(w).unwrap();
            sha256_digest(&canonical).to_hex()
        })
        .collect();

    let root_1 = json!({
        "root_type": "witness-dir",
        "entries": digests_1
    });
    let root_digest_1 = sha256_digest(&CanonicalBytes::new(&root_1).unwrap());

    // Second computation (same data, fresh construction)
    let digests_2: Vec<String> = witness_data
        .iter()
        .map(|w| {
            let canonical = CanonicalBytes::new(w).unwrap();
            sha256_digest(&canonical).to_hex()
        })
        .collect();

    let root_2 = json!({
        "root_type": "witness-dir",
        "entries": digests_2
    });
    let root_digest_2 = sha256_digest(&CanonicalBytes::new(&root_2).unwrap());

    assert_eq!(
        root_digest_1, root_digest_2,
        "directory root digest must be deterministic"
    );
}
