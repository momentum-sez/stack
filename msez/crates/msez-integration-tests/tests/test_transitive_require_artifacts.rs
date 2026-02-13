//! # Transitive Artifact Requirements
//!
//! Tests transitive artifact dependencies (A requires B requires C) stored in
//! CAS. Verifies that diamond dependencies (A depends on B and C, both depend
//! on D) produce consistent digests and that all transitive dependencies are
//! independently resolvable from the store.

use msez_crypto::ContentAddressedStore;
use serde_json::json;

fn make_store() -> (tempfile::TempDir, ContentAddressedStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    (dir, store)
}

#[test]
fn transitive_dependency_chain() {
    let (_dir, store) = make_store();

    // Level 0: leaf artifact (no dependencies)
    let leaf = json!({"module": "base-sanctions-list", "version": 1});
    let leaf_ref = store.store("module", &leaf).unwrap();

    // Level 1: depends on leaf
    let mid = json!({
        "module": "sanctions-checker",
        "version": 1,
        "requires": [{"type": "module", "digest": leaf_ref.digest.to_hex()}]
    });
    let mid_ref = store.store("module", &mid).unwrap();

    // Level 2: depends on mid (and transitively on leaf)
    let top = json!({
        "module": "compliance-evaluator",
        "version": 1,
        "requires": [{"type": "module", "digest": mid_ref.digest.to_hex()}]
    });
    let top_ref = store.store("module", &top).unwrap();

    // All three levels must be independently resolvable
    assert!(store.contains("module", &leaf_ref.digest).unwrap());
    assert!(store.contains("module", &mid_ref.digest).unwrap());
    assert!(store.contains("module", &top_ref.digest).unwrap());

    // Verify the chain: top -> mid -> leaf
    let top_bytes = store.resolve("module", &top_ref.digest).unwrap().unwrap();
    let top_parsed: serde_json::Value = serde_json::from_slice(&top_bytes).unwrap();
    let top_dep_digest = top_parsed["requires"][0]["digest"].as_str().unwrap();
    assert_eq!(top_dep_digest, mid_ref.digest.to_hex());

    let mid_bytes = store.resolve("module", &mid_ref.digest).unwrap().unwrap();
    let mid_parsed: serde_json::Value = serde_json::from_slice(&mid_bytes).unwrap();
    let mid_dep_digest = mid_parsed["requires"][0]["digest"].as_str().unwrap();
    assert_eq!(mid_dep_digest, leaf_ref.digest.to_hex());
}

#[test]
fn diamond_dependency() {
    let (_dir, store) = make_store();

    // D: shared base dependency
    let d = json!({"module": "core-types", "version": 1});
    let d_ref = store.store("module", &d).unwrap();

    // B depends on D
    let b = json!({
        "module": "aml-engine",
        "requires": [{"type": "module", "digest": d_ref.digest.to_hex()}]
    });
    let b_ref = store.store("module", &b).unwrap();

    // C depends on D
    let c = json!({
        "module": "kyc-engine",
        "requires": [{"type": "module", "digest": d_ref.digest.to_hex()}]
    });
    let c_ref = store.store("module", &c).unwrap();

    // A depends on B and C (diamond: A -> B -> D, A -> C -> D)
    let a = json!({
        "module": "compliance-suite",
        "requires": [
            {"type": "module", "digest": b_ref.digest.to_hex()},
            {"type": "module", "digest": c_ref.digest.to_hex()}
        ]
    });
    let a_ref = store.store("module", &a).unwrap();

    // All four must be resolvable
    assert!(store.contains("module", &d_ref.digest).unwrap());
    assert!(store.contains("module", &b_ref.digest).unwrap());
    assert!(store.contains("module", &c_ref.digest).unwrap());
    assert!(store.contains("module", &a_ref.digest).unwrap());

    // B and C both reference the same D digest
    let b_parsed: serde_json::Value =
        serde_json::from_slice(&store.resolve("module", &b_ref.digest).unwrap().unwrap()).unwrap();
    let c_parsed: serde_json::Value =
        serde_json::from_slice(&store.resolve("module", &c_ref.digest).unwrap().unwrap()).unwrap();

    assert_eq!(
        b_parsed["requires"][0]["digest"].as_str().unwrap(),
        c_parsed["requires"][0]["digest"].as_str().unwrap(),
        "diamond base dependency D must have the same digest in both B and C"
    );
}

#[test]
fn all_transitive_deps_resolvable() {
    let (_dir, store) = make_store();

    // Build a chain of 5 levels
    let mut prev_digest: Option<String> = None;
    let mut all_refs = Vec::new();

    for depth in 0..5 {
        let mut module = json!({
            "module": format!("level-{depth}"),
            "depth": depth
        });

        if let Some(ref dep_digest) = prev_digest {
            module["requires"] = json!([{"type": "module", "digest": dep_digest}]);
        }

        let artifact_ref = store.store("module", &module).unwrap();
        prev_digest = Some(artifact_ref.digest.to_hex());
        all_refs.push(artifact_ref);
    }

    // All 5 levels must be resolvable
    for (i, r) in all_refs.iter().enumerate() {
        assert!(
            store.contains("module", &r.digest).unwrap(),
            "level {i} artifact must be resolvable"
        );
    }

    // Traverse the chain from top to bottom
    let mut current_digest = all_refs.last().unwrap().digest.to_hex();
    let mut traversed = 0;
    loop {
        let bytes = store
            .resolve(
                "module",
                &all_refs
                    .iter()
                    .find(|r| r.digest.to_hex() == current_digest)
                    .unwrap()
                    .digest,
            )
            .unwrap()
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        traversed += 1;

        if parsed.get("requires").is_some() {
            current_digest = parsed["requires"][0]["digest"]
                .as_str()
                .unwrap()
                .to_string();
        } else {
            break;
        }
    }

    assert_eq!(traversed, 5, "must traverse all 5 levels");
}
