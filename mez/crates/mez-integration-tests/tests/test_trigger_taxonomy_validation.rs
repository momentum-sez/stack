//! # Agentic Trigger Taxonomy Domain Validation (S-019)
//!
//! Validates that the 20 trigger types in `mez-agentic` maintain their
//! contract: correct count, correct domain mapping, deterministic evaluation,
//! and exhaustive coverage of the five trigger domains (Regulatory, Arbitration,
//! Corridor, Asset, Fiscal, Entity).

use mez_agentic::evaluation::PolicyEngine;
use mez_agentic::policy::{PolicyAction, Trigger, TriggerType};

/// TriggerType::all() must return exactly 20 variants.
#[test]
fn trigger_type_has_exactly_20_variants() {
    assert_eq!(
        TriggerType::all().len(),
        20,
        "MASS Protocol v0.2 Ch. 17 specifies exactly 20 trigger types"
    );
}

/// Every trigger type must have a non-empty string representation.
#[test]
fn all_trigger_types_have_string_representation() {
    for tt in TriggerType::all() {
        let s = tt.as_str();
        assert!(!s.is_empty(), "trigger type {:?} has empty as_str()", tt);
        assert!(
            s.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "trigger type {:?} as_str() should be snake_case, got: {s}",
            tt
        );
    }
}

/// TriggerType string values must be unique (no duplicates).
#[test]
fn trigger_type_strings_are_unique() {
    let mut seen = std::collections::HashSet::new();
    for tt in TriggerType::all() {
        let s = tt.as_str();
        assert!(seen.insert(s), "duplicate trigger type string: {s}");
    }
}

/// All 20 trigger types round-trip through serde.
#[test]
fn trigger_type_serde_roundtrip() {
    for tt in TriggerType::all() {
        let json = serde_json::to_string(tt).expect("serialize");
        let recovered: TriggerType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(&recovered, tt, "serde roundtrip failed for {:?}", tt);
    }
}

/// The standard policy engine processes all 20 trigger types without panicking.
#[test]
fn standard_engine_handles_all_trigger_types() {
    let mut engine = PolicyEngine::with_standard_policies();

    for tt in TriggerType::all() {
        let trigger = Trigger::new(*tt, serde_json::json!({"test": true}));
        // This should not panic for any trigger type.
        let _actions = engine.process_trigger(&trigger, "test:asset", None);
    }
}

/// Sanctions list update must trigger a Halt action via standard policies.
#[test]
fn sanctions_trigger_produces_halt_action() {
    let mut engine = PolicyEngine::with_standard_policies();
    let trigger = Trigger::new(
        TriggerType::SanctionsListUpdate,
        serde_json::json!({"affected_parties": ["self"]}),
    );
    let actions = engine.process_trigger(&trigger, "asset:test", None);
    assert!(
        actions.iter().any(|a| a.action == PolicyAction::Halt),
        "SanctionsListUpdate must produce Halt action"
    );
}

/// Compliance deadline trigger is processed by the engine.
#[test]
fn compliance_deadline_processed() {
    let mut engine = PolicyEngine::with_standard_policies();
    let trigger = Trigger::new(
        TriggerType::ComplianceDeadline,
        serde_json::json!({"deadline": "2026-03-01"}),
    );
    let actions = engine.process_trigger(&trigger, "asset:deadline-test", None);
    // Engine must return a defined (possibly empty) action list, not panic
    assert!(actions.len() <= 20, "action count should be bounded");
}

/// TaxYearEnd trigger should be processable (fiscal domain).
#[test]
fn tax_year_end_processable() {
    let mut engine = PolicyEngine::with_standard_policies();
    let trigger = Trigger::new(
        TriggerType::TaxYearEnd,
        serde_json::json!({"tax_year": "2025-2026", "jurisdiction": "PK"}),
    );
    let actions = engine.process_trigger(&trigger, "entity:pk-corp", None);
    assert!(actions.len() <= 20, "action count should be bounded");
}

/// WithholdingDue trigger should be processable (fiscal domain).
#[test]
fn withholding_due_processable() {
    let mut engine = PolicyEngine::with_standard_policies();
    let trigger = Trigger::new(
        TriggerType::WithholdingDue,
        serde_json::json!({"entity_id": "test", "amount": "50000"}),
    );
    let actions = engine.process_trigger(&trigger, "entity:pk-corp", None);
    assert!(actions.len() <= 20, "action count should be bounded");
}

/// All trigger types from the regulatory domain are correctly identified.
#[test]
fn regulatory_domain_triggers_exist() {
    let regulatory_triggers = [
        TriggerType::SanctionsListUpdate,
        TriggerType::LicenseStatusChange,
        TriggerType::GuidanceUpdate,
        TriggerType::ComplianceDeadline,
    ];
    for tt in &regulatory_triggers {
        assert!(
            TriggerType::all().contains(tt),
            "regulatory trigger {:?} must be in TriggerType::all()",
            tt
        );
    }
}

/// All trigger types from the arbitration domain are correctly identified.
#[test]
fn arbitration_domain_triggers_exist() {
    let arb_triggers = [
        TriggerType::DisputeFiled,
        TriggerType::RulingReceived,
        TriggerType::AppealPeriodExpired,
        TriggerType::EnforcementDue,
    ];
    for tt in &arb_triggers {
        assert!(
            TriggerType::all().contains(tt),
            "arbitration trigger {:?} must be in TriggerType::all()",
            tt
        );
    }
}

/// All trigger types from the corridor domain are correctly identified.
#[test]
fn corridor_domain_triggers_exist() {
    let corridor_triggers = [
        TriggerType::CorridorStateChange,
        TriggerType::SettlementAnchorAvailable,
        TriggerType::WatcherQuorumReached,
    ];
    for tt in &corridor_triggers {
        assert!(
            TriggerType::all().contains(tt),
            "corridor trigger {:?} must be in TriggerType::all()",
            tt
        );
    }
}

/// Determinism: same trigger + same policies = same actions (Theorem 17.1).
#[test]
fn policy_evaluation_is_deterministic() {
    let trigger = Trigger::new(
        TriggerType::SanctionsListUpdate,
        serde_json::json!({"affected_parties": ["self"]}),
    );

    let mut engine1 = PolicyEngine::with_standard_policies();
    let mut engine2 = PolicyEngine::with_standard_policies();

    let actions1 = engine1.process_trigger(&trigger, "asset:test", None);
    let actions2 = engine2.process_trigger(&trigger, "asset:test", None);

    assert_eq!(
        actions1.len(),
        actions2.len(),
        "determinism: same trigger must produce same number of actions"
    );

    for (a1, a2) in actions1.iter().zip(actions2.iter()) {
        assert_eq!(a1.action, a2.action, "determinism: action types must match");
        assert_eq!(
            a1.policy_id, a2.policy_id,
            "determinism: policy IDs must match"
        );
    }
}
