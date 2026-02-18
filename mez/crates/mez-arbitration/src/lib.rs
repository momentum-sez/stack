//! # mez-arbitration — Dispute Resolution
//!
//! Manages the full dispute lifecycle within and across jurisdictions:
//!
//! - **Error** ([`error`]): Structured error hierarchy for the arbitration
//!   subsystem.
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
pub mod error;
pub mod escrow;
pub mod evidence;

// Re-export primary types for ergonomic imports.

// Error types
pub use error::ArbitrationError;

// Dispute lifecycle
pub use dispute::{
    ArbitrationInstitution, Claim, ClosureEvidence, DecisionEvidence, DismissalEvidence, Dispute,
    DisputeId, DisputeState, DisputeType, EnforcementInitiationEvidence, EvidencePhaseEvidence,
    FilingEvidence, HearingScheduleEvidence, Money, Party, ReviewInitiationEvidence,
    SettlementEvidence, TransitionRecord,
};

// Evidence management
pub use evidence::{
    AuthenticityAttestation, AuthenticityType, ChainOfCustodyEntry, EvidenceItem, EvidenceItemId,
    EvidencePackage, EvidencePackageId, EvidenceType,
};

// Escrow operations
pub use escrow::{
    EscrowAccount, EscrowId, EscrowStatus, EscrowTransaction, EscrowType, ReleaseCondition,
    ReleaseConditionType, TransactionType,
};

// Enforcement
pub use enforcement::{
    EnforcementAction, EnforcementOrder, EnforcementOrderId, EnforcementPrecondition,
    EnforcementReceipt, EnforcementReceiptId, EnforcementStatus,
};

// Registry
pub use dispute::institution_registry;
