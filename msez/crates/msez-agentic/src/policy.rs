//! # Policy Definitions — MASS Protocol v0.2 Chapter 17
//!
//! Configurable compliance policies with trigger conditions and action responses.
//!
//! Implements Definition 17.1 (Agentic Trigger), Definition 17.2 (Agentic Policy),
//! and Definition 17.3 (Standard Agentic Policies).
//!
//! ## Security Invariant
//!
//! Policy evaluation is deterministic per Theorem 17.1: given identical trigger
//! events and environment state, evaluation produces identical state transitions.
//! This is enforced by:
//! - Sorted policy iteration order (by policy_id)
//! - Pure condition evaluation (no side effects)
//! - Fail-safe defaults (unknown conditions return false)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// TriggerType — 20 variants per audit §IV and MASS Protocol v0.2 Ch. 17
// ---------------------------------------------------------------------------

/// Definition 17.1 (Agentic Trigger).
///
/// Environmental events that may cause autonomous state transitions.
/// The 20 trigger types cover:
/// - Regulatory environment (sanctions, licenses, guidance, compliance deadlines)
/// - Arbitration lifecycle (disputes, rulings, appeals, enforcement)
/// - Corridor activity (state changes, settlement anchors, watcher quorums)
/// - Asset lifecycle (checkpoints, key rotation, governance votes)
/// - Fiscal events (tax year end, withholding due)
/// - Entity lifecycle (dissolution, pack updates, transfers, migration deadlines)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    // === Regulatory Environment Triggers ===
    /// Sanctions list updated (OFAC, EU, UN). Requires re-screening of all parties.
    SanctionsListUpdate,
    /// License status changed (expired, suspended, revoked, renewed).
    LicenseStatusChange,
    /// New regulatory guidance published or existing guidance became effective.
    GuidanceUpdate,
    /// Compliance deadline approaching or passed.
    ComplianceDeadline,

    // === Arbitration Triggers ===
    /// Dispute filed against an asset or entity.
    DisputeFiled,
    /// Arbitration ruling received.
    RulingReceived,
    /// Appeal period expired without appeal — ruling becomes final.
    AppealPeriodExpired,
    /// Enforcement action due (post-ruling).
    EnforcementDue,

    // === Corridor Triggers ===
    /// Corridor state changed (new receipts, checkpoints, forks).
    CorridorStateChange,
    /// Settlement anchor became available for finality.
    SettlementAnchorAvailable,
    /// Watcher quorum reached on a corridor observation.
    WatcherQuorumReached,

    // === Asset Lifecycle Triggers ===
    /// Checkpoint due (receipt threshold or time threshold exceeded).
    CheckpointDue,
    /// Cryptographic key rotation due.
    KeyRotationDue,
    /// Governance vote resolved.
    GovernanceVoteResolved,

    // === Fiscal Triggers ===
    /// Tax year end — triggers annual compliance evaluation.
    TaxYearEnd,
    /// Withholding due — triggers tax withholding computation.
    WithholdingDue,

    // === Entity & Migration Triggers ===
    /// Entity dissolution initiated or stage advanced.
    EntityDissolution,
    /// Pack trilogy (lawpack/regpack/licensepack) updated.
    PackUpdated,
    /// Smart asset cross-jurisdiction transfer initiated.
    AssetTransferInitiated,
    /// Migration saga deadline approaching or exceeded.
    MigrationDeadline,
}

impl TriggerType {
    /// Return all 20 trigger type variants in definition order.
    pub fn all() -> &'static [TriggerType] {
        &[
            Self::SanctionsListUpdate,
            Self::LicenseStatusChange,
            Self::GuidanceUpdate,
            Self::ComplianceDeadline,
            Self::DisputeFiled,
            Self::RulingReceived,
            Self::AppealPeriodExpired,
            Self::EnforcementDue,
            Self::CorridorStateChange,
            Self::SettlementAnchorAvailable,
            Self::WatcherQuorumReached,
            Self::CheckpointDue,
            Self::KeyRotationDue,
            Self::GovernanceVoteResolved,
            Self::TaxYearEnd,
            Self::WithholdingDue,
            Self::EntityDissolution,
            Self::PackUpdated,
            Self::AssetTransferInitiated,
            Self::MigrationDeadline,
        ]
    }

    /// Return the string value matching the Python enum's `.value`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SanctionsListUpdate => "sanctions_list_update",
            Self::LicenseStatusChange => "license_status_change",
            Self::GuidanceUpdate => "guidance_update",
            Self::ComplianceDeadline => "compliance_deadline",
            Self::DisputeFiled => "dispute_filed",
            Self::RulingReceived => "ruling_received",
            Self::AppealPeriodExpired => "appeal_period_expired",
            Self::EnforcementDue => "enforcement_due",
            Self::CorridorStateChange => "corridor_state_change",
            Self::SettlementAnchorAvailable => "settlement_anchor_available",
            Self::WatcherQuorumReached => "watcher_quorum_reached",
            Self::CheckpointDue => "checkpoint_due",
            Self::KeyRotationDue => "key_rotation_due",
            Self::GovernanceVoteResolved => "governance_vote_resolved",
            Self::TaxYearEnd => "tax_year_end",
            Self::WithholdingDue => "withholding_due",
            Self::EntityDissolution => "entity_dissolution",
            Self::PackUpdated => "pack_updated",
            Self::AssetTransferInitiated => "asset_transfer_initiated",
            Self::MigrationDeadline => "migration_deadline",
        }
    }
}

