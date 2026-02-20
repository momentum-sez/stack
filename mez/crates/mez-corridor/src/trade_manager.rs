// SPDX-License-Identifier: BUSL-1.1
//! # Trade Flow Manager
//!
//! In-memory trade flow lifecycle manager backed by `DashMap`.
//! Manages the creation, transition, and querying of trade flows.
//!
//! Each trade flow is bound to an archetype (Export, Import, LetterOfCredit,
//! OpenAccount) and enforces transition ordering via [`validate_transition`].

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::trade::{
    compute_trade_document_digest, validate_transition, TradeError, TradeFlowState, TradeFlowType,
    TradeParty, TradeTransitionPayload,
};

// ---------------------------------------------------------------------------
// Record types
// ---------------------------------------------------------------------------

/// A single transition record within a trade flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeTransitionRecord {
    pub transition_id: Uuid,
    /// Transition kind string (e.g. "trade.invoice.issue.v1").
    pub kind: String,
    pub from_state: TradeFlowState,
    pub to_state: TradeFlowState,
    pub payload: serde_json::Value,
    /// SHA-256 hex digests of CAS-stored documents embedded in this transition.
    pub document_digests: Vec<String>,
    /// Corridor receipt digest, if produced.
    pub receipt_digest: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Complete trade flow record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeFlowRecord {
    pub flow_id: Uuid,
    pub corridor_id: Option<Uuid>,
    pub flow_type: TradeFlowType,
    pub state: TradeFlowState,
    pub seller: TradeParty,
    pub buyer: TradeParty,
    pub transitions: Vec<TradeTransitionRecord>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Trade Flow Manager
// ---------------------------------------------------------------------------

/// In-memory trade flow manager.
///
/// Thread-safe via `DashMap`. The `try_update` pattern ensures TOCTOU-free
/// state transitions: read-validate-update runs under a single write lock.
pub struct TradeFlowManager {
    flows: DashMap<Uuid, TradeFlowRecord>,
}

impl TradeFlowManager {
    /// Create a new empty manager.
    pub fn new() -> Self {
        Self {
            flows: DashMap::new(),
        }
    }

    /// Create a new trade flow.
    pub fn create_flow(
        &self,
        flow_type: TradeFlowType,
        seller: TradeParty,
        buyer: TradeParty,
    ) -> TradeFlowRecord {
        let now = Utc::now();
        let record = TradeFlowRecord {
            flow_id: Uuid::new_v4(),
            corridor_id: None,
            flow_type,
            state: TradeFlowState::Created,
            seller,
            buyer,
            transitions: Vec::new(),
            created_at: now,
            updated_at: now,
        };
        self.flows.insert(record.flow_id, record.clone());
        record
    }

    /// Submit a transition to a trade flow.
    ///
    /// Atomically validates the transition against the archetype state machine,
    /// computes document digests, records the transition, and advances the state.
    pub fn submit_transition(
        &self,
        flow_id: Uuid,
        payload: TradeTransitionPayload,
    ) -> Result<TradeFlowRecord, TradeError> {
        let mut entry = self
            .flows
            .get_mut(&flow_id)
            .ok_or_else(|| TradeError::NotFound(flow_id.to_string()))?;

        let flow = entry.value_mut();

        // Validate transition against the archetype state machine.
        let next_state = validate_transition(flow.flow_type, flow.state, &payload)?;

        // Compute document digests for embedded documents.
        let document_digests = extract_document_digests(&payload)?;

        // Serialize the payload for storage.
        let payload_value = serde_json::to_value(&payload)
            .map_err(|e| TradeError::SerializationError(e.to_string()))?;

        let kind = payload.kind().to_string();
        let from_state = flow.state;
        let now = Utc::now();

        let transition = TradeTransitionRecord {
            transition_id: Uuid::new_v4(),
            kind,
            from_state,
            to_state: next_state,
            payload: payload_value,
            document_digests,
            receipt_digest: None,
            created_at: now,
        };

        flow.state = next_state;
        flow.updated_at = now;
        flow.transitions.push(transition);

        Ok(flow.clone())
    }

    /// Get a trade flow by ID.
    pub fn get_flow(&self, flow_id: &Uuid) -> Option<TradeFlowRecord> {
        self.flows.get(flow_id).map(|r| r.value().clone())
    }

    /// List all trade flows.
    pub fn list_flows(&self) -> Vec<TradeFlowRecord> {
        self.flows.iter().map(|r| r.value().clone()).collect()
    }

    /// Insert a flow record directly (used for hydration from DB).
    pub fn insert(&self, record: TradeFlowRecord) {
        self.flows.insert(record.flow_id, record);
    }
}

impl Default for TradeFlowManager {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for TradeFlowManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TradeFlowManager")
            .field("flows_count", &self.flows.len())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract content digests from embedded documents in a transition payload.
fn extract_document_digests(payload: &TradeTransitionPayload) -> Result<Vec<String>, TradeError> {
    let mut digests = Vec::new();

    match payload {
        TradeTransitionPayload::InvoiceIssue { invoice, .. } => {
            if let Some(inv) = invoice {
                let d = compute_trade_document_digest(inv)?;
                digests.push(d.to_hex());
            }
        }
        TradeTransitionPayload::BolIssue { bol, .. } => {
            if let Some(b) = bol {
                let d = compute_trade_document_digest(b)?;
                digests.push(d.to_hex());
            }
        }
        TradeTransitionPayload::LcIssue { lc, .. } => {
            if let Some(l) = lc {
                let d = compute_trade_document_digest(l)?;
                digests.push(d.to_hex());
            }
        }
        // Non-document-issuing transitions have no embedded docs to digest.
        _ => {}
    }

    Ok(digests)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trade::{BillOfLading, BolGoods, TradeAmount, TradeInvoice};

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
            invoice_number: None,
            issue_date: "2026-02-20".to_string(),
            due_date: None,
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
            consignment_type: None,
            carrier: sample_party("carrier-1"),
            shipper: sample_party("seller-1"),
            consignee: sample_party("buyer-1"),
            notify_party: None,
            vessel_name: None,
            voyage_number: None,
            port_of_loading: "PKQCT".to_string(),
            port_of_discharge: "AEJEA".to_string(),
            place_of_receipt: None,
            place_of_delivery: None,
            freight_terms: None,
            goods: vec![BolGoods {
                description: "Textiles".to_string(),
                hs_code: None,
                marks: None,
                packages: "100 cartons".to_string(),
                gross_weight: None,
                net_weight: None,
                volume: None,
                meta: None,
            }],
            originals_count: None,
            endorsements: None,
            attachment_refs: None,
            meta: None,
        }
    }

    #[test]
    fn create_flow_initializes_correctly() {
        let manager = TradeFlowManager::new();
        let flow = manager.create_flow(
            TradeFlowType::Export,
            sample_party("seller"),
            sample_party("buyer"),
        );
        assert_eq!(flow.state, TradeFlowState::Created);
        assert_eq!(flow.flow_type, TradeFlowType::Export);
        assert!(flow.transitions.is_empty());
    }

    #[test]
    fn submit_transition_advances_state() {
        let manager = TradeFlowManager::new();
        let flow = manager.create_flow(
            TradeFlowType::Export,
            sample_party("seller"),
            sample_party("buyer"),
        );

        let payload = TradeTransitionPayload::InvoiceIssue {
            invoice: Some(sample_invoice()),
            invoice_ref: None,
            issued_by_party_id: None,
            notes: None,
        };

        let updated = manager
            .submit_transition(flow.flow_id, payload)
            .expect("transition");
        assert_eq!(updated.state, TradeFlowState::InvoiceIssued);
        assert_eq!(updated.transitions.len(), 1);
        assert!(!updated.transitions[0].document_digests.is_empty());
    }

    #[test]
    fn submit_invalid_transition_returns_error() {
        let manager = TradeFlowManager::new();
        let flow = manager.create_flow(
            TradeFlowType::Export,
            sample_party("seller"),
            sample_party("buyer"),
        );

        let payload = TradeTransitionPayload::InvoiceSettle {
            invoice_id: Some("INV-001".to_string()),
            invoice_ref: None,
            settled_at: "2026-02-25T10:00:00Z".to_string(),
            amount: sample_amount("USD", "50000.00"),
            settlement_corridor_id: None,
            settlement_reference: None,
            notes: None,
        };

        let result = manager.submit_transition(flow.flow_id, payload);
        assert!(result.is_err());
    }

    #[test]
    fn get_flow_returns_clone() {
        let manager = TradeFlowManager::new();
        let flow = manager.create_flow(
            TradeFlowType::Import,
            sample_party("seller"),
            sample_party("buyer"),
        );

        let retrieved = manager.get_flow(&flow.flow_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.as_ref().map(|f| f.flow_id), Some(flow.flow_id));
    }

    #[test]
    fn get_missing_flow_returns_none() {
        let manager = TradeFlowManager::new();
        assert!(manager.get_flow(&Uuid::new_v4()).is_none());
    }

    #[test]
    fn list_flows_returns_all() {
        let manager = TradeFlowManager::new();
        manager.create_flow(
            TradeFlowType::Export,
            sample_party("s1"),
            sample_party("b1"),
        );
        manager.create_flow(
            TradeFlowType::Import,
            sample_party("s2"),
            sample_party("b2"),
        );
        assert_eq!(manager.list_flows().len(), 2);
    }

    #[test]
    fn submit_to_missing_flow_returns_not_found() {
        let manager = TradeFlowManager::new();
        let payload = TradeTransitionPayload::InvoiceIssue {
            invoice: None,
            invoice_ref: None,
            issued_by_party_id: None,
            notes: None,
        };
        let result = manager.submit_transition(Uuid::new_v4(), payload);
        assert!(matches!(result, Err(TradeError::NotFound(_))));
    }

    #[test]
    fn export_flow_full_lifecycle_via_manager() {
        let manager = TradeFlowManager::new();
        let flow = manager.create_flow(
            TradeFlowType::Export,
            sample_party("seller-1"),
            sample_party("buyer-1"),
        );
        let fid = flow.flow_id;

        // invoice.issue
        let r = manager
            .submit_transition(
                fid,
                TradeTransitionPayload::InvoiceIssue {
                    invoice: Some(sample_invoice()),
                    invoice_ref: None,
                    issued_by_party_id: None,
                    notes: None,
                },
            )
            .expect("invoice.issue");
        assert_eq!(r.state, TradeFlowState::InvoiceIssued);

        // invoice.accept
        let r = manager
            .submit_transition(
                fid,
                TradeTransitionPayload::InvoiceAccept {
                    invoice_id: Some("INV-001".to_string()),
                    invoice_ref: None,
                    accepted_by_party_id: "buyer-1".to_string(),
                    accepted_at: "2026-02-20T12:00:00Z".to_string(),
                    status: None,
                    dispute_reason: None,
                    notes: None,
                },
            )
            .expect("invoice.accept");
        assert_eq!(r.state, TradeFlowState::InvoiceAccepted);

        // bol.issue
        let r = manager
            .submit_transition(
                fid,
                TradeTransitionPayload::BolIssue {
                    bol: Some(sample_bol()),
                    bol_ref: None,
                    issued_by_party_id: None,
                    notes: None,
                },
            )
            .expect("bol.issue");
        assert_eq!(r.state, TradeFlowState::GoodsShipped);

        // bol.endorse
        let r = manager
            .submit_transition(
                fid,
                TradeTransitionPayload::BolEndorse {
                    bol_id: Some("BOL-001".to_string()),
                    bol_ref: None,
                    from_party_id: "seller-1".to_string(),
                    to_party_id: "buyer-1".to_string(),
                    endorsed_at: "2026-02-22T10:00:00Z".to_string(),
                    endorsement_type: None,
                    notes: None,
                },
            )
            .expect("bol.endorse");
        assert_eq!(r.state, TradeFlowState::BolEndorsed);

        // bol.release
        let r = manager
            .submit_transition(
                fid,
                TradeTransitionPayload::BolRelease {
                    bol_id: Some("BOL-001".to_string()),
                    bol_ref: None,
                    released_at: "2026-02-23T10:00:00Z".to_string(),
                    released_to_party_id: "buyer-1".to_string(),
                    release_location: None,
                    notes: None,
                },
            )
            .expect("bol.release");
        assert_eq!(r.state, TradeFlowState::GoodsReleased);

        // invoice.settle
        let r = manager
            .submit_transition(
                fid,
                TradeTransitionPayload::InvoiceSettle {
                    invoice_id: Some("INV-001".to_string()),
                    invoice_ref: None,
                    settled_at: "2026-02-25T10:00:00Z".to_string(),
                    amount: sample_amount("USD", "50000.00"),
                    settlement_corridor_id: None,
                    settlement_reference: None,
                    notes: None,
                },
            )
            .expect("invoice.settle");
        assert_eq!(r.state, TradeFlowState::Settled);
        assert_eq!(r.transitions.len(), 6);

        // Verify all transition records
        let flow = manager.get_flow(&fid).expect("flow exists");
        assert_eq!(flow.transitions[0].kind, "trade.invoice.issue.v1");
        assert_eq!(flow.transitions[5].kind, "trade.invoice.settle.v1");
    }
}
