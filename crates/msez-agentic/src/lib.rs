//! # msez-agentic — Agentic Policy Engine
//!
//! A genuinely novel contribution to programmable compliance. The agentic
//! policy engine evaluates trigger conditions against the compliance tensor
//! and executes automated compliance actions.
//!
//! ## Architecture
//!
//! - **Policy** (`policy.rs`): Policy definitions with 20 trigger types
//!   and 7 standard policy templates.
//!
//! - **Evaluation** (`evaluation.rs`): Policy evaluation engine that
//!   matches trigger conditions against compliance tensor states.
//!
//! - **Scheduler** (`scheduler.rs`): Action scheduling for deferred
//!   and periodic compliance actions.
//!
//! - **Audit** (`audit.rs`): Policy audit trail with content-addressed
//!   evidence for every policy evaluation and action execution.
//!
//! ## Crate Policy
//!
//! - Depends on `msez-core` and `msez-tensor` internally.
//! - Policy evaluation uses exhaustive `match` on `ComplianceDomain`
//!   to ensure no domain is silently skipped.

pub mod audit;
pub mod evaluation;
pub mod policy;
pub mod scheduler;

pub use policy::{Policy, TriggerType};
