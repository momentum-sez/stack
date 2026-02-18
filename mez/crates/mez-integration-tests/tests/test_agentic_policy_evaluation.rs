//! # Agentic Policy Evaluation Test
//!
//! Tests the agentic policy engine (MASS Protocol v0.2 Chapter 17):
//! - Policy registration and matching
//! - Multiple trigger types
//! - Priority-based conflict resolution
//! - Audit trail completeness
//! - Determinism (Theorem 17.1)

use mez_agentic::{
    evaluation::PolicyEngine,
    policy::{Condition, Policy, PolicyAction, Trigger, TriggerType},
};
use serde_json::json;

// ---------------------------------------------------------------------------
// 1. Standard policies load correctly
// ---------------------------------------------------------------------------

#[test]
fn standard_policies_load() {
    let engine = PolicyEngine::with_standard_policies();
    assert_eq!(engine.policy_count(), 4);
}

#[test]
fn extended_policies_load() {
    let engine = PolicyEngine::with_extended_policies();
    assert!(engine.policy_count() >= 10);
}

// ---------------------------------------------------------------------------
// 2. Policy matching with trigger types
// ---------------------------------------------------------------------------

#[test]
fn sanctions_trigger_matches_standard_policy() {
    let mut engine = PolicyEngine::with_standard_policies();
    let trigger = Trigger::new(
        TriggerType::SanctionsListUpdate,
        json!({"affected_parties": ["self"]}),
    );

    let actions = engine.process_trigger(&trigger, "asset:test", None);
    assert!(
        actions.iter().any(|a| a.action == PolicyAction::Halt),
        "sanctions update should trigger halt action"
    );
}

#[test]
fn unrelated_trigger_does_not_match() {
    let mut engine = PolicyEngine::new();
    let policy = Policy::new(
        "test-policy",
        TriggerType::SanctionsListUpdate,
        PolicyAction::Halt,
    );
    engine.register_policy(policy);

    let trigger = Trigger::new(TriggerType::CheckpointDue, json!({}));
    let results = engine.evaluate(&trigger, Some("asset:test"), None);

    // The policy should not match (wrong trigger type)
    let matched: Vec<_> = results.iter().filter(|r| r.matched).collect();
    assert!(matched.is_empty());
}

// ---------------------------------------------------------------------------
// 3. Condition evaluation
// ---------------------------------------------------------------------------

#[test]
fn condition_equals() {
    let mut engine = PolicyEngine::new();
    let policy = Policy::new(
        "status-check",
        TriggerType::LicenseStatusChange,
        PolicyAction::Halt,
    )
    .with_condition(Condition::Equals {
        field: "status".to_string(),
        value: json!("expired"),
    });
    engine.register_policy(policy);

    // Matching trigger
    let trigger_match = Trigger::new(
        TriggerType::LicenseStatusChange,
        json!({"status": "expired"}),
    );
    let results = engine.evaluate(&trigger_match, Some("asset:test"), None);
    assert!(results.iter().any(|r| r.matched));

    // Non-matching trigger
    let trigger_no_match = Trigger::new(
        TriggerType::LicenseStatusChange,
        json!({"status": "active"}),
    );
    let results = engine.evaluate(&trigger_no_match, Some("asset:test"), None);
    assert!(!results.iter().any(|r| r.matched));
}

#[test]
fn condition_threshold() {
    let mut engine = PolicyEngine::new();
    let policy = Policy::new(
        "risk-threshold",
        TriggerType::CorridorStateChange,
        PolicyAction::Halt,
    )
    .with_condition(Condition::Threshold {
        field: "risk_score".to_string(),
        threshold: json!(80),
    });
    engine.register_policy(policy);

    // Above threshold
    let trigger_high = Trigger::new(TriggerType::CorridorStateChange, json!({"risk_score": 95}));
    let results = engine.evaluate(&trigger_high, Some("asset:test"), None);
    assert!(results.iter().any(|r| r.matched));

    // Below threshold
    let trigger_low = Trigger::new(TriggerType::CorridorStateChange, json!({"risk_score": 50}));
    let results = engine.evaluate(&trigger_low, Some("asset:test"), None);
    assert!(!results.iter().any(|r| r.matched));
}

