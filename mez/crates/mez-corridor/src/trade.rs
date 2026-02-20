// SPDX-License-Identifier: BUSL-1.1
//! # Trade Corridor Instruments
//!
//! Typed trade document structs, transition payloads, flow state machine,
//! and content digest computation for cross-border trade flows.
//!
//! Trade flows are corridors with trade-typed transitions. Each transition
//! carries a trade document payload validated against the corresponding
//! schema, CAS-stored as an artifact, and recorded in the corridor receipt
//! chain via `CorridorReceipt`.
//!
//! ## Document Schemas
//!
//! - `trade.invoice.v1` — Invoice with line items
//! - `trade.bill-of-lading.v1` — Bill of Lading (title document)
//! - `trade.letter-of-credit.v1` — Letter of Credit (UCP 600)
//! - `trade.party.v1` — Trade party identification
//! - `trade.amount.v1` — Deterministic decimal amount
//!
//! ## Flow Archetypes
//!
//! Each trade flow follows one of four archetype state machines:
//! Export, Import, LetterOfCredit, or OpenAccount. The state machine
//! enforces transition ordering per archetype at runtime.

use mez_core::{sha256_digest, CanonicalBytes, ContentDigest};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Trade error type
// ---------------------------------------------------------------------------

/// Errors arising from trade flow operations.
#[derive(Error, Debug)]
pub enum TradeError {
    /// Invalid state transition for the given flow archetype.
    #[error("invalid transition: cannot apply '{transition_kind}' in state {current_state:?} for {flow_type:?} flow")]
    InvalidTransition {
        flow_type: TradeFlowType,
        current_state: TradeFlowState,
        transition_kind: String,
    },

    /// Trade flow not found.
    #[error("trade flow not found: {0}")]
    NotFound(String),

    /// Content digest computation failure.
    #[error("digest computation failed: {0}")]
    DigestError(String),

    /// Serialization failure.
    #[error("serialization error: {0}")]
    SerializationError(String),
}

// ---------------------------------------------------------------------------
// Shared types: TradeAmount, TradeParty, TradePartyAddress, ArtifactRef
// ---------------------------------------------------------------------------

/// Deterministic currency amount. `value` is a decimal string (never float).
/// Matches `schemas/trade.amount.v1.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TradeAmount {
    /// Currency or unit code (ISO 4217 preferred, e.g. "USD", "PKR").
    pub currency: String,
    /// Decimal string with up to 18 fractional digits.
    pub value: String,
    /// Optional explicit scale.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<u32>,
}

/// Trade party address.
/// Matches `trade.party.v1.schema.json` → `address` sub-object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TradePartyAddress {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

/// Trade party identification.
/// Matches `schemas/trade.party.v1.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TradeParty {
    /// Stable identifier (DID, LEI, registry number, internal).
    pub party_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lei: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub did: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<TradePartyAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

/// Content-addressed artifact reference.
/// Matches `schemas/artifact-ref.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactRef {
    /// Artifact category/type (e.g. "invoice", "bol", "lc").
    pub artifact_type: String,
    /// SHA-256 hex digest of the canonical artifact content.
    pub digest_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_length: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

// ---------------------------------------------------------------------------
// Trade document types
// ---------------------------------------------------------------------------

/// Invoice line item.
/// Matches `trade.invoice.v1.schema.json` → `line_items[*]`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InvoiceLineItem {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sku: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hs_code: Option<String>,
    pub quantity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_of_measure: Option<String>,
    pub unit_price: TradeAmount,
    pub amount: TradeAmount,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

/// Trade invoice.
/// Matches `schemas/trade.invoice.v1.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TradeInvoice {
    pub invoice_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_number: Option<String>,
    pub issue_date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    pub seller: TradeParty,
    pub buyer: TradeParty,
    pub total: TradeAmount,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax: Option<TradeAmount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_items: Option<Vec<InvoiceLineItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_order_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incoterms: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shipment_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governing_law: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jurisdiction_tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_refs: Option<Vec<ArtifactRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

/// BOL goods item.
/// Matches `trade.bill-of-lading.v1.schema.json` → `goods[*]`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BolGoods {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hs_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marks: Option<String>,
    pub packages: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gross_weight: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_weight: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

/// BOL endorsement record.
/// Matches `trade.bill-of-lading.v1.schema.json` → `endorsements[*]`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BolEndorsement {
    pub from_party_id: String,
    pub to_party_id: String,
    pub endorsed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Bill of Lading.
