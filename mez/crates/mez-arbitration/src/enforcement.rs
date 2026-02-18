//! # Award Enforcement
//!
//! Enforces arbitration awards with corridor receipt generation for
//! cross-border dispute resolution.
//!
//! ## Enforcement Model
//!
//! An [`EnforcementOrder`] is created after a dispute reaches the `Decided`
//! state. Each order contains one or more [`EnforcementAction`]s that are
//! executed sequentially. As each action completes, an
//! [`EnforcementReceipt`] is generated with a content-addressed digest
//! for inclusion in the corridor receipt chain.
//!
//! Corridor-level enforcement can suspend corridor operations pending
//! resolution via the `CorridorSuspension` action type.
//!
//! ## Security Invariant
//!
//! Enforcement orders are immutable after creation. All receipts are
//! content-addressed via `CanonicalBytes` → `sha256_digest()`. The
//! enforcement log provides a tamper-evident audit trail. Preconditions
//! (e.g., appeal period expiration) are checked before execution.
//!
//! ## Spec Reference
//!
//! Implements Definition 26.8 (Award Enforcement) from the specification.
//! Enforcement actions and receipt chain match the Python
//! `tools/arbitration.py` enforcement handling.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use mez_core::{sha256_digest, CanonicalBytes, ContentDigest, CorridorId, Did, Timestamp};

use crate::dispute::DisputeId;
use crate::error::ArbitrationError;
use crate::escrow::EscrowId;

// ── Identifiers ────────────────────────────────────────────────────────

/// A unique identifier for an enforcement order.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EnforcementOrderId(Uuid);

impl EnforcementOrderId {
    /// Create a new random enforcement order identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from an existing UUID.
    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Access the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for EnforcementOrderId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EnforcementOrderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "enforcement:{}", self.0)
    }
}

/// A unique identifier for an enforcement receipt.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EnforcementReceiptId(Uuid);

impl EnforcementReceiptId {
    /// Create a new random enforcement receipt identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from an existing UUID.
    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Access the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for EnforcementReceiptId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EnforcementReceiptId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "enforcement-receipt:{}", self.0)
    }
}

// ── Enforcement Action Types ─────────────────────────────────────────

/// The type of enforcement action to execute.
///
/// Each variant corresponds to a distinct enforcement mechanism available
/// within the EZ Stack. Actions are executed as part of an
/// [`EnforcementOrder`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnforcementAction {
    /// Release funds from escrow to the prevailing party.
    ///
    /// Triggers an escrow release operation with the associated condition.
    EscrowRelease {
        /// The escrow account to release funds from.
        escrow_id: EscrowId,
        /// DID of the beneficiary receiving the funds.
        beneficiary: Did,
        /// Amount to release (string for precision). If `None`, release full balance.
        amount: Option<String>,
    },

    /// Suspend the respondent's operating license in the jurisdiction.
    LicenseSuspension {
        /// Identifier of the license to suspend.
        license_id: String,
        /// Reason for suspension.
        reason: String,
    },

    /// Suspend corridor operations pending enforcement completion.
    ///
    /// This is the corridor-level enforcement mechanism. A suspended corridor
    /// will not process new transactions until the suspension is lifted.
    CorridorSuspension {
        /// The corridor to suspend.
        corridor_id: CorridorId,
        /// Reason for the suspension.
        reason: String,
    },

    /// Generate a corridor receipt recording the enforcement for the
    /// corridor receipt chain.
    CorridorReceiptGeneration {
        /// The corridor to record the enforcement receipt in.
        corridor_id: CorridorId,
    },

    /// Transfer an asset or right to the prevailing party.
    AssetTransfer {
        /// Digest of the asset to transfer.
        asset_digest: ContentDigest,
        /// DID of the party receiving the asset.
        recipient: Did,
    },

    /// Monetary penalty imposed on a party.
    MonetaryPenalty {
        /// DID of the party being penalized.
        party: Did,
        /// Penalty amount (string for precision).
        amount: String,
        /// Currency code (ISO 4217).
        currency: String,
    },
}

impl std::fmt::Display for EnforcementAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EscrowRelease { escrow_id, .. } => {
                write!(f, "escrow_release:{escrow_id}")
            }
            Self::LicenseSuspension { license_id, .. } => {
                write!(f, "license_suspension:{license_id}")
            }
            Self::CorridorSuspension { corridor_id, .. } => {
                write!(f, "corridor_suspension:{corridor_id}")
            }
            Self::CorridorReceiptGeneration { corridor_id } => {
                write!(f, "corridor_receipt:{corridor_id}")
            }
            Self::AssetTransfer { recipient, .. } => {
                write!(f, "asset_transfer:{recipient}")
            }
            Self::MonetaryPenalty {
                party,
                amount,
                currency,
                ..
            } => {
                write!(f, "monetary_penalty:{party}:{amount}{currency}")
            }
        }
    }
}

