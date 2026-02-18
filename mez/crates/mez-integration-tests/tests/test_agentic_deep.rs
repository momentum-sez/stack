//! # Agentic Policy Engine — Deep Integration Tests
//!
//! Python counterpart: `tests/test_agentic_deep.py`
//!
//! Tests advanced policy engine behaviors:
//! - Evaluation determinism (Theorem 17.1)
//! - Jurisdiction-scoped policies
//! - Priority-based conflict resolution
//! - Audit trail records evaluations
//! - Condition evaluation (Equals, Threshold, Contains, And/Or)

use mez_agentic::evaluation::PolicyEngine;
use mez_agentic::policy::{Condition, Policy, PolicyAction, Trigger, TriggerType};
use mez_agentic::{AuditEntry, AuditEntryType, AuditTrail};
use serde_json::json;

// ---------------------------------------------------------------------------
// 1. Evaluation determinism (Theorem 17.1)
// ---------------------------------------------------------------------------

#[test]
fn evaluation_determinism() {
    let trigger = Trigger::new(
        TriggerType::SanctionsListUpdate,
        json!({"affected_parties": ["self"]}),
    );

    let mut engine1 = PolicyEngine::with_standard_policies();
    let mut engine2 = PolicyEngine::with_standard_policies();

    let r1 = engine1.evaluate(&trigger, Some("asset:test"), None);
    let r2 = engine2.evaluate(&trigger, Some("asset:test"), None);

    assert_eq!(
        r1.len(),
        r2.len(),
        "evaluation result count must be identical"
    );
    for (a, b) in r1.iter().zip(r2.iter()) {
        assert_eq!(a.policy_id, b.policy_id);
        assert_eq!(a.matched, b.matched);
        assert_eq!(a.action, b.action);
        assert_eq!(a.priority, b.priority);
    }
}

// ---------------------------------------------------------------------------
// 2. Jurisdiction-scoped policy
// ---------------------------------------------------------------------------

#[test]
fn jurisdiction_scoped_policy() {
    let mut engine = PolicyEngine::new();

    let pk_policy = Policy::new("pk-only", TriggerType::TaxYearEnd, PolicyAction::Halt)
        .with_jurisdiction_scope(vec!["PK-RSEZ".to_string()]);
    let global_policy = Policy::new("global", TriggerType::TaxYearEnd, PolicyAction::Resume);

    engine.register_policy(pk_policy);
    engine.register_policy(global_policy);

    // PK jurisdiction: both should match
    let trigger = Trigger::new(TriggerType::TaxYearEnd, json!({}));
    let pk_results = engine.evaluate(&trigger, Some("asset:test"), Some("PK-RSEZ"));
    let pk_matched: Vec<_> = pk_results.iter().filter(|r| r.matched).collect();
    assert_eq!(
        pk_matched.len(),
        2,
        "PK jurisdiction should match both policies"
    );

    // AE jurisdiction: only global should match
    let ae_results = engine.evaluate(&trigger, Some("asset:test"), Some("AE-DIFC"));
    let ae_matched: Vec<_> = ae_results.iter().filter(|r| r.matched).collect();
    assert_eq!(
        ae_matched.len(),
        1,
        "AE jurisdiction should match only global policy"
    );
    assert_eq!(ae_matched[0].policy_id, "global");
}

// ---------------------------------------------------------------------------
// 3. Priority-based conflict resolution
// ---------------------------------------------------------------------------

#[test]
fn priority_conflict_resolution() {
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
    assert_eq!(
        resolved[0].priority, 100,
        "highest priority should be first"
    );
    assert_eq!(resolved[0].action, Some(PolicyAction::Halt));
}

// ---------------------------------------------------------------------------
// 4. Audit trail records evaluations
// ---------------------------------------------------------------------------

#[test]
fn audit_trail_records_evaluations() {
    let mut engine = PolicyEngine::with_standard_policies();
    let initial_len = engine.audit_trail.len();

    let trigger = Trigger::new(
        TriggerType::SanctionsListUpdate,
        json!({"affected_parties": ["self"]}),
    );
    engine.evaluate(&trigger, Some("asset:test"), None);

    assert!(
        engine.audit_trail.len() > initial_len,
        "audit trail must grow after evaluation"
    );
}

#[test]
fn audit_trail_standalone_append_and_entries() {
    let mut trail = AuditTrail::new(1000);
    assert!(trail.is_empty());

    let entry = AuditEntry::new(
        AuditEntryType::TriggerReceived,
        Some("asset:001".to_string()),
        Some(json!({"trigger_type": "sanctions_list_update"})),
    );
    trail.append(entry);
    assert_eq!(trail.len(), 1);
    assert_eq!(
        trail.entries()[0].entry_type,
        AuditEntryType::TriggerReceived
    );
}

// ---------------------------------------------------------------------------
// 5. Condition evaluation — Equals
// ---------------------------------------------------------------------------

#[test]
fn condition_evaluation_equals() {
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

    // Matching
    let trigger_match = Trigger::new(
        TriggerType::LicenseStatusChange,
        json!({"status": "expired"}),
    );
    let results = engine.evaluate(&trigger_match, Some("asset:test"), None);
    assert!(
        results.iter().any(|r| r.matched),
        "Equals condition should match"
    );

    // Non-matching
    let trigger_no_match = Trigger::new(
        TriggerType::LicenseStatusChange,
        json!({"status": "active"}),
    );
    let results = engine.evaluate(&trigger_no_match, Some("asset:test"), None);
    assert!(
        !results.iter().any(|r| r.matched),
        "Equals condition should not match"
    );
}

// ---------------------------------------------------------------------------
// 6. Condition evaluation — Threshold
// ---------------------------------------------------------------------------

#[test]
fn condition_evaluation_threshold() {
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
    assert!(
        results.iter().any(|r| r.matched),
        "value above threshold should match"
    );

    // Below threshold
    let trigger_low = Trigger::new(TriggerType::CorridorStateChange, json!({"risk_score": 50}));
    let results = engine.evaluate(&trigger_low, Some("asset:test"), None);
    assert!(
        !results.iter().any(|r| r.matched),
        "value below threshold should not match"
    );
}

// ---------------------------------------------------------------------------
// 7. Compound condition (And)
// ---------------------------------------------------------------------------

#[test]
fn condition_evaluation_and_composition() {
    let mut engine = PolicyEngine::new();
    let policy = Policy::new(
        "compound-and",
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

    // Both met
    let trigger = Trigger::new(
        TriggerType::CorridorStateChange,
        json!({"type": "fork_detected", "severity": 8}),
    );
    let results = engine.evaluate(&trigger, Some("asset:test"), None);
    assert!(results.iter().any(|r| r.matched));

    // Only one met
    let trigger = Trigger::new(
        TriggerType::CorridorStateChange,
        json!({"type": "fork_detected", "severity": 3}),
    );
    let results = engine.evaluate(&trigger, Some("asset:test"), None);
    assert!(!results.iter().any(|r| r.matched));
}