/// Matches `schemas/trade.bill-of-lading.v1.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BillOfLading {
    pub bol_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bol_number: Option<String>,
    pub issue_date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consignment_type: Option<String>,
    pub carrier: TradeParty,
    pub shipper: TradeParty,
    pub consignee: TradeParty,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify_party: Option<TradeParty>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vessel_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voyage_number: Option<String>,
    pub port_of_loading: String,
    pub port_of_discharge: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place_of_receipt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place_of_delivery: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freight_terms: Option<String>,
    pub goods: Vec<BolGoods>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub originals_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endorsements: Option<Vec<BolEndorsement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_refs: Option<Vec<ArtifactRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

/// LC document requirement.
/// Matches `trade.letter-of-credit.v1.schema.json` → `documents_required[*]`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LcDocumentRequirement {
    pub doc_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_copies: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Letter of Credit.
/// Matches `schemas/trade.letter-of-credit.v1.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LetterOfCredit {
    pub lc_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lc_number: Option<String>,
    pub issue_date: String,
    pub expiry_date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<String>,
    pub applicant: TradeParty,
    pub beneficiary: TradeParty,
    pub issuing_bank: TradeParty,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advising_bank: Option<TradeParty>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirming_bank: Option<TradeParty>,
    pub amount: TradeAmount,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_shipment_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation_period_days: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incoterms: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documents_required: Option<Vec<LcDocumentRequirement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governing_law: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jurisdiction_tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_refs: Option<Vec<ArtifactRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// LC Amendment sub-object
// ---------------------------------------------------------------------------

/// Structured amendment fields for LC amendments.
/// Matches `transition.payload.trade.lc.amend.v1.schema.json` → `amendment`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LcAmendmentDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_expiry_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_amount: Option<TradeAmount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

// ---------------------------------------------------------------------------
// Transition payloads — exact match to transition payload schemas
// ---------------------------------------------------------------------------

/// Trade transition payload.
///
/// Each variant matches one of the 10 transition payload schemas.
/// Field names match schema `properties` exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TradeTransitionPayload {
    // -- Invoice transitions --
    #[serde(rename = "trade.invoice.issue.v1")]
    InvoiceIssue {
        #[serde(skip_serializing_if = "Option::is_none")]
        invoice: Option<Box<TradeInvoice>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        invoice_ref: Option<ArtifactRef>,
        #[serde(skip_serializing_if = "Option::is_none")]
        issued_by_party_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
    },

    #[serde(rename = "trade.invoice.accept.v1")]
    InvoiceAccept {
        #[serde(skip_serializing_if = "Option::is_none")]
        invoice_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        invoice_ref: Option<ArtifactRef>,
        accepted_by_party_id: String,
        accepted_at: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        dispute_reason: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
    },

    #[serde(rename = "trade.invoice.settle.v1")]
    InvoiceSettle {
        #[serde(skip_serializing_if = "Option::is_none")]
        invoice_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        invoice_ref: Option<ArtifactRef>,
        settled_at: String,
        amount: TradeAmount,
        #[serde(skip_serializing_if = "Option::is_none")]
        settlement_corridor_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        settlement_reference: Option<ArtifactRef>,
        #[serde(skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
    },

    // -- BOL transitions --
    #[serde(rename = "trade.bol.issue.v1")]
    BolIssue {
        #[serde(skip_serializing_if = "Option::is_none")]
        bol: Option<Box<BillOfLading>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        bol_ref: Option<ArtifactRef>,
        #[serde(skip_serializing_if = "Option::is_none")]
        issued_by_party_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
    },

    #[serde(rename = "trade.bol.endorse.v1")]
    BolEndorse {
        #[serde(skip_serializing_if = "Option::is_none")]
        bol_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        bol_ref: Option<ArtifactRef>,
        from_party_id: String,
        to_party_id: String,
        endorsed_at: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        endorsement_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
    },

    #[serde(rename = "trade.bol.release.v1")]
    BolRelease {
        #[serde(skip_serializing_if = "Option::is_none")]
        bol_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        bol_ref: Option<ArtifactRef>,
        released_at: String,
        released_to_party_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        release_location: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
    },

    // -- LC transitions --
    #[serde(rename = "trade.lc.issue.v1")]
    LcIssue {
        #[serde(skip_serializing_if = "Option::is_none")]
        lc: Option<Box<LetterOfCredit>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        lc_ref: Option<ArtifactRef>,
        #[serde(skip_serializing_if = "Option::is_none")]
        issued_by_party_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
    },

    #[serde(rename = "trade.lc.amend.v1")]
    LcAmend {
        #[serde(skip_serializing_if = "Option::is_none")]
        lc_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        lc_ref: Option<ArtifactRef>,
        amended_at: String,
        amendment_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        amended_by_party_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        amendment: Option<LcAmendmentDetails>,
        #[serde(skip_serializing_if = "Option::is_none")]
        amendment_ref: Option<ArtifactRef>,
        #[serde(skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
    },

    #[serde(rename = "trade.lc.present.v1")]
    LcPresent {
        #[serde(skip_serializing_if = "Option::is_none")]
        lc_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        lc_ref: Option<ArtifactRef>,
        presented_at: String,
        presented_by_party_id: String,
        document_refs: Vec<ArtifactRef>,
        #[serde(skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
    },

    #[serde(rename = "trade.lc.honor.v1")]
    LcHonor {
        #[serde(skip_serializing_if = "Option::is_none")]
        lc_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        lc_ref: Option<ArtifactRef>,
        decision: String,
        decision_at: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        decided_by_party_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        honor_amount: Option<TradeAmount>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        settlement_reference: Option<ArtifactRef>,
        #[serde(skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
    },
}