// ── Enforcement Status ───────────────────────────────────────────────

/// The execution status of an enforcement order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnforcementStatus {
    /// Order created, awaiting precondition checks.
    Pending,
    /// Preconditions met, actions are being executed.
    InProgress,
    /// All actions have been executed successfully.
    Completed,
    /// Enforcement was blocked by a precondition failure.
    Blocked,
    /// Enforcement was cancelled (e.g., settlement after decision).
    Cancelled,
}

impl EnforcementStatus {
    /// Whether this status is terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled)
    }

    /// The canonical string name.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::InProgress => "IN_PROGRESS",
            Self::Completed => "COMPLETED",
            Self::Blocked => "BLOCKED",
            Self::Cancelled => "CANCELLED",
        }
    }
}

impl std::fmt::Display for EnforcementStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── Enforcement Precondition ─────────────────────────────────────────

/// A precondition that must be met before enforcement can proceed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcementPrecondition {
    /// Description of the precondition.
    pub description: String,
    /// Whether this precondition has been satisfied.
    pub satisfied: bool,
    /// Digest of the evidence that the precondition was satisfied.
    pub evidence_digest: Option<ContentDigest>,
    /// When the precondition was checked.
    pub checked_at: Option<Timestamp>,
}

// ── Enforcement Receipt ──────────────────────────────────────────────

/// A receipt recording the execution of an enforcement action.
///
/// Receipts are content-addressed for inclusion in the corridor receipt
/// chain, providing a tamper-evident record of enforcement actions across
/// jurisdictions.
///
/// ## Security Invariant
///
/// The receipt digest is computed via `CanonicalBytes` → `sha256_digest()`
/// from the receipt's serializable content. This ensures corridor nodes
/// can independently verify the receipt's integrity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcementReceipt {
    /// Unique receipt identifier.
    pub id: EnforcementReceiptId,
    /// The enforcement order this receipt belongs to.
    pub order_id: EnforcementOrderId,
    /// The action that was executed.
    pub action: EnforcementAction,
    /// Whether the action succeeded.
    pub success: bool,
    /// Human-readable detail of the execution result.
    pub detail: String,
    /// When the action was executed.
    pub executed_at: DateTime<Utc>,
    /// Content digest of this receipt for chain inclusion.
    pub receipt_digest: ContentDigest,
}

/// Content used to compute the receipt digest.
///
/// This is the serializable representation of the receipt's essential
/// fields. The digest is computed from this structure, not the full
/// receipt (which would create a circular dependency with `receipt_digest`).
#[derive(Debug, Serialize)]
struct ReceiptDigestContent {
    order_id: String,
    action: String,
    success: bool,
    detail: String,
    executed_at: String,
}

// ── Enforcement Order ────────────────────────────────────────────────

/// An enforcement order for an arbitration award.
///
/// Created after a dispute reaches the `Decided` state. Contains the
/// award digest, the actions to execute, preconditions that must be met,
/// and the generated receipts.
///
/// ## Security Invariant
///
/// The order tracks its status through `Pending → InProgress → Completed`.
/// Preconditions are checked before execution begins. Each executed action
/// generates a content-addressed receipt. Terminal statuses reject further
/// operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcementOrder {
    /// Unique order identifier.
    pub id: EnforcementOrderId,
    /// The dispute this enforcement is for.
    pub dispute_id: DisputeId,
    /// Digest of the arbitration award being enforced.
    pub award_digest: ContentDigest,
    /// The enforcement actions to execute.
    pub actions: Vec<EnforcementAction>,
    /// Preconditions that must be met before execution.
    pub preconditions: Vec<EnforcementPrecondition>,
    /// Current execution status.
    pub status: EnforcementStatus,
    /// Receipts for completed actions.
    pub receipts: Vec<EnforcementReceipt>,
    /// When the order was created.
    pub created_at: Timestamp,
    /// When the order was last updated.
    pub updated_at: Timestamp,
    /// Optional appeal period deadline. Enforcement cannot begin until
    /// this deadline has passed (if set).
    pub appeal_deadline: Option<DateTime<Utc>>,
}

