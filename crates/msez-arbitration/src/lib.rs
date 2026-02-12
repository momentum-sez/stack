//! # msez-arbitration — Dispute Resolution
//!
//! Implements the dispute resolution lifecycle for the SEZ Stack:
//!
//! - **Dispute** (`dispute.rs`): Dispute lifecycle state machine with
//!   filing, response, hearing, and resolution phases.
//!
//! - **Evidence** (`evidence.rs`): Content-addressed evidence package
//!   management for dispute proceedings.
//!
//! - **Escrow** (`escrow.rs`): Escrow operations for disputed amounts
//!   during the arbitration process.
//!
//! - **Enforcement** (`enforcement.rs`): Award enforcement and receipt
//!   generation for resolved disputes.
//!
//! ## Crate Policy
//!
//! - Depends on `msez-core` and `msez-state` internally.
//! - All evidence digests use `CanonicalBytes` → SHA-256.

pub mod dispute;
pub mod enforcement;
pub mod escrow;
pub mod evidence;

pub use dispute::{Dispute, DisputeState};
