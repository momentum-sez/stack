//! # Policy Definitions
//!
//! Defines the 20 trigger types and policy structure for the agentic
//! policy engine.
//!
//! ## Implements
//!
//! Spec §20 — Agentic policy engine trigger taxonomy.

use serde::{Deserialize, Serialize};

/// The 20 trigger types that can activate a policy evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TriggerType {
    /// Entity formation event.
    EntityFormation,
    /// Entity status change.
    EntityStatusChange,
    /// Ownership transfer.
    OwnershipTransfer,
    /// Capital event (issuance, buyback).
    CapitalEvent,
    /// Tax filing deadline approaching.
    TaxDeadline,
    /// License expiry approaching.
    LicenseExpiry,
    /// Sanctions list update.
    SanctionsUpdate,
    /// Regulatory rule change.
    RegulatoryChange,
    /// Corridor state transition.
    CorridorTransition,
    /// Migration initiated.
    MigrationInitiated,
    /// Migration completed.
    MigrationCompleted,
    /// Fork detected.
    ForkDetected,
    /// Watcher slashing event.
    WatcherSlashing,
    /// Settlement netting cycle.
    NettingCycle,
    /// Compliance evaluation requested.
    ComplianceEvaluation,
    /// Beneficial ownership change.
    BeneficialOwnershipChange,
    /// Cross-border payment initiated.
    CrossBorderPayment,
    /// Dispute filed.
    DisputeFiled,
    /// Arbitration award issued.
    ArbitrationAward,
    /// Periodic compliance review.
    PeriodicReview,
}

/// A policy definition with trigger conditions and actions.
///
/// Placeholder — full implementation will include condition predicates,
/// action definitions, and jurisdiction scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Human-readable policy name.
    pub name: String,
    /// The trigger that activates this policy.
    pub trigger: TriggerType,
}
