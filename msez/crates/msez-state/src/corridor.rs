//! # Corridor Typestate Machine
//!
//! Implements the corridor lifecycle from spec §40-corridors:
//! `DRAFT → PENDING → ACTIVE` with `HALTED` and `SUSPENDED` branches.
//!
//! Each state is a distinct type. Only valid transitions exist as methods.
//! Invalid transitions are compile errors.
//!
//! ## Audit Reference
//!
//! Finding §2.3: The Python implementation used string states `"PROPOSED"`
//! and `"OPERATIONAL"` that diverged from the spec's `"DRAFT"`, `"PENDING"`,
//! `"ACTIVE"`. This typestate encoding makes divergence structurally impossible.

use std::marker::PhantomData;

use chrono::{DateTime, Utc};

use msez_core::{ContentDigest, CorridorId, JurisdictionId};

// ── State Types ──────────────────────────────────────────────────────

/// The initial draft state for a new corridor.
#[derive(Debug)]
pub struct Draft;

/// Corridor has been submitted and is pending regulatory approval.
#[derive(Debug)]
pub struct Pending;

/// Corridor is active and processing cross-border transactions.
#[derive(Debug)]
pub struct Active;

/// Corridor has been halted by a jurisdiction authority.
#[derive(Debug)]
pub struct Halted;

/// Corridor is temporarily suspended with an expected resume date.
#[derive(Debug)]
pub struct Suspended;

/// Corridor has been permanently deprecated. Terminal state.
#[derive(Debug)]
pub struct Deprecated;

/// Marker trait for all valid corridor states. Sealed.
pub trait CorridorState: private::Sealed + std::fmt::Debug {
    /// The canonical state name as it appears in spec and audit trail.
    fn name() -> &'static str;
    /// Whether this is a terminal state (no further transitions).
    fn is_terminal() -> bool {
        false
    }
}

mod private {
    pub trait Sealed {}
    impl Sealed for super::Draft {}
    impl Sealed for super::Pending {}
    impl Sealed for super::Active {}
    impl Sealed for super::Halted {}
    impl Sealed for super::Suspended {}
    impl Sealed for super::Deprecated {}
}

impl CorridorState for Draft {
    fn name() -> &'static str {
        "DRAFT"
    }
}
impl CorridorState for Pending {
    fn name() -> &'static str {
        "PENDING"
    }
}
impl CorridorState for Active {
    fn name() -> &'static str {
        "ACTIVE"
    }
}
impl CorridorState for Halted {
    fn name() -> &'static str {
        "HALTED"
    }
}
impl CorridorState for Suspended {
    fn name() -> &'static str {
        "SUSPENDED"
    }
}
impl CorridorState for Deprecated {
    fn name() -> &'static str {
        "DEPRECATED"
    }
    fn is_terminal() -> bool {
        true
    }
}

// ── Evidence Types ───────────────────────────────────────────────────

/// Evidence required to submit a corridor from Draft to Pending.
#[derive(Debug)]
pub struct SubmissionEvidence {
    /// Digest of the bilateral agreement between jurisdictions.
    pub bilateral_agreement_digest: ContentDigest,
    /// Digest of the pack trilogy (lawpack + regpack + licensepack).
    pub pack_trilogy_digest: ContentDigest,
}

/// Evidence required to activate a corridor from Pending to Active.
#[derive(Debug)]
pub struct ActivationEvidence {
    /// Regulatory approval digest from jurisdiction A.
    pub regulatory_approval_a: ContentDigest,
    /// Regulatory approval digest from jurisdiction B.
    pub regulatory_approval_b: ContentDigest,
}

/// Reason for halting a corridor.
#[derive(Debug)]
pub struct HaltReason {
    /// Human-readable reason for the halt.
    pub reason: String,
    /// The jurisdiction authority that issued the halt.
    pub authority: JurisdictionId,
    /// Digest of the halt evidence.
    pub evidence: ContentDigest,
}

/// Reason for suspending a corridor.
#[derive(Debug)]
pub struct SuspendReason {
    /// Human-readable reason for the suspension.
    pub reason: String,
    /// Expected resume date, if known.
    pub expected_resume: Option<DateTime<Utc>>,
}

/// Evidence required to resume a suspended corridor.
#[derive(Debug)]
pub struct ResumeEvidence {
    /// Digest of the resolution attestation.
    pub resolution_attestation: ContentDigest,
}

// ── The Corridor ─────────────────────────────────────────────────────

