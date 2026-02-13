//! # Agentic Policy Engine — Basic Integration Tests
//!
//! Python counterpart: `tests/test_agentic.py`
//!
//! Tests the agentic policy engine fundamentals:
//! - Standard policy loading and count verification
//! - Sanctions trigger producing halt actions
//! - Unrelated trigger type not matching
//! - Standard policy count is 4
//! - Trigger type count is 20

use msez_agentic::evaluation::PolicyEngine;
use msez_agentic::policy::{
    extended_policies, standard_policies, PolicyAction, Trigger, TriggerType,
};
use serde_json::json;

// ---------------------------------------------------------------------------
// 1. Engine construction with standard policies
// ---------------------------------------------------------------------------

#[test]
fn engine_with_standard_policies() {
    let engine = PolicyEngine::with_standard_policies();
    assert_eq!(engine.policy_count(), 4);
}

#[test]
fn engine_with_extended_policies_has_more() {
    let engine = PolicyEngine::with_extended_policies();
    assert!(
        engine.policy_count() > 4,
        "extended policies must include more than standard 4, got {}",
        engine.policy_count()
    );
}

// ---------------------------------------------------------------------------
// 2. Sanctions trigger produces halt action
// ---------------------------------------------------------------------------

#[test]
fn sanctions_trigger_produces_halt() {
    let mut engine = PolicyEngine::with_standard_policies();
    let trigger = Trigger::new(
        TriggerType::SanctionsListUpdate,
        json!({"affected_parties": ["self"]}),
    );

    let actions = engine.process_trigger(&trigger, "asset:mining-license-001", None);
    assert!(
        !actions.is_empty(),
        "sanctions update should produce at least one action"
    );
    assert!(
        actions.iter().any(|a| a.action == PolicyAction::Halt),
        "sanctions update should trigger a Halt action"
    );
}

// ---------------------------------------------------------------------------
// 3. Unrelated trigger produces no matching policies
// ---------------------------------------------------------------------------

#[test]
fn unrelated_trigger_no_match() {
    let mut engine = PolicyEngine::new();
    let policy = msez_agentic::policy::Policy::new(
        "sanctions-only",
        TriggerType::SanctionsListUpdate,
        PolicyAction::Halt,
    );
    engine.register_policy(policy);

    // Fire a completely different trigger type
    let trigger = Trigger::new(TriggerType::KeyRotationDue, json!({}));
    let results = engine.evaluate(&trigger, Some("asset:test"), None);
    let matched: Vec<_> = results.iter().filter(|r| r.matched).collect();
    assert!(
        matched.is_empty(),
        "unrelated trigger should not match any policy"
    );
}

// ---------------------------------------------------------------------------
// 4. Standard policy count is exactly 4
// ---------------------------------------------------------------------------

#[test]
fn standard_policy_count_is_4() {
    assert_eq!(standard_policies().len(), 4);
}

// ---------------------------------------------------------------------------
// 5. Extended policies are a superset of standard
// ---------------------------------------------------------------------------

#[test]
fn extended_policies_superset_of_standard() {
    let std_policies = standard_policies();
    let ext_policies = extended_policies();
    assert!(ext_policies.len() >= std_policies.len());
    for key in std_policies.keys() {
        assert!(
            ext_policies.contains_key(key),
            "extended policies must contain standard policy '{key}'"
        );
    }
}

// ---------------------------------------------------------------------------
// 6. Trigger type count is exactly 20
// ---------------------------------------------------------------------------

#[test]
fn trigger_type_count_is_20() {
    assert_eq!(TriggerType::all().len(), 20);
}

// ---------------------------------------------------------------------------
// 7. Process trigger returns ScheduledActions with correct asset_id
// ---------------------------------------------------------------------------

#[test]
fn process_trigger_populates_asset_id() {
    let mut engine = PolicyEngine::with_standard_policies();
    let trigger = Trigger::new(
        TriggerType::SanctionsListUpdate,
        json!({"affected_parties": ["self"]}),
    );

    let actions = engine.process_trigger(&trigger, "asset:gold-mine-007", None);
    for action in &actions {
        assert_eq!(
            action.asset_id, "asset:gold-mine-007",
            "all scheduled actions must reference the correct asset"
        );
    }
}

// ---------------------------------------------------------------------------
// 8. Disabled policy is not matched
// ---------------------------------------------------------------------------

#[test]
fn disabled_policy_not_evaluated() {
    let mut engine = PolicyEngine::new();
    let policy = msez_agentic::policy::Policy::new(
        "disabled-sanctions",
        TriggerType::SanctionsListUpdate,
        PolicyAction::Halt,
    )
    .with_enabled(false);
    engine.register_policy(policy);

    let trigger = Trigger::new(
        TriggerType::SanctionsListUpdate,
        json!({"affected_parties": ["self"]}),
    );
    let results = engine.evaluate(&trigger, Some("asset:test"), None);
    assert!(!results.iter().any(|r| r.matched));
}
