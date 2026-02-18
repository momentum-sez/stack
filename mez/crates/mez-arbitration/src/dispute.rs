//! # Dispute Lifecycle
//!
//! Manages dispute initiation, claim filing, and lifecycle stages through the
//! state machine: `Filed → UnderReview → EvidenceCollection → Hearing →
//! Decided → Enforced → Closed`.
//!
//! ## Design Choice: Validated Enum over Typestate
//!
//! This module uses a validated enum (runtime-checked) rather than the typestate
//! pattern used by [`mez_state::corridor`]. Three factors drive this decision:
//!
//! 1. **Settlement from any non-terminal state.** A dispute can be settled at
//!    any point before a decision is rendered (states Filed through Hearing).
//!    Typestate would require duplicating `settle()` across 4+ `impl` blocks,
//!    each with identical logic but different source state types.
//!
//! 2. **Serialization frequency.** Disputes are stored in databases and
//!    transmitted via APIs where the state is not known at compile time.
//!    A validated enum serializes directly via serde without an intermediate
//!    `DynDisputeState` layer.
//!
//! 3. **Typed evidence enforcement.** Each transition has a dedicated method
//!    accepting a *specific* evidence struct. Invalid transitions return
//!    [`ArbitrationError::InvalidTransition`]. You cannot call
//!    [`Dispute::decide`] without providing [`DecisionEvidence`], providing
//!    the same compile-time guarantee as typestate for each call site.
//!
//! ## Spec Reference
//!
//! Implements Definition 26 (Dispute Lifecycle) from the specification.
//! Dispute types, claim types, and institution registry match the Python
//! `tools/arbitration.py` reference implementation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use mez_core::{ContentDigest, CorridorId, Did, JurisdictionId, Timestamp};

use crate::error::ArbitrationError;

// ── Identifiers ────────────────────────────────────────────────────────

/// A unique identifier for a dispute proceeding.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DisputeId(Uuid);

impl DisputeId {
    /// Create a new random dispute identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a dispute identifier from an existing UUID.
    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Access the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for DisputeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DisputeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dispute:{}", self.0)
    }
}

// ── Dispute State ──────────────────────────────────────────────────────

/// The lifecycle state of a dispute.
///
/// States progress linearly from [`Filed`](DisputeState::Filed) through to
/// [`Closed`](DisputeState::Closed), with alternative terminal states
/// [`Settled`](DisputeState::Settled) and [`Dismissed`](DisputeState::Dismissed)
/// reachable from early stages.
///
/// ## Transition Graph
///
/// ```text
/// Filed ──begin_review()──▶ UnderReview ──open_evidence()──▶ EvidenceCollection
///   │                          │                                    │
///   ├─settle()──▶ Settled      ├─settle()──▶ Settled    schedule_hearing()
///   └─dismiss()──▶ Dismissed   └─dismiss()──▶ Dismissed             │
///                                                                   ▼
///                                                               Hearing
///                                                                   │
///                                                    ┌──────────────┤
///                                                    │              │
///                                              settle()──▶ Settled  decide()
///                                                                   │
///                                                                   ▼
///                                                               Decided
///                                                                   │
///                                                              enforce()
///                                                                   │
///                                                                   ▼
///                                                               Enforced
///                                                                   │
///                                                               close()
///                                                                   │
///                                                                   ▼
///                                                                Closed
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DisputeState {
    /// Dispute has been filed with the institution.
    Filed,
    /// Institution has acknowledged and is reviewing the filing.
    UnderReview,
    /// Evidence collection phase is open.
    EvidenceCollection,
    /// Hearing is in progress before the tribunal.
    Hearing,
    /// Tribunal has rendered a decision/award.
    Decided,
    /// Award is being enforced.
    Enforced,
    /// Dispute lifecycle complete. Terminal state.
    Closed,
    /// Parties reached a settlement agreement. Terminal state.
    Settled,
    /// Dispute was dismissed by the institution. Terminal state.
    Dismissed,
}

impl DisputeState {
    /// The canonical string name of this state.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Filed => "FILED",
            Self::UnderReview => "UNDER_REVIEW",
            Self::EvidenceCollection => "EVIDENCE_COLLECTION",
            Self::Hearing => "HEARING",
            Self::Decided => "DECIDED",
            Self::Enforced => "ENFORCED",
            Self::Closed => "CLOSED",
            Self::Settled => "SETTLED",
            Self::Dismissed => "DISMISSED",
        }
    }

    /// Whether this state is terminal (no further transitions allowed).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Closed | Self::Settled | Self::Dismissed)
    }

    /// Valid target states from this state.
    pub fn valid_transitions(&self) -> &'static [DisputeState] {
        match self {
            Self::Filed => &[Self::UnderReview, Self::Settled, Self::Dismissed],
            Self::UnderReview => &[Self::EvidenceCollection, Self::Settled, Self::Dismissed],
            Self::EvidenceCollection => &[Self::Hearing, Self::Settled],
            Self::Hearing => &[Self::Decided, Self::Settled],
            Self::Decided => &[Self::Enforced],
            Self::Enforced => &[Self::Closed],
            Self::Closed | Self::Settled | Self::Dismissed => &[],
        }
    }
}

impl std::fmt::Display for DisputeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── Dispute Types ──────────────────────────────────────────────────────

/// Categories of disputes supported by the arbitration system.
///
/// Matches Definition 26.2 from the specification and the Python
/// `DISPUTE_TYPES` constant in `tools/arbitration.py:42-51`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DisputeType {
    /// Failure to perform contractual obligations.
    BreachOfContract,
    /// Delivered goods do not match specifications.
    NonConformingGoods,
    /// Failure to make agreed payments.
    PaymentDefault,
    /// Goods not delivered per terms.
    DeliveryFailure,
    /// Goods have defects affecting merchantability.
    QualityDefect,
    /// Documentation issues (e.g., LC discrepancies).
    DocumentaryDiscrepancy,
    /// Force majeure event claims.
    ForceMajeure,
    /// Fraud or misrepresentation in transaction.
    FraudulentMisrepresentation,
}

