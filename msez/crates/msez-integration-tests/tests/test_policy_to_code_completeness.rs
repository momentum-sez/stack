//! # Policy-to-Code Completeness Verification
//!
//! Python counterpart: `tests/test_policy_to_code_completeness.py`
//!
//! Verifies that the agentic policy engine has complete coverage:
//! - All 20 trigger types have string representations
//! - All trigger types are unique
//! - Standard + extended policies cover all critical triggers
//! - Every PolicyAction variant exists and has a string representation

use msez_agentic::evaluation::PolicyEngine;
use msez_agentic::policy::{extended_policies, PolicyAction, Trigger, TriggerType};
use serde_json::json;
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// 1. All trigger types have string representation
// ---------------------------------------------------------------------------

#[test]
fn all_trigger_types_have_string_representation() {
    for tt in TriggerType::all() {
        let s = tt.as_str();
        assert!(
            !s.is_empty(),
            "trigger type {tt:?} has empty string representation"
        );
        assert!(
            s.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "trigger type string '{}' should be snake_case",
            s
        );
    }
}

// ---------------------------------------------------------------------------
// 2. All trigger types are unique
// ---------------------------------------------------------------------------

#[test]
fn all_trigger_types_are_unique() {
    let all = TriggerType::all();
    let unique: HashSet<&TriggerType> = all.iter().collect();
    assert_eq!(
        unique.len(),
        all.len(),
        "all trigger types must be unique; got {} unique out of {}",
        unique.len(),
        all.len()
    );
}

#[test]
fn all_trigger_type_strings_are_unique() {
    let strings: Vec<&str> = TriggerType::all().iter().map(|t| t.as_str()).collect();
    let unique: HashSet<&str> = strings.iter().copied().collect();
    assert_eq!(
        unique.len(),
        strings.len(),
        "all trigger type strings must be unique"
    );
}

// ---------------------------------------------------------------------------
// 3. Standard + extended cover critical trigger types
// ---------------------------------------------------------------------------

#[test]
fn standard_and_extended_cover_critical_triggers() {
    let ext_policies = extended_policies();

    // Critical triggers that MUST have at least one policy
    let critical_triggers = [
        TriggerType::SanctionsListUpdate,
        TriggerType::LicenseStatusChange,
        TriggerType::ComplianceDeadline,
        TriggerType::CorridorStateChange,
    ];

    for tt in &critical_triggers {
        let has_policy = ext_policies.values().any(|p| p.trigger_type == *tt);
        assert!(
            has_policy,
            "extended policies must cover critical trigger type {:?}",
            tt
        );
    }
}

// ---------------------------------------------------------------------------
// 4. Every PolicyAction variant exists and has string
// ---------------------------------------------------------------------------

#[test]
fn every_policy_action_variant_exists() {
    let actions = [
        PolicyAction::Transfer,
        PolicyAction::Mint,
        PolicyAction::Burn,
        PolicyAction::ActivateBinding,
        PolicyAction::DeactivateBinding,
        PolicyAction::MigrateBinding,
        PolicyAction::UpdateManifest,
        PolicyAction::AmendGovernance,
        PolicyAction::AddGovernor,
        PolicyAction::RemoveGovernor,
        PolicyAction::Dividend,
        PolicyAction::Split,
        PolicyAction::Merger,
        PolicyAction::Halt,
        PolicyAction::Resume,
        PolicyAction::ArbitrationEnforce,
    ];

    let mut seen_strings = HashSet::new();
    for action in &actions {
        let s = action.as_str();
        assert!(!s.is_empty(), "action {action:?} has empty string");
        assert!(seen_strings.insert(s), "duplicate action string: '{s}'");
    }
    assert_eq!(actions.len(), 16, "expecting 16 PolicyAction variants");
}

// ---------------------------------------------------------------------------
// 5. Every trigger type can be used to construct a Trigger
// ---------------------------------------------------------------------------

#[test]
fn every_trigger_type_constructible() {
    for tt in TriggerType::all() {
        let trigger = Trigger::new(*tt, json!({}));
        assert_eq!(trigger.trigger_type, *tt);
    }
}

// ---------------------------------------------------------------------------
// 6. Engine processes all trigger types without panic
// ---------------------------------------------------------------------------

#[test]
fn engine_handles_all_trigger_types_gracefully() {
    let mut engine = PolicyEngine::with_extended_policies();

    for tt in TriggerType::all() {
        let trigger = Trigger::new(*tt, json!({"test": true}));
        // Must not panic
        let _actions = engine.process_trigger(&trigger, "asset:test-001", None);
    }
}

// ---------------------------------------------------------------------------
// 7. Policy action serde roundtrip
// ---------------------------------------------------------------------------

#[test]
fn policy_action_serde_roundtrip() {
    let actions = [
        PolicyAction::Halt,
        PolicyAction::Resume,
        PolicyAction::Transfer,
        PolicyAction::ArbitrationEnforce,
    ];

    for action in &actions {
        let serialized = serde_json::to_string(action).unwrap();
        let deserialized: PolicyAction = serde_json::from_str(&serialized).unwrap();
        assert_eq!(*action, deserialized);
    }
}