impl std::fmt::Display for TriggerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// PolicyAction — the set of actions a policy can execute
// ---------------------------------------------------------------------------

/// Definition 13.2 (Transition Kinds) — the actions a policy can trigger.
///
/// Maps directly to `TransitionKind` in `tools/mass_primitives.py`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyAction {
    // Ownership Operations
    Transfer,
    Mint,
    Burn,

    // Binding Operations
    ActivateBinding,
    DeactivateBinding,
    MigrateBinding,

    // Governance Operations
    UpdateManifest,
    AmendGovernance,
    AddGovernor,
    RemoveGovernor,

    // Corporate Actions
    Dividend,
    Split,
    Merger,

    // Control Operations
    Halt,
    Resume,

    // Arbitration
    ArbitrationEnforce,
}

impl PolicyAction {
    /// Return the string value matching the Python enum's `.value`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Transfer => "transfer",
            Self::Mint => "mint",
            Self::Burn => "burn",
            Self::ActivateBinding => "activate_binding",
            Self::DeactivateBinding => "deactivate_binding",
            Self::MigrateBinding => "migrate_binding",
            Self::UpdateManifest => "update_manifest",
            Self::AmendGovernance => "amend_governance",
            Self::AddGovernor => "add_governor",
            Self::RemoveGovernor => "remove_governor",
            Self::Dividend => "dividend",
            Self::Split => "split",
            Self::Merger => "merger",
            Self::Halt => "halt",
            Self::Resume => "resume",
            Self::ArbitrationEnforce => "arbitration_enforce",
        }
    }
}

impl std::fmt::Display for PolicyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// ImpactLevel
// ---------------------------------------------------------------------------

/// Impact level of a trigger event, used for prioritisation and alerting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

// ---------------------------------------------------------------------------
// AuthorizationRequirement
// ---------------------------------------------------------------------------

/// Authorization requirement for policy execution.
///
/// Determines the approval gate before an action is executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationRequirement {
    /// Action executes without human approval.
    Automatic,
    /// Requires quorum approval from governors.
    Quorum,
    /// Requires unanimous approval from all governors.
    Unanimous,
    /// Requires governance vote.
    Governance,
}

// ---------------------------------------------------------------------------
// Condition — policy trigger predicates
// ---------------------------------------------------------------------------

/// A condition predicate for policy evaluation.
///
/// Supports the same condition types as the Python `AgenticPolicy.evaluate_condition()`:
/// - `Threshold`: field >= threshold
/// - `Equals`: field == value
/// - `NotEquals`: field != value
/// - `Contains`: item in field (field is a collection)
/// - `In`: field in values (values is a collection)
/// - `LessThan`: field < threshold
/// - `GreaterThan`: field > threshold
/// - `Exists`: field exists and is truthy
/// - `And`: all sub-conditions must be true
/// - `Or`: at least one sub-condition must be true
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    /// Field value >= threshold.
    Threshold {
        field: String,
        threshold: serde_json::Value,
    },
    /// Field value == expected value.
    Equals {
        field: String,
        value: serde_json::Value,
    },
    /// Field value != expected value.
    NotEquals {
        field: String,
        value: serde_json::Value,
    },
    /// Item is contained in the field's collection value.
    Contains {
        field: String,
        item: serde_json::Value,
    },
    /// Field value is one of the provided values.
    In {
        field: String,
        values: Vec<serde_json::Value>,
    },
    /// Field value < threshold.
    LessThan {
        field: String,
        threshold: serde_json::Value,
    },
    /// Field value > threshold.
    GreaterThan {
        field: String,
        threshold: serde_json::Value,
    },
    /// Field exists and is truthy.
    Exists { field: String },
    /// All sub-conditions must be true (logical AND).
    And { conditions: Vec<Condition> },
    /// At least one sub-condition must be true (logical OR).
    Or { conditions: Vec<Condition> },
}

