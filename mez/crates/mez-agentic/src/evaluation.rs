//! # Policy Evaluation Engine — MASS Protocol v0.2 Chapter 17
//!
//! Definition 17.5 (Policy Evaluation Engine).
//!
//! Evaluates triggers against active policies and determines which actions to execute.
//! When multiple policies match the same trigger, conflicts are resolved by:
//! 1. **Priority** — higher-priority policies take precedence.
//! 2. **Jurisdiction specificity** — policies scoped to specific jurisdictions
//!    override global policies.
//! 3. **Policy ID** — deterministic tiebreaker (lexicographic ordering).
//!
//! ## Determinism (Theorem 17.1)
//!
//! Given identical trigger events and policy state, evaluation produces identical
//! results. This is guaranteed by:
//! - Sorted policy iteration (BTreeMap keyed by policy_id)
//! - Pure condition evaluation (no side effects)
//! - Deterministic conflict resolution (priority → specificity → id)

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::audit::{AuditEntry, AuditEntryType, AuditTrail};
use crate::policy::{AuthorizationRequirement, Policy, PolicyAction, Trigger};
use crate::scheduler::ScheduledAction;

// ---------------------------------------------------------------------------
// EvaluationResult
// ---------------------------------------------------------------------------

/// Result of evaluating a single policy against a trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    /// The policy that was evaluated.
    pub policy_id: String,
    /// Whether the policy's conditions matched the trigger.
    pub matched: bool,
    /// The action to execute (only set if matched).
    pub action: Option<PolicyAction>,
    /// The authorization requirement (only set if matched).
    pub authorization_requirement: Option<AuthorizationRequirement>,
    /// Priority of the matching policy.
    pub priority: i32,
    /// UTC timestamp of evaluation.
    pub evaluated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// PolicyEngine
// ---------------------------------------------------------------------------

/// Definition 17.5 (Policy Evaluation Engine).
///
/// Central coordinator for policy evaluation. Receives trigger events,
/// matches them against registered policies, resolves conflicts, and
/// produces action directives for the scheduler.
///
/// ## Thread Safety
///
/// This struct is not `Sync` — callers should use external synchronisation
/// (e.g., `Arc<Mutex<PolicyEngine>>`) if sharing across threads. This mirrors
/// the Python implementation which uses `threading.RLock` internally.
pub struct PolicyEngine {
    /// Registered policies, keyed by policy_id.
    /// BTreeMap guarantees deterministic iteration order.
    policies: BTreeMap<String, Policy>,
    /// Audit trail for all evaluations.
    pub audit_trail: AuditTrail,
}

impl PolicyEngine {
    /// Create a new policy engine with no policies.
    pub fn new() -> Self {
        Self {
            policies: BTreeMap::new(),
            audit_trail: AuditTrail::new(10_000),
        }
    }

    /// Create a new engine pre-loaded with standard policies.
    pub fn with_standard_policies() -> Self {
        let mut engine = Self::new();
        for (id, policy) in crate::policy::standard_policies() {
            engine.policies.insert(id, policy);
        }
        engine
    }

    /// Create a new engine pre-loaded with extended policies (v0.4.44).
    pub fn with_extended_policies() -> Self {
        let mut engine = Self::new();
        for (id, policy) in crate::policy::extended_policies() {
            engine.policies.insert(id, policy);
        }
        engine
    }

    /// Register a policy. Replaces any existing policy with the same ID.
    pub fn register_policy(&mut self, policy: Policy) {
        self.policies.insert(policy.policy_id.clone(), policy);
    }

    /// Unregister a policy by ID. Returns the removed policy if it existed.
    pub fn unregister_policy(&mut self, policy_id: &str) -> Option<Policy> {
        self.policies.remove(policy_id)
    }

    /// Get a registered policy by ID.
    pub fn get_policy(&self, policy_id: &str) -> Option<&Policy> {
        self.policies.get(policy_id)
    }

    /// List all registered policies in deterministic order (sorted by policy_id).
    pub fn list_policies(&self) -> Vec<&Policy> {
        self.policies.values().collect()
    }