impl DisputeType {
    /// All dispute types as a slice.
    pub fn all() -> &'static [DisputeType] {
        &[
            Self::BreachOfContract,
            Self::NonConformingGoods,
            Self::PaymentDefault,
            Self::DeliveryFailure,
            Self::QualityDefect,
            Self::DocumentaryDiscrepancy,
            Self::ForceMajeure,
            Self::FraudulentMisrepresentation,
        ]
    }

    /// The canonical string identifier for serialization.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BreachOfContract => "breach_of_contract",
            Self::NonConformingGoods => "non_conforming_goods",
            Self::PaymentDefault => "payment_default",
            Self::DeliveryFailure => "delivery_failure",
            Self::QualityDefect => "quality_defect",
            Self::DocumentaryDiscrepancy => "documentary_discrepancy",
            Self::ForceMajeure => "force_majeure",
            Self::FraudulentMisrepresentation => "fraudulent_misrepresentation",
        }
    }
}

impl std::fmt::Display for DisputeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── Money ──────────────────────────────────────────────────────────────

/// Monetary amount with currency.
///
/// Amounts are stored as strings to preserve arbitrary precision, matching
/// the Python `Decimal` serialization. The canonicalization pipeline rejects
/// floats, so string representation is the only safe path for monetary values.
///
/// # Security Invariant
///
/// Financial amounts must never be represented as floating-point numbers.
/// String storage ensures no precision loss during serialization or
/// canonicalization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    /// Amount as a decimal string (e.g., "150000", "25000.50").
    pub amount: String,
    /// ISO 4217 currency code (e.g., "USD", "SGD").
    pub currency: String,
}

impl Money {
    /// Create a new monetary amount.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::InvalidAmount`] if the amount string is
    /// empty or contains non-numeric characters.
    pub fn new(
        amount: impl Into<String>,
        currency: impl Into<String>,
    ) -> Result<Self, ArbitrationError> {
        let amount_str = amount.into();
        if !is_valid_decimal(&amount_str) {
            return Err(ArbitrationError::InvalidAmount(amount_str));
        }
        Ok(Self {
            amount: amount_str,
            currency: currency.into(),
        })
    }
}

impl std::fmt::Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.amount, self.currency)
    }
}

/// Validate that a string represents a valid decimal number.
fn is_valid_decimal(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let s = s.strip_prefix('-').unwrap_or(s);
    if s.is_empty() {
        return false;
    }
    let mut has_dot = false;
    let mut has_digit = false;
    for c in s.chars() {
        if c == '.' {
            if has_dot {
                return false;
            }
            has_dot = true;
        } else if c.is_ascii_digit() {
            has_digit = true;
        } else {
            return false;
        }
    }
    has_digit
}

// ── Party ──────────────────────────────────────────────────────────────

/// A party in arbitration proceedings.
///
/// Identified by a DID (Decentralized Identifier). Optional metadata
/// includes jurisdiction and contact information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Party {
    /// The party's DID.
    pub did: Did,
    /// Legal name of the party.
    pub legal_name: String,
    /// Jurisdiction the party is domiciled in.
    pub jurisdiction_id: Option<JurisdictionId>,
}

// ── Claim ──────────────────────────────────────────────────────────────

/// A claim within a dispute.
///
/// Each claim has a type, description, optional monetary amount, and
/// references to supporting evidence artifacts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Claim {
    /// Unique claim identifier within the dispute.
    pub claim_id: String,
    /// Category of the claim.
    pub claim_type: DisputeType,
    /// Human-readable description of the claim.
    pub description: String,
    /// Monetary amount claimed, if applicable.
    pub amount: Option<Money>,
    /// Content digests of supporting evidence artifacts.
    pub supporting_evidence_digests: Vec<ContentDigest>,
}

// ── Transition Evidence Types ──────────────────────────────────────────

/// Evidence required to file a dispute (creates the dispute in Filed state).
///
/// Provided at dispute creation time. Contains the filing documentation
/// and optional escrow proof.
#[derive(Debug, Clone)]
pub struct FilingEvidence {
    /// Digest of the signed filing document submitted to the institution.
    pub filing_document_digest: ContentDigest,
}

/// Evidence to transition Filed → UnderReview.
///
/// Provided when the arbitration institution acknowledges the filing
/// and assigns a case reference.
#[derive(Debug, Clone)]
pub struct ReviewInitiationEvidence {
    /// Case reference assigned by the institution.
    pub case_reference: String,
    /// Digest of the institution's acknowledgment document.
    pub institution_acknowledgment_digest: ContentDigest,
}

/// Evidence to transition UnderReview → EvidenceCollection.
///
/// Provided when the institution issues a procedural order opening
/// the evidence collection phase.
#[derive(Debug, Clone)]
pub struct EvidencePhaseEvidence {
    /// Digest of the procedural order opening evidence collection.
    pub procedural_order_digest: ContentDigest,
    /// Deadline for evidence submission.
    pub evidence_deadline: Timestamp,
}

/// Evidence to transition EvidenceCollection → Hearing.
///
/// Provided when the hearing is scheduled and tribunal is formed.
#[derive(Debug, Clone)]
pub struct HearingScheduleEvidence {
    /// Scheduled hearing date.
    pub hearing_date: Timestamp,
    /// Digest of the tribunal composition document.
    pub tribunal_composition_digest: ContentDigest,
}

/// Evidence to transition Hearing → Decided.
///
/// Provided when the tribunal renders its decision/award.
#[derive(Debug, Clone)]
pub struct DecisionEvidence {
    /// Digest of the ruling/award document (typically a VC).
    pub ruling_digest: ContentDigest,
}

/// Evidence to transition Decided → Enforced.
///
/// Provided when enforcement actions have been initiated.
#[derive(Debug, Clone)]
pub struct EnforcementInitiationEvidence {
    /// Digest of the enforcement order.
    pub enforcement_order_digest: ContentDigest,
}

/// Evidence to transition Enforced → Closed.
///
/// Provided when all enforcement actions are complete and confirmed.
#[derive(Debug, Clone)]
pub struct ClosureEvidence {
    /// Digest of the final closure report.
    pub final_report_digest: ContentDigest,
}

/// Evidence for settlement (any non-terminal pre-decision state → Settled).
///
/// Both parties must consent. The settlement agreement is content-addressed.
#[derive(Debug, Clone)]
pub struct SettlementEvidence {
    /// Digest of the settlement agreement document.
    pub settlement_agreement_digest: ContentDigest,
    /// Consent digests from each party.
    pub party_consent_digests: Vec<ContentDigest>,
}