/// A trade corridor between two jurisdictions, parameterized by its
/// current lifecycle state.
///
/// State-specific methods ensure that only valid transitions can be
/// called. For example, `Corridor<Draft>` has `.submit()` but not
/// `.halt()`, while `Corridor<Active>` has `.halt()` and `.suspend()`
/// but not `.submit()`.
#[derive(Debug)]
pub struct Corridor<S: CorridorState> {
    /// Unique corridor identifier.
    pub id: CorridorId,
    /// The first jurisdiction in the bilateral corridor.
    pub jurisdiction_a: JurisdictionId,
    /// The second jurisdiction in the bilateral corridor.
    pub jurisdiction_b: JurisdictionId,
    /// When the corridor was created.
    pub created_at: DateTime<Utc>,
    /// When the corridor was last updated.
    pub updated_at: DateTime<Utc>,
    /// Internal state data.
    _inner: CorridorInner,
    /// Phantom data for the state type parameter.
    _state: PhantomData<S>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct CorridorInner {
    pack_trilogy_digest: Option<ContentDigest>,
    halt_reason: Option<HaltReason>,
    suspend_reason: Option<SuspendReason>,
}

// ── State-Specific Methods ───────────────────────────────────────────

impl Corridor<Draft> {
    /// Create a new corridor in Draft state.
    pub fn new(
        id: CorridorId,
        jurisdiction_a: JurisdictionId,
        jurisdiction_b: JurisdictionId,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            jurisdiction_a,
            jurisdiction_b,
            created_at: now,
            updated_at: now,
            _inner: CorridorInner {
                pack_trilogy_digest: None,
                halt_reason: None,
                suspend_reason: None,
            },
            _state: PhantomData,
        }
    }

    /// Submit the corridor for regulatory approval.
    /// Transitions: Draft → Pending.
    pub fn submit(self, evidence: SubmissionEvidence) -> Corridor<Pending> {
        Corridor {
            id: self.id,
            jurisdiction_a: self.jurisdiction_a,
            jurisdiction_b: self.jurisdiction_b,
            created_at: self.created_at,
            updated_at: Utc::now(),
            _inner: CorridorInner {
                pack_trilogy_digest: Some(evidence.pack_trilogy_digest),
                ..self._inner
            },
            _state: PhantomData,
        }
    }
}

impl Corridor<Pending> {
    /// Activate the corridor after regulatory approval.
    /// Transitions: Pending → Active.
    pub fn activate(self, _evidence: ActivationEvidence) -> Corridor<Active> {
        Corridor {
            id: self.id,
            jurisdiction_a: self.jurisdiction_a,
            jurisdiction_b: self.jurisdiction_b,
            created_at: self.created_at,
            updated_at: Utc::now(),
            _inner: self._inner,
            _state: PhantomData,
        }
    }
}

impl Corridor<Active> {
    /// Halt the corridor by jurisdiction authority order.
    /// Transitions: Active → Halted.
    pub fn halt(self, reason: HaltReason) -> Corridor<Halted> {
        Corridor {
            id: self.id,
            jurisdiction_a: self.jurisdiction_a,
            jurisdiction_b: self.jurisdiction_b,
            created_at: self.created_at,
            updated_at: Utc::now(),
            _inner: CorridorInner {
                halt_reason: Some(reason),
                ..self._inner
            },
            _state: PhantomData,
        }
    }

    /// Suspend the corridor temporarily.
    /// Transitions: Active → Suspended.
    pub fn suspend(self, reason: SuspendReason) -> Corridor<Suspended> {
        Corridor {
            id: self.id,
            jurisdiction_a: self.jurisdiction_a,
            jurisdiction_b: self.jurisdiction_b,
            created_at: self.created_at,
            updated_at: Utc::now(),
            _inner: CorridorInner {
                suspend_reason: Some(reason),
                ..self._inner
            },
            _state: PhantomData,
        }
    }
}

impl Corridor<Suspended> {
    /// Resume the corridor after the suspension condition is resolved.
    /// Transitions: Suspended → Active.
    pub fn resume(self, _evidence: ResumeEvidence) -> Corridor<Active> {
        Corridor {
            id: self.id,
            jurisdiction_a: self.jurisdiction_a,
            jurisdiction_b: self.jurisdiction_b,
            created_at: self.created_at,
            updated_at: Utc::now(),
            _inner: CorridorInner {
                suspend_reason: None,
                ..self._inner
            },
            _state: PhantomData,
        }
    }
}
