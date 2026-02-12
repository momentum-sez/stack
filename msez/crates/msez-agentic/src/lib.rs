//! # msez-agentic — Agentic Policy Engine
//!
//! MASS Protocol v0.2 Chapter 17: A genuinely novel contribution to programmable
//! compliance — an autonomous policy engine that evaluates compliance triggers
//! and executes actions based on configurable policy definitions.
//!
//! ## Architecture
//!
//! The crate is organized into four modules:
//!
//! - **[`policy`]** — Policy definitions: 20 `TriggerType` variants, `PolicyAction`
//!   enum, `Condition` predicates, and the `Policy` struct with builder pattern.
//!   Includes `standard_policies()` (4) and `extended_policies()` (19).
//!
//! - **[`evaluation`]** — Policy Evaluation Engine (`PolicyEngine`): evaluates
//!   triggers against registered policies, resolves conflicts by priority +
//!   jurisdiction specificity + policy ID, and produces `ScheduledAction` directives.
//!   Guarantees determinism per Theorem 17.1.
//!
//! - **[`scheduler`]** — Action Scheduler: manages the lifecycle of scheduled
//!   actions (pending → executing → completed/failed/cancelled) with retry
//!   semantics, deadlines, and cron-like recurring schedules.
//!
//! - **[`audit`]** — Policy Audit Trail: append-only audit log with
//!   `CanonicalBytes`-based digests for tamper evidence. Circular buffer
//!   trims oldest 10% when capacity is exceeded.
//!
//! ## Determinism (Theorem 17.1)
//!
//! Given identical trigger events and policy state, evaluation produces identical
//! results. This is guaranteed by:
//! - Sorted policy iteration (BTreeMap keyed by policy_id)
//! - Pure condition evaluation (no side effects)
//! - Deterministic conflict resolution (priority → specificity → id)
//!
//! ## Example
//!
//! ```rust
//! use msez_agentic::policy::{TriggerType, Trigger, PolicyAction};
//! use msez_agentic::evaluation::PolicyEngine;
//!
//! let mut engine = PolicyEngine::with_standard_policies();
//! let trigger = Trigger::new(
//!     TriggerType::SanctionsListUpdate,
//!     serde_json::json!({"affected_parties": ["self"]}),
//! );
//! let actions = engine.process_trigger(&trigger, "asset:example", None);
//! assert!(actions.iter().any(|a| a.action == PolicyAction::Halt));
//! ```

pub mod audit;
pub mod evaluation;
pub mod policy;
pub mod scheduler;

// Re-export primary types at crate root for ergonomic imports.
pub use audit::{AuditEntry, AuditEntryType, AuditTrail};
pub use evaluation::{EvaluationResult, PolicyEngine};
pub use policy::{
    AuthorizationRequirement, Condition, ImpactLevel, Policy, PolicyAction, Trigger, TriggerType,
};
pub use scheduler::{
    ActionScheduler, ActionStatus, CronSchedule, SchedulePattern, ScheduledAction,
};