impl EnforcementOrder {
    /// Create a new enforcement order tied to a dispute and award.
    ///
    /// The order starts in [`Pending`](EnforcementStatus::Pending) status.
    /// Actions and preconditions must be added before execution.
    pub fn new(
        dispute_id: DisputeId,
        award_digest: ContentDigest,
        actions: Vec<EnforcementAction>,
        appeal_deadline: Option<DateTime<Utc>>,
    ) -> Self {
        let now = Timestamp::now();
        Self {
            id: EnforcementOrderId::new(),
            dispute_id,
            award_digest,
            actions,
            preconditions: Vec::new(),
            status: EnforcementStatus::Pending,
            receipts: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
            appeal_deadline,
        }
    }

    /// Add a precondition that must be satisfied before enforcement begins.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::EnforcementPreconditionFailed`] if the
    /// order is not in Pending status.
    pub fn add_precondition(&mut self, description: String) -> Result<(), ArbitrationError> {
        if self.status != EnforcementStatus::Pending {
            return Err(ArbitrationError::EnforcementPreconditionFailed {
                order_id: self.id.to_string(),
                reason: format!("cannot add preconditions in {} status", self.status),
            });
        }
        self.preconditions.push(EnforcementPrecondition {
            description,
            satisfied: false,
            evidence_digest: None,
            checked_at: None,
        });
        Ok(())
    }

    /// Satisfy a precondition by index with evidence.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::EnforcementPreconditionFailed`] if the
    /// index is out of bounds or the order is not in Pending status.
    pub fn satisfy_precondition(
        &mut self,
        index: usize,
        evidence_digest: ContentDigest,
    ) -> Result<(), ArbitrationError> {
        if self.status != EnforcementStatus::Pending {
            return Err(ArbitrationError::EnforcementPreconditionFailed {
                order_id: self.id.to_string(),
                reason: format!("cannot satisfy preconditions in {} status", self.status),
            });
        }
        let precondition = self.preconditions.get_mut(index).ok_or_else(|| {
            ArbitrationError::EnforcementPreconditionFailed {
                order_id: self.id.to_string(),
                reason: format!("precondition index {} out of bounds", index),
            }
        })?;
        precondition.satisfied = true;
        precondition.evidence_digest = Some(evidence_digest);
        precondition.checked_at = Some(Timestamp::now());
        self.updated_at = Timestamp::now();
        Ok(())
    }

    /// Check whether all preconditions are satisfied and the appeal
    /// deadline (if any) has passed.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::EnforcementPreconditionFailed`] with
    /// a descriptive reason if any precondition is unsatisfied or the
    /// appeal period has not expired.
    pub fn check_preconditions(&self) -> Result<(), ArbitrationError> {
        // Check appeal deadline
        if let Some(deadline) = self.appeal_deadline {
            if Utc::now() < deadline {
                return Err(ArbitrationError::EnforcementPreconditionFailed {
                    order_id: self.id.to_string(),
                    reason: format!(
                        "appeal period has not expired (deadline: {})",
                        deadline.to_rfc3339()
                    ),
                });
            }
        }

        // Check all preconditions
        for (i, precondition) in self.preconditions.iter().enumerate() {
            if !precondition.satisfied {
                return Err(ArbitrationError::EnforcementPreconditionFailed {
                    order_id: self.id.to_string(),
                    reason: format!(
                        "precondition {} not satisfied: {}",
                        i, precondition.description
                    ),
                });
            }
        }

        Ok(())
    }

    /// Begin enforcement execution.
    ///
    /// Transitions Pending → InProgress after verifying all preconditions
    /// are satisfied and the appeal deadline has passed.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::EnforcementPreconditionFailed`] if
    /// preconditions are not met or if the order is not in Pending status.
    pub fn begin_enforcement(&mut self) -> Result<(), ArbitrationError> {
        if self.status != EnforcementStatus::Pending {
            return Err(ArbitrationError::EnforcementPreconditionFailed {
                order_id: self.id.to_string(),
                reason: format!("cannot begin enforcement in {} status", self.status),
            });
        }

        self.check_preconditions()?;

        self.status = EnforcementStatus::InProgress;
        self.updated_at = Timestamp::now();
        Ok(())
    }