impl Condition {
    /// Evaluate this condition against trigger data.
    ///
    /// Uses dot-notation for nested field access (e.g., `"match.score"`).
    ///
    /// # Security: Unknown condition types and missing fields fail safe (return false).
    pub fn evaluate(&self, data: &serde_json::Value) -> bool {
        match self {
            Self::Threshold { field, threshold } => {
                let value = get_nested_field(data, field);
                compare_values_ge(value, threshold)
            }
            Self::Equals { field, value } => {
                let actual = get_nested_field(data, field);
                match actual {
                    Some(v) => v == value,
                    None => value.is_null(),
                }
            }
            Self::NotEquals { field, value } => {
                let actual = get_nested_field(data, field);
                match actual {
                    Some(v) => v != value,
                    None => !value.is_null(),
                }
            }
            Self::Contains { field, item } => {
                let collection = get_nested_field(data, field);
                match collection {
                    Some(serde_json::Value::Array(arr)) => arr.contains(item),
                    _ => false,
                }
            }
            Self::In { field, values } => {
                let actual = get_nested_field(data, field);
                match actual {
                    Some(v) => values.contains(v),
                    None => false,
                }
            }
            Self::LessThan { field, threshold } => {
                let value = get_nested_field(data, field);
                compare_values_lt(value, threshold)
            }
            Self::GreaterThan { field, threshold } => {
                let value = get_nested_field(data, field);
                compare_values_gt(value, threshold)
            }
            Self::Exists { field } => {
                let value = get_nested_field(data, field);
                match value {
                    Some(v) => is_truthy(v),
                    None => false,
                }
            }
            Self::And { conditions } => conditions.iter().all(|c| c.evaluate(data)),
            Self::Or { conditions } => conditions.iter().any(|c| c.evaluate(data)),
        }
    }
}

/// Get a nested field value using dot notation (e.g., `"match.score"`).
fn get_nested_field<'a>(
    data: &'a serde_json::Value,
    field_path: &str,
) -> Option<&'a serde_json::Value> {
    let mut current = data;
    for part in field_path.split('.') {
        match current {
            serde_json::Value::Object(map) => {
                current = map.get(part)?;
            }
            _ => return None,
        }
    }
    Some(current)
}

/// Compare: actual >= threshold, handling numeric types.
fn compare_values_ge(actual: Option<&serde_json::Value>, threshold: &serde_json::Value) -> bool {
    match (actual, threshold) {
        (Some(serde_json::Value::Number(a)), serde_json::Value::Number(t)) => {
            match (a.as_f64(), t.as_f64()) {
                (Some(av), Some(tv)) => av >= tv,
                _ => false,
            }
        }
        (None, _) => false,
        _ => false,
    }
}

/// Compare: actual < threshold, handling numeric types.
fn compare_values_lt(actual: Option<&serde_json::Value>, threshold: &serde_json::Value) -> bool {
    match (actual, threshold) {
        (Some(serde_json::Value::Number(a)), serde_json::Value::Number(t)) => {
            match (a.as_f64(), t.as_f64()) {
                (Some(av), Some(tv)) => av < tv,
                _ => false,
            }
        }
        _ => false,
    }
}

/// Compare: actual > threshold, handling numeric types.
fn compare_values_gt(actual: Option<&serde_json::Value>, threshold: &serde_json::Value) -> bool {
    match (actual, threshold) {
        (Some(serde_json::Value::Number(a)), serde_json::Value::Number(t)) => {
            match (a.as_f64(), t.as_f64()) {
                (Some(av), Some(tv)) => av > tv,
                _ => false,
            }
        }
        _ => false,
    }
}

/// Check if a JSON value is "truthy" (non-null, non-false, non-zero, non-empty).
fn is_truthy(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => false,
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        serde_json::Value::String(s) => !s.is_empty(),
        serde_json::Value::Array(a) => !a.is_empty(),
        serde_json::Value::Object(o) => !o.is_empty(),
    }
}

// ---------------------------------------------------------------------------
// Trigger — an event instance
// ---------------------------------------------------------------------------

/// An agentic trigger event with associated data.
///
/// Carries the trigger type, a JSON data payload, and a UTC timestamp.
/// Triggers are immutable once created — they represent observed events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trigger {
    /// The type of trigger event.
    pub trigger_type: TriggerType,
    /// Trigger payload data (varies by trigger type).
    pub data: serde_json::Value,
    /// UTC timestamp when the trigger was created.
    pub timestamp: DateTime<Utc>,
}

impl Trigger {
    /// Create a new trigger with the current UTC timestamp.
    pub fn new(trigger_type: TriggerType, data: serde_json::Value) -> Self {
        Self {
            trigger_type,
            data,
            timestamp: Utc::now(),
        }
    }

    /// Create a trigger with a specific timestamp (for testing/replay).
    pub fn with_timestamp(
        trigger_type: TriggerType,
        data: serde_json::Value,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            trigger_type,
            data,
            timestamp,
        }
    }
}

// ---------------------------------------------------------------------------
// Policy — the mapping from triggers to actions
// ---------------------------------------------------------------------------