#[test]
fn condition_contains() {
    let mut engine = PolicyEngine::new();
    let policy = Policy::new(
        "party-check",
        TriggerType::SanctionsListUpdate,
        PolicyAction::Halt,
    )
    .with_condition(Condition::Contains {
        field: "affected_parties".to_string(),
        item: json!("self"),
    });
    engine.register_policy(policy);

    let trigger_match = Trigger::new(
        TriggerType::SanctionsListUpdate,
        json!({"affected_parties": ["counterparty", "self"]}),
    );
    let results = engine.evaluate(&trigger_match, Some("asset:test"), None);
    assert!(results.iter().any(|r| r.matched));

    let trigger_no_match = Trigger::new(
        TriggerType::SanctionsListUpdate,
        json!({"affected_parties": ["counterparty"]}),
    );
    let results = engine.evaluate(&trigger_no_match, Some("asset:test"), None);
    assert!(!results.iter().any(|r| r.matched));
}

#[test]
fn condition_and_or_composition() {
    let mut engine = PolicyEngine::new();

    let policy = Policy::new(
        "compound",
        TriggerType::CorridorStateChange,
        PolicyAction::Halt,
    )
    .with_condition(Condition::And {
        conditions: vec![
            Condition::Equals {
                field: "type".to_string(),
                value: json!("fork_detected"),
            },
            Condition::GreaterThan {
                field: "severity".to_string(),
                threshold: json!(5),
            },
        ],
    });
    engine.register_policy(policy);

    // Both conditions met
    let trigger = Trigger::new(
        TriggerType::CorridorStateChange,
        json!({"type": "fork_detected", "severity": 8}),
    );
    let results = engine.evaluate(&trigger, Some("asset:test"), None);
    assert!(results.iter().any(|r| r.matched));

    // Only one condition met
    let trigger = Trigger::new(
        TriggerType::CorridorStateChange,
        json!({"type": "fork_detected", "severity": 3}),
    );
    let results = engine.evaluate(&trigger, Some("asset:test"), None);
    assert!(!results.iter().any(|r| r.matched));
}

// ---------------------------------------------------------------------------
// 4. Priority-based conflict resolution
// ---------------------------------------------------------------------------

#[test]
fn higher_priority_wins_conflict() {
    let mut engine = PolicyEngine::new();

    let low =
        Policy::new("low-pri", TriggerType::CheckpointDue, PolicyAction::Resume).with_priority(1);
    let high =
        Policy::new("high-pri", TriggerType::CheckpointDue, PolicyAction::Halt).with_priority(100);

    engine.register_policy(low);
    engine.register_policy(high);

    let trigger = Trigger::new(TriggerType::CheckpointDue, json!({}));
    let resolved = engine.evaluate_and_resolve(&trigger, Some("asset:test"), None);

    assert!(!resolved.is_empty());
    // The highest priority should be first
    assert_eq!(resolved[0].priority, 100);
    assert_eq!(resolved[0].action, Some(PolicyAction::Halt));
}

#[test]
fn same_action_deduplicated_by_priority() {
    let mut engine = PolicyEngine::new();

    let p1 = Policy::new("p1", TriggerType::CheckpointDue, PolicyAction::Halt).with_priority(1);
    let p2 = Policy::new("p2", TriggerType::CheckpointDue, PolicyAction::Halt).with_priority(100);

    engine.register_policy(p1);
    engine.register_policy(p2);

    let trigger = Trigger::new(TriggerType::CheckpointDue, json!({}));
    let resolved = engine.evaluate_and_resolve(&trigger, Some("asset:test"), None);

    // Deduplicate by action: only one Halt action (from highest priority)
    let halt_results: Vec<_> = resolved
        .iter()
        .filter(|r| r.action == Some(PolicyAction::Halt))
        .collect();
    assert_eq!(halt_results.len(), 1);
    assert_eq!(halt_results[0].policy_id, "p2"); // Higher priority
}

// ---------------------------------------------------------------------------
// 5. Jurisdiction scope filtering
// ---------------------------------------------------------------------------

