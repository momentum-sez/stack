//! # Transition Types Registry Integration Tests
//!
//! Python counterpart: `tests/test_transition_types_registry.py`
//!
//! Tests that all state machine states have canonical string names,
//! terminal states are correctly identified, and the DynCorridorState
//! enum correctly serializes and deserializes.

use mez_core::{sha256_digest, CanonicalBytes};
use mez_state::{DynCorridorState, EntityLifecycleState, LicenseState, MigrationState};
use serde_json::json;

// ---------------------------------------------------------------------------
// 1. Entity lifecycle states all have canonical string names
// ---------------------------------------------------------------------------

#[test]
fn entity_states_all_named() {
    let states = [
        (EntityLifecycleState::Applied, "APPLIED"),
        (EntityLifecycleState::Active, "ACTIVE"),
        (EntityLifecycleState::Suspended, "SUSPENDED"),
        (EntityLifecycleState::Dissolving, "DISSOLVING"),
        (EntityLifecycleState::Dissolved, "DISSOLVED"),
        (EntityLifecycleState::Rejected, "REJECTED"),
    ];

    for (state, expected_name) in &states {
        assert_eq!(
            state.as_str(),
            *expected_name,
            "entity state {:?} has wrong canonical name",
            state
        );
    }
}

// ---------------------------------------------------------------------------
// 2. Corridor states all have canonical string names
// ---------------------------------------------------------------------------

#[test]
fn corridor_states_all_named() {
    let states = [
        (DynCorridorState::Draft, "DRAFT"),
        (DynCorridorState::Pending, "PENDING"),
        (DynCorridorState::Active, "ACTIVE"),
        (DynCorridorState::Halted, "HALTED"),
        (DynCorridorState::Suspended, "SUSPENDED"),
        (DynCorridorState::Deprecated, "DEPRECATED"),
    ];

    for (state, expected_name) in &states {
        assert_eq!(
            state.as_str(),
            *expected_name,
            "corridor state {:?} has wrong canonical name",
            state
        );
    }
}

// ---------------------------------------------------------------------------
// 3. License states all have canonical string names
// ---------------------------------------------------------------------------

#[test]
fn license_states_all_named() {
    let states = [
        (LicenseState::Applied, "APPLIED"),
        (LicenseState::UnderReview, "UNDER_REVIEW"),
        (LicenseState::Active, "ACTIVE"),
        (LicenseState::Suspended, "SUSPENDED"),
        (LicenseState::Revoked, "REVOKED"),
        (LicenseState::Expired, "EXPIRED"),
        (LicenseState::Surrendered, "SURRENDERED"),
        (LicenseState::Rejected, "REJECTED"),
    ];

    for (state, expected_name) in &states {
        assert_eq!(
            state.as_str(),
            *expected_name,
            "license state {:?} has wrong canonical name",
            state
        );
    }
}

// ---------------------------------------------------------------------------
// 4. Terminal states correctly identified across all machines
// ---------------------------------------------------------------------------

#[test]
fn terminal_states_identified() {
    // Entity terminal states
    assert!(EntityLifecycleState::Dissolved.is_terminal());
    assert!(EntityLifecycleState::Rejected.is_terminal());
    assert!(!EntityLifecycleState::Applied.is_terminal());
    assert!(!EntityLifecycleState::Active.is_terminal());
    assert!(!EntityLifecycleState::Suspended.is_terminal());
    assert!(!EntityLifecycleState::Dissolving.is_terminal());

    // License terminal states
    assert!(LicenseState::Revoked.is_terminal());
    assert!(LicenseState::Expired.is_terminal());
    assert!(LicenseState::Surrendered.is_terminal());
    assert!(LicenseState::Rejected.is_terminal());
    assert!(!LicenseState::Applied.is_terminal());
    assert!(!LicenseState::UnderReview.is_terminal());
    assert!(!LicenseState::Active.is_terminal());
    assert!(!LicenseState::Suspended.is_terminal());

    // Corridor terminal states
    assert!(DynCorridorState::Deprecated.is_terminal());
    assert!(!DynCorridorState::Draft.is_terminal());
    assert!(!DynCorridorState::Pending.is_terminal());
    assert!(!DynCorridorState::Active.is_terminal());
    assert!(!DynCorridorState::Halted.is_terminal());
    assert!(!DynCorridorState::Suspended.is_terminal());

    // Migration terminal states
    assert!(MigrationState::Completed.is_terminal());
    assert!(MigrationState::Compensated.is_terminal());
    assert!(MigrationState::TimedOut.is_terminal());
    assert!(MigrationState::Cancelled.is_terminal());
    assert!(!MigrationState::Initiated.is_terminal());
    assert!(!MigrationState::ComplianceCheck.is_terminal());
    assert!(!MigrationState::InTransit.is_terminal());
}

// ---------------------------------------------------------------------------
// 5. DynCorridorState serde roundtrip
// ---------------------------------------------------------------------------

#[test]
fn corridor_state_serde_roundtrip() {
    let states = [
        DynCorridorState::Draft,
        DynCorridorState::Pending,
        DynCorridorState::Active,
        DynCorridorState::Halted,
        DynCorridorState::Suspended,
        DynCorridorState::Deprecated,
    ];

    for state in &states {
        let serialized = serde_json::to_string(state).unwrap();
        let deserialized: DynCorridorState = serde_json::from_str(&serialized).unwrap();
        assert_eq!(*state, deserialized);
    }
}

// ---------------------------------------------------------------------------
// 6. Migration state display names match spec
// ---------------------------------------------------------------------------

#[test]
fn migration_state_names_match_spec() {
    assert_eq!(MigrationState::Initiated.as_str(), "INITIATED");
    assert_eq!(MigrationState::ComplianceCheck.as_str(), "COMPLIANCE_CHECK");
    assert_eq!(
        MigrationState::AttestationGathering.as_str(),
        "ATTESTATION_GATHERING"
    );
    assert_eq!(MigrationState::SourceLocked.as_str(), "SOURCE_LOCKED");
    assert_eq!(MigrationState::InTransit.as_str(), "IN_TRANSIT");
    assert_eq!(
        MigrationState::DestinationVerification.as_str(),
        "DESTINATION_VERIFICATION"
    );
    assert_eq!(
        MigrationState::DestinationUnlock.as_str(),
        "DESTINATION_UNLOCK"
    );
    assert_eq!(MigrationState::Completed.as_str(), "COMPLETED");
    assert_eq!(MigrationState::Compensated.as_str(), "COMPENSATED");
    assert_eq!(MigrationState::TimedOut.as_str(), "TIMED_OUT");
    assert_eq!(MigrationState::Cancelled.as_str(), "CANCELLED");
}

// ---------------------------------------------------------------------------
// 7. State data is canonicalizable
// ---------------------------------------------------------------------------

#[test]
fn state_data_canonicalizable() {
    let state_data = json!({
        "entity_state": "ACTIVE",
        "corridor_state": "PENDING",
        "license_state": "UNDER_REVIEW",
        "migration_state": "IN_TRANSIT"
    });

    let canonical = CanonicalBytes::new(&state_data).unwrap();
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);
}