    /// Record the execution of an enforcement action, generating a
    /// content-addressed receipt.
    ///
    /// Each call records one action's result. After all actions are
    /// recorded, call [`complete`](EnforcementOrder::complete) to
    /// finalize the order.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::EnforcementPreconditionFailed`] if the
    /// order is not in InProgress status.
    /// Returns [`ArbitrationError::Canonicalization`] if receipt digest
    /// computation fails.
    pub fn record_action_result(
        &mut self,
        action: EnforcementAction,
        success: bool,
        detail: String,
    ) -> Result<EnforcementReceipt, ArbitrationError> {
        if self.status != EnforcementStatus::InProgress {
            return Err(ArbitrationError::EnforcementPreconditionFailed {
                order_id: self.id.to_string(),
                reason: format!("cannot record action results in {} status", self.status),
            });
        }

        let executed_at = Utc::now();

        let digest_content = ReceiptDigestContent {
            order_id: self.id.to_string(),
            action: action.to_string(),
            success,
            detail: detail.clone(),
            executed_at: executed_at.to_rfc3339(),
        };
        let canonical = CanonicalBytes::new(&digest_content)?;
        let receipt_digest = sha256_digest(&canonical);

        let receipt = EnforcementReceipt {
            id: EnforcementReceiptId::new(),
            order_id: self.id.clone(),
            action,
            success,
            detail,
            executed_at,
            receipt_digest,
        };

        self.receipts.push(receipt.clone());
        self.updated_at = Timestamp::now();

        Ok(receipt)
    }

    /// Complete the enforcement order.
    ///
    /// Transitions InProgress → Completed. All actions should have been
    /// recorded via [`record_action_result`](EnforcementOrder::record_action_result)
    /// before calling this method.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::EnforcementPreconditionFailed`] if the
    /// order is not in InProgress status.
    pub fn complete(&mut self) -> Result<(), ArbitrationError> {
        if self.status != EnforcementStatus::InProgress {
            return Err(ArbitrationError::EnforcementPreconditionFailed {
                order_id: self.id.to_string(),
                reason: format!("cannot complete enforcement in {} status", self.status),
            });
        }
        self.status = EnforcementStatus::Completed;
        self.updated_at = Timestamp::now();
        Ok(())
    }

    /// Cancel the enforcement order.
    ///
    /// Can only be called from Pending status. Blocked orders cannot be
    /// cancelled as this could bypass the appeals process. InProgress
    /// orders should be completed, not cancelled.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::EnforcementPreconditionFailed`] if the
    /// order is not in a cancellable status.
    pub fn cancel(&mut self) -> Result<(), ArbitrationError> {
        if self.status != EnforcementStatus::Pending {
            return Err(ArbitrationError::EnforcementPreconditionFailed {
                order_id: self.id.to_string(),
                reason: format!("cannot cancel enforcement in {} status", self.status),
            });
        }
        self.status = EnforcementStatus::Cancelled;
        self.updated_at = Timestamp::now();
        Ok(())
    }

    /// Mark the order as blocked due to a precondition failure.
    ///
    /// Can be called from Pending or InProgress status.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::EnforcementPreconditionFailed`] if the
    /// order is not in Pending or InProgress status.
    pub fn block(&mut self, reason: &str) -> Result<(), ArbitrationError> {
        if !matches!(
            self.status,
            EnforcementStatus::Pending | EnforcementStatus::InProgress
        ) {
            return Err(ArbitrationError::EnforcementPreconditionFailed {
                order_id: self.id.to_string(),
                reason: format!(
                    "cannot block enforcement in {} status: {reason}",
                    self.status
                ),
            });
        }
        self.status = EnforcementStatus::Blocked;
        self.updated_at = Timestamp::now();
        Ok(())
    }

    /// Return the number of receipts generated so far.
    pub fn receipt_count(&self) -> usize {
        self.receipts.len()
    }

    /// Return the number of successful action receipts.
    pub fn successful_action_count(&self) -> usize {
        self.receipts.iter().filter(|r| r.success).count()
    }

