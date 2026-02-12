//! # msez-corridor — Corridor Operations
//!
//! Provides the operational infrastructure for cross-border trade corridors:
//!
//! - **Bridge** ([`bridge`]): Dijkstra-weighted routing across corridor
//!   graphs, with fee computation per hop.
//!
//! - **Receipt Chain** ([`receipt`]): Append-only corridor receipts backed
//!   by MMR for efficient inclusion proofs.
//!
//! - **Fork Resolution** ([`fork`]): Fork detection with three-level
//!   ordering: timestamp (primary), watcher attestation count (secondary),
//!   and lexicographic digest tiebreaker (tertiary). Includes 5-minute
//!   maximum clock skew tolerance.
//!
//! - **Anchoring** ([`anchor`]): L1 chain anchoring for corridor checkpoints.
//!   L1 is optional — the system works without blockchain dependencies.
//!
//! - **Netting** ([`netting`]): Settlement netting engine for bilateral
//!   obligation compression.
//!
//! - **SWIFT** ([`swift`]): SWIFT pacs.008 payment instruction adapter
//!   for traditional settlement rails.

pub mod anchor;
pub mod bridge;
pub mod fork;
pub mod netting;
pub mod receipt;
pub mod swift;

// Re-export primary types.
pub use anchor::AnchorCommitment;
pub use bridge::CorridorBridge;
pub use fork::ForkResolution;
pub use netting::NettingEngine;
pub use receipt::CorridorReceipt;
pub use swift::SwiftPacs008;