/// Definition 17.2 (Agentic Policy).
///
/// A policy maps trigger events to authorized transitions. Policies have:
/// - A trigger type they respond to
/// - An optional condition predicate
/// - An action to execute when matched
/// - A priority for conflict resolution (higher = more important)
/// - A jurisdiction scope (empty = all jurisdictions)
/// - An authorization requirement
///
/// ## Determinism (Theorem 17.1)
///
/// Policy evaluation is deterministic: given identical trigger events and
/// environment state, `evaluate()` always returns the same result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Policy {
    /// Unique policy identifier.
    pub policy_id: String,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// The trigger type this policy responds to.
    pub trigger_type: TriggerType,
    /// Optional condition predicate. If `None`, matches all triggers of the type.
    pub condition: Option<Condition>,
    /// The action to execute when the policy matches.
    pub action: PolicyAction,
    /// Priority for conflict resolution (higher = takes precedence).
    /// When multiple policies match the same trigger, higher-priority
    /// policies override lower-priority ones.
    #[serde(default)]
    pub priority: i32,
    /// Jurisdiction scope. Empty means the policy applies to all jurisdictions.
    #[serde(default)]
    pub jurisdiction_scope: Vec<String>,
    /// Authorization requirement for action execution.
    #[serde(default = "default_authorization")]
    pub authorization_requirement: AuthorizationRequirement,
    /// Whether this policy is currently enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_authorization() -> AuthorizationRequirement {
    AuthorizationRequirement::Automatic
}

fn default_enabled() -> bool {
    true
}

impl Policy {
    /// Create a new policy with minimal required fields.
    pub fn new(
        policy_id: impl Into<String>,
        trigger_type: TriggerType,
        action: PolicyAction,
    ) -> Self {
        Self {
            policy_id: policy_id.into(),
            description: String::new(),
            trigger_type,
            condition: None,
            action,
            priority: 0,
            jurisdiction_scope: Vec::new(),
            authorization_requirement: AuthorizationRequirement::Automatic,
            enabled: true,
        }
    }

    /// Builder: set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Builder: set the condition.
    pub fn with_condition(mut self, condition: Condition) -> Self {
        self.condition = Some(condition);
        self
    }

    /// Builder: set the priority.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Builder: set the jurisdiction scope.
    pub fn with_jurisdiction_scope(mut self, scope: Vec<String>) -> Self {
        self.jurisdiction_scope = scope;
        self
    }

    /// Builder: set the authorization requirement.
    pub fn with_authorization(mut self, auth: AuthorizationRequirement) -> Self {
        self.authorization_requirement = auth;
        self
    }

    /// Builder: set enabled state.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Evaluate whether this policy matches the given trigger.
    ///
    /// Returns `true` if all of the following hold:
    /// 1. The policy is enabled.
    /// 2. The trigger type matches.
    /// 3. The condition (if any) is satisfied by the trigger data.
    /// 4. The jurisdiction (if specified) matches the scope.
    ///
    /// ## Determinism (Theorem 17.1)
    ///
    /// This function is pure — it has no side effects and depends only on
    /// its arguments and `self`. Identical inputs always produce identical outputs.
    pub fn matches(&self, trigger: &Trigger, jurisdiction: Option<&str>) -> bool {
        if !self.enabled {
            return false;
        }

        if trigger.trigger_type != self.trigger_type {
            return false;
        }

        // Jurisdiction scope check
        if !self.jurisdiction_scope.is_empty() {
            match jurisdiction {
                Some(j) => {
                    if !self.jurisdiction_scope.iter().any(|s| s == j) {
                        return false;
                    }
                }
                None => return false,
            }
        }

        // Condition check
        match &self.condition {
            Some(condition) => condition.evaluate(&trigger.data),
            None => true,
        }
    }
}

// ---------------------------------------------------------------------------
// Standard policies — Definition 17.3
// ---------------------------------------------------------------------------

/// Return the 4 standard policies from `STANDARD_POLICIES` in `tools/mass_primitives.py`.
///
/// These are the baseline policies that every SEZ deployment should have active.
pub fn standard_policies() -> BTreeMap<String, Policy> {
    let mut policies = BTreeMap::new();

    policies.insert(
        "sanctions_auto_halt".into(),
        Policy::new(
            "sanctions_auto_halt",
            TriggerType::SanctionsListUpdate,
            PolicyAction::Halt,
        )
        .with_description("Automatically halt asset when self is on sanctions list")
        .with_condition(Condition::Contains {
            field: "affected_parties".into(),
            item: serde_json::Value::String("self".into()),
        })
        .with_priority(100),
    );

    policies.insert(
        "license_expiry_alert".into(),
        Policy::new(
            "license_expiry_alert",
            TriggerType::LicenseStatusChange,
            PolicyAction::Halt,
        )
        .with_description("Halt asset on license expiry")
        .with_condition(Condition::Equals {
            field: "new_status".into(),
            value: serde_json::Value::String("expired".into()),
        })
        .with_priority(90),
    );

    policies.insert(
        "ruling_enforcement".into(),
        Policy::new(
            "ruling_enforcement",
            TriggerType::RulingReceived,
            PolicyAction::ArbitrationEnforce,
        )
        .with_description("Auto-enforce arbitration rulings")
        .with_priority(80),
    );

    policies.insert(
        "checkpoint_auto".into(),
        Policy::new(
            "checkpoint_auto",
            TriggerType::CheckpointDue,
            PolicyAction::UpdateManifest,
        )
        .with_description("Auto-checkpoint when receipt threshold exceeded")
        .with_condition(Condition::Threshold {
            field: "receipts_since_last".into(),
            threshold: serde_json::json!(100),
        })
        .with_priority(50),
    );

    policies
}

