//! # Corridor Typestate Machine
//!
//! Implements the corridor lifecycle using the typestate pattern.
//! Each state is a distinct type — invalid transitions are compile errors.
//!
//! ## States (spec-aligned per §40)
//!
//! - `Draft` → initial state, corridor proposal under construction.
//! - `Pending` → submitted for regulatory review.
//! - `Active` → approved and operational for cross-border settlement.
//! - `Halted` → emergency stop by jurisdictional authority.
//! - `Suspended` → temporary pause with expected resumption.
//! - `Deprecated` → terminal state, corridor permanently decommissioned.
//!
//! ## Security Invariant
//!
//! State names are types, not strings. The string "OPERATIONAL" from the
//! defective v1 state machine (audit §2.3) cannot exist in this system.

use std::marker::PhantomData;

use msez_core::{CorridorId, ContentDigest, JurisdictionId};

// ─── State Types ─────────────────────────────────────────────────────

/// Corridor state: proposal under construction.
#[derive(Debug)]
pub struct Draft;

/// Corridor state: submitted for regulatory review.
#[derive(Debug)]
pub struct Pending;

/// Corridor state: approved and operational.
#[derive(Debug)]
pub struct Active;

/// Corridor state: emergency stop by authority.
#[derive(Debug)]
pub struct Halted;

/// Corridor state: temporary pause with expected resumption.
#[derive(Debug)]
pub struct Suspended;

/// Corridor state: permanently decommissioned (terminal).
#[derive(Debug)]
pub struct Deprecated;

// ─── Sealed Trait ────────────────────────────────────────────────────

mod private {
    pub trait Sealed {}
    impl Sealed for super::Draft {}
    impl Sealed for super::Pending {}
    impl Sealed for super::Active {}
    impl Sealed for super::Halted {}
    impl Sealed for super::Suspended {}
    impl Sealed for super::Deprecated {}
}

/// Marker trait for all valid corridor states.
///
/// Sealed — only the six states defined in this module implement it.
/// External crates cannot add new states.
pub trait CorridorState: private::Sealed + std::fmt::Debug {
    /// The canonical string name of this state (e.g., "DRAFT").
    fn name() -> &'static str;

    /// Whether this state is terminal (no further transitions allowed).
    fn is_terminal() -> bool {
        false
    }
}

impl CorridorState for Draft {
    fn name() -> &'static str { "DRAFT" }
}
impl CorridorState for Pending {
    fn name() -> &'static str { "PENDING" }
}
impl CorridorState for Active {
    fn name() -> &'static str { "ACTIVE" }
}
impl CorridorState for Halted {
    fn name() -> &'static str { "HALTED" }
}
impl CorridorState for Suspended {
    fn name() -> &'static str { "SUSPENDED" }
}
impl CorridorState for Deprecated {
    fn name() -> &'static str { "DEPRECATED" }
    fn is_terminal() -> bool { true }
}

// ─── Evidence Types ──────────────────────────────────────────────────

/// Evidence required to submit a corridor for review.
#[derive(Debug, Clone)]
pub struct SubmissionEvidence {
    /// Digest of the bilateral agreement between jurisdictions.
    pub bilateral_agreement_digest: ContentDigest,
    /// Digest of the pack trilogy (lawpack + regpack + licensepack).
    pub pack_trilogy_digest: ContentDigest,
}

/// Evidence required to activate a corridor.
#[derive(Debug, Clone)]
pub struct ActivationEvidence {
    /// Regulatory approval digest from jurisdiction A.
    pub regulatory_approval_a: ContentDigest,
    /// Regulatory approval digest from jurisdiction B.
    pub regulatory_approval_b: ContentDigest,
}

/// Reason for halting a corridor.
#[derive(Debug, Clone)]
pub struct HaltReason {
    /// Human-readable reason for the halt.
    pub reason: String,
    /// The jurisdiction that issued the halt order.
    pub authority: JurisdictionId,
}

/// Reason for suspending a corridor.
#[derive(Debug, Clone)]
pub struct SuspendReason {
    /// Human-readable reason for the suspension.
    pub reason: String,
}

/// Evidence required to resume a suspended corridor.
#[derive(Debug, Clone)]
pub struct ResumeEvidence {
    /// Digest of the resolution attestation.
    pub resolution_attestation: ContentDigest,
}

// ─── The Corridor ────────────────────────────────────────────────────

/// A trade corridor between two jurisdictions, parameterized by its lifecycle state.
///
/// Only state-appropriate methods are available at compile time.
/// `Corridor<Draft>` has `.submit()` but not `.halt()`.
/// `Corridor<Active>` has `.halt()` and `.suspend()` but not `.submit()`.
#[derive(Debug)]
pub struct Corridor<S: CorridorState> {
    /// Unique corridor identifier.
    pub id: CorridorId,
    /// First jurisdiction in the corridor.
    pub jurisdiction_a: JurisdictionId,
    /// Second jurisdiction in the corridor.
    pub jurisdiction_b: JurisdictionId,
    _state: PhantomData<S>,
}

impl Corridor<Draft> {
    /// Create a new corridor in DRAFT state.
    pub fn new(
        id: CorridorId,
        jurisdiction_a: JurisdictionId,
        jurisdiction_b: JurisdictionId,
    ) -> Self {
        Self {
            id,
            jurisdiction_a,
            jurisdiction_b,
            _state: PhantomData,
        }
    }

    /// Submit the corridor for regulatory review (DRAFT → PENDING).
    pub fn submit(self, _evidence: SubmissionEvidence) -> Corridor<Pending> {
        Corridor {
            id: self.id,
            jurisdiction_a: self.jurisdiction_a,
            jurisdiction_b: self.jurisdiction_b,
            _state: PhantomData,
        }
    }
}

impl Corridor<Pending> {
    /// Activate the corridor after regulatory approval (PENDING → ACTIVE).
    pub fn activate(self, _evidence: ActivationEvidence) -> Corridor<Active> {
        Corridor {
            id: self.id,
            jurisdiction_a: self.jurisdiction_a,
            jurisdiction_b: self.jurisdiction_b,
            _state: PhantomData,
        }
    }
}

impl Corridor<Active> {
    /// Emergency halt by jurisdictional authority (ACTIVE → HALTED).
    pub fn halt(self, _reason: HaltReason) -> Corridor<Halted> {
        Corridor {
            id: self.id,
            jurisdiction_a: self.jurisdiction_a,
            jurisdiction_b: self.jurisdiction_b,
            _state: PhantomData,
        }
    }

    /// Temporary suspension with expected resumption (ACTIVE → SUSPENDED).
    pub fn suspend(self, _reason: SuspendReason) -> Corridor<Suspended> {
        Corridor {
            id: self.id,
            jurisdiction_a: self.jurisdiction_a,
            jurisdiction_b: self.jurisdiction_b,
            _state: PhantomData,
        }
    }
}

impl Corridor<Suspended> {
    /// Resume a suspended corridor (SUSPENDED → ACTIVE).
    pub fn resume(self, _evidence: ResumeEvidence) -> Corridor<Active> {
        Corridor {
            id: self.id,
            jurisdiction_a: self.jurisdiction_a,
            jurisdiction_b: self.jurisdiction_b,
            _state: PhantomData,
        }
    }
}
