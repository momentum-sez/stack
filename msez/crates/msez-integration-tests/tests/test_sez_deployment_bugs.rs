//! Regression tests for SEZ deployment bugs.
//!
//! Validates that the defective Python v1 state names (PROPOSED, OPERATIONAL)
//! are structurally excluded from the Rust type system, and that the corridor
//! state machine uses spec-aligned names throughout.

use msez_core::{CanonicalBytes, sha256_digest, JurisdictionId, CorridorId};
use msez_state::{
    Corridor, Draft,
    DynCorridorState,
};
use msez_state::corridor::{SubmissionEvidence, ActivationEvidence, HaltReason};
use serde_json::json;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_digest() -> msez_core::ContentDigest {
    let canonical = CanonicalBytes::new(&json!({"test": "state_machine"})).unwrap();
    sha256_digest(&canonical)
}

// ---------------------------------------------------------------------------
// Defective state names rejected
// ---------------------------------------------------------------------------

#[test]
fn defective_state_names_rejected() {
    // The DynCorridorState enum must NOT have PROPOSED or OPERATIONAL variants.
    // Deserialization of the defective v1 state names must fail.
    let proposed_json = r#""PROPOSED""#;
    let result: Result<DynCorridorState, _> = serde_json::from_str(proposed_json);
    assert!(
        result.is_err(),
        "PROPOSED is a defective v1 state name and must be rejected"
    );

    let operational_json = r#""OPERATIONAL""#;
    let result: Result<DynCorridorState, _> = serde_json::from_str(operational_json);
    assert!(
        result.is_err(),
        "OPERATIONAL is a defective v1 state name and must be rejected"
    );
}

// ---------------------------------------------------------------------------
// Jurisdiction ID format validation
// ---------------------------------------------------------------------------

#[test]
fn jurisdiction_id_format_validation() {
    // Valid jurisdiction IDs.
    assert!(JurisdictionId::new("PK-RSEZ").is_ok());
    assert!(JurisdictionId::new("AE-DIFC").is_ok());
    assert!(JurisdictionId::new("KZ-AIFC").is_ok());

    // Empty must be rejected.
    assert!(JurisdictionId::new("").is_err());
}

// ---------------------------------------------------------------------------
// Corridor state machine spec-aligned
// ---------------------------------------------------------------------------

#[test]
fn corridor_state_machine_spec_aligned() {
    // Verify the typestate-encoded corridor state machine uses spec names.
    let ja = JurisdictionId::new("PK-RSEZ").unwrap();
    let jb = JurisdictionId::new("AE-DIFC").unwrap();

    let draft = Corridor::<Draft>::new(CorridorId::new(), ja, jb);
    assert_eq!(draft.state_name(), "DRAFT");
    assert!(!draft.is_terminal());
}

#[test]
fn corridor_draft_to_pending_to_active() {
    let ja = JurisdictionId::new("PK-RSEZ").unwrap();
    let jb = JurisdictionId::new("AE-DIFC").unwrap();

    let draft = Corridor::<Draft>::new(CorridorId::new(), ja, jb);
    assert_eq!(draft.state_name(), "DRAFT");

    let pending = draft.submit(SubmissionEvidence {
        bilateral_agreement_digest: test_digest(),
        pack_trilogy_digest: test_digest(),
    });
    assert_eq!(pending.state_name(), "PENDING");

    let active = pending.activate(ActivationEvidence {
        regulatory_approval_a: test_digest(),
        regulatory_approval_b: test_digest(),
    });
    assert_eq!(active.state_name(), "ACTIVE");
    assert!(!active.is_terminal());
}

#[test]
fn corridor_active_to_halted_to_deprecated() {
    let ja = JurisdictionId::new("PK-RSEZ").unwrap();
    let jb = JurisdictionId::new("AE-DIFC").unwrap();

    let draft = Corridor::<Draft>::new(CorridorId::new(), ja.clone(), jb);

    let pending = draft.submit(SubmissionEvidence {
        bilateral_agreement_digest: test_digest(),
        pack_trilogy_digest: test_digest(),
    });

    let active = pending.activate(ActivationEvidence {
        regulatory_approval_a: test_digest(),
        regulatory_approval_b: test_digest(),
    });

    let halted = active.halt(HaltReason {
        reason: "Compliance violation detected".to_string(),
        authority: ja,
        evidence: test_digest(),
    });
    assert_eq!(halted.state_name(), "HALTED");

    let deprecated = halted.deprecate(msez_state::corridor::DeprecationEvidence {
        deprecation_decision_digest: test_digest(),
        reason: "Corridor permanently closed".to_string(),
    });
    assert_eq!(deprecated.state_name(), "DEPRECATED");
    assert!(deprecated.is_terminal());
}

#[test]
fn dyn_corridor_state_serialization() {
    // DynCorridorState must serialize to spec-aligned names.
    let draft = DynCorridorState::Draft;
    let json_str = serde_json::to_string(&draft).unwrap();
    assert_eq!(json_str, r#""DRAFT""#);

    let active = DynCorridorState::Active;
    let json_str = serde_json::to_string(&active).unwrap();
    assert_eq!(json_str, r#""ACTIVE""#);

    let deprecated = DynCorridorState::Deprecated;
    let json_str = serde_json::to_string(&deprecated).unwrap();
    assert_eq!(json_str, r#""DEPRECATED""#);
}

#[test]
fn dyn_corridor_state_deserialization() {
    // Spec-aligned names must deserialize correctly.
    let states = [
        ("\"DRAFT\"", DynCorridorState::Draft),
        ("\"PENDING\"", DynCorridorState::Pending),
        ("\"ACTIVE\"", DynCorridorState::Active),
        ("\"HALTED\"", DynCorridorState::Halted),
        ("\"SUSPENDED\"", DynCorridorState::Suspended),
        ("\"DEPRECATED\"", DynCorridorState::Deprecated),
    ];

    for (json_str, expected) in &states {
        let deserialized: DynCorridorState = serde_json::from_str(json_str).unwrap();
        assert_eq!(&deserialized, expected, "Mismatch for {}", json_str);
    }
}