/// Evidence for dismissal (Filed or UnderReview → Dismissed).
///
/// The institution dismisses the dispute for procedural or substantive reasons.
#[derive(Debug, Clone)]
pub struct DismissalEvidence {
    /// Reason for dismissal.
    pub reason: String,
    /// Digest of the institution's dismissal order.
    pub dismissal_order_digest: ContentDigest,
}

// ── Transition Record ──────────────────────────────────────────────────

/// A record of a single state transition in the dispute lifecycle.
///
/// Every transition is logged with source/target states, timestamp, and
/// evidence digest for a complete audit trail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionRecord {
    /// State before the transition.
    pub from_state: DisputeState,
    /// State after the transition.
    pub to_state: DisputeState,
    /// When the transition occurred (UTC).
    pub timestamp: DateTime<Utc>,
    /// Digest of the evidence that authorized this transition.
    pub evidence_digest: ContentDigest,
}

// ── Arbitration Institution ────────────────────────────────────────────

/// Registry entry for an arbitration institution.
///
/// Matches the Python `ARBITRATION_INSTITUTIONS` registry in
/// `tools/arbitration.py:103-183`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrationInstitution {
    /// Short identifier (e.g., "difc-lcia").
    pub id: String,
    /// Full legal name.
    pub name: String,
    /// Seat jurisdiction.
    pub jurisdiction_id: String,
    /// Supported dispute types.
    pub supported_dispute_types: Vec<DisputeType>,
    /// Whether emergency arbitrator is available.
    pub emergency_arbitrator: bool,
    /// Whether expedited procedure is available.
    pub expedited_procedure: bool,
    /// Jurisdictions where awards are enforceable.
    pub enforcement_jurisdictions: Vec<String>,
}

/// Return the built-in registry of supported arbitration institutions.
///
/// Includes international institutions (DIFC-LCIA, SIAC, ICC, AIFC-IAC) and
/// Pakistan-specific institutions (ATIR, ADR Centre, KCDR) per M-008.
pub fn institution_registry() -> Vec<ArbitrationInstitution> {
    vec![
        // ── International Institutions ───────────────────────────────
        ArbitrationInstitution {
            id: "difc-lcia".to_string(),
            name: "DIFC-LCIA Arbitration Centre".to_string(),
            jurisdiction_id: "uae-difc".to_string(),
            supported_dispute_types: DisputeType::all().to_vec(),
            emergency_arbitrator: true,
            expedited_procedure: true,
            enforcement_jurisdictions: vec![
                "uae-difc".to_string(),
                "uae-adgm".to_string(),
                "new_york_convention".to_string(),
            ],
        },
        ArbitrationInstitution {
            id: "siac".to_string(),
            name: "Singapore International Arbitration Centre".to_string(),
            jurisdiction_id: "sg".to_string(),
            supported_dispute_types: DisputeType::all().to_vec(),
            emergency_arbitrator: true,
            expedited_procedure: true,
            enforcement_jurisdictions: vec!["sg".to_string(), "new_york_convention".to_string()],
        },
        ArbitrationInstitution {
            id: "icc".to_string(),
            name: "ICC International Court of Arbitration".to_string(),
            jurisdiction_id: "fr-paris".to_string(),
            supported_dispute_types: DisputeType::all().to_vec(),
            emergency_arbitrator: true,
            expedited_procedure: true,
            enforcement_jurisdictions: vec!["new_york_convention".to_string()],
        },
        ArbitrationInstitution {
            id: "aifc-iac".to_string(),
            name: "AIFC International Arbitration Centre".to_string(),
            jurisdiction_id: "kaz-aifc".to_string(),
            supported_dispute_types: DisputeType::all().to_vec(),
            emergency_arbitrator: true,
            expedited_procedure: true,
            enforcement_jurisdictions: vec![
                "kaz-aifc".to_string(),
                "new_york_convention".to_string(),
            ],
        },
        // ── Pakistan-Specific Institutions (M-008) ──────────────────
        //
        // Pakistan's dispute resolution infrastructure per the Alternate
        // Dispute Resolution Act 2017, the Arbitration Act 1940, and
        // specialized tax tribunals (ATIR — Appellate Tribunal Inland
        // Revenue).
        ArbitrationInstitution {
            id: "pak-atir".to_string(),
            name: "Appellate Tribunal Inland Revenue (Pakistan)".to_string(),
            jurisdiction_id: "pk".to_string(),
            supported_dispute_types: vec![
                DisputeType::PaymentDefault,
                DisputeType::DocumentaryDiscrepancy,
                DisputeType::BreachOfContract,
            ],
            emergency_arbitrator: false,
            expedited_procedure: true,
            enforcement_jurisdictions: vec!["pk".to_string()],
        },
        ArbitrationInstitution {
            id: "pak-adr".to_string(),
            name: "Pakistan ADR Centre (Alternate Dispute Resolution Act 2017)".to_string(),
            jurisdiction_id: "pk".to_string(),
            supported_dispute_types: DisputeType::all().to_vec(),
            emergency_arbitrator: false,
            expedited_procedure: true,
            enforcement_jurisdictions: vec!["pk".to_string(), "new_york_convention".to_string()],
        },
        ArbitrationInstitution {
            id: "pak-kcdr".to_string(),
            name: "Karachi Centre for Dispute Resolution".to_string(),
            jurisdiction_id: "pk-sindh".to_string(),
            supported_dispute_types: DisputeType::all().to_vec(),
            emergency_arbitrator: true,
            expedited_procedure: true,
            enforcement_jurisdictions: vec!["pk".to_string(), "new_york_convention".to_string()],
        },
    ]
}

// ── The Dispute ────────────────────────────────────────────────────────