#[test]
fn jurisdiction_scope_filters_correctly() {
    let mut engine = PolicyEngine::new();

    let pk_policy = Policy::new("pk-only", TriggerType::TaxYearEnd, PolicyAction::Halt)
        .with_jurisdiction_scope(vec!["PK-RSEZ".to_string()]);

    let global_policy = Policy::new("global", TriggerType::TaxYearEnd, PolicyAction::Resume);

    engine.register_policy(pk_policy);
    engine.register_policy(global_policy);

    // PK jurisdiction: both should match
    let trigger = Trigger::new(TriggerType::TaxYearEnd, json!({}));
    let results = engine.evaluate(&trigger, Some("asset:test"), Some("PK-RSEZ"));
    let matched: Vec<_> = results.iter().filter(|r| r.matched).collect();
    assert_eq!(matched.len(), 2);

    // AE jurisdiction: only global should match
    let results = engine.evaluate(&trigger, Some("asset:test"), Some("AE-DIFC"));
    let matched: Vec<_> = results.iter().filter(|r| r.matched).collect();
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0].policy_id, "global");
}

// ---------------------------------------------------------------------------
// 6. Audit trail completeness
// ---------------------------------------------------------------------------

#[test]
fn audit_trail_records_trigger_and_evaluations() {
    let mut engine = PolicyEngine::with_standard_policies();
    let initial_len = engine.audit_trail.len();

    let trigger = Trigger::new(
        TriggerType::SanctionsListUpdate,
        json!({"affected_parties": ["self"]}),
    );
    engine.evaluate(&trigger, Some("asset:test"), None);

    // Should have at least: 1 trigger_received + N policy_evaluated entries
    assert!(
        engine.audit_trail.len() > initial_len,
        "audit trail must grow after evaluation"
    );
    // At minimum 1 trigger + 4 policy evaluations = 5 entries
    assert!(engine.audit_trail.len() >= initial_len + 5);
}

// ---------------------------------------------------------------------------
// 7. Determinism (Theorem 17.1)
// ---------------------------------------------------------------------------

#[test]
fn evaluation_is_deterministic() {
    let trigger = Trigger::new(
        TriggerType::SanctionsListUpdate,
        json!({"affected_parties": ["self"]}),
    );

    // Run evaluation twice with identical state
    let mut engine1 = PolicyEngine::with_standard_policies();
    let mut engine2 = PolicyEngine::with_standard_policies();

    let r1 = engine1.evaluate(&trigger, Some("asset:test"), None);
    let r2 = engine2.evaluate(&trigger, Some("asset:test"), None);

    assert_eq!(r1.len(), r2.len());
    for (a, b) in r1.iter().zip(r2.iter()) {
        assert_eq!(a.policy_id, b.policy_id);
        assert_eq!(a.matched, b.matched);
        assert_eq!(a.action, b.action);
        assert_eq!(a.priority, b.priority);
    }
}

// ---------------------------------------------------------------------------
// 8. process_trigger end-to-end
// ---------------------------------------------------------------------------

#[test]
fn process_trigger_produces_scheduled_actions() {
    let mut engine = PolicyEngine::with_standard_policies();
    let trigger = Trigger::new(
        TriggerType::SanctionsListUpdate,
        json!({"affected_parties": ["self"]}),
    );

    let actions = engine.process_trigger(&trigger, "asset:mining-license-001", None);
    assert!(
        !actions.is_empty(),
        "sanctions update should produce actions"
    );

    for action in &actions {
        assert_eq!(action.asset_id, "asset:mining-license-001");
    }
}

// ---------------------------------------------------------------------------
// 9. Disabled policies are not matched
// ---------------------------------------------------------------------------

#[test]
fn disabled_policy_not_matched() {
    let mut engine = PolicyEngine::new();
    let policy =
        Policy::new("disabled", TriggerType::CheckpointDue, PolicyAction::Halt).with_enabled(false);
    engine.register_policy(policy);

    let trigger = Trigger::new(TriggerType::CheckpointDue, json!({}));
    let results = engine.evaluate(&trigger, Some("asset:test"), None);
    assert!(!results.iter().any(|r| r.matched));
}

// ---------------------------------------------------------------------------
// 10. All 20 trigger types are defined
// ---------------------------------------------------------------------------

#[test]
fn all_20_trigger_types_exist() {
    assert_eq!(TriggerType::all().len(), 20);
}

#[test]
fn all_trigger_types_have_string_representation() {
    for tt in TriggerType::all() {
        let s = tt.as_str();
        assert!(!s.is_empty(), "trigger type {tt:?} has empty string");
    }
}
