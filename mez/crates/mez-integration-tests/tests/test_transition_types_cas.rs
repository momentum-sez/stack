//! # Transition Types in Content-Addressed Storage Integration Tests
//!
//! Python counterpart: `tests/test_transition_types_cas.py`
//!
//! Tests that various transition type artifacts can be stored in CAS,
//! producing deterministic digests, and that different transition types
//! produce different digests.

use mez_core::{sha256_digest, CanonicalBytes};
use mez_crypto::ContentAddressedStore;
use serde_json::json;

fn make_store() -> (tempfile::TempDir, ContentAddressedStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentAddressedStore::new(dir.path());
    (dir, store)
}

// ---------------------------------------------------------------------------
// 1. Store formation transition artifact
// ---------------------------------------------------------------------------

#[test]
fn store_formation_transition() {
    let (_dir, store) = make_store();

    let formation = json!({
        "transition_type": "formation",
        "entity_id": "ent-001",
        "jurisdiction_id": "PK-RSEZ",
        "from_state": "APPLIED",
        "to_state": "ACTIVE",
        "timestamp": "2026-01-15T12:00:00Z",
        "evidence_digest": "aa".repeat(32)
    });

    let artifact_ref = store.store("transition", &formation).unwrap();
    assert_eq!(artifact_ref.artifact_type, "transition");
    assert_eq!(artifact_ref.digest.to_hex().len(), 64);

    let resolved = store.resolve("transition", &artifact_ref.digest).unwrap();
    assert!(resolved.is_some());
}

// ---------------------------------------------------------------------------
// 2. Store activation transition artifact
// ---------------------------------------------------------------------------

#[test]
fn store_activation_transition() {
    let (_dir, store) = make_store();

    let activation = json!({
        "transition_type": "activation",
        "corridor_id": "corr-pk-ae-001",
        "from_state": "PENDING",
        "to_state": "ACTIVE",
        "regulatory_approval_a": "bb".repeat(32),
        "regulatory_approval_b": "cc".repeat(32),
        "timestamp": "2026-02-01T09:00:00Z"
    });

    let artifact_ref = store.store("transition", &activation).unwrap();
    assert_eq!(artifact_ref.artifact_type, "transition");

    let resolved = store.resolve("transition", &artifact_ref.digest).unwrap();
    assert!(resolved.is_some());
}

// ---------------------------------------------------------------------------
// 3. Store dissolution transition artifact
// ---------------------------------------------------------------------------

#[test]
fn store_dissolution_transition() {
    let (_dir, store) = make_store();

    let dissolution = json!({
        "transition_type": "dissolution",
        "entity_id": "ent-002",
        "from_state": "DISSOLVING",
        "to_state": "DISSOLVED",
        "dissolution_stage": 10,
        "final_meeting_digest": "dd".repeat(32),
        "timestamp": "2026-06-15T18:00:00Z"
    });

    let artifact_ref = store.store("transition", &dissolution).unwrap();
    assert_eq!(artifact_ref.artifact_type, "transition");

    let resolved = store.resolve("transition", &artifact_ref.digest).unwrap();
    assert!(resolved.is_some());
}

// ---------------------------------------------------------------------------
// 4. Different transition types produce different digests
// ---------------------------------------------------------------------------

#[test]
fn transition_digests_differ_by_type() {
    let formation = json!({
        "transition_type": "formation",
        "entity_id": "ent-001",
        "timestamp": "2026-01-15T12:00:00Z"
    });

    let activation = json!({
        "transition_type": "activation",
        "entity_id": "ent-001",
        "timestamp": "2026-01-15T12:00:00Z"
    });

    let dissolution = json!({
        "transition_type": "dissolution",
        "entity_id": "ent-001",
        "timestamp": "2026-01-15T12:00:00Z"
    });

    let d_formation = sha256_digest(&CanonicalBytes::new(&formation).unwrap());
    let d_activation = sha256_digest(&CanonicalBytes::new(&activation).unwrap());
    let d_dissolution = sha256_digest(&CanonicalBytes::new(&dissolution).unwrap());

    assert_ne!(
        d_formation, d_activation,
        "formation and activation must differ"
    );
    assert_ne!(
        d_activation, d_dissolution,
        "activation and dissolution must differ"
    );
    assert_ne!(
        d_formation, d_dissolution,
        "formation and dissolution must differ"
    );
}

// ---------------------------------------------------------------------------
// 5. Transition stored in CAS is resolvable by digest
// ---------------------------------------------------------------------------

#[test]
fn transition_resolvable_by_digest() {
    let (_dir, store) = make_store();

    let transition = json!({
        "transition_type": "halt",
        "corridor_id": "corr-001",
        "from_state": "ACTIVE",
        "to_state": "HALTED",
        "reason": "Fork detected"
    });

    let ref1 = store.store("transition", &transition).unwrap();
    assert!(store.contains("transition", &ref1.digest).unwrap());

    // Verify content matches
    let resolved = store.resolve("transition", &ref1.digest).unwrap().unwrap();
    let reparsed: serde_json::Value = serde_json::from_slice(&resolved).unwrap();
    assert_eq!(reparsed["transition_type"], "halt");
    assert_eq!(reparsed["to_state"], "HALTED");
}

// ---------------------------------------------------------------------------
// 6. Same transition stored twice yields same digest
// ---------------------------------------------------------------------------

#[test]
fn same_transition_idempotent() {
    let (_dir, store) = make_store();

    let transition = json!({
        "transition_type": "resume",
        "corridor_id": "corr-002",
        "from_state": "SUSPENDED",
        "to_state": "ACTIVE"
    });

    let ref1 = store.store("transition", &transition).unwrap();
    let ref2 = store.store("transition", &transition).unwrap();
    assert_eq!(
        ref1.digest, ref2.digest,
        "storing same transition twice must yield same digest"
    );
}