    /// Collect all receipt digests for corridor chain inclusion.
    ///
    /// Returns a vector of content digests from all receipts, suitable
    /// for anchoring into the corridor receipt chain.
    pub fn receipt_digests(&self) -> Vec<ContentDigest> {
        self.receipts
            .iter()
            .map(|r| r.receipt_digest.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispute::DisputeId;
    use crate::escrow::EscrowId;
    use mez_core::{sha256_digest as core_sha256, CanonicalBytes, CorridorId, Did};
    use serde_json::json;

    fn test_digest() -> ContentDigest {
        let canonical = CanonicalBytes::new(&json!({"test": "enforcement"})).unwrap();
        core_sha256(&canonical)
    }

    fn test_did(name: &str) -> Did {
        Did::new(format!("did:key:z6Mk{name}")).unwrap()
    }

    fn basic_order() -> EnforcementOrder {
        EnforcementOrder::new(
            DisputeId::new(),
            test_digest(),
            vec![EnforcementAction::EscrowRelease {
                escrow_id: EscrowId::new(),
                beneficiary: test_did("Claimant123"),
                amount: None,
            }],
            None,
        )
    }

    fn order_with_appeal_deadline(deadline: DateTime<Utc>) -> EnforcementOrder {
        EnforcementOrder::new(
            DisputeId::new(),
            test_digest(),
            vec![EnforcementAction::EscrowRelease {
                escrow_id: EscrowId::new(),
                beneficiary: test_did("Claimant123"),
                amount: None,
            }],
            Some(deadline),
        )
    }

    #[test]
    fn create_enforcement_order_in_pending_status() {
        let order = basic_order();
        assert_eq!(order.status, EnforcementStatus::Pending);
        assert_eq!(order.actions.len(), 1);
        assert!(order.receipts.is_empty());
    }

    #[test]
    fn begin_enforcement_transitions_to_in_progress() {
        let mut order = basic_order();
        order.begin_enforcement().unwrap();
        assert_eq!(order.status, EnforcementStatus::InProgress);
    }

    #[test]
    fn full_enforcement_lifecycle() {
        let mut order = EnforcementOrder::new(
            DisputeId::new(),
            test_digest(),
            vec![
                EnforcementAction::EscrowRelease {
                    escrow_id: EscrowId::new(),
                    beneficiary: test_did("Winner"),
                    amount: Some("150000".to_string()),
                },
                EnforcementAction::CorridorReceiptGeneration {
                    corridor_id: CorridorId::new(),
                },
            ],
            None,
        );

        assert_eq!(order.status, EnforcementStatus::Pending);

        order.begin_enforcement().unwrap();
        assert_eq!(order.status, EnforcementStatus::InProgress);

        let receipt1 = order
            .record_action_result(
                EnforcementAction::EscrowRelease {
                    escrow_id: EscrowId::new(),
                    beneficiary: test_did("Winner"),
                    amount: Some("150000".to_string()),
                },
                true,
                "Escrow released to beneficiary".to_string(),
            )
            .unwrap();
        assert!(receipt1.success);
        assert_eq!(receipt1.receipt_digest.to_hex().len(), 64);

        let receipt2 = order
            .record_action_result(
                EnforcementAction::CorridorReceiptGeneration {
                    corridor_id: CorridorId::new(),
                },
                true,
                "Corridor receipt generated".to_string(),
            )
            .unwrap();
        assert!(receipt2.success);

        order.complete().unwrap();
        assert_eq!(order.status, EnforcementStatus::Completed);
        assert!(order.status.is_terminal());
        assert_eq!(order.receipt_count(), 2);
        assert_eq!(order.successful_action_count(), 2);
    }

    #[test]
    fn precondition_enforcement() {
        let mut order = basic_order();
        order
            .add_precondition("Appeal period must expire".to_string())
            .unwrap();

        // Cannot begin with unsatisfied preconditions
        let result = order.begin_enforcement();
        assert!(result.is_err());

        // Satisfy the precondition
        order.satisfy_precondition(0, test_digest()).unwrap();

        // Now can begin enforcement
        order.begin_enforcement().unwrap();
        assert_eq!(order.status, EnforcementStatus::InProgress);
    }

    #[test]
    fn appeal_deadline_blocks_enforcement() {
        let future_deadline = Utc::now() + chrono::Duration::hours(24);
        let mut order = order_with_appeal_deadline(future_deadline);

        let result = order.begin_enforcement();
        assert!(result.is_err());
        assert_eq!(order.status, EnforcementStatus::Pending);
    }

    #[test]
    fn past_appeal_deadline_allows_enforcement() {
        let past_deadline = Utc::now() - chrono::Duration::hours(1);
        let mut order = order_with_appeal_deadline(past_deadline);

        order.begin_enforcement().unwrap();
        assert_eq!(order.status, EnforcementStatus::InProgress);
    }

    #[test]
    fn record_action_result_rejected_when_pending() {
        let mut order = basic_order();
        let result = order.record_action_result(
            EnforcementAction::EscrowRelease {
                escrow_id: EscrowId::new(),
                beneficiary: test_did("Test"),
                amount: None,
            },
            true,
            "Should fail".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn complete_rejected_when_pending() {
        let mut order = basic_order();
        let result = order.complete();
        assert!(result.is_err());
    }

    #[test]
    fn cancel_from_pending() {
        let mut order = basic_order();
        order.cancel().unwrap();
        assert_eq!(order.status, EnforcementStatus::Cancelled);
        assert!(order.status.is_terminal());
    }

    #[test]
    fn cancel_rejected_from_blocked() {
        let mut order = basic_order();
        order.block("test block").unwrap();
        assert_eq!(order.status, EnforcementStatus::Blocked);

        let result = order.cancel();
        assert!(result.is_err());
        assert_eq!(order.status, EnforcementStatus::Blocked);
    }

    #[test]
    fn cancel_rejected_when_in_progress() {
        let mut order = basic_order();
        order.begin_enforcement().unwrap();
        let result = order.cancel();
        assert!(result.is_err());
    }

    #[test]
    fn corridor_suspension_action() {
        let mut order = EnforcementOrder::new(
            DisputeId::new(),
            test_digest(),
            vec![EnforcementAction::CorridorSuspension {
                corridor_id: CorridorId::new(),
                reason: "Pending enforcement of arbitration award".to_string(),
            }],
            None,
        );

        order.begin_enforcement().unwrap();
        let receipt = order
            .record_action_result(
                EnforcementAction::CorridorSuspension {
                    corridor_id: CorridorId::new(),
                    reason: "Pending enforcement of arbitration award".to_string(),
                },
                true,
                "Corridor suspended".to_string(),
            )
            .unwrap();

        assert!(receipt.success);
        assert_eq!(receipt.receipt_digest.to_hex().len(), 64);
    }

    #[test]
    fn receipt_digests_collection() {
        let mut order = basic_order();
        order.begin_enforcement().unwrap();

        order
            .record_action_result(
                EnforcementAction::EscrowRelease {
                    escrow_id: EscrowId::new(),
                    beneficiary: test_did("Test"),
                    amount: None,
                },
                true,
                "Released".to_string(),
            )
            .unwrap();

        order
            .record_action_result(
                EnforcementAction::CorridorReceiptGeneration {
                    corridor_id: CorridorId::new(),
                },
                true,
                "Receipt generated".to_string(),
            )
            .unwrap();

        let digests = order.receipt_digests();
        assert_eq!(digests.len(), 2);
        // Each digest should be unique
        assert_ne!(digests[0], digests[1]);
    }

    #[test]
    fn enforcement_order_serialization_roundtrip() {
        let order = basic_order();
        let json_str = serde_json::to_string(&order).unwrap();
        let deserialized: EnforcementOrder = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.id, order.id);
        assert_eq!(deserialized.status, order.status);
        assert_eq!(deserialized.actions.len(), order.actions.len());
    }

    #[test]
    fn enforcement_receipt_digest_is_deterministic() {
        let mut order1 = basic_order();
        order1.begin_enforcement().unwrap();

        let mut order2 = basic_order();
        order2.begin_enforcement().unwrap();

        // Receipts from different orders should have different digests
        // because the order_id differs
        let receipt1 = order1
            .record_action_result(
                EnforcementAction::EscrowRelease {
                    escrow_id: EscrowId::new(),
                    beneficiary: test_did("Test"),
                    amount: None,
                },
                true,
                "Released".to_string(),
            )
            .unwrap();

        let receipt2 = order2
            .record_action_result(
                EnforcementAction::EscrowRelease {
                    escrow_id: EscrowId::new(),
                    beneficiary: test_did("Test"),
                    amount: None,
                },
                true,
                "Released".to_string(),
            )
            .unwrap();

        // Different order IDs produce different receipt digests
        assert_ne!(receipt1.receipt_digest, receipt2.receipt_digest);
    }

    #[test]
    fn satisfy_precondition_out_of_bounds() {
        let mut order = basic_order();
        let result = order.satisfy_precondition(0, test_digest());
        assert!(result.is_err());
    }

    #[test]
    fn terminal_status_rejects_operations() {
        let mut order = basic_order();
        order.cancel().unwrap();
        assert!(order.status.is_terminal());

        assert!(order.begin_enforcement().is_err());
        assert!(order.add_precondition("Test".to_string()).is_err());
    }

    #[test]
    fn enforcement_action_display() {
        let action = EnforcementAction::CorridorSuspension {
            corridor_id: CorridorId::new(),
            reason: "Test".to_string(),
        };
        let display = format!("{action}");
        assert!(display.starts_with("corridor_suspension:"));
    }

    #[test]
    fn enforcement_with_failed_action() {
        let mut order = basic_order();
        order.begin_enforcement().unwrap();

        let receipt = order
            .record_action_result(
                EnforcementAction::LicenseSuspension {
                    license_id: "LIC-001".to_string(),
                    reason: "Award enforcement".to_string(),
                },
                false,
                "License suspension rejected by jurisdiction".to_string(),
            )
            .unwrap();

        assert!(!receipt.success);
        assert_eq!(order.successful_action_count(), 0);
        assert_eq!(order.receipt_count(), 1);
    }

    // ── Coverage expansion tests ─────────────────────────────────────

    #[test]
    fn enforcement_order_id_default() {
        let id1 = EnforcementOrderId::default();
        let id2 = EnforcementOrderId::default();
        assert_ne!(id1, id2);
    }

    #[test]
    fn enforcement_order_id_display() {
        let id = EnforcementOrderId::new();
        let display = format!("{id}");
        assert!(display.starts_with("enforcement:"));
    }

    #[test]
    fn enforcement_receipt_id_default() {
        let id1 = EnforcementReceiptId::default();
        let id2 = EnforcementReceiptId::default();
        assert_ne!(id1, id2);
    }

    #[test]
    fn enforcement_receipt_id_display() {
        let id = EnforcementReceiptId::new();
        let display = format!("{id}");
        assert!(display.starts_with("enforcement-receipt:"));
    }

    #[test]
    fn enforcement_receipt_id_from_uuid_roundtrip() {
        let uuid = uuid::Uuid::new_v4();
        let id = EnforcementReceiptId::from_uuid(uuid);
        assert_eq!(*id.as_uuid(), uuid);
    }

    #[test]
    fn enforcement_action_display_all_variants() {
        let escrow_release = EnforcementAction::EscrowRelease {
            escrow_id: EscrowId::new(),
            beneficiary: test_did("Test"),
            amount: Some("1000".to_string()),
        };
        assert!(format!("{escrow_release}").starts_with("escrow_release:"));

        let license_suspension = EnforcementAction::LicenseSuspension {
            license_id: "LIC-001".to_string(),
            reason: "test".to_string(),
        };
        assert!(format!("{license_suspension}").starts_with("license_suspension:"));

        let corridor_suspension = EnforcementAction::CorridorSuspension {
            corridor_id: CorridorId::new(),
            reason: "test".to_string(),
        };
        assert!(format!("{corridor_suspension}").starts_with("corridor_suspension:"));

        let corridor_receipt = EnforcementAction::CorridorReceiptGeneration {
            corridor_id: CorridorId::new(),
        };
        assert!(format!("{corridor_receipt}").starts_with("corridor_receipt:"));

        let asset_transfer = EnforcementAction::AssetTransfer {
            asset_digest: test_digest(),
            recipient: test_did("Recipient"),
        };
        assert!(format!("{asset_transfer}").starts_with("asset_transfer:"));

        let monetary_penalty = EnforcementAction::MonetaryPenalty {
            party: test_did("Penalized"),
            amount: "50000".to_string(),
            currency: "USD".to_string(),
        };
        let penalty_display = format!("{monetary_penalty}");
        assert!(penalty_display.starts_with("monetary_penalty:"));
        assert!(penalty_display.contains("50000USD"));
    }

    #[test]
    fn enforcement_status_display_all_variants() {
        assert_eq!(format!("{}", EnforcementStatus::Pending), "PENDING");
        assert_eq!(format!("{}", EnforcementStatus::InProgress), "IN_PROGRESS");
        assert_eq!(format!("{}", EnforcementStatus::Completed), "COMPLETED");
        assert_eq!(format!("{}", EnforcementStatus::Blocked), "BLOCKED");
        assert_eq!(format!("{}", EnforcementStatus::Cancelled), "CANCELLED");
    }

    #[test]
    fn enforcement_status_as_str_all_variants() {
        assert_eq!(EnforcementStatus::Pending.as_str(), "PENDING");
        assert_eq!(EnforcementStatus::InProgress.as_str(), "IN_PROGRESS");
        assert_eq!(EnforcementStatus::Completed.as_str(), "COMPLETED");
        assert_eq!(EnforcementStatus::Blocked.as_str(), "BLOCKED");
        assert_eq!(EnforcementStatus::Cancelled.as_str(), "CANCELLED");
    }

    #[test]
    fn enforcement_status_is_terminal() {
        assert!(!EnforcementStatus::Pending.is_terminal());
        assert!(!EnforcementStatus::InProgress.is_terminal());
        assert!(EnforcementStatus::Completed.is_terminal());
        assert!(!EnforcementStatus::Blocked.is_terminal());
        assert!(EnforcementStatus::Cancelled.is_terminal());
    }

    #[test]
    fn satisfy_precondition_rejected_when_in_progress() {
        let mut order = basic_order();
        order.add_precondition("Test".to_string()).unwrap();
        order.satisfy_precondition(0, test_digest()).unwrap();
        order.begin_enforcement().unwrap();

        let result = order.satisfy_precondition(0, test_digest());
        assert!(result.is_err());
    }

    #[test]
    fn block_from_pending() {
        let mut order = basic_order();
        order.block("precondition not met").unwrap();
        assert_eq!(order.status, EnforcementStatus::Blocked);
    }

    #[test]
    fn block_from_in_progress() {
        let mut order = basic_order();
        order.begin_enforcement().unwrap();
        order
            .block("precondition discovered during execution")
            .unwrap();
        assert_eq!(order.status, EnforcementStatus::Blocked);
    }

    #[test]
    fn block_rejected_when_completed() {
        let mut order = basic_order();
        order.begin_enforcement().unwrap();
        order.complete().unwrap();
        let result = order.block("should fail");
        assert!(result.is_err());
    }

    #[test]
    fn cancel_rejected_when_completed() {
        let mut order = basic_order();
        order.begin_enforcement().unwrap();
        order.complete().unwrap();
        let result = order.cancel();
        assert!(result.is_err());
    }

    #[test]
    fn add_precondition_rejected_when_in_progress() {
        let mut order = basic_order();
        order.begin_enforcement().unwrap();
        let result = order.add_precondition("Should fail".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn complete_rejected_when_completed() {
        let mut order = basic_order();
        order.begin_enforcement().unwrap();
        order.complete().unwrap();
        let result = order.complete();
        assert!(result.is_err());
    }

    #[test]
    fn record_action_result_rejected_when_completed() {
        let mut order = basic_order();
        order.begin_enforcement().unwrap();
        order.complete().unwrap();
        let result = order.record_action_result(
            EnforcementAction::EscrowRelease {
                escrow_id: EscrowId::new(),
                beneficiary: test_did("Test"),
                amount: None,
            },
            true,
            "Should fail".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn multiple_preconditions() {
        let mut order = basic_order();
        order
            .add_precondition("Appeal period must expire".to_string())
            .unwrap();
        order
            .add_precondition("Bond must be posted".to_string())
            .unwrap();

        // Cannot begin with unsatisfied preconditions
        assert!(order.begin_enforcement().is_err());

        // Satisfy only first
        order.satisfy_precondition(0, test_digest()).unwrap();
        assert!(order.begin_enforcement().is_err());

        // Satisfy second
        order.satisfy_precondition(1, test_digest()).unwrap();
        order.begin_enforcement().unwrap();
        assert_eq!(order.status, EnforcementStatus::InProgress);
    }

    #[test]
    fn asset_transfer_action_in_order() {
        let mut order = EnforcementOrder::new(
            DisputeId::new(),
            test_digest(),
            vec![EnforcementAction::AssetTransfer {
                asset_digest: test_digest(),
                recipient: test_did("Recipient"),
            }],
            None,
        );
        order.begin_enforcement().unwrap();
        let receipt = order
            .record_action_result(
                EnforcementAction::AssetTransfer {
                    asset_digest: test_digest(),
                    recipient: test_did("Recipient"),
                },
                true,
                "Asset transferred".to_string(),
            )
            .unwrap();
        assert!(receipt.success);
    }

    #[test]
    fn monetary_penalty_action_in_order() {
        let mut order = EnforcementOrder::new(
            DisputeId::new(),
            test_digest(),
            vec![EnforcementAction::MonetaryPenalty {
                party: test_did("Penalized"),
                amount: "50000".to_string(),
                currency: "USD".to_string(),
            }],
            None,
        );
        order.begin_enforcement().unwrap();
        let receipt = order
            .record_action_result(
                EnforcementAction::MonetaryPenalty {
                    party: test_did("Penalized"),
                    amount: "50000".to_string(),
                    currency: "USD".to_string(),
                },
                true,
                "Penalty applied".to_string(),
            )
            .unwrap();
        assert!(receipt.success);
        assert_eq!(order.successful_action_count(), 1);
    }

    #[test]
    fn begin_enforcement_rejected_when_blocked() {
        let mut order = basic_order();
        order.block("test block").unwrap();
        let result = order.begin_enforcement();
        assert!(result.is_err());
    }
}
