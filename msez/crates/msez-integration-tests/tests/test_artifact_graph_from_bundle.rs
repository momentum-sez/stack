//! # Artifact Dependency Graph from Bundles
//!
//! Tests building artifact dependency graphs from bundles stored in CAS.
//! Verifies that artifacts can reference other artifacts by digest, forming
//! a directed acyclic graph of dependencies, and that all referenced artifacts
//! are resolvable from the store.

use msez_core::{sha256_digest, CanonicalBytes};
use msez_crypto::ContentAddressedStore;
use serde_json::json;

fn make_store() -> (tempfile::TempDir, ContentAddressedStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    (dir, store)
}

#[test]
fn single_artifact_graph() {
    let (_dir, store) = make_store();

    let leaf = json!({"name": "base-lawpack", "rules": ["rule-1", "rule-2"]});
    let leaf_ref = store.store("lawpack", &leaf).unwrap();

    // Build a graph node referencing the leaf
    let graph_node = json!({
        "graph_type": "dependency",
        "root": leaf_ref.digest.to_hex(),
        "children": []
    });
    let graph_ref = store.store("graph", &graph_node).unwrap();

    // Both the leaf and the graph node should be resolvable
    assert!(store.contains("lawpack", &leaf_ref.digest).unwrap());
    assert!(store.contains("graph", &graph_ref.digest).unwrap());
}

#[test]
fn two_level_dependency_graph() {
    let (_dir, store) = make_store();

    // Level 0: two leaf artifacts
    let leaf_a = json!({"module": "aml", "version": 1});
    let leaf_b = json!({"module": "kyc", "version": 1});
    let ref_a = store.store("module", &leaf_a).unwrap();
    let ref_b = store.store("module", &leaf_b).unwrap();

    // Level 1: parent artifact referencing both leaves
    let parent = json!({
        "bundle_type": "compliance-pack",
        "dependencies": [
            {"type": "module", "digest": ref_a.digest.to_hex()},
            {"type": "module", "digest": ref_b.digest.to_hex()}
        ]
    });
    let ref_parent = store.store("bundle", &parent).unwrap();

    // All three artifacts should be resolvable
    assert!(store.contains("module", &ref_a.digest).unwrap());
    assert!(store.contains("module", &ref_b.digest).unwrap());
    assert!(store.contains("bundle", &ref_parent.digest).unwrap());

    // Verify the parent's stored content references the correct digests
    let resolved = store.resolve("bundle", &ref_parent.digest).unwrap().unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&resolved).unwrap();
    let deps = parsed["dependencies"].as_array().unwrap();
    assert_eq!(deps.len(), 2);
    assert_eq!(deps[0]["digest"].as_str().unwrap(), ref_a.digest.to_hex());
    assert_eq!(deps[1]["digest"].as_str().unwrap(), ref_b.digest.to_hex());
}

#[test]
fn artifact_graph_digest_consistency() {
    // Verify that the same graph structure produces the same digest
    // regardless of when or where it is computed.
    let leaf = json!({"data": "immutable-content", "version": 42});
    let canonical = CanonicalBytes::new(&leaf).unwrap();
    let digest = sha256_digest(&canonical);

    let graph = json!({
        "root_digest": digest.to_hex(),
        "depth": 1,
        "leaf_count": 1
    });

    let canonical_graph_1 = CanonicalBytes::new(&graph).unwrap();
    let graph_digest_1 = sha256_digest(&canonical_graph_1);

    let canonical_graph_2 = CanonicalBytes::new(&graph).unwrap();
    let graph_digest_2 = sha256_digest(&canonical_graph_2);

    assert_eq!(graph_digest_1, graph_digest_2);
    assert_ne!(digest, graph_digest_1, "leaf and graph digests must differ");
}
