//! # msez-agentic — Agentic Policy Engine
//!
//! A genuinely novel contribution to programmable compliance: an autonomous
//! policy engine that evaluates compliance triggers and executes actions
//! based on configurable policy definitions.
//!
//! ## Capabilities
//!
//! - **20 trigger types** covering entity lifecycle events, compliance state
//!   changes, corridor activity, regulatory updates, and temporal deadlines.
//!
//! - **7 standard policies** for common compliance scenarios (AML monitoring,
//!   license expiry, tax deadline, sanctions screening, etc.).
//!
//! - **Action scheduling** for automated compliance responses with
//!   configurable delays and approval gates.
//!
//! - **Policy audit trail** recording every trigger evaluation and action
//!   execution for regulatory review.

pub mod audit;
pub mod evaluation;
pub mod policy;
pub mod scheduler;

// Re-export primary types.
pub use evaluation::PolicyEngine;
pub use policy::{Policy, Trigger};
pub use scheduler::ActionScheduler;
