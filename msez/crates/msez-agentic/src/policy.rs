//! # Policy Definitions
//!
//! Configurable compliance policies with trigger conditions and action responses.

use serde::{Deserialize, Serialize};

/// A compliance policy that maps triggers to actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Unique policy identifier.
    pub id: String,
    /// Human-readable policy name.
    pub name: String,
    /// The trigger condition for this policy.
    pub trigger: Trigger,
    /// Whether this policy is currently active.
    pub enabled: bool,
}

/// A trigger condition that activates a policy.
///
/// The engine supports 20 trigger types covering entity lifecycle,
/// compliance state, corridor activity, regulatory, and temporal events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Trigger {
    /// Entity lifecycle state change.
    EntityStateChange { entity_type: String },
    /// Compliance domain state change.
    ComplianceStateChange { domain: String },
    /// Corridor activity threshold exceeded.
    CorridorActivityThreshold { threshold: u64 },
    /// License approaching expiry.
    LicenseExpiryWarning { days_before: u32 },
    /// Tax filing deadline approaching.
    TaxDeadlineApproaching { days_before: u32 },
    /// Sanctions list updated.
    SanctionsListUpdate,
    /// Scheduled temporal trigger (cron-like).
    Scheduled { cron_expression: String },
}
