//! # msez-arbitration — Dispute Resolution
//!
//! Manages the full dispute lifecycle within and across jurisdictions:
//!
//! - **Dispute** ([`dispute`]): Dispute initiation, claim filing, and
//!   lifecycle management through hearing, deliberation, and award stages.
//!
//! - **Evidence** ([`evidence`]): Evidence package management with
//!   content-addressed storage and chain-of-custody tracking.
//!
//! - **Escrow** ([`escrow`]): Escrow operations for disputed amounts,
//!   including conditional release and clawback.
//!
//! - **Enforcement** ([`enforcement`]): Award enforcement with corridor
//!   receipt generation for cross-border dispute resolution.

pub mod dispute;
pub mod enforcement;
pub mod escrow;
pub mod evidence;

// Re-export primary types.
pub use dispute::{Dispute, DisputeState};
pub use enforcement::EnforcementOrder;
pub use escrow::EscrowAccount;
pub use evidence::EvidencePackage;