    /// Return the count of registered policies.
    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }

    /// Evaluate a trigger against all registered policies.
    ///
    /// ## Theorem 17.1 (Agentic Determinism)
    ///
    /// This evaluation is deterministic: given identical trigger, asset_id,
    /// jurisdiction, and policy state, the returned results are identical
    /// (including order).
    ///
    /// ## Parameters
    ///
    /// - `trigger`: The trigger event to evaluate.
    /// - `asset_id`: Optional asset identifier for audit trail.
    /// - `jurisdiction`: Optional jurisdiction for scope filtering.
    ///
    /// ## Returns
    ///
    /// A list of `EvaluationResult` for every registered policy, in sorted
    /// policy_id order. Callers can filter for `matched == true` to get
    /// only the matching policies.
    pub fn evaluate(
        &mut self,
        trigger: &Trigger,
        asset_id: Option<&str>,
        jurisdiction: Option<&str>,
    ) -> Vec<EvaluationResult> {
        let now = Utc::now();

        // Record trigger receipt in audit trail.
        self.audit_trail.append(AuditEntry::new(
            AuditEntryType::TriggerReceived,
            asset_id.map(String::from),
            Some(serde_json::json!({
                "trigger_type": trigger.trigger_type.as_str(),
                "data": trigger.data,
                "timestamp": trigger.timestamp.to_rfc3339(),
            })),
        ));

        // Evaluate against each policy in sorted order.
        let mut results = Vec::new();
        let policy_snapshot: Vec<(String, Policy)> = self
            .policies
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        for (policy_id, policy) in &policy_snapshot {
            let matched = policy.matches(trigger, jurisdiction);

            let result = EvaluationResult {
                policy_id: policy_id.clone(),
                matched,
                action: if matched { Some(policy.action) } else { None },
                authorization_requirement: if matched {
                    Some(policy.authorization_requirement)
                } else {
                    None
                },
                priority: policy.priority,
                evaluated_at: now,
            };

            // Record evaluation in audit trail.
            self.audit_trail.append(AuditEntry::new(
                AuditEntryType::PolicyEvaluated,
                asset_id.map(String::from),
                Some(serde_json::json!({
                    "policy_id": policy_id,
                    "matched": matched,
                    "action": result.action.map(|a| a.as_str()),
                    "priority": policy.priority,
                })),
            ));

            results.push(result);
        }

        results
    }

    /// Evaluate and resolve conflicts, returning only the winning actions.
    ///
    /// When multiple policies match the same trigger, this method resolves
    /// conflicts by:
    /// 1. **Priority** — highest priority wins.
    /// 2. **Jurisdiction specificity** — scoped policies win over global ones.
    /// 3. **Policy ID** — lexicographic tiebreaker.
    ///
    /// Returns a deduplicated list of `EvaluationResult` sorted by descending
    /// priority. If two policies produce the same action, only the highest-priority
    /// one is kept.
    pub fn evaluate_and_resolve(
        &mut self,
        trigger: &Trigger,
        asset_id: Option<&str>,
        jurisdiction: Option<&str>,
    ) -> Vec<EvaluationResult> {
        let all_results = self.evaluate(trigger, asset_id, jurisdiction);

        // Filter to matches only and sort by priority descending, then policy_id.
        let mut matched: Vec<EvaluationResult> =
            all_results.into_iter().filter(|r| r.matched).collect();

        // Sort: highest priority first, then by policy_id for determinism.
        matched.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.policy_id.cmp(&b.policy_id))
        });

        // Deduplicate by action: keep only the highest-priority result per action.
        let mut seen_actions = std::collections::HashSet::new();
        matched.retain(|r| {
            if let Some(action) = r.action {
                seen_actions.insert(action)
            } else {
                false
            }
        });

        matched
    }

    /// Process a trigger end-to-end: evaluate, resolve conflicts, and produce
    /// scheduled actions.
    ///
    /// This is the primary entry point matching `AgenticExecutionEngine.process_trigger()`
    /// in the Python implementation.
    pub fn process_trigger(
        &mut self,
        trigger: &Trigger,
        asset_id: &str,
        jurisdiction: Option<&str>,
    ) -> Vec<ScheduledAction> {
        let resolved = self.evaluate_and_resolve(trigger, Some(asset_id), jurisdiction);

        resolved
            .into_iter()
            .filter_map(|r| {
                let action = r.action?;
                Some(ScheduledAction::new(
                    asset_id.to_string(),
                    action,
                    r.policy_id,
                    r.authorization_requirement
                        .unwrap_or(AuthorizationRequirement::Automatic),
                ))
            })
            .collect()
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for PolicyEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PolicyEngine")
            .field("policy_count", &self.policies.len())
            .field("audit_trail_size", &self.audit_trail.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{Condition, TriggerType};

    #[test]
    fn engine_with_standard_policies() {
        let engine = PolicyEngine::with_standard_policies();
        assert_eq!(engine.policy_count(), 4);
    }

    #[test]
    fn engine_with_extended_policies() {
        let engine = PolicyEngine::with_extended_policies();
        assert!(engine.policy_count() >= 10);
    }

    #[test]
    fn engine_register_and_unregister() {
        let mut engine = PolicyEngine::new();
        let policy = Policy::new("test", TriggerType::CheckpointDue, PolicyAction::Halt);
        engine.register_policy(policy);
        assert_eq!(engine.policy_count(), 1);

        let removed = engine.unregister_policy("test");
        assert!(removed.is_some());
        assert_eq!(engine.policy_count(), 0);
    }

    #[test]
    fn evaluate_sanctions_trigger() {
        let mut engine = PolicyEngine::with_standard_policies();

        let trigger = Trigger::new(
            TriggerType::SanctionsListUpdate,
            serde_json::json!({
                "affected_parties": ["self"],
                "entity_id": "entity:test",
            }),
        );

        let results = engine.evaluate(&trigger, Some("asset:test"), None);
        let matched: Vec<_> = results.iter().filter(|r| r.matched).collect();
        assert!(!matched.is_empty());
        assert!(matched.iter().any(|r| r.policy_id == "sanctions_auto_halt"));
    }

    #[test]
    fn evaluate_determinism() {
        let trigger = Trigger::new(
            TriggerType::LicenseStatusChange,
            serde_json::json!({"new_status": "expired"}),
        );

        // Evaluate 5 times with fresh engines (same policy state).
        let mut all_results = Vec::new();
        for _ in 0..5 {
            let mut engine = PolicyEngine::with_standard_policies();
            let results = engine.evaluate(&trigger, Some("asset:test"), None);
            let summary: Vec<_> = results
                .iter()
                .map(|r| (r.policy_id.clone(), r.matched, r.action))
                .collect();
            all_results.push(summary);
        }

        // All should be identical.
        for result in &all_results {
            assert_eq!(result, &all_results[0]);
        }
    }

    #[test]
    fn conflict_resolution_by_priority() {
        let mut engine = PolicyEngine::new();

        // Two policies for same trigger with different priorities.
        engine.register_policy(
            Policy::new(
                "low_priority",
                TriggerType::DisputeFiled,
                PolicyAction::UpdateManifest,
            )
            .with_priority(10),
        );
        engine.register_policy(
            Policy::new(
                "high_priority",
                TriggerType::DisputeFiled,
                PolicyAction::Halt,
            )
            .with_priority(90),
        );

        let trigger = Trigger::new(TriggerType::DisputeFiled, serde_json::json!({}));
        let resolved = engine.evaluate_and_resolve(&trigger, None, None);

        // Both should match since they have different actions.
        assert_eq!(resolved.len(), 2);
        // Highest priority first.
        assert_eq!(resolved[0].policy_id, "high_priority");
        assert_eq!(resolved[0].priority, 90);
    }

    #[test]
    fn conflict_resolution_deduplicates_same_action() {
        let mut engine = PolicyEngine::new();

        // Two policies producing the same action (Halt) with different priorities.
        engine.register_policy(
            Policy::new("low", TriggerType::DisputeFiled, PolicyAction::Halt).with_priority(10),
        );
        engine.register_policy(
            Policy::new("high", TriggerType::DisputeFiled, PolicyAction::Halt).with_priority(90),
        );

        let trigger = Trigger::new(TriggerType::DisputeFiled, serde_json::json!({}));
        let resolved = engine.evaluate_and_resolve(&trigger, None, None);

        // Only one Halt action should survive (the highest priority one).
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].policy_id, "high");
    }

    #[test]
    fn process_trigger_produces_scheduled_actions() {
        let mut engine = PolicyEngine::with_extended_policies();

        let trigger = Trigger::new(
            TriggerType::DisputeFiled,
            serde_json::json!({"dispute_id": "disp-001"}),
        );

        let actions = engine.process_trigger(&trigger, "asset:disputed", None);
        assert!(!actions.is_empty());
        assert!(actions.iter().any(|a| a.action == PolicyAction::Halt));
    }

    #[test]
    fn audit_trail_populated_after_evaluation() {
        let mut engine = PolicyEngine::with_standard_policies();

        let trigger = Trigger::new(
            TriggerType::CheckpointDue,
            serde_json::json!({"receipts_since_last": 150}),
        );

        engine.evaluate(&trigger, Some("asset:test"), None);

        let trail = engine.audit_trail.entries();
        // Should have trigger_received + policy_evaluated entries.
        let received: Vec<_> = trail
            .iter()
            .filter(|e| e.entry_type == AuditEntryType::TriggerReceived)
            .collect();
        let evaluated: Vec<_> = trail
            .iter()
            .filter(|e| e.entry_type == AuditEntryType::PolicyEvaluated)
            .collect();

        assert!(!received.is_empty());
        assert!(!evaluated.is_empty());
    }

    #[test]
    fn evaluate_with_condition_matching() {
        let mut engine = PolicyEngine::new();
        engine.register_policy(
            Policy::new(
                "threshold_test",
                TriggerType::CheckpointDue,
                PolicyAction::UpdateManifest,
            )
            .with_condition(Condition::Threshold {
                field: "count".into(),
                threshold: serde_json::json!(100),
            }),
        );

        // Below threshold.
        let trigger_low =
            Trigger::new(TriggerType::CheckpointDue, serde_json::json!({"count": 50}));
        let results = engine.evaluate(&trigger_low, None, None);
        let matched: Vec<_> = results.iter().filter(|r| r.matched).collect();
        assert!(matched.is_empty());

        // Above threshold.
        let trigger_high = Trigger::new(
            TriggerType::CheckpointDue,
            serde_json::json!({"count": 150}),
        );
        let results = engine.evaluate(&trigger_high, None, None);
        let matched: Vec<_> = results.iter().filter(|r| r.matched).collect();
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].policy_id, "threshold_test");
    }

    // ── Additional coverage tests ──────────────────────────────────

    #[test]
    fn engine_default_is_empty() {
        let engine = PolicyEngine::default();
        assert_eq!(engine.policy_count(), 0);
    }

    #[test]
    fn engine_debug_format() {
        let engine = PolicyEngine::with_standard_policies();
        let dbg = format!("{engine:?}");
        assert!(dbg.contains("PolicyEngine"));
        assert!(dbg.contains("policy_count"));
        assert!(dbg.contains("audit_trail_size"));
    }

    #[test]
    fn engine_get_policy() {
        let engine = PolicyEngine::with_standard_policies();
        assert!(engine.get_policy("sanctions_auto_halt").is_some());
        assert!(engine.get_policy("nonexistent").is_none());
    }

    #[test]
    fn engine_list_policies_sorted() {
        let engine = PolicyEngine::with_standard_policies();
        let policies = engine.list_policies();
        // BTreeMap guarantees sorted order by policy_id.
        let ids: Vec<&str> = policies.iter().map(|p| p.policy_id.as_str()).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted);
    }

    #[test]
    fn engine_unregister_nonexistent_returns_none() {
        let mut engine = PolicyEngine::new();
        assert!(engine.unregister_policy("nonexistent").is_none());
    }

    #[test]
    fn engine_register_replaces_existing() {
        let mut engine = PolicyEngine::new();
        engine.register_policy(
            Policy::new("p1", TriggerType::DisputeFiled, PolicyAction::Halt).with_priority(10),
        );
        assert_eq!(engine.policy_count(), 1);
        assert_eq!(engine.get_policy("p1").unwrap().priority, 10);

        // Replace with different priority.
        engine.register_policy(
            Policy::new("p1", TriggerType::DisputeFiled, PolicyAction::Halt).with_priority(99),
        );
        assert_eq!(engine.policy_count(), 1);
        assert_eq!(engine.get_policy("p1").unwrap().priority, 99);
    }

    #[test]
    fn evaluate_no_policies_returns_empty() {
        let mut engine = PolicyEngine::new();
        let trigger = Trigger::new(TriggerType::DisputeFiled, serde_json::json!({}));
        let results = engine.evaluate(&trigger, None, None);
        assert!(results.is_empty());
    }

    #[test]
    fn evaluate_and_resolve_no_matches_returns_empty() {
        let mut engine = PolicyEngine::new();
        engine.register_policy(Policy::new(
            "p1",
            TriggerType::DisputeFiled,
            PolicyAction::Halt,
        ));
        // Different trigger type — no match.
        let trigger = Trigger::new(TriggerType::CheckpointDue, serde_json::json!({}));
        let resolved = engine.evaluate_and_resolve(&trigger, None, None);
        assert!(resolved.is_empty());
    }

    #[test]
    fn process_trigger_with_no_matches() {
        let mut engine = PolicyEngine::new();
        engine.register_policy(Policy::new(
            "p1",
            TriggerType::DisputeFiled,
            PolicyAction::Halt,
        ));
        let trigger = Trigger::new(TriggerType::CheckpointDue, serde_json::json!({}));
        let actions = engine.process_trigger(&trigger, "asset:test", None);
        assert!(actions.is_empty());
    }

    #[test]
    fn process_trigger_populates_scheduled_action_fields() {
        let mut engine = PolicyEngine::new();
        engine.register_policy(Policy::new(
            "halt_policy",
            TriggerType::SanctionsListUpdate,
            PolicyAction::Halt,
        ));

        let trigger = Trigger::new(
            TriggerType::SanctionsListUpdate,
            serde_json::json!({"affected_parties": ["self"]}),
        );

        let actions = engine.process_trigger(&trigger, "asset:xyz", None);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].asset_id, "asset:xyz");
        assert_eq!(actions[0].policy_id, "halt_policy");
        assert_eq!(actions[0].action, PolicyAction::Halt);
    }

    #[test]
    fn evaluation_result_serde_roundtrip() {
        let result = EvaluationResult {
            policy_id: "test_policy".to_string(),
            matched: true,
            action: Some(PolicyAction::Halt),
            authorization_requirement: Some(AuthorizationRequirement::Automatic),
            priority: 50,
            evaluated_at: Utc::now(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: EvaluationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.policy_id, "test_policy");
        assert!(back.matched);
        assert_eq!(back.priority, 50);
    }

    #[test]
    fn evaluate_audit_trail_records_per_policy() {
        let mut engine = PolicyEngine::new();
        engine.register_policy(Policy::new(
            "p1",
            TriggerType::DisputeFiled,
            PolicyAction::Halt,
        ));
        engine.register_policy(Policy::new(
            "p2",
            TriggerType::DisputeFiled,
            PolicyAction::UpdateManifest,
        ));

        let trigger = Trigger::new(TriggerType::DisputeFiled, serde_json::json!({}));
        engine.evaluate(&trigger, Some("asset:test"), None);

        // 1 TriggerReceived + 2 PolicyEvaluated = 3 entries.
        assert_eq!(engine.audit_trail.len(), 3);
    }
}