impl TradeTransitionPayload {
    /// Return the transition kind string (e.g. "trade.invoice.issue.v1").
    pub fn kind(&self) -> &'static str {
        match self {
            Self::InvoiceIssue { .. } => "trade.invoice.issue.v1",
            Self::InvoiceAccept { .. } => "trade.invoice.accept.v1",
            Self::InvoiceSettle { .. } => "trade.invoice.settle.v1",
            Self::BolIssue { .. } => "trade.bol.issue.v1",
            Self::BolEndorse { .. } => "trade.bol.endorse.v1",
            Self::BolRelease { .. } => "trade.bol.release.v1",
            Self::LcIssue { .. } => "trade.lc.issue.v1",
            Self::LcAmend { .. } => "trade.lc.amend.v1",
            Self::LcPresent { .. } => "trade.lc.present.v1",
            Self::LcHonor { .. } => "trade.lc.honor.v1",
        }
    }
}

// ---------------------------------------------------------------------------
// Trade flow state machine
// ---------------------------------------------------------------------------

/// Trade flow lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeFlowState {
    Created,
    InvoiceIssued,
    InvoiceAccepted,
    GoodsShipped,
    BolEndorsed,
    GoodsReleased,
    LcIssued,
    LcAmended,
    DocumentsPresented,
    LcHonored,
    SettlementInitiated,
    Settled,
}

impl std::fmt::Display for TradeFlowState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "Created"),
            Self::InvoiceIssued => write!(f, "InvoiceIssued"),
            Self::InvoiceAccepted => write!(f, "InvoiceAccepted"),
            Self::GoodsShipped => write!(f, "GoodsShipped"),
            Self::BolEndorsed => write!(f, "BolEndorsed"),
            Self::GoodsReleased => write!(f, "GoodsReleased"),
            Self::LcIssued => write!(f, "LcIssued"),
            Self::LcAmended => write!(f, "LcAmended"),
            Self::DocumentsPresented => write!(f, "DocumentsPresented"),
            Self::LcHonored => write!(f, "LcHonored"),
            Self::SettlementInitiated => write!(f, "SettlementInitiated"),
            Self::Settled => write!(f, "Settled"),
        }
    }
}

/// Trade flow archetype — defines the valid transition ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeFlowType {
    /// invoice.issue → invoice.accept → bol.issue → bol.endorse → bol.release → invoice.settle
    Export,
    /// invoice.issue → invoice.accept → bol.release → invoice.settle
    Import,
    /// lc.issue → bol.issue → lc.present → lc.honor → invoice.settle
    LetterOfCredit,
    /// invoice.issue → bol.issue → bol.release → invoice.settle
    OpenAccount,
}

impl std::fmt::Display for TradeFlowType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Export => write!(f, "Export"),
            Self::Import => write!(f, "Import"),
            Self::LetterOfCredit => write!(f, "LetterOfCredit"),
            Self::OpenAccount => write!(f, "OpenAccount"),
        }
    }
}

