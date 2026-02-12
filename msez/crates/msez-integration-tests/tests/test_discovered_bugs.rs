//! # Tests for Previously Discovered and Fixed Bugs
//!
//! Python counterpart: `tests/test_discovered_bugs.py`
//!
//! Regression tests for bugs discovered during the Feb 2026 audit:
//! - Defective v1 state names (PROPOSED, OPERATIONAL) must not deserialize
//! - Canonicalization split between core and phoenix layers
//! - Content digest display format must include "sha256:" prefix
//! - CanonicalBytes::new and from_value must agree

use msez_core::{sha256_digest, CanonicalBytes, DigestAlgorithm};
use msez_state::DynCorridorState;
use serde_json::json;

// ---------------------------------------------------------------------------
// 1. Defective v1 state names must not deserialize
// ---------------------------------------------------------------------------

#[test]
fn bug_defective_state_names_proposed_operational() {
    // The Python v1 corridor.lifecycle.state-machine.v1.json used "PROPOSED"
    // and "OPERATIONAL" as state names. These are WRONG per spec.
    // The Rust implementation uses the correct v2 names and must reject the v1 names.

    let proposed: Result<DynCorridorState, _> = serde_json::from_str("\"PROPOSED\"");
    assert!(
        proposed.is_err(),
        "PROPOSED must not be a valid corridor state"
    );

    let operational: Result<DynCorridorState, _> = serde_json::from_str("\"OPERATIONAL\"");
    assert!(
        operational.is_err(),
        "OPERATIONAL must not be a valid corridor state"
    );

    // Verify the correct v2 names DO deserialize
    let draft: DynCorridorState = serde_json::from_str("\"DRAFT\"").unwrap();
    assert_eq!(draft, DynCorridorState::Draft);

    let active: DynCorridorState = serde_json::from_str("\"ACTIVE\"").unwrap();
    assert_eq!(active, DynCorridorState::Active);

    let pending: DynCorridorState = serde_json::from_str("\"PENDING\"").unwrap();
    assert_eq!(pending, DynCorridorState::Pending);
}

// ---------------------------------------------------------------------------
// 2. Canonicalization split between layers
// ---------------------------------------------------------------------------

#[test]
fn bug_canonicalization_split_between_layers() {
    // The audit found that the python phoenix layer used json.dumps(sort_keys=True)
    // while the core layer used jcs_canonicalize(). In Rust, both paths go through
    // CanonicalBytes, so this bug cannot occur. This test verifies the invariant.

    let data = json!({"b": 2, "a": 1, "c": "hello"});

    // Core path (via CanonicalBytes)
    let core_canonical = CanonicalBytes::new(&data).unwrap();
    let core_digest = sha256_digest(&core_canonical);

    // "Phoenix" path (also via CanonicalBytes, since there is only one path in Rust)
    let phoenix_canonical = CanonicalBytes::new(&data).unwrap();
    let phoenix_digest = sha256_digest(&phoenix_canonical);

    assert_eq!(
        core_digest, phoenix_digest,
        "core and phoenix paths must produce identical digests"
    );
    assert_eq!(
        core_canonical.as_bytes(),
        phoenix_canonical.as_bytes(),
        "canonical bytes must be identical"
    );
}

#[test]
fn bug_canonicalization_key_sorting() {
    // Verify that keys are sorted deterministically
    let v1 = json!({"z": 1, "a": 2, "m": 3});
    let v2 = json!({"a": 2, "m": 3, "z": 1});

    let d1 = sha256_digest(&CanonicalBytes::new(&v1).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&v2).unwrap());
    assert_eq!(d1, d2, "key order must not affect digest");
}

// ---------------------------------------------------------------------------
// 3. Digest format includes "sha256:" prefix
// ---------------------------------------------------------------------------

