//! # Elite Tier Validation Tests
//!
//! Cross-cutting validation tests that exercise fundamental invariants across
//! multiple tiers of the EZ stack: canonicalization correctness, corridor
//! state alignment with the spec, compliance domain count, digest format,
//! and canonical bytes UTF-8 validity.

use mez_core::{sha256_digest, CanonicalBytes, CanonicalizationError, ComplianceDomain};
use mez_state::DynCorridorState;
use serde_json::json;

// ---------------------------------------------------------------------------
// 1. Canonicalization rejects floating-point values
// ---------------------------------------------------------------------------

#[test]
fn canonicalization_rejects_floats() {
    let data = json!({"amount": 3.15});
    let result = CanonicalBytes::new(&data);
    assert!(
        result.is_err(),
        "canonicalization must reject floating-point values"
    );
    match result.unwrap_err() {
        CanonicalizationError::FloatRejected { .. } => { /* expected */ }
        other => panic!("expected FloatDetected, got: {other}"),
    }
}

#[test]
fn canonicalization_rejects_deeply_nested_float() {
    let data = json!({"a": {"b": {"c": [1, 2, 3.0]}}});
    assert!(
        CanonicalBytes::new(&data).is_err(),
        "deeply nested float must be rejected"
    );
}

// ---------------------------------------------------------------------------
// 2. Corridor states match the spec (6 states, no PROPOSED/OPERATIONAL)
// ---------------------------------------------------------------------------

#[test]
fn corridor_states_spec_aligned() {
    // The spec defines exactly 6 states: DRAFT, PENDING, ACTIVE, HALTED, SUSPENDED, DEPRECATED
    let expected_states = [
        DynCorridorState::Draft,
        DynCorridorState::Pending,
        DynCorridorState::Active,
        DynCorridorState::Halted,
        DynCorridorState::Suspended,
        DynCorridorState::Deprecated,
    ];

    // All 6 exist
    for state in &expected_states {
        let s = state.as_str();
        assert!(!s.is_empty(), "state {state:?} has empty string name");
    }

    // Defective v1 names MUST NOT deserialize
    let proposed: Result<DynCorridorState, _> = serde_json::from_str("\"PROPOSED\"");
    assert!(proposed.is_err(), "PROPOSED must not be a valid state");

    let operational: Result<DynCorridorState, _> = serde_json::from_str("\"OPERATIONAL\"");
    assert!(
        operational.is_err(),
        "OPERATIONAL must not be a valid state"
    );
}

#[test]
fn corridor_state_names_are_uppercase() {
    let states = [
        DynCorridorState::Draft,
        DynCorridorState::Pending,
        DynCorridorState::Active,
        DynCorridorState::Halted,
        DynCorridorState::Suspended,
        DynCorridorState::Deprecated,
    ];
    for state in &states {
        let name = state.as_str();
        assert_eq!(
            name,
            name.to_uppercase(),
            "corridor state name {name} must be uppercase"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. Compliance domains count is 20
// ---------------------------------------------------------------------------

#[test]
fn compliance_domains_count_20() {
    assert_eq!(
        ComplianceDomain::all().len(),
        20,
        "the spec defines exactly 20 compliance domains"
    );
    assert_eq!(ComplianceDomain::COUNT, 20);
}

#[test]
fn compliance_domains_are_unique() {
    let all = ComplianceDomain::all();
    let mut seen = std::collections::HashSet::new();
    for &domain in all {
        assert!(
            seen.insert(domain.as_str()),
            "duplicate compliance domain: {}",
            domain.as_str()
        );
    }
}

// ---------------------------------------------------------------------------
// 4. Digest format: sha256: prefix + 64 hex chars
// ---------------------------------------------------------------------------

#[test]
fn digest_format_correct() {
    let data = json!({"tier": "elite", "id": 1});
    let canonical = CanonicalBytes::new(&data).unwrap();
    let digest = sha256_digest(&canonical);

    // Hex representation: 64 hex characters
    let hex = digest.to_hex();
    assert_eq!(hex.len(), 64, "SHA-256 hex must be 64 chars");
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));

    // Display format: sha256:<hex>
    let display = format!("{digest}");
    assert!(
        display.starts_with("sha256:"),
        "digest display must start with sha256: prefix"
    );
    assert_eq!(display.len(), 7 + 64);

    // Algorithm tag
    assert_eq!(digest.algorithm(), mez_core::DigestAlgorithm::Sha256);
}

// ---------------------------------------------------------------------------
// 5. Canonical bytes are valid UTF-8
// ---------------------------------------------------------------------------

#[test]
fn canonical_bytes_valid_utf8() {
    let test_cases = vec![
        json!(null),
        json!(true),
        json!(false),
        json!(42),
        json!(-1),
        json!("hello world"),
        json!([1, 2, 3]),
        json!({"key": "value"}),
        json!({"nested": {"deep": [1, "two", null, false]}}),
        json!([]),
        json!({}),
    ];

    for data in test_cases {
        let cb = CanonicalBytes::new(&data).unwrap();
        assert!(
            std::str::from_utf8(cb.as_bytes()).is_ok(),
            "canonical bytes for {data:?} must be valid UTF-8"
        );
    }
}

#[test]
fn canonical_bytes_contain_no_extra_whitespace() {
    let data = json!({"b": 2, "a": 1});
    let cb = CanonicalBytes::new(&data).unwrap();
    let s = std::str::from_utf8(cb.as_bytes()).unwrap();
    // Canonical serialization uses compact separators -- no spaces
    assert!(
        !s.contains("  "),
        "canonical bytes must not contain double spaces"
    );
    assert!(
        !s.contains(" :"),
        "canonical bytes must not have spaces before colons"
    );
    assert!(
        !s.contains(": "),
        "canonical bytes must not have spaces after colons"
    );
}