/// Validate a transition and return the next state, or error if invalid.
///
/// Encodes the valid state transitions per archetype as a match table.
pub fn validate_transition(
    flow_type: TradeFlowType,
    current_state: TradeFlowState,
    payload: &TradeTransitionPayload,
) -> Result<TradeFlowState, TradeError> {
    let kind = payload.kind();
    let next = match flow_type {
        TradeFlowType::Export => match (current_state, kind) {
            (TradeFlowState::Created, "trade.invoice.issue.v1") => TradeFlowState::InvoiceIssued,
            (TradeFlowState::InvoiceIssued, "trade.invoice.accept.v1") => {
                TradeFlowState::InvoiceAccepted
            }
            (TradeFlowState::InvoiceAccepted, "trade.bol.issue.v1") => {
                TradeFlowState::GoodsShipped
            }
            (TradeFlowState::GoodsShipped, "trade.bol.endorse.v1") => {
                TradeFlowState::BolEndorsed
            }
            (TradeFlowState::BolEndorsed, "trade.bol.release.v1") => {
                TradeFlowState::GoodsReleased
            }
            (TradeFlowState::GoodsReleased, "trade.invoice.settle.v1") => {
                TradeFlowState::Settled
            }
            _ => {
                return Err(TradeError::InvalidTransition {
                    flow_type,
                    current_state,
                    transition_kind: kind.to_string(),
                })
            }
        },
        TradeFlowType::Import => match (current_state, kind) {
            (TradeFlowState::Created, "trade.invoice.issue.v1") => TradeFlowState::InvoiceIssued,
            (TradeFlowState::InvoiceIssued, "trade.invoice.accept.v1") => {
                TradeFlowState::InvoiceAccepted
            }
            (TradeFlowState::InvoiceAccepted, "trade.bol.release.v1") => {
                TradeFlowState::GoodsReleased
            }
            (TradeFlowState::GoodsReleased, "trade.invoice.settle.v1") => {
                TradeFlowState::Settled
            }
            _ => {
                return Err(TradeError::InvalidTransition {
                    flow_type,
                    current_state,
                    transition_kind: kind.to_string(),
                })
            }
        },
        TradeFlowType::LetterOfCredit => match (current_state, kind) {
            (TradeFlowState::Created, "trade.lc.issue.v1") => TradeFlowState::LcIssued,
            (TradeFlowState::LcIssued, "trade.lc.amend.v1") => TradeFlowState::LcAmended,
            (TradeFlowState::LcIssued, "trade.bol.issue.v1") => TradeFlowState::GoodsShipped,
            (TradeFlowState::LcAmended, "trade.bol.issue.v1") => TradeFlowState::GoodsShipped,
            (TradeFlowState::GoodsShipped, "trade.lc.present.v1") => {
                TradeFlowState::DocumentsPresented
            }
            (TradeFlowState::DocumentsPresented, "trade.lc.honor.v1") => {
                TradeFlowState::LcHonored
            }
            (TradeFlowState::LcHonored, "trade.invoice.settle.v1") => TradeFlowState::Settled,
            _ => {
                return Err(TradeError::InvalidTransition {
                    flow_type,
                    current_state,
                    transition_kind: kind.to_string(),
                })
            }
        },
        TradeFlowType::OpenAccount => match (current_state, kind) {
            (TradeFlowState::Created, "trade.invoice.issue.v1") => TradeFlowState::InvoiceIssued,
            (TradeFlowState::InvoiceIssued, "trade.bol.issue.v1") => {
                TradeFlowState::GoodsShipped
            }
            (TradeFlowState::GoodsShipped, "trade.bol.release.v1") => {
                TradeFlowState::GoodsReleased
            }
            (TradeFlowState::GoodsReleased, "trade.invoice.settle.v1") => {
                TradeFlowState::Settled
            }
            _ => {
                return Err(TradeError::InvalidTransition {
                    flow_type,
                    current_state,
                    transition_kind: kind.to_string(),
                })
            }
        },
    };
    Ok(next)
}

// ---------------------------------------------------------------------------
// Content digest computation
// ---------------------------------------------------------------------------

