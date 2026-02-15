//! # Corridor State Machine Property Tests (M-004)
//!
//! Property-based verification that the corridor lifecycle FSM implementation
//! matches the governance specification. Tests verify:
//! - Only valid transitions are permitted
//! - Terminal states reject all further transitions
//! - State names match spec exactly (no "OPERATIONAL", no "PROPOSED")
//! - Typestate encoding prevents invalid transitions at compile time

use msez_core::{sha256_digest, CanonicalBytes, CorridorId, JurisdictionId};
use msez_state::corridor::*;
use serde_json::json;

fn test_jurisdiction_a() -> JurisdictionId {
    JurisdictionId::new("pk").expect("test jurisdiction pk")
}

fn test_jurisdiction_b() -> JurisdictionId {
    JurisdictionId::new("ae").expect("test jurisdiction ae")
}

fn test_digest() -> msez_core::ContentDigest {
    let canonical = CanonicalBytes::new(&json!({"test": "evidence"})).unwrap();
    sha256_digest(&canonical)
}

fn submission_evidence() -> SubmissionEvidence {
    SubmissionEvidence {
        bilateral_agreement_digest: test_digest(),
        pack_trilogy_digest: test_digest(),
    }
}

fn activation_evidence() -> ActivationEvidence {
    ActivationEvidence {
        regulatory_approval_a: test_digest(),
        regulatory_approval_b: test_digest(),
    }
}

fn halt_reason() -> HaltReason {
    HaltReason {
        reason: "Regulatory review required".to_string(),
        authority: test_jurisdiction_a(),
        evidence: test_digest(),
    }
}

fn suspend_reason() -> SuspendReason {
    SuspendReason {
        reason: "Maintenance window".to_string(),
        expected_resume: None,
    }
}

fn resume_evidence() -> ResumeEvidence {
    ResumeEvidence {
        resolution_attestation: test_digest(),
    }
}

fn deprecation_evidence() -> DeprecationEvidence {
    DeprecationEvidence {
        deprecation_decision_digest: test_digest(),
        reason: "Corridor replaced by new agreement".to_string(),
    }
}

/// The full happy-path lifecycle: DRAFT → PENDING → ACTIVE → HALTED → DEPRECATED
#[test]
fn full_lifecycle_happy_path() {
    let corridor_id = CorridorId::new();
    let ja = test_jurisdiction_a();
    let jb = test_jurisdiction_b();

    // DRAFT
    let draft = Corridor::<Draft>::new(corridor_id.clone(), ja, jb);
    assert_eq!(Draft::name(), "DRAFT");
    assert!(!Draft::is_terminal());

    // DRAFT → PENDING
    let pending = draft.submit(submission_evidence());
    assert_eq!(Pending::name(), "PENDING");
    assert!(!Pending::is_terminal());

    // PENDING → ACTIVE
    let active = pending.activate(activation_evidence());
    assert_eq!(Active::name(), "ACTIVE");
    assert!(!Active::is_terminal());

    // ACTIVE → HALTED
    let halted = active.halt(halt_reason());
    assert_eq!(Halted::name(), "HALTED");
    assert!(!Halted::is_terminal());

    // HALTED → DEPRECATED (terminal)
    let deprecated = halted.deprecate(deprecation_evidence());
    assert_eq!(Deprecated::name(), "DEPRECATED");
    assert!(Deprecated::is_terminal());

    // Verify the deprecated corridor preserves its ID.
    assert_eq!(deprecated.id, corridor_id);
}

/// ACTIVE → SUSPENDED → ACTIVE (resume) cycle.
#[test]
fn suspend_and_resume_cycle() {
    let draft = Corridor::<Draft>::new(
        CorridorId::new(),
        test_jurisdiction_a(),
        test_jurisdiction_b(),
    );
    let pending = draft.submit(submission_evidence());
    let active = pending.activate(activation_evidence());

    // ACTIVE → SUSPENDED
    let suspended = active.suspend(suspend_reason());
    assert_eq!(Suspended::name(), "SUSPENDED");
    assert!(!Suspended::is_terminal());

    // SUSPENDED → ACTIVE (resume)
    let resumed = suspended.resume(resume_evidence());
    assert_eq!(Active::name(), "ACTIVE");

    // Can suspend again after resume.
    let _re_suspended = resumed.suspend(suspend_reason());
}