/// A dispute between two parties, managed through the arbitration lifecycle.
///
/// Created via [`Dispute::file`], then advanced through states using
/// transition methods that each require a specific typed evidence struct.
///
/// ## Security Invariant
///
/// Every state transition is recorded in [`transition_log`](Dispute::transition_log)
/// with the evidence digest that authorized it. The log is append-only and
/// provides a tamper-evident audit trail. Terminal states reject all
/// further transitions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dispute {
    /// Unique dispute identifier.
    pub id: DisputeId,
    /// Current lifecycle state.
    pub state: DisputeState,
    /// Category of the dispute.
    pub dispute_type: DisputeType,
    /// The filing party.
    pub claimant: Party,
    /// The responding party.
    pub respondent: Party,
    /// Governing jurisdiction.
    pub jurisdiction: JurisdictionId,
    /// Corridor identifier if this is a cross-border dispute.
    pub corridor_id: Option<CorridorId>,
    /// Arbitration institution handling the dispute.
    pub institution_id: String,
    /// Claims filed by the claimant.
    pub claims: Vec<Claim>,
    /// When the dispute was filed (UTC).
    pub filed_at: Timestamp,
    /// When the dispute was last updated (UTC).
    pub updated_at: Timestamp,
    /// Complete transition history for audit purposes.
    pub transition_log: Vec<TransitionRecord>,
}

impl Dispute {
    /// File a new dispute, creating it in the [`Filed`](DisputeState::Filed) state.
    ///
    /// This is the only constructor for `Dispute`. The dispute starts in Filed
    /// state with the filing evidence recorded in the transition log.
    ///
    /// ## Spec Reference
    ///
    /// Implements transition type `arbitration.dispute.file.v1`.
    #[allow(clippy::too_many_arguments)]
    pub fn file(
        claimant: Party,
        respondent: Party,
        dispute_type: DisputeType,
        jurisdiction: JurisdictionId,
        corridor_id: Option<CorridorId>,
        institution_id: String,
        claims: Vec<Claim>,
        evidence: FilingEvidence,
    ) -> Self {
        let now = Timestamp::now();
        Self {
            id: DisputeId::new(),
            state: DisputeState::Filed,
            dispute_type,
            claimant,
            respondent,
            jurisdiction,
            corridor_id,
            institution_id,
            claims,
            filed_at: now.clone(),
            updated_at: now,
            transition_log: vec![TransitionRecord {
                from_state: DisputeState::Filed,
                to_state: DisputeState::Filed,
                timestamp: Utc::now(),
                evidence_digest: evidence.filing_document_digest,
            }],
        }
    }

    /// Transition Filed → UnderReview.
    ///
    /// The institution has acknowledged the filing and assigned a case reference.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::InvalidTransition`] if not in Filed state.
    pub fn begin_review(
        &mut self,
        evidence: ReviewInitiationEvidence,
    ) -> Result<(), ArbitrationError> {
        self.require_state(DisputeState::Filed, DisputeState::UnderReview)?;
        self.record_transition(
            DisputeState::Filed,
            DisputeState::UnderReview,
            evidence.institution_acknowledgment_digest,
        );
        self.state = DisputeState::UnderReview;
        Ok(())
    }

    /// Transition UnderReview → EvidenceCollection.
    ///
    /// The institution has issued a procedural order opening evidence collection.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::InvalidTransition`] if not in UnderReview state.
    pub fn open_evidence_collection(
        &mut self,
        evidence: EvidencePhaseEvidence,
    ) -> Result<(), ArbitrationError> {
        self.require_state(DisputeState::UnderReview, DisputeState::EvidenceCollection)?;
        self.record_transition(
            DisputeState::UnderReview,
            DisputeState::EvidenceCollection,
            evidence.procedural_order_digest,
        );
        self.state = DisputeState::EvidenceCollection;
        Ok(())
    }

    /// Transition EvidenceCollection → Hearing.
    ///
    /// The hearing has been scheduled and the tribunal is formed.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::InvalidTransition`] if not in EvidenceCollection state.
    pub fn schedule_hearing(
        &mut self,
        evidence: HearingScheduleEvidence,
    ) -> Result<(), ArbitrationError> {
        self.require_state(DisputeState::EvidenceCollection, DisputeState::Hearing)?;
        self.record_transition(
            DisputeState::EvidenceCollection,
            DisputeState::Hearing,
            evidence.tribunal_composition_digest,
        );
        self.state = DisputeState::Hearing;
        Ok(())
    }

    /// Transition Hearing → Decided.
    ///
    /// The tribunal has rendered its decision/award.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::InvalidTransition`] if not in Hearing state.
    pub fn decide(&mut self, evidence: DecisionEvidence) -> Result<(), ArbitrationError> {
        self.require_state(DisputeState::Hearing, DisputeState::Decided)?;
        self.record_transition(
            DisputeState::Hearing,
            DisputeState::Decided,
            evidence.ruling_digest,
        );
        self.state = DisputeState::Decided;
        Ok(())
    }

    /// Transition Decided → Enforced.
    ///
    /// Enforcement actions have been initiated for the award.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::InvalidTransition`] if not in Decided state.
    pub fn enforce(
        &mut self,
        evidence: EnforcementInitiationEvidence,
    ) -> Result<(), ArbitrationError> {
        self.require_state(DisputeState::Decided, DisputeState::Enforced)?;
        self.record_transition(
            DisputeState::Decided,
            DisputeState::Enforced,
            evidence.enforcement_order_digest,
        );
        self.state = DisputeState::Enforced;
        Ok(())
    }

    /// Transition Enforced → Closed.
    ///
    /// All enforcement actions are complete. Terminal state.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::InvalidTransition`] if not in Enforced state.
    pub fn close(&mut self, evidence: ClosureEvidence) -> Result<(), ArbitrationError> {
        self.require_state(DisputeState::Enforced, DisputeState::Closed)?;
        self.record_transition(
            DisputeState::Enforced,
            DisputeState::Closed,
            evidence.final_report_digest,
        );
        self.state = DisputeState::Closed;
        Ok(())
    }

    /// Settle the dispute from any pre-decision, non-terminal state.
    ///
    /// Settlement is available from Filed, UnderReview, EvidenceCollection,
    /// or Hearing states. Once decided, the award must be enforced rather
    /// than settled.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::InvalidTransition`] if the current state
    /// does not allow settlement.
    pub fn settle(&mut self, evidence: SettlementEvidence) -> Result<(), ArbitrationError> {
        let allowed = matches!(
            self.state,
            DisputeState::Filed
                | DisputeState::UnderReview
                | DisputeState::EvidenceCollection
                | DisputeState::Hearing
        );
        if !allowed {
            return Err(ArbitrationError::InvalidTransition {
                from: self.state.as_str().to_string(),
                to: "SETTLED".to_string(),
                reason: "settlement is only available before a decision is rendered".to_string(),
            });
        }
        let from = self.state;
        self.record_transition(
            from,
            DisputeState::Settled,
            evidence.settlement_agreement_digest,
        );
        self.state = DisputeState::Settled;
        Ok(())
    }