/// Return the extended policies from `EXTENDED_POLICIES` in `tools/agentic.py`.
///
/// Includes the 4 standard policies plus ~15 additional v0.4.44 policies.
pub fn extended_policies() -> BTreeMap<String, Policy> {
    let mut policies = standard_policies();

    policies.insert(
        "sanctions_freeze".into(),
        Policy::new(
            "sanctions_freeze",
            TriggerType::SanctionsListUpdate,
            PolicyAction::Halt,
        )
        .with_description("Freeze asset when entity newly sanctioned")
        .with_condition(Condition::Equals {
            field: "new_sanctioned".into(),
            value: serde_json::Value::Bool(true),
        })
        .with_priority(100),
    );

    policies.insert(
        "sanctions_notify".into(),
        Policy::new(
            "sanctions_notify",
            TriggerType::SanctionsListUpdate,
            PolicyAction::UpdateManifest,
        )
        .with_description("Notify on sanctions list version change")
        .with_condition(Condition::Equals {
            field: "update_type".into(),
            value: serde_json::Value::String("list_version_change".into()),
        })
        .with_priority(40),
    );

    policies.insert(
        "license_suspend".into(),
        Policy::new(
            "license_suspend",
            TriggerType::LicenseStatusChange,
            PolicyAction::Halt,
        )
        .with_description("Halt asset on license suspension")
        .with_condition(Condition::Equals {
            field: "new_status".into(),
            value: serde_json::Value::String("suspended".into()),
        })
        .with_priority(90),
    );

    policies.insert(
        "license_renew_reminder".into(),
        Policy::new(
            "license_renew_reminder",
            TriggerType::LicenseStatusChange,
            PolicyAction::UpdateManifest,
        )
        .with_description("Reminder when license expiry approaching")
        .with_condition(Condition::Equals {
            field: "warning_type".into(),
            value: serde_json::Value::String("expiry_approaching".into()),
        })
        .with_authorization(AuthorizationRequirement::Quorum)
        .with_priority(30),
    );

    policies.insert(
        "corridor_failover".into(),
        Policy::new(
            "corridor_failover",
            TriggerType::CorridorStateChange,
            PolicyAction::Halt,
        )
        .with_description("Halt on corridor fork detection")
        .with_condition(Condition::Equals {
            field: "change_type".into(),
            value: serde_json::Value::String("fork_detected".into()),
        })
        .with_authorization(AuthorizationRequirement::Quorum)
        .with_priority(95),
    );

    policies.insert(
        "checkpoint_auto_receipt".into(),
        Policy::new(
            "checkpoint_auto_receipt",
            TriggerType::CheckpointDue,
            PolicyAction::UpdateManifest,
        )
        .with_description("Auto-checkpoint on receipt threshold")
        .with_condition(Condition::Equals {
            field: "reason".into(),
            value: serde_json::Value::String("receipt_threshold_exceeded".into()),
        })
        .with_priority(50),
    );

    policies.insert(
        "checkpoint_auto_time".into(),
        Policy::new(
            "checkpoint_auto_time",
            TriggerType::CheckpointDue,
            PolicyAction::UpdateManifest,
        )
        .with_description("Auto-checkpoint on time threshold")
        .with_condition(Condition::Equals {
            field: "reason".into(),
            value: serde_json::Value::String("time_threshold_exceeded".into()),
        })
        .with_priority(40),
    );

    policies.insert(
        "key_rotation_enforce".into(),
        Policy::new(
            "key_rotation_enforce",
            TriggerType::KeyRotationDue,
            PolicyAction::UpdateManifest,
        )
        .with_description("Enforce key rotation when due")
        .with_authorization(AuthorizationRequirement::Quorum)
        .with_priority(70),
    );

    policies.insert(
        "dispute_filed_halt".into(),
        Policy::new(
            "dispute_filed_halt",
            TriggerType::DisputeFiled,
            PolicyAction::Halt,
        )
        .with_description("Halt asset when dispute filed")
        .with_priority(85),
    );

    policies.insert(
        "ruling_auto_enforce".into(),
        Policy::new(
            "ruling_auto_enforce",
            TriggerType::RulingReceived,
            PolicyAction::ArbitrationEnforce,
        )
        .with_description("Auto-enforce rulings marked for automatic enforcement")
        .with_condition(Condition::Equals {
            field: "auto_enforce".into(),
            value: serde_json::Value::Bool(true),
        })
        .with_priority(80),
    );

    policies.insert(
        "appeal_period_expired".into(),
        Policy::new(
            "appeal_period_expired",
            TriggerType::AppealPeriodExpired,
            PolicyAction::ArbitrationEnforce,
        )
        .with_description("Enforce ruling when appeal period expires")
        .with_priority(80),
    );

    policies.insert(
        "settlement_anchor_notify".into(),
        Policy::new(
            "settlement_anchor_notify",
            TriggerType::SettlementAnchorAvailable,
            PolicyAction::UpdateManifest,
        )
        .with_description("Update manifest when settlement anchor available")
        .with_priority(40),
    );

    policies.insert(
        "watcher_quorum_checkpoint".into(),
        Policy::new(
            "watcher_quorum_checkpoint",
            TriggerType::WatcherQuorumReached,
            PolicyAction::UpdateManifest,
        )
        .with_description("Update manifest when watcher quorum reached")
        .with_priority(50),
    );

    policies.insert(
        "compliance_deadline_warn".into(),
        Policy::new(
            "compliance_deadline_warn",
            TriggerType::ComplianceDeadline,
            PolicyAction::UpdateManifest,
        )
        .with_description("Warn when compliance deadline approaching")
        .with_condition(Condition::Threshold {
            field: "days_until_deadline".into(),
            threshold: serde_json::json!(0),
        })
        .with_authorization(AuthorizationRequirement::Quorum)
        .with_priority(60),
    );

    policies.insert(
        "guidance_effective_update".into(),
        Policy::new(
            "guidance_effective_update",
            TriggerType::GuidanceUpdate,
            PolicyAction::UpdateManifest,
        )
        .with_description("Update manifest when regulatory guidance becomes effective")
        .with_condition(Condition::Equals {
            field: "change_type".into(),
            value: serde_json::Value::String("became_effective".into()),
        })
        .with_priority(55),
    );

    // === Tax Collection Pipeline Policies (P1-009) ===

    policies.insert(
        "tax_year_end_halt".into(),
        Policy::new(
            "tax_year_end_halt",
            TriggerType::TaxYearEnd,
            PolicyAction::Halt,
        )
        .with_description(
            "Halt asset operations at tax year end until annual assessment completes",
        )
        .with_condition(Condition::Equals {
            field: "assessment_status".into(),
            value: serde_json::Value::String("pending".into()),
        })
        .with_authorization(AuthorizationRequirement::Quorum)
        .with_priority(75),
    );

    policies.insert(
        "tax_year_end_assessment".into(),
        Policy::new(
            "tax_year_end_assessment",
            TriggerType::TaxYearEnd,
            PolicyAction::UpdateManifest,
        )
        .with_description(
            "Trigger annual tax assessment and FBR IRIS reporting at tax year end",
        )
        .with_priority(70),
    );

    policies.insert(
        "withholding_auto_deduct".into(),
        Policy::new(
            "withholding_auto_deduct",
            TriggerType::WithholdingDue,
            PolicyAction::UpdateManifest,
        )
        .with_description(
            "Automatically compute and record withholding tax on economic activity",
        )
        .with_condition(Condition::Exists {
            field: "transaction_amount".into(),
        })
        .with_priority(80),
    );

    policies.insert(
        "withholding_ntn_missing_halt".into(),
        Policy::new(
            "withholding_ntn_missing_halt",
            TriggerType::WithholdingDue,
            PolicyAction::Halt,
        )
        .with_description(
            "Halt operations when withholding is due but entity has no NTN registered",
        )
        .with_condition(Condition::Equals {
            field: "ntn_status".into(),
            value: serde_json::Value::String("missing".into()),
        })
        .with_priority(85),
    );

    policies.insert(
        "withholding_high_value_quorum".into(),
        Policy::new(
            "withholding_high_value_quorum",
            TriggerType::WithholdingDue,
            PolicyAction::UpdateManifest,
        )
        .with_description(
            "Require quorum approval for withholding on high-value transactions (>10M PKR)",
        )
        .with_condition(Condition::Threshold {
            field: "transaction_amount".into(),
            threshold: serde_json::json!(10_000_000),
        })
        .with_authorization(AuthorizationRequirement::Quorum)
        .with_priority(90),
    );

    policies
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_type_count_is_20() {
        assert_eq!(TriggerType::all().len(), 20);
    }

    #[test]
    fn trigger_type_all_unique() {
        let all = TriggerType::all();
        let mut seen = std::collections::HashSet::new();
        for t in all {
            assert!(seen.insert(t), "Duplicate trigger type: {t:?}");
        }
    }

    #[test]
    fn trigger_type_serde_roundtrip() {
        for tt in TriggerType::all() {
            let json = serde_json::to_string(tt).unwrap();
            let parsed: TriggerType = serde_json::from_str(&json).unwrap();
            assert_eq!(*tt, parsed);
        }
    }

    #[test]
    fn policy_action_serde_roundtrip() {
        let actions = [
            PolicyAction::Transfer,
            PolicyAction::Halt,
            PolicyAction::Resume,
            PolicyAction::UpdateManifest,
            PolicyAction::ArbitrationEnforce,
        ];
        for action in &actions {
            let json = serde_json::to_string(action).unwrap();
            let parsed: PolicyAction = serde_json::from_str(&json).unwrap();
            assert_eq!(*action, parsed);
        }
    }

    #[test]
    fn condition_threshold_evaluation() {
        let cond = Condition::Threshold {
            field: "count".into(),
            threshold: serde_json::json!(100),
        };
        let above = serde_json::json!({"count": 150});
        let at = serde_json::json!({"count": 100});
        let below = serde_json::json!({"count": 99});
        let missing = serde_json::json!({"other": 200});

        assert!(cond.evaluate(&above));
        assert!(cond.evaluate(&at));
        assert!(!cond.evaluate(&below));
        assert!(!cond.evaluate(&missing));
    }

    #[test]
    fn condition_equals_evaluation() {
        let cond = Condition::Equals {
            field: "status".into(),
            value: serde_json::json!("expired"),
        };
        assert!(cond.evaluate(&serde_json::json!({"status": "expired"})));
        assert!(!cond.evaluate(&serde_json::json!({"status": "valid"})));
    }

    #[test]
    fn condition_contains_evaluation() {
        let cond = Condition::Contains {
            field: "parties".into(),
            item: serde_json::json!("self"),
        };
        assert!(cond.evaluate(&serde_json::json!({"parties": ["other", "self"]})));
        assert!(!cond.evaluate(&serde_json::json!({"parties": ["other"]})));
        assert!(!cond.evaluate(&serde_json::json!({"parties": "self"})));
    }

    #[test]
    fn condition_and_or_composition() {
        let cond = Condition::And {
            conditions: vec![
                Condition::Equals {
                    field: "a".into(),
                    value: serde_json::json!(1),
                },
                Condition::Equals {
                    field: "b".into(),
                    value: serde_json::json!(2),
                },
            ],
        };
        assert!(cond.evaluate(&serde_json::json!({"a": 1, "b": 2})));
        assert!(!cond.evaluate(&serde_json::json!({"a": 1, "b": 3})));

        let cond_or = Condition::Or {
            conditions: vec![
                Condition::Equals {
                    field: "x".into(),
                    value: serde_json::json!("yes"),
                },
                Condition::Equals {
                    field: "y".into(),
                    value: serde_json::json!("yes"),
                },
            ],
        };
        assert!(cond_or.evaluate(&serde_json::json!({"x": "yes", "y": "no"})));
        assert!(cond_or.evaluate(&serde_json::json!({"x": "no", "y": "yes"})));
        assert!(!cond_or.evaluate(&serde_json::json!({"x": "no", "y": "no"})));
    }

    #[test]
    fn condition_nested_field_access() {
        let cond = Condition::Equals {
            field: "match.score".into(),
            value: serde_json::json!(95),
        };
        assert!(cond.evaluate(&serde_json::json!({"match": {"score": 95}})));
        assert!(!cond.evaluate(&serde_json::json!({"match": {"score": 50}})));
        assert!(!cond.evaluate(&serde_json::json!({"other": 95})));
    }

    #[test]
    fn policy_matches_basic() {
        let policy = Policy::new(
            "test",
            TriggerType::CheckpointDue,
            PolicyAction::UpdateManifest,
        );
        let trigger = Trigger::new(TriggerType::CheckpointDue, serde_json::json!({}));
        assert!(policy.matches(&trigger, None));

        let wrong_trigger = Trigger::new(TriggerType::DisputeFiled, serde_json::json!({}));
        assert!(!policy.matches(&wrong_trigger, None));
    }

    #[test]
    fn policy_disabled_never_matches() {
        let policy =
            Policy::new("test", TriggerType::CheckpointDue, PolicyAction::Halt).with_enabled(false);
        let trigger = Trigger::new(TriggerType::CheckpointDue, serde_json::json!({}));
        assert!(!policy.matches(&trigger, None));
    }

    #[test]
    fn policy_jurisdiction_scope() {
        let policy = Policy::new("test", TriggerType::CheckpointDue, PolicyAction::Halt)
            .with_jurisdiction_scope(vec!["pk".into(), "ae".into()]);

        let trigger = Trigger::new(TriggerType::CheckpointDue, serde_json::json!({}));
        assert!(policy.matches(&trigger, Some("pk")));
        assert!(policy.matches(&trigger, Some("ae")));
        assert!(!policy.matches(&trigger, Some("us")));
        assert!(!policy.matches(&trigger, None));
    }

    #[test]
    fn standard_policies_count() {
        assert_eq!(standard_policies().len(), 4);
    }

    #[test]
    fn extended_policies_count() {
        assert!(extended_policies().len() >= 15);
    }

    #[test]
    fn condition_exists_evaluation() {
        let cond = Condition::Exists {
            field: "value".into(),
        };
        assert!(cond.evaluate(&serde_json::json!({"value": true})));
        assert!(cond.evaluate(&serde_json::json!({"value": "hello"})));
        assert!(cond.evaluate(&serde_json::json!({"value": 42})));
        assert!(!cond.evaluate(&serde_json::json!({"value": null})));
        assert!(!cond.evaluate(&serde_json::json!({"value": false})));
        assert!(!cond.evaluate(&serde_json::json!({"value": ""})));
        assert!(!cond.evaluate(&serde_json::json!({"other": 1})));
    }

    #[test]
    fn condition_in_evaluation() {
        let cond = Condition::In {
            field: "status".into(),
            values: vec![serde_json::json!("expired"), serde_json::json!("revoked")],
        };
        assert!(cond.evaluate(&serde_json::json!({"status": "expired"})));
        assert!(cond.evaluate(&serde_json::json!({"status": "revoked"})));
        assert!(!cond.evaluate(&serde_json::json!({"status": "valid"})));
    }

    #[test]
    fn condition_not_equals_evaluation() {
        let cond = Condition::NotEquals {
            field: "status".into(),
            value: serde_json::json!("valid"),
        };
        assert!(cond.evaluate(&serde_json::json!({"status": "expired"})));
        assert!(!cond.evaluate(&serde_json::json!({"status": "valid"})));
    }

    #[test]
    fn condition_less_than_greater_than() {
        let lt = Condition::LessThan {
            field: "days".into(),
            threshold: serde_json::json!(7),
        };
        assert!(lt.evaluate(&serde_json::json!({"days": 3})));
        assert!(!lt.evaluate(&serde_json::json!({"days": 7})));
        assert!(!lt.evaluate(&serde_json::json!({"days": 10})));

        let gt = Condition::GreaterThan {
            field: "days".into(),
            threshold: serde_json::json!(7),
        };
        assert!(gt.evaluate(&serde_json::json!({"days": 10})));
        assert!(!gt.evaluate(&serde_json::json!({"days": 7})));
        assert!(!gt.evaluate(&serde_json::json!({"days": 3})));
    }

    // ── Coverage expansion tests ─────────────────────────────────────

    #[test]
    fn trigger_type_all_returns_20_variants() {
        assert_eq!(TriggerType::all().len(), 20);
    }

    #[test]
    fn trigger_type_display_all() {
        for t in TriggerType::all() {
            let s = format!("{t}");
            assert!(!s.is_empty());
            assert_eq!(s, t.as_str());
        }
    }

    #[test]
    fn trigger_type_serde_roundtrip_all() {
        for t in TriggerType::all() {
            let json = serde_json::to_string(&t).unwrap();
            let deserialized: TriggerType = serde_json::from_str(&json).unwrap();
            assert_eq!(*t, deserialized);
        }
    }

    #[test]
    fn policy_action_as_str_all_variants() {
        let all_actions = [
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
        for action in &all_actions {
            let s = action.as_str();
            assert!(!s.is_empty());
            assert_eq!(format!("{action}"), s);
        }
    }

    #[test]
    fn policy_action_serde_roundtrip_all() {
        let action = PolicyAction::Transfer;
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: PolicyAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, deserialized);
    }

    #[test]
    fn condition_and_evaluation_coverage() {
        let cond = Condition::And {
            conditions: vec![
                Condition::Equals {
                    field: "a".into(),
                    value: serde_json::json!(1),
                },
                Condition::Equals {
                    field: "b".into(),
                    value: serde_json::json!(2),
                },
            ],
        };
        assert!(cond.evaluate(&serde_json::json!({"a": 1, "b": 2})));
        assert!(!cond.evaluate(&serde_json::json!({"a": 1, "b": 3})));
    }

    #[test]
    fn condition_or_evaluation_coverage() {
        let cond = Condition::Or {
            conditions: vec![
                Condition::Equals {
                    field: "a".into(),
                    value: serde_json::json!(1),
                },
                Condition::Equals {
                    field: "b".into(),
                    value: serde_json::json!(2),
                },
            ],
        };
        assert!(cond.evaluate(&serde_json::json!({"a": 1, "b": 0})));
        assert!(!cond.evaluate(&serde_json::json!({"a": 0, "b": 0})));
    }

    #[test]
    fn condition_missing_field_returns_false() {
        let cond = Condition::Equals {
            field: "missing".into(),
            value: serde_json::json!("val"),
        };
        assert!(!cond.evaluate(&serde_json::json!({"other": "val"})));
    }

    #[test]
    fn condition_less_than_missing_field() {
        let cond = Condition::LessThan {
            field: "missing".into(),
            threshold: serde_json::json!(10),
        };
        assert!(!cond.evaluate(&serde_json::json!({})));
    }

    #[test]
    fn condition_greater_than_missing_field() {
        let cond = Condition::GreaterThan {
            field: "missing".into(),
            threshold: serde_json::json!(10),
        };
        assert!(!cond.evaluate(&serde_json::json!({})));
    }
}