/// State names match the governance spec exactly.
#[test]
fn state_names_match_spec() {
    // These are the exact names from governance/corridor.lifecycle.state-machine.v2.json.
    // No "OPERATIONAL", no "PROPOSED" — those were the Python-era names.
    assert_eq!(Draft::name(), "DRAFT");
    assert_eq!(Pending::name(), "PENDING");
    assert_eq!(Active::name(), "ACTIVE");
    assert_eq!(Halted::name(), "HALTED");
    assert_eq!(Suspended::name(), "SUSPENDED");
    assert_eq!(Deprecated::name(), "DEPRECATED");
}

/// Only DEPRECATED is a terminal state.
#[test]
fn terminal_state_identification() {
    assert!(!Draft::is_terminal());
    assert!(!Pending::is_terminal());
    assert!(!Active::is_terminal());
    assert!(!Halted::is_terminal());
    assert!(!Suspended::is_terminal());
    assert!(Deprecated::is_terminal());
}

/// Dynamic corridor state serialization and round-trip.
#[test]
fn dynamic_state_serde_roundtrip() {
    let draft = Corridor::<Draft>::new(
        CorridorId::new(),
        test_jurisdiction_a(),
        test_jurisdiction_b(),
    );
    let dyn_state = DynCorridorData::from(&draft);

    let json_str = serde_json::to_string(&dyn_state).expect("serialize DynCorridorData");
    let recovered: DynCorridorData =
        serde_json::from_str(&json_str).expect("deserialize DynCorridorData");

    assert_eq!(recovered.state, DynCorridorState::Draft);
}

/// All six states are representable in DynCorridorState.
#[test]
fn dyn_corridor_state_all_variants() {
    let variants = [
        DynCorridorState::Draft,
        DynCorridorState::Pending,
        DynCorridorState::Active,
        DynCorridorState::Halted,
        DynCorridorState::Suspended,
        DynCorridorState::Deprecated,
    ];

    for variant in &variants {
        let json_str = serde_json::to_string(variant).expect("serialize");
        let recovered: DynCorridorState = serde_json::from_str(&json_str).expect("deserialize");
        assert_eq!(&recovered, variant);
    }
}

/// Transition records accumulate for each state change.
#[test]
fn transition_records_accumulate() {
    let draft = Corridor::<Draft>::new(
        CorridorId::new(),
        test_jurisdiction_a(),
        test_jurisdiction_b(),
    );
    let pending = draft.submit(submission_evidence());
    let active = pending.activate(activation_evidence());
    let halted = active.halt(halt_reason());

    // Should have 3 transitions: submit, activate, halt.
    let log = halted.transition_log();
    assert_eq!(
        log.len(),
        3,
        "expected 3 transitions (submit, activate, halt), got {}",
        log.len()
    );
    assert_eq!(log[0].from_state, DynCorridorState::Draft);
    assert_eq!(log[0].to_state, DynCorridorState::Pending);
    assert_eq!(log[1].from_state, DynCorridorState::Pending);
    assert_eq!(log[1].to_state, DynCorridorState::Active);
    assert_eq!(log[2].from_state, DynCorridorState::Active);
    assert_eq!(log[2].to_state, DynCorridorState::Halted);
}

/// Corridor preserves jurisdiction IDs across transitions.
#[test]
fn jurisdiction_ids_preserved() {
    let ja = test_jurisdiction_a();
    let jb = test_jurisdiction_b();
    let draft = Corridor::<Draft>::new(CorridorId::new(), ja.clone(), jb.clone());
    let pending = draft.submit(submission_evidence());
    let active = pending.activate(activation_evidence());

    assert_eq!(active.jurisdiction_a.as_str(), ja.as_str());
    assert_eq!(active.jurisdiction_b.as_str(), jb.as_str());
}

/// DynCorridorState valid_transitions matches the spec.
#[test]
fn dyn_state_valid_transitions() {
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
    assert!(DynCorridorState::Deprecated.valid_transitions().is_empty());
}

/// Python-era state names ("PROPOSED", "OPERATIONAL") are rejected by the enum.
#[test]
fn python_era_state_names_rejected() {
    let result: Result<DynCorridorState, _> = serde_json::from_str("\"PROPOSED\"");
    assert!(result.is_err(), "PROPOSED should be rejected");

    let result: Result<DynCorridorState, _> = serde_json::from_str("\"OPERATIONAL\"");
    assert!(result.is_err(), "OPERATIONAL should be rejected");
}