/// Compute a content digest for a trade document via `SHA256(CanonicalBytes)`.
///
/// Satisfies invariant I-CANON: all trade document digests go through
/// the canonical bytes path.
pub fn compute_trade_document_digest(
    doc: &impl Serialize,
) -> Result<ContentDigest, TradeError> {
    let canonical = CanonicalBytes::new(doc)
        .map_err(|e| TradeError::DigestError(format!("canonicalization failed: {e}")))?;
    Ok(sha256_digest(&canonical))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_party(id: &str) -> TradeParty {
        TradeParty {
            party_id: id.to_string(),
            name: Some(format!("Party {id}")),
            lei: None,
            did: None,
            account_id: None,
            agent_id: None,
            address: None,
            meta: None,
        }
    }

    fn sample_amount(currency: &str, value: &str) -> TradeAmount {
        TradeAmount {
            currency: currency.to_string(),
            value: value.to_string(),
            scale: None,
        }
    }

    fn sample_invoice() -> TradeInvoice {
        TradeInvoice {
            invoice_id: "INV-001".to_string(),
            invoice_number: Some("2026-001".to_string()),
            issue_date: "2026-02-20".to_string(),
            due_date: Some("2026-03-20".to_string()),
            seller: sample_party("seller-1"),
            buyer: sample_party("buyer-1"),
            total: sample_amount("USD", "50000.00"),
            tax: None,
            line_items: None,
            purchase_order_ref: None,
            contract_ref: None,
            incoterms: None,
            shipment_ref: None,
            governing_law: None,
            jurisdiction_tags: None,
            attachment_refs: None,
            meta: None,
        }
    }

    fn sample_bol() -> BillOfLading {
        BillOfLading {
            bol_id: "BOL-001".to_string(),
            bol_number: None,
            issue_date: "2026-02-21".to_string(),
            consignment_type: Some("to-order".to_string()),
            carrier: sample_party("carrier-1"),
            shipper: sample_party("seller-1"),
            consignee: sample_party("buyer-1"),
            notify_party: None,
            vessel_name: Some("MV Trade Star".to_string()),
            voyage_number: Some("V-2026-42".to_string()),
            port_of_loading: "PKQCT".to_string(),
            port_of_discharge: "AEJEA".to_string(),
            place_of_receipt: None,
            place_of_delivery: None,
            freight_terms: None,
            goods: vec![BolGoods {
                description: "Textiles".to_string(),
                hs_code: Some("52".to_string()),
                marks: None,
                packages: "100 cartons".to_string(),
                gross_weight: Some("5000 kg".to_string()),
                net_weight: None,
                volume: None,
                meta: None,
            }],
            originals_count: Some(3),
            endorsements: None,
            attachment_refs: None,
            meta: None,
        }
    }

    fn sample_lc() -> LetterOfCredit {
        LetterOfCredit {
            lc_id: "LC-001".to_string(),
            lc_number: Some("SWIFT-LC-2026-001".to_string()),
            issue_date: "2026-02-20".to_string(),
            expiry_date: "2026-05-20".to_string(),
            rules: Some("UCP600".to_string()),
            applicant: sample_party("buyer-1"),
            beneficiary: sample_party("seller-1"),
            issuing_bank: sample_party("bank-issuing"),
            advising_bank: None,
            confirming_bank: None,
            amount: sample_amount("USD", "50000.00"),
            latest_shipment_date: None,
            presentation_period_days: Some(21),
            incoterms: None,
            documents_required: Some(vec![
                LcDocumentRequirement {
                    doc_type: "invoice".to_string(),
                    required: Some(true),
                    min_copies: Some(3),
                    notes: None,
                },
                LcDocumentRequirement {
                    doc_type: "bill_of_lading".to_string(),
                    required: Some(true),
                    min_copies: Some(3),
                    notes: None,
                },
            ]),
            governing_law: None,
            jurisdiction_tags: None,
            attachment_refs: None,
            meta: None,
        }
    }

    // -- Serde round-trip tests --

    #[test]
    fn trade_invoice_serde_roundtrip() {
        let invoice = sample_invoice();
        let json = serde_json::to_string(&invoice).expect("serialize");
        let deserialized: TradeInvoice = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.invoice_id, invoice.invoice_id);
        assert_eq!(deserialized.total, invoice.total);
    }

    #[test]
    fn bill_of_lading_serde_roundtrip() {
        let bol = sample_bol();
        let json = serde_json::to_string(&bol).expect("serialize");
        let deserialized: BillOfLading = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.bol_id, bol.bol_id);
        assert_eq!(deserialized.goods.len(), 1);
    }

    #[test]
    fn letter_of_credit_serde_roundtrip() {
        let lc = sample_lc();
        let json = serde_json::to_string(&lc).expect("serialize");
        let deserialized: LetterOfCredit = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.lc_id, lc.lc_id);
        assert_eq!(deserialized.amount, lc.amount);
    }

    #[test]
    fn trade_transition_payload_serde_roundtrip() {
        let payload = TradeTransitionPayload::InvoiceIssue {
            invoice: Some(Box::new(sample_invoice())),
            invoice_ref: None,
            issued_by_party_id: Some("seller-1".to_string()),
            notes: None,
        };
        let json = serde_json::to_string(&payload).expect("serialize");
        assert!(json.contains("trade.invoice.issue.v1"));
        let deserialized: TradeTransitionPayload =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.kind(), "trade.invoice.issue.v1");
    }

    // -- Export flow state transitions --

    #[test]
    fn export_flow_full_lifecycle() {
        let ft = TradeFlowType::Export;
        let mut state = TradeFlowState::Created;

        // Step 1: invoice.issue
        let p = TradeTransitionPayload::InvoiceIssue {
            invoice: Some(Box::new(sample_invoice())),
            invoice_ref: None,
            issued_by_party_id: None,
            notes: None,
        };
        state = validate_transition(ft, state, &p).expect("invoice.issue");
        assert_eq!(state, TradeFlowState::InvoiceIssued);

        // Step 2: invoice.accept
        let p = TradeTransitionPayload::InvoiceAccept {
            invoice_id: Some("INV-001".to_string()),
            invoice_ref: None,
            accepted_by_party_id: "buyer-1".to_string(),
            accepted_at: "2026-02-20T12:00:00Z".to_string(),
            status: None,
            dispute_reason: None,
            notes: None,
        };
        state = validate_transition(ft, state, &p).expect("invoice.accept");
        assert_eq!(state, TradeFlowState::InvoiceAccepted);

        // Step 3: bol.issue
        let p = TradeTransitionPayload::BolIssue {
            bol: Some(Box::new(sample_bol())),
            bol_ref: None,
            issued_by_party_id: None,
            notes: None,
        };
        state = validate_transition(ft, state, &p).expect("bol.issue");
        assert_eq!(state, TradeFlowState::GoodsShipped);

        // Step 4: bol.endorse
        let p = TradeTransitionPayload::BolEndorse {
            bol_id: Some("BOL-001".to_string()),
            bol_ref: None,
            from_party_id: "seller-1".to_string(),
            to_party_id: "buyer-1".to_string(),
            endorsed_at: "2026-02-22T10:00:00Z".to_string(),
            endorsement_type: None,
            notes: None,
        };
        state = validate_transition(ft, state, &p).expect("bol.endorse");
        assert_eq!(state, TradeFlowState::BolEndorsed);

        // Step 5: bol.release
        let p = TradeTransitionPayload::BolRelease {
            bol_id: Some("BOL-001".to_string()),
            bol_ref: None,
            released_at: "2026-02-23T10:00:00Z".to_string(),
            released_to_party_id: "buyer-1".to_string(),
            release_location: None,
            notes: None,
        };
        state = validate_transition(ft, state, &p).expect("bol.release");
        assert_eq!(state, TradeFlowState::GoodsReleased);

        // Step 6: invoice.settle
        let p = TradeTransitionPayload::InvoiceSettle {
            invoice_id: Some("INV-001".to_string()),
            invoice_ref: None,
            settled_at: "2026-02-25T10:00:00Z".to_string(),
            amount: sample_amount("USD", "50000.00"),
            settlement_corridor_id: None,
            settlement_reference: None,
            notes: None,
        };
        state = validate_transition(ft, state, &p).expect("invoice.settle");
        assert_eq!(state, TradeFlowState::Settled);
    }

    #[test]
    fn lc_flow_full_lifecycle() {
        let ft = TradeFlowType::LetterOfCredit;
        let mut state = TradeFlowState::Created;

        // lc.issue
        let p = TradeTransitionPayload::LcIssue {
            lc: Some(Box::new(sample_lc())),
            lc_ref: None,
            issued_by_party_id: None,
            notes: None,
        };
        state = validate_transition(ft, state, &p).expect("lc.issue");
        assert_eq!(state, TradeFlowState::LcIssued);

        // bol.issue
        let p = TradeTransitionPayload::BolIssue {
            bol: Some(Box::new(sample_bol())),
            bol_ref: None,
            issued_by_party_id: None,
            notes: None,
        };
        state = validate_transition(ft, state, &p).expect("bol.issue");
        assert_eq!(state, TradeFlowState::GoodsShipped);

        // lc.present
        let p = TradeTransitionPayload::LcPresent {
            lc_id: Some("LC-001".to_string()),
            lc_ref: None,
            presented_at: "2026-02-24T10:00:00Z".to_string(),
            presented_by_party_id: "seller-1".to_string(),
            document_refs: vec![ArtifactRef {
                artifact_type: "invoice".to_string(),
                digest_sha256: "a".repeat(64),
                uri: None,
                media_type: None,
                byte_length: None,
                display_name: None,
                notes: None,
            }],
            notes: None,
        };
        state = validate_transition(ft, state, &p).expect("lc.present");
        assert_eq!(state, TradeFlowState::DocumentsPresented);

        // lc.honor
        let p = TradeTransitionPayload::LcHonor {
            lc_id: Some("LC-001".to_string()),
            lc_ref: None,
            decision: "honor".to_string(),
            decision_at: "2026-02-25T10:00:00Z".to_string(),
            decided_by_party_id: None,
            honor_amount: Some(sample_amount("USD", "50000.00")),
            reason: None,
            settlement_reference: None,
            notes: None,
        };
        state = validate_transition(ft, state, &p).expect("lc.honor");
        assert_eq!(state, TradeFlowState::LcHonored);

        // invoice.settle
        let p = TradeTransitionPayload::InvoiceSettle {
            invoice_id: Some("INV-001".to_string()),
            invoice_ref: None,
            settled_at: "2026-02-26T10:00:00Z".to_string(),
            amount: sample_amount("USD", "50000.00"),
            settlement_corridor_id: None,
            settlement_reference: None,
            notes: None,
        };
        state = validate_transition(ft, state, &p).expect("invoice.settle");
        assert_eq!(state, TradeFlowState::Settled);
    }

    #[test]
    fn import_flow_full_lifecycle() {
        let ft = TradeFlowType::Import;
        let mut state = TradeFlowState::Created;

        let p = TradeTransitionPayload::InvoiceIssue {
            invoice: Some(Box::new(sample_invoice())),
            invoice_ref: None,
            issued_by_party_id: None,
            notes: None,
        };
        state = validate_transition(ft, state, &p).expect("invoice.issue");

        let p = TradeTransitionPayload::InvoiceAccept {
            invoice_id: Some("INV-001".to_string()),
            invoice_ref: None,
            accepted_by_party_id: "buyer-1".to_string(),
            accepted_at: "2026-02-20T12:00:00Z".to_string(),
            status: None,
            dispute_reason: None,
            notes: None,
        };
        state = validate_transition(ft, state, &p).expect("invoice.accept");

        let p = TradeTransitionPayload::BolRelease {
            bol_id: Some("BOL-001".to_string()),
            bol_ref: None,
            released_at: "2026-02-23T10:00:00Z".to_string(),
            released_to_party_id: "buyer-1".to_string(),
            release_location: None,
            notes: None,
        };
        state = validate_transition(ft, state, &p).expect("bol.release");

        let p = TradeTransitionPayload::InvoiceSettle {
            invoice_id: Some("INV-001".to_string()),
            invoice_ref: None,
            settled_at: "2026-02-25T10:00:00Z".to_string(),
            amount: sample_amount("USD", "50000.00"),
            settlement_corridor_id: None,
            settlement_reference: None,
            notes: None,
        };
        state = validate_transition(ft, state, &p).expect("invoice.settle");
        assert_eq!(state, TradeFlowState::Settled);
    }

    #[test]
    fn open_account_flow_full_lifecycle() {
        let ft = TradeFlowType::OpenAccount;
        let mut state = TradeFlowState::Created;

        let p = TradeTransitionPayload::InvoiceIssue {
            invoice: Some(Box::new(sample_invoice())),
            invoice_ref: None,
            issued_by_party_id: None,
            notes: None,
        };
        state = validate_transition(ft, state, &p).expect("invoice.issue");

        let p = TradeTransitionPayload::BolIssue {
            bol: Some(Box::new(sample_bol())),
            bol_ref: None,
            issued_by_party_id: None,
            notes: None,
        };
        state = validate_transition(ft, state, &p).expect("bol.issue");

        let p = TradeTransitionPayload::BolRelease {
            bol_id: Some("BOL-001".to_string()),
            bol_ref: None,
            released_at: "2026-02-23T10:00:00Z".to_string(),
            released_to_party_id: "buyer-1".to_string(),
            release_location: None,
            notes: None,
        };
        state = validate_transition(ft, state, &p).expect("bol.release");

        let p = TradeTransitionPayload::InvoiceSettle {
            invoice_id: Some("INV-001".to_string()),
            invoice_ref: None,
            settled_at: "2026-02-25T10:00:00Z".to_string(),
            amount: sample_amount("USD", "50000.00"),
            settlement_corridor_id: None,
            settlement_reference: None,
            notes: None,
        };
        state = validate_transition(ft, state, &p).expect("invoice.settle");
        assert_eq!(state, TradeFlowState::Settled);
    }

    // -- Invalid transition tests --

    #[test]
    fn invalid_transition_returns_error() {
        let result = validate_transition(
            TradeFlowType::Export,
            TradeFlowState::Created,
            &TradeTransitionPayload::InvoiceSettle {
                invoice_id: Some("INV-001".to_string()),
                invoice_ref: None,
                settled_at: "2026-02-25T10:00:00Z".to_string(),
                amount: sample_amount("USD", "50000.00"),
                settlement_corridor_id: None,
                settlement_reference: None,
                notes: None,
            },
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("invalid transition"));
        assert!(err.to_string().contains("trade.invoice.settle.v1"));
    }

    #[test]
    fn cannot_transition_from_settled() {
        let result = validate_transition(
            TradeFlowType::Export,
            TradeFlowState::Settled,
            &TradeTransitionPayload::InvoiceIssue {
                invoice: None,
                invoice_ref: None,
                issued_by_party_id: None,
                notes: None,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn lc_amend_from_lc_issued() {
        let ft = TradeFlowType::LetterOfCredit;
        let state = TradeFlowState::LcIssued;

        let p = TradeTransitionPayload::LcAmend {
            lc_id: Some("LC-001".to_string()),
            lc_ref: None,
            amended_at: "2026-02-21T10:00:00Z".to_string(),
            amendment_id: "AMD-001".to_string(),
            amended_by_party_id: None,
            amendment: Some(LcAmendmentDetails {
                new_expiry_date: Some("2026-06-20".to_string()),
                new_amount: None,
                notes: None,
            }),
            amendment_ref: None,
            notes: None,
        };
        let next = validate_transition(ft, state, &p).expect("lc.amend");
        assert_eq!(next, TradeFlowState::LcAmended);
    }

    // -- Digest determinism tests --

    #[test]
    fn digest_determinism_same_invoice() {
        let invoice1 = sample_invoice();
        let invoice2 = sample_invoice();
        let d1 = compute_trade_document_digest(&invoice1).expect("d1");
        let d2 = compute_trade_document_digest(&invoice2).expect("d2");
        assert_eq!(d1.to_hex(), d2.to_hex());
    }

    #[test]
    fn digest_determinism_different_invoices() {
        let mut invoice1 = sample_invoice();
        let invoice2 = sample_invoice();
        invoice1.invoice_id = "INV-999".to_string();
        let d1 = compute_trade_document_digest(&invoice1).expect("d1");
        let d2 = compute_trade_document_digest(&invoice2).expect("d2");
        assert_ne!(d1.to_hex(), d2.to_hex());
    }

    #[test]
    fn digest_determinism_bol() {
        let bol1 = sample_bol();
        let bol2 = sample_bol();
        let d1 = compute_trade_document_digest(&bol1).expect("d1");
        let d2 = compute_trade_document_digest(&bol2).expect("d2");
        assert_eq!(d1.to_hex(), d2.to_hex());
    }

    #[test]
    fn digest_determinism_lc() {
        let lc1 = sample_lc();
        let lc2 = sample_lc();
        let d1 = compute_trade_document_digest(&lc1).expect("d1");
        let d2 = compute_trade_document_digest(&lc2).expect("d2");
        assert_eq!(d1.to_hex(), d2.to_hex());
    }

    #[test]
    fn transition_kind_strings() {
        assert_eq!(
            TradeTransitionPayload::InvoiceIssue {
                invoice: None,
                invoice_ref: None,
                issued_by_party_id: None,
                notes: None,
            }
            .kind(),
            "trade.invoice.issue.v1"
        );
        assert_eq!(
            TradeTransitionPayload::LcHonor {
                lc_id: None,
                lc_ref: None,
                decision: "honor".to_string(),
                decision_at: "2026-02-25T10:00:00Z".to_string(),
                decided_by_party_id: None,
                honor_amount: None,
                reason: None,
                settlement_reference: None,
                notes: None,
            }
            .kind(),
            "trade.lc.honor.v1"
        );
    }
}
