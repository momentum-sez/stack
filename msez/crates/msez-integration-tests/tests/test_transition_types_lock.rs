//! # Transition Types in Lock Files Integration Tests
//!
//! Python counterpart: `tests/test_transition_types_lock.py`
//!
//! Tests that transition types produce deterministic lock file digests
//! and that different transitions yield different lock digests.

use msez_core::{sha256_digest, CanonicalBytes};
use serde_json::json;

// ---------------------------------------------------------------------------
// 1. Transition lock is deterministic
// ---------------------------------------------------------------------------

#[test]
fn transition_lock_deterministic() {
    let transition = json!({
        "transition_type": "formation",
        "entity_id": "ent-001",
        "jurisdiction_id": "PK-RSEZ",
        "from_state": "APPLIED",
        "to_state": "ACTIVE",
        "evidence_digest": "aa".repeat(32)
    });

    let d1 = sha256_digest(&CanonicalBytes::new(&transition).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&transition).unwrap());
    let d3 = sha256_digest(&CanonicalBytes::new(&transition).unwrap());

    assert_eq!(d1, d2, "first and second must match");
    assert_eq!(d2, d3, "second and third must match");
}

// ---------------------------------------------------------------------------
// 2. Different transitions produce different lock digests
// ---------------------------------------------------------------------------

#[test]
fn different_transitions_different_locks() {
    let transitions = [
        json!({
            "transition_type": "formation",
            "entity_id": "ent-001",
            "from_state": "APPLIED",
            "to_state": "ACTIVE"
        }),
        json!({
            "transition_type": "suspension",
            "entity_id": "ent-001",
            "from_state": "ACTIVE",
            "to_state": "SUSPENDED"
        }),
        json!({
            "transition_type": "dissolution",
            "entity_id": "ent-001",
            "from_state": "DISSOLVING",
            "to_state": "DISSOLVED"
        }),
    ];

    let digests: Vec<_> = transitions
        .iter()
        .map(|t| sha256_digest(&CanonicalBytes::new(t).unwrap()).to_hex())
        .collect();

    // All digests must be unique
    for i in 0..digests.len() {
        for j in (i + 1)..digests.len() {
            assert_ne!(
                digests[i], digests[j],
                "transition {i} and {j} must have different lock digests"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Lock includes transition type in canonical bytes
// ---------------------------------------------------------------------------

#[test]
fn lock_includes_transition_type() {
    let transition = json!({
        "transition_type": "halt",
        "corridor_id": "corr-001",
        "reason": "Fork detected"
    });

    let canonical = CanonicalBytes::new(&transition).unwrap();
    let canonical_str = std::str::from_utf8(canonical.as_bytes()).unwrap();

    // The canonical representation should include the transition_type
    assert!(
        canonical_str.contains("transition_type"),
        "canonical bytes must include transition_type field"
    );
    assert!(
        canonical_str.contains("halt"),
        "canonical bytes must include transition_type value"
    );
}

// ---------------------------------------------------------------------------
// 4. Key ordering in lock data is deterministic
// ---------------------------------------------------------------------------

#[test]
fn lock_key_ordering_deterministic() {
    let v1 = json!({
        "z_field": "last",
        "a_field": "first",
        "transition_type": "formation"
    });
    let v2 = json!({
        "a_field": "first",
        "transition_type": "formation",
        "z_field": "last"
    });

    let d1 = sha256_digest(&CanonicalBytes::new(&v1).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&v2).unwrap());
    assert_eq!(d1, d2, "key insertion order must not affect lock digest");
}

// ---------------------------------------------------------------------------
// 5. Nested transition data produces stable lock digest
// ---------------------------------------------------------------------------

#[test]
fn nested_transition_lock_stable() {
    let transition = json!({
        "transition_type": "migration",
        "migration": {
            "source": "PK-RSEZ",
            "destination": "AE-DIFC",
            "phases": [
                {"name": "compliance_check", "completed": true},
                {"name": "attestation_gathering", "completed": true},
                {"name": "source_locked", "completed": false}
            ]
        }
    });

    let d1 = sha256_digest(&CanonicalBytes::new(&transition).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&transition).unwrap());
    assert_eq!(d1, d2, "nested transition lock must be stable");
}

// ---------------------------------------------------------------------------
// 6. Empty evidence produces valid lock digest
// ---------------------------------------------------------------------------

#[test]
fn empty_evidence_valid_lock() {
    let transition = json!({
        "transition_type": "reinstate",
        "from_state": "SUSPENDED",
        "to_state": "ACTIVE",
        "evidence": {}
    });

    let canonical = CanonicalBytes::new(&transition).unwrap();
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);
}