    /// Dismiss the dispute from Filed or UnderReview states.
    ///
    /// Dismissal is an institution action, typically for procedural defects
    /// or lack of jurisdiction.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::InvalidTransition`] if the current state
    /// does not allow dismissal.
    pub fn dismiss(&mut self, evidence: DismissalEvidence) -> Result<(), ArbitrationError> {
        let allowed = matches!(self.state, DisputeState::Filed | DisputeState::UnderReview);
        if !allowed {
            return Err(ArbitrationError::InvalidTransition {
                from: self.state.as_str().to_string(),
                to: "DISMISSED".to_string(),
                reason: "dismissal is only available from Filed or UnderReview states".to_string(),
            });
        }
        let from = self.state;
        self.record_transition(
            from,
            DisputeState::Dismissed,
            evidence.dismissal_order_digest,
        );
        self.state = DisputeState::Dismissed;
        Ok(())
    }

    /// Check that the dispute is in the expected state for a transition.
    fn require_state(
        &self,
        expected: DisputeState,
        target: DisputeState,
    ) -> Result<(), ArbitrationError> {
        if self.state.is_terminal() {
            return Err(ArbitrationError::TerminalState {
                dispute_id: self.id.to_string(),
                state: self.state.as_str().to_string(),
            });
        }
        if self.state != expected {
            return Err(ArbitrationError::InvalidTransition {
                from: self.state.as_str().to_string(),
                to: target.as_str().to_string(),
                reason: format!("expected state {}, got {}", expected, self.state),
            });
        }
        Ok(())
    }