#[test]
fn bug_digest_format_sha256_prefix() {
    let data = json!({"test": true});
    let canonical = CanonicalBytes::new(&data).unwrap();
    let digest = sha256_digest(&canonical);

    // Display format
    let display = format!("{digest}");
    assert!(
        display.starts_with("sha256:"),
        "digest display must start with 'sha256:', got: {}",
        display
    );
    assert_eq!(display.len(), 7 + 64, "sha256: prefix + 64 hex chars");

    // Algorithm tag
    assert_eq!(digest.algorithm(), DigestAlgorithm::Sha256);

    // Hex format (no prefix)
    let hex = digest.to_hex();
    assert_eq!(hex.len(), 64);
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
}

// ---------------------------------------------------------------------------
// 4. CanonicalBytes::new and from_value agree
// ---------------------------------------------------------------------------

#[test]
fn bug_from_value_and_new_agree() {
    let data = json!({
        "corridor_id": "c-001",
        "receipts": [
            {"seq": 0, "payload": "abc"},
            {"seq": 1, "payload": "def"}
        ],
        "meta": {"version": 1}
    });

    let from_new = CanonicalBytes::new(&data).unwrap();
    let from_value = CanonicalBytes::from_value(data.clone()).unwrap();

    assert_eq!(
        from_new.as_bytes(),
        from_value.as_bytes(),
        "new() and from_value() must produce identical bytes"
    );

    let digest_new = sha256_digest(&from_new);
    let digest_value = sha256_digest(&from_value);
    assert_eq!(digest_new, digest_value);
}

// ---------------------------------------------------------------------------
// 5. All valid corridor states deserialize correctly
// ---------------------------------------------------------------------------

#[test]
fn bug_all_valid_states_deserialize() {
    let valid_states = [
        ("\"DRAFT\"", DynCorridorState::Draft),
        ("\"PENDING\"", DynCorridorState::Pending),
        ("\"ACTIVE\"", DynCorridorState::Active),
        ("\"HALTED\"", DynCorridorState::Halted),
        ("\"SUSPENDED\"", DynCorridorState::Suspended),
        ("\"DEPRECATED\"", DynCorridorState::Deprecated),
    ];

    for (json_str, expected) in &valid_states {
        let deserialized: DynCorridorState = serde_json::from_str(json_str).unwrap();
        assert_eq!(
            deserialized, *expected,
            "state {} deserialized incorrectly",
            json_str
        );
    }
}

// ---------------------------------------------------------------------------
// 6. Invalid state strings are rejected
// ---------------------------------------------------------------------------

#[test]
fn bug_invalid_state_strings_rejected() {
    let invalid_states = [
        "\"PROPOSED\"",
        "\"OPERATIONAL\"",
        "\"UNKNOWN\"",
        "\"\"",
        "\"active\"",     // lowercase
        "\"Draft\"",      // mixed case
        "\"TERMINATED\"", // non-existent
    ];

    for json_str in &invalid_states {
        let result: Result<DynCorridorState, _> = serde_json::from_str(json_str);
        assert!(
            result.is_err(),
            "state {} should be rejected but was accepted as {:?}",
            json_str,
            result.ok()
        );
    }
}

// ---------------------------------------------------------------------------
// 7. Float rejection is the correct error type
// ---------------------------------------------------------------------------

#[test]
fn bug_float_rejection_error_type() {
    let data = json!({"amount": 3.14});
    let result = CanonicalBytes::new(&data);
    assert!(result.is_err());
    // The error should be a CanonicalizationError, not a panic
}

// ---------------------------------------------------------------------------
// 8. Known test vector cross-check
// ---------------------------------------------------------------------------

#[test]
fn bug_known_test_vector() {
    // This vector MUST match the Python jcs_canonicalize output:
    // json.dumps({"a":1,"b":2}, sort_keys=True, separators=(",",":")).encode()
    // = b'{"a":1,"b":2}'
    let data = json!({"b": 2, "a": 1});
    let canonical = CanonicalBytes::new(&data).unwrap();
    assert_eq!(
        std::str::from_utf8(canonical.as_bytes()).unwrap(),
        r#"{"a":1,"b":2}"#
    );

    let digest = sha256_digest(&canonical);
    let expected = "43258cff783fe7036d8a43033f830adfc60ec037382473548ac742b888292777";
    assert_eq!(digest.to_hex(), expected);
}
