//! # Artifact Graph Integrity Verification
//!
//! Tests artifact graph integrity verification by ensuring that digests match
//! stored content and that tampering is detected. Verifies the CAS integrity
//! model where stored artifacts are re-digested on retrieval and compared
//! against the filename-encoded digest.

use msez_core::{sha256_digest, CanonicalBytes};
use msez_crypto::ContentAddressedStore;
use serde_json::json;

fn make_store() -> (tempfile::TempDir, ContentAddressedStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    (dir, store)
}

#[test]
fn artifact_verification_succeeds_for_valid() {
    let (_dir, store) = make_store();

    let data = json!({
        "type": "corridor-receipt",
        "sequence": 0,
        "corridor_id": "test-corridor-001",
        "payload": {"action": "transfer", "amount": 50000}
    });

    let artifact_ref = store.store("receipt", &data).unwrap();

    // Resolution performs integrity verification internally
    let resolved = store.resolve("receipt", &artifact_ref.digest).unwrap();
    assert!(
        resolved.is_some(),
        "valid artifact must resolve successfully"
    );

    // Verify the digest matches what we compute independently
    let canonical = CanonicalBytes::new(&data).unwrap();
    let expected_digest = sha256_digest(&canonical);
    assert_eq!(artifact_ref.digest, expected_digest);
}

#[test]
fn artifact_verification_fails_for_tampered() {
    let (dir, store) = make_store();

    let data = json!({"critical": "untampered-data", "seq": 42});
    let artifact_ref = store.store("receipt", &data).unwrap();

    // Manually corrupt the stored file
    let path = artifact_ref.path_in(dir.path());
    std::fs::write(&path, b"{\"critical\":\"TAMPERED\",\"seq\":42}").unwrap();

    // Resolution must detect the integrity violation
    let result = store.resolve("receipt", &artifact_ref.digest);
    assert!(result.is_err(), "tampered artifact must fail verification");
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("integrity violation"),
        "error must mention integrity violation, got: {err_msg}"
    );
}

#[test]
fn empty_graph_verification() {
    let (_dir, store) = make_store();

    // An empty JSON object is a valid artifact
    let empty = json!({});
    let artifact_ref = store.store("meta", &empty).unwrap();

    let resolved = store.resolve("meta", &artifact_ref.digest).unwrap();
    assert!(resolved.is_some());

    let bytes = resolved.unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(parsed.is_object());
    assert_eq!(parsed.as_object().unwrap().len(), 0);
}

#[test]
fn nonexistent_artifact_returns_none() {
    let (_dir, store) = make_store();

    // Compute a digest for content that was never stored
    let phantom_data = json!({"never": "stored"});
    let canonical = CanonicalBytes::new(&phantom_data).unwrap();
    let phantom_digest = sha256_digest(&canonical);

    let result = store.resolve("receipt", &phantom_digest).unwrap();
    assert!(result.is_none(), "nonexistent artifact must return None");
}

#[test]
fn recomputed_digest_matches_stored_ref() {
    let (_dir, store) = make_store();

    let data = json!({
        "graph": {
            "nodes": [
                {"id": "n1", "data": "alpha"},
                {"id": "n2", "data": "beta"}
            ],
            "edges": [["n1", "n2"]]
        }
    });

    let artifact_ref = store.store("graph", &data).unwrap();

    // Independently compute the digest
    let canonical = CanonicalBytes::new(&data).unwrap();
    let independent_digest = sha256_digest(&canonical);

    assert_eq!(
        artifact_ref.digest, independent_digest,
        "stored artifact digest must match independent computation"
    );
}