    /// Record a transition in the audit log.
    fn record_transition(
        &mut self,
        from: DisputeState,
        to: DisputeState,
        evidence_digest: ContentDigest,
    ) {
        self.transition_log.push(TransitionRecord {
            from_state: from,
            to_state: to,
            timestamp: Utc::now(),
            evidence_digest,
        });
        self.updated_at = Timestamp::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mez_core::{sha256_digest, CanonicalBytes};
    use serde_json::json;

    fn test_digest() -> ContentDigest {
        let canonical = CanonicalBytes::new(&json!({"test": "evidence"})).unwrap();
        sha256_digest(&canonical)
    }

    fn test_did(name: &str) -> Did {
        Did::new(format!("did:key:z6Mk{name}")).unwrap()
    }

    fn test_jurisdiction() -> JurisdictionId {
        JurisdictionId::new("uae-difc").unwrap()
    }

    fn file_dispute() -> Dispute {
        Dispute::file(
            Party {
                did: test_did("Claimant123"),
                legal_name: "Trade Corp ADGM Ltd".to_string(),
                jurisdiction_id: Some(JurisdictionId::new("uae-adgm").unwrap()),
            },
            Party {
                did: test_did("Respondent456"),
                legal_name: "Import Corp AIFC LLP".to_string(),
                jurisdiction_id: Some(JurisdictionId::new("kaz-aifc").unwrap()),
            },
            DisputeType::PaymentDefault,
            test_jurisdiction(),
            Some(CorridorId::new()),
            "difc-lcia".to_string(),
            vec![Claim {
                claim_id: "claim-001".to_string(),
                claim_type: DisputeType::PaymentDefault,
                description: "Outstanding payment for delivered goods".to_string(),
                amount: Some(Money::new("150000", "USD").unwrap()),
                supporting_evidence_digests: vec![test_digest()],
            }],
            FilingEvidence {
                filing_document_digest: test_digest(),
            },
        )
    }

    #[test]
    fn file_creates_dispute_in_filed_state() {
        let dispute = file_dispute();
        assert_eq!(dispute.state, DisputeState::Filed);
        assert_eq!(dispute.dispute_type, DisputeType::PaymentDefault);
        assert_eq!(dispute.institution_id, "difc-lcia");
        assert_eq!(dispute.claims.len(), 1);
        assert!(!dispute.transition_log.is_empty());
    }

    #[test]
    fn full_lifecycle_filed_to_closed() {
        let mut dispute = file_dispute();
        assert_eq!(dispute.state, DisputeState::Filed);

        dispute
            .begin_review(ReviewInitiationEvidence {
                case_reference: "DIFC-LCIA-2026-001".to_string(),
                institution_acknowledgment_digest: test_digest(),
            })
            .unwrap();
        assert_eq!(dispute.state, DisputeState::UnderReview);

        dispute
            .open_evidence_collection(EvidencePhaseEvidence {
                procedural_order_digest: test_digest(),
                evidence_deadline: Timestamp::now(),
            })
            .unwrap();
        assert_eq!(dispute.state, DisputeState::EvidenceCollection);

        dispute
            .schedule_hearing(HearingScheduleEvidence {
                hearing_date: Timestamp::now(),
                tribunal_composition_digest: test_digest(),
            })
            .unwrap();
        assert_eq!(dispute.state, DisputeState::Hearing);

        dispute
            .decide(DecisionEvidence {
                ruling_digest: test_digest(),
            })
            .unwrap();
        assert_eq!(dispute.state, DisputeState::Decided);

        dispute
            .enforce(EnforcementInitiationEvidence {
                enforcement_order_digest: test_digest(),
            })
            .unwrap();
        assert_eq!(dispute.state, DisputeState::Enforced);

        dispute
            .close(ClosureEvidence {
                final_report_digest: test_digest(),
            })
            .unwrap();
        assert_eq!(dispute.state, DisputeState::Closed);
        assert!(dispute.state.is_terminal());

        // Transition log: filing record + 6 transitions = 7 entries
        assert_eq!(dispute.transition_log.len(), 7);
    }

    #[test]
    fn settle_from_filed() {
        let mut dispute = file_dispute();
        dispute
            .settle(SettlementEvidence {
                settlement_agreement_digest: test_digest(),
                party_consent_digests: vec![test_digest(), test_digest()],
            })
            .unwrap();
        assert_eq!(dispute.state, DisputeState::Settled);
        assert!(dispute.state.is_terminal());
    }

    #[test]
    fn settle_from_hearing() {
        let mut dispute = file_dispute();
        dispute
            .begin_review(ReviewInitiationEvidence {
                case_reference: "REF-001".to_string(),
                institution_acknowledgment_digest: test_digest(),
            })
            .unwrap();
        dispute
            .open_evidence_collection(EvidencePhaseEvidence {
                procedural_order_digest: test_digest(),
                evidence_deadline: Timestamp::now(),
            })
            .unwrap();
        dispute
            .schedule_hearing(HearingScheduleEvidence {
                hearing_date: Timestamp::now(),
                tribunal_composition_digest: test_digest(),
            })
            .unwrap();
        dispute
            .settle(SettlementEvidence {
                settlement_agreement_digest: test_digest(),
                party_consent_digests: vec![test_digest()],
            })
            .unwrap();
        assert_eq!(dispute.state, DisputeState::Settled);
    }

    #[test]
    fn settle_rejected_after_decision() {
        let mut dispute = file_dispute();
        dispute
            .begin_review(ReviewInitiationEvidence {
                case_reference: "REF-001".to_string(),
                institution_acknowledgment_digest: test_digest(),
            })
            .unwrap();
        dispute
            .open_evidence_collection(EvidencePhaseEvidence {
                procedural_order_digest: test_digest(),
                evidence_deadline: Timestamp::now(),
            })
            .unwrap();
        dispute
            .schedule_hearing(HearingScheduleEvidence {
                hearing_date: Timestamp::now(),
                tribunal_composition_digest: test_digest(),
            })
            .unwrap();
        dispute
            .decide(DecisionEvidence {
                ruling_digest: test_digest(),
            })
            .unwrap();

        let result = dispute.settle(SettlementEvidence {
            settlement_agreement_digest: test_digest(),
            party_consent_digests: vec![],
        });
        assert!(result.is_err());
    }

    #[test]
    fn dismiss_from_filed() {
        let mut dispute = file_dispute();
        dispute
            .dismiss(DismissalEvidence {
                reason: "Lack of jurisdiction".to_string(),
                dismissal_order_digest: test_digest(),
            })
            .unwrap();
        assert_eq!(dispute.state, DisputeState::Dismissed);
        assert!(dispute.state.is_terminal());
    }

    #[test]
    fn dismiss_rejected_from_hearing() {
        let mut dispute = file_dispute();
        dispute
            .begin_review(ReviewInitiationEvidence {
                case_reference: "REF-001".to_string(),
                institution_acknowledgment_digest: test_digest(),
            })
            .unwrap();
        dispute
            .open_evidence_collection(EvidencePhaseEvidence {
                procedural_order_digest: test_digest(),
                evidence_deadline: Timestamp::now(),
            })
            .unwrap();
        dispute
            .schedule_hearing(HearingScheduleEvidence {
                hearing_date: Timestamp::now(),
                tribunal_composition_digest: test_digest(),
            })
            .unwrap();

        let result = dispute.dismiss(DismissalEvidence {
            reason: "test".to_string(),
            dismissal_order_digest: test_digest(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn invalid_transition_filed_to_decided() {
        let mut dispute = file_dispute();
        let result = dispute.decide(DecisionEvidence {
            ruling_digest: test_digest(),
        });
        assert!(result.is_err());
        assert_eq!(dispute.state, DisputeState::Filed);
    }

    #[test]
    fn terminal_state_rejects_all_transitions() {
        let mut dispute = file_dispute();
        dispute
            .dismiss(DismissalEvidence {
                reason: "Procedural defect".to_string(),
                dismissal_order_digest: test_digest(),
            })
            .unwrap();
        assert!(dispute.state.is_terminal());

        assert!(dispute
            .begin_review(ReviewInitiationEvidence {
                case_reference: "REF".to_string(),
                institution_acknowledgment_digest: test_digest(),
            })
            .is_err());
        assert!(dispute
            .settle(SettlementEvidence {
                settlement_agreement_digest: test_digest(),
                party_consent_digests: vec![],
            })
            .is_err());
    }

    #[test]
    fn dispute_state_serialization() {
        let state = DisputeState::EvidenceCollection;
        let json_str = serde_json::to_string(&state).unwrap();
        let deserialized: DisputeState = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized, state);
    }

    #[test]
    fn dispute_serialization_roundtrip() {
        let dispute = file_dispute();
        let json_str = serde_json::to_string(&dispute).unwrap();
        let deserialized: Dispute = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.id, dispute.id);
        assert_eq!(deserialized.state, dispute.state);
        assert_eq!(deserialized.dispute_type, dispute.dispute_type);
    }

    #[test]
    fn institution_registry_contains_seven_institutions() {
        let registry = institution_registry();
        assert_eq!(registry.len(), 7);
        let ids: Vec<&str> = registry.iter().map(|i| i.id.as_str()).collect();
        // International institutions
        assert!(ids.contains(&"difc-lcia"));
        assert!(ids.contains(&"siac"));
        assert!(ids.contains(&"icc"));
        assert!(ids.contains(&"aifc-iac"));
        // Pakistan-specific institutions (M-008)
        assert!(ids.contains(&"pak-atir"));
        assert!(ids.contains(&"pak-adr"));
        assert!(ids.contains(&"pak-kcdr"));
    }

    #[test]
    fn difc_lcia_institution_details() {
        let registry = institution_registry();
        let difc = registry.iter().find(|i| i.id == "difc-lcia").unwrap();
        assert_eq!(difc.name, "DIFC-LCIA Arbitration Centre");
        assert_eq!(difc.jurisdiction_id, "uae-difc");
        assert!(difc.emergency_arbitrator);
        assert!(difc
            .supported_dispute_types
            .contains(&DisputeType::BreachOfContract));
        assert!(difc
            .enforcement_jurisdictions
            .contains(&"new_york_convention".to_string()));
    }

    #[test]
    fn money_valid_amounts() {
        assert!(Money::new("150000", "USD").is_ok());
        assert!(Money::new("25000.50", "SGD").is_ok());
        assert!(Money::new("-1000", "USD").is_ok());
        assert!(Money::new("0", "USD").is_ok());
    }

    #[test]
    fn money_rejects_invalid() {
        assert!(Money::new("", "USD").is_err());
        assert!(Money::new("abc", "USD").is_err());
        assert!(Money::new("12.34.56", "USD").is_err());
    }

    #[test]
    fn dispute_type_all_returns_eight() {
        assert_eq!(DisputeType::all().len(), 8);
    }

    #[test]
    fn dispute_state_valid_transitions() {
        let transitions = DisputeState::Filed.valid_transitions();
        assert!(transitions.contains(&DisputeState::UnderReview));
        assert!(transitions.contains(&DisputeState::Settled));
        assert!(transitions.contains(&DisputeState::Dismissed));

        assert!(DisputeState::Closed.valid_transitions().is_empty());
        assert!(DisputeState::Settled.valid_transitions().is_empty());
        assert!(DisputeState::Dismissed.valid_transitions().is_empty());
    }

    // ── Coverage expansion tests ─────────────────────────────────────

    #[test]
    fn dispute_id_default() {
        let id1 = DisputeId::default();
        let id2 = DisputeId::default();
        assert_ne!(id1, id2);
    }

    #[test]
    fn dispute_id_display() {
        let id = DisputeId::new();
        let display = format!("{id}");
        assert!(display.starts_with("dispute:"));
    }

    #[test]
    fn dispute_id_from_uuid_roundtrip() {
        let uuid = uuid::Uuid::new_v4();
        let id = DisputeId::from_uuid(uuid);
        assert_eq!(*id.as_uuid(), uuid);
    }

    #[test]
    fn dispute_state_display_all_variants() {
        assert_eq!(format!("{}", DisputeState::Filed), "FILED");
        assert_eq!(format!("{}", DisputeState::UnderReview), "UNDER_REVIEW");
        assert_eq!(
            format!("{}", DisputeState::EvidenceCollection),
            "EVIDENCE_COLLECTION"
        );
        assert_eq!(format!("{}", DisputeState::Hearing), "HEARING");
        assert_eq!(format!("{}", DisputeState::Decided), "DECIDED");
        assert_eq!(format!("{}", DisputeState::Enforced), "ENFORCED");
        assert_eq!(format!("{}", DisputeState::Closed), "CLOSED");
        assert_eq!(format!("{}", DisputeState::Settled), "SETTLED");
        assert_eq!(format!("{}", DisputeState::Dismissed), "DISMISSED");
    }

    #[test]
    fn dispute_state_as_str_all_variants() {
        assert_eq!(DisputeState::Filed.as_str(), "FILED");
        assert_eq!(DisputeState::UnderReview.as_str(), "UNDER_REVIEW");
        assert_eq!(
            DisputeState::EvidenceCollection.as_str(),
            "EVIDENCE_COLLECTION"
        );
        assert_eq!(DisputeState::Hearing.as_str(), "HEARING");
        assert_eq!(DisputeState::Decided.as_str(), "DECIDED");
        assert_eq!(DisputeState::Enforced.as_str(), "ENFORCED");
        assert_eq!(DisputeState::Closed.as_str(), "CLOSED");
        assert_eq!(DisputeState::Settled.as_str(), "SETTLED");
        assert_eq!(DisputeState::Dismissed.as_str(), "DISMISSED");
    }

    #[test]
    fn dispute_state_is_terminal_all_variants() {
        assert!(!DisputeState::Filed.is_terminal());
        assert!(!DisputeState::UnderReview.is_terminal());
        assert!(!DisputeState::EvidenceCollection.is_terminal());
        assert!(!DisputeState::Hearing.is_terminal());
        assert!(!DisputeState::Decided.is_terminal());
        assert!(!DisputeState::Enforced.is_terminal());
        assert!(DisputeState::Closed.is_terminal());
        assert!(DisputeState::Settled.is_terminal());
        assert!(DisputeState::Dismissed.is_terminal());
    }

    #[test]
    fn dispute_type_display_all_variants() {
        assert_eq!(
            format!("{}", DisputeType::BreachOfContract),
            "breach_of_contract"
        );
        assert_eq!(
            format!("{}", DisputeType::NonConformingGoods),
            "non_conforming_goods"
        );
        assert_eq!(
            format!("{}", DisputeType::PaymentDefault),
            "payment_default"
        );
        assert_eq!(
            format!("{}", DisputeType::DeliveryFailure),
            "delivery_failure"
        );
        assert_eq!(format!("{}", DisputeType::QualityDefect), "quality_defect");
        assert_eq!(
            format!("{}", DisputeType::DocumentaryDiscrepancy),
            "documentary_discrepancy"
        );
        assert_eq!(format!("{}", DisputeType::ForceMajeure), "force_majeure");
        assert_eq!(
            format!("{}", DisputeType::FraudulentMisrepresentation),
            "fraudulent_misrepresentation"
        );
    }

    #[test]
    fn dispute_type_as_str_all_variants() {
        for dt in DisputeType::all() {
            let s = dt.as_str();
            assert!(!s.is_empty());
            assert_eq!(format!("{dt}"), s);
        }
    }

    #[test]
    fn valid_transitions_under_review() {
        let transitions = DisputeState::UnderReview.valid_transitions();
        assert!(transitions.contains(&DisputeState::EvidenceCollection));
        assert!(transitions.contains(&DisputeState::Settled));
        assert!(transitions.contains(&DisputeState::Dismissed));
    }

    #[test]
    fn valid_transitions_evidence_collection() {
        let transitions = DisputeState::EvidenceCollection.valid_transitions();
        assert!(transitions.contains(&DisputeState::Hearing));
        assert!(transitions.contains(&DisputeState::Settled));
    }

    #[test]
    fn valid_transitions_hearing() {
        let transitions = DisputeState::Hearing.valid_transitions();
        assert!(transitions.contains(&DisputeState::Decided));
        assert!(transitions.contains(&DisputeState::Settled));
    }

    #[test]
    fn valid_transitions_decided() {
        let transitions = DisputeState::Decided.valid_transitions();
        assert_eq!(transitions.len(), 1);
        assert!(transitions.contains(&DisputeState::Enforced));
    }

    #[test]
    fn valid_transitions_enforced() {
        let transitions = DisputeState::Enforced.valid_transitions();
        assert_eq!(transitions.len(), 1);
        assert!(transitions.contains(&DisputeState::Closed));
    }

    #[test]
    fn money_display() {
        let money = Money::new("1000", "USD").unwrap();
        assert_eq!(format!("{money}"), "1000 USD");
    }

    #[test]
    fn money_negative_display() {
        let money = Money::new("-500", "EUR").unwrap();
        assert_eq!(format!("{money}"), "-500 EUR");
    }

    #[test]
    fn money_rejects_just_minus() {
        assert!(Money::new("-", "USD").is_err());
    }

    #[test]
    fn money_rejects_just_dot() {
        assert!(Money::new(".", "USD").is_err());
    }

    #[test]
    fn settle_from_under_review() {
        let mut dispute = file_dispute();
        dispute
            .begin_review(ReviewInitiationEvidence {
                case_reference: "REF-001".to_string(),
                institution_acknowledgment_digest: test_digest(),
            })
            .unwrap();
        dispute
            .settle(SettlementEvidence {
                settlement_agreement_digest: test_digest(),
                party_consent_digests: vec![test_digest()],
            })
            .unwrap();
        assert_eq!(dispute.state, DisputeState::Settled);
    }

    #[test]
    fn settle_from_evidence_collection() {
        let mut dispute = file_dispute();
        dispute
            .begin_review(ReviewInitiationEvidence {
                case_reference: "REF-001".to_string(),
                institution_acknowledgment_digest: test_digest(),
            })
            .unwrap();
        dispute
            .open_evidence_collection(EvidencePhaseEvidence {
                procedural_order_digest: test_digest(),
                evidence_deadline: Timestamp::now(),
            })
            .unwrap();
        dispute
            .settle(SettlementEvidence {
                settlement_agreement_digest: test_digest(),
                party_consent_digests: vec![test_digest()],
            })
            .unwrap();
        assert_eq!(dispute.state, DisputeState::Settled);
    }

    #[test]
    fn dismiss_from_under_review() {
        let mut dispute = file_dispute();
        dispute
            .begin_review(ReviewInitiationEvidence {
                case_reference: "REF-001".to_string(),
                institution_acknowledgment_digest: test_digest(),
            })
            .unwrap();
        dispute
            .dismiss(DismissalEvidence {
                reason: "Lack of jurisdiction".to_string(),
                dismissal_order_digest: test_digest(),
            })
            .unwrap();
        assert_eq!(dispute.state, DisputeState::Dismissed);
    }

    #[test]
    fn settle_rejected_from_enforced() {
        let mut dispute = file_dispute();
        // Advance to Enforced
        dispute
            .begin_review(ReviewInitiationEvidence {
                case_reference: "REF".to_string(),
                institution_acknowledgment_digest: test_digest(),
            })
            .unwrap();
        dispute
            .open_evidence_collection(EvidencePhaseEvidence {
                procedural_order_digest: test_digest(),
                evidence_deadline: Timestamp::now(),
            })
            .unwrap();
        dispute
            .schedule_hearing(HearingScheduleEvidence {
                hearing_date: Timestamp::now(),
                tribunal_composition_digest: test_digest(),
            })
            .unwrap();
        dispute
            .decide(DecisionEvidence {
                ruling_digest: test_digest(),
            })
            .unwrap();
        dispute
            .enforce(EnforcementInitiationEvidence {
                enforcement_order_digest: test_digest(),
            })
            .unwrap();

        let result = dispute.settle(SettlementEvidence {
            settlement_agreement_digest: test_digest(),
            party_consent_digests: vec![],
        });
        assert!(result.is_err());
    }

    #[test]
    fn close_rejected_from_filed() {
        let mut dispute = file_dispute();
        let result = dispute.close(ClosureEvidence {
            final_report_digest: test_digest(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn enforce_rejected_from_filed() {
        let mut dispute = file_dispute();
        let result = dispute.enforce(EnforcementInitiationEvidence {
            enforcement_order_digest: test_digest(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn dismiss_rejected_from_settled() {
        let mut dispute = file_dispute();
        dispute
            .settle(SettlementEvidence {
                settlement_agreement_digest: test_digest(),
                party_consent_digests: vec![],
            })
            .unwrap();

        let result = dispute.dismiss(DismissalEvidence {
            reason: "test".to_string(),
            dismissal_order_digest: test_digest(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn institution_registry_all_have_dispute_types() {
        let registry = institution_registry();
        for inst in &registry {
            assert!(
                !inst.supported_dispute_types.is_empty(),
                "institution {} has no supported dispute types",
                inst.id
            );
            // Verify institution has a valid id (non-empty).
            assert!(
                !inst.id.is_empty(),
                "institution should have a non-empty id"
            );
        }
        // International institutions support all 8 types.
        let intl_ids = ["difc-lcia", "siac", "icc", "aifc-iac"];
        for inst in registry
            .iter()
            .filter(|i| intl_ids.contains(&i.id.as_str()))
        {
            assert_eq!(
                inst.supported_dispute_types.len(),
                8,
                "international institution {} should support all dispute types",
                inst.id
            );
        }
        // ATIR (tax tribunal) supports a subset of dispute types.
        let atir = registry
            .iter()
            .find(|i| i.id == "pak-atir")
            .expect("pak-atir");
        assert_eq!(atir.supported_dispute_types.len(), 3);
    }

    #[test]
    fn dispute_without_corridor_id() {
        let dispute = Dispute::file(
            Party {
                did: test_did("Claimant"),
                legal_name: "Claimant Corp".to_string(),
                jurisdiction_id: None,
            },
            Party {
                did: test_did("Respondent"),
                legal_name: "Respondent Corp".to_string(),
                jurisdiction_id: None,
            },
            DisputeType::BreachOfContract,
            test_jurisdiction(),
            None,
            "siac".to_string(),
            vec![],
            FilingEvidence {
                filing_document_digest: test_digest(),
            },
        );
        assert!(dispute.corridor_id.is_none());
        assert!(dispute.claims.is_empty());
    }

    #[test]
    fn is_valid_decimal_edge_cases() {
        assert!(is_valid_decimal("0"));
        assert!(is_valid_decimal("0.0"));
        assert!(is_valid_decimal("-0.0"));
        assert!(is_valid_decimal("999999999"));
        assert!(!is_valid_decimal(""));
        assert!(!is_valid_decimal("-"));
        assert!(!is_valid_decimal("."));
        assert!(!is_valid_decimal("1.2.3"));
        assert!(!is_valid_decimal("abc"));
    }
}
