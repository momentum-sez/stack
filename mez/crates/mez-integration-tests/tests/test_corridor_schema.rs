//! # Corridor Schema and State Alignment
//!
//! Tests that the corridor data model uses spec-aligned state names
//! (DRAFT, PENDING, ACTIVE, HALTED, SUSPENDED, DEPRECATED) and rejects
//! the defective v1 names (PROPOSED, OPERATIONAL). Verifies that all
//! states are accessible and that the transition log captures state
//! changes correctly.

use mez_core::{sha256_digest, CanonicalBytes, ContentDigest, CorridorId, JurisdictionId};
use mez_state::corridor::{
    ActivationEvidence, DeprecationEvidence, HaltReason, ResumeEvidence, SubmissionEvidence,
    SuspendReason,
};
use mez_state::{Corridor, Draft, DynCorridorData, DynCorridorState};
use serde_json::json;

fn test_digest(label: &str) -> ContentDigest {
    let canonical = CanonicalBytes::new(&json!({"evidence": label})).unwrap();
    sha256_digest(&canonical)
}

#[test]
fn corridor_schema_states_are_spec_aligned() {
    // All six spec-defined states must deserialize correctly
    let states = [
        "DRAFT",
        "PENDING",
        "ACTIVE",
        "HALTED",
        "SUSPENDED",
        "DEPRECATED",
    ];
    for state_name in &states {
        let json_str = format!("\"{state_name}\"");
        let result: Result<DynCorridorState, _> = serde_json::from_str(&json_str);
        assert!(
            result.is_ok(),
            "state {state_name} must deserialize: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap().as_str(), *state_name);
    }
}

#[test]
fn corridor_schema_rejects_defective_names() {
    // The defective v1 state names must NOT deserialize
    let defective = ["PROPOSED", "OPERATIONAL", "RUNNING", "STOPPED"];
    for name in &defective {
        let json_str = format!("\"{name}\"");
        let result: Result<DynCorridorState, _> = serde_json::from_str(&json_str);
        assert!(
            result.is_err(),
            "defective state name {name} must be rejected"
        );
    }
}

#[test]
fn corridor_schema_all_states_accessible() {
    let id = CorridorId::new();
    let ja = JurisdictionId::new("PK-REZ").unwrap();
    let jb = JurisdictionId::new("AE-DIFC").unwrap();

    // DRAFT
    let draft = Corridor::<Draft>::new(id, ja.clone(), jb);
    assert_eq!(draft.state_name(), "DRAFT");
    let dyn_data = DynCorridorData::from(&draft);
    assert_eq!(dyn_data.state, DynCorridorState::Draft);

    // PENDING
    let pending = draft.submit(SubmissionEvidence {
        bilateral_agreement_digest: test_digest("agreement"),
        pack_trilogy_digest: test_digest("packs"),
    });
    assert_eq!(pending.state_name(), "PENDING");

    // ACTIVE
    let active = pending.activate(ActivationEvidence {
        regulatory_approval_a: test_digest("approval-a"),
        regulatory_approval_b: test_digest("approval-b"),
    });
    assert_eq!(active.state_name(), "ACTIVE");

    // SUSPENDED
    let suspended = active.suspend(SuspendReason {
        reason: "maintenance".to_string(),
        expected_resume: None,
    });
    assert_eq!(suspended.state_name(), "SUSPENDED");

    // ACTIVE (resumed)
    let active_again = suspended.resume(ResumeEvidence {
        resolution_attestation: test_digest("resume"),
    });
    assert_eq!(active_again.state_name(), "ACTIVE");

    // HALTED
    let halted = active_again.halt(HaltReason {
        reason: "fork detected".to_string(),
        authority: ja,
        evidence: test_digest("fork"),
    });
    assert_eq!(halted.state_name(), "HALTED");

    // DEPRECATED
    let deprecated = halted.deprecate(DeprecationEvidence {
        deprecation_decision_digest: test_digest("deprecation"),
        reason: "sunset".to_string(),
    });
    assert_eq!(deprecated.state_name(), "DEPRECATED");
    assert!(deprecated.is_terminal());
}

#[test]
fn corridor_transition_log_structure() {
    let id = CorridorId::new();
    let ja = JurisdictionId::new("PK-REZ").unwrap();
    let jb = JurisdictionId::new("AE-DIFC").unwrap();

    let draft = Corridor::<Draft>::new(id, ja, jb);
    let pending = draft.submit(SubmissionEvidence {
        bilateral_agreement_digest: test_digest("agreement"),
        pack_trilogy_digest: test_digest("packs"),
    });
    let active = pending.activate(ActivationEvidence {
        regulatory_approval_a: test_digest("approval-a"),
        regulatory_approval_b: test_digest("approval-b"),
    });

    let log = active.transition_log();
    assert_eq!(log.len(), 2);

    // First transition: DRAFT -> PENDING
    assert_eq!(log[0].from_state, DynCorridorState::Draft);
    assert_eq!(log[0].to_state, DynCorridorState::Pending);

    // Second transition: PENDING -> ACTIVE
    assert_eq!(log[1].from_state, DynCorridorState::Pending);
    assert_eq!(log[1].to_state, DynCorridorState::Active);

    // Timestamps should be present and ordered
    assert!(log[0].timestamp <= log[1].timestamp);
}

#[test]
fn dyn_corridor_state_valid_transitions() {
    assert_eq!(
        DynCorridorState::Draft.valid_transitions(),
        &[DynCorridorState::Pending]
    );
    assert_eq!(
        DynCorridorState::Pending.valid_transitions(),
        &[DynCorridorState::Active]
    );
    assert_eq!(
        DynCorridorState::Active.valid_transitions(),
        &[DynCorridorState::Halted, DynCorridorState::Suspended]
    );
    assert_eq!(
        DynCorridorState::Halted.valid_transitions(),
        &[DynCorridorState::Deprecated]
    );
    assert_eq!(
        DynCorridorState::Suspended.valid_transitions(),
        &[DynCorridorState::Active]
    );
    assert!(
        DynCorridorState::Deprecated.valid_transitions().is_empty(),
        "terminal state must have no valid transitions"
    );
}
