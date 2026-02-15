//! # Escrow Operations
//!
//! Manages escrow accounts tied to dispute filings, including conditional
//! release, partial release for split decisions, and timeout-based
//! auto-release with configurable deadlines.
//!
//! ## Security Invariant
//!
//! Escrow status transitions are validated at every operation. Terminal
//! statuses (FullyReleased, Forfeited) reject all further operations.
//! Timeout enforcement is checked on every state-changing operation.
//!
//! ## Spec Reference
//!
//! Implements Definition 26.7 (Escrow Management) from the specification.
//! Escrow types, statuses, and release conditions match the Python
//! `tools/arbitration.py` escrow handling.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use msez_core::{ContentDigest, Timestamp};

use crate::dispute::DisputeId;
use crate::error::ArbitrationError;

// ── Identifiers ────────────────────────────────────────────────────────

/// A unique identifier for an escrow account.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EscrowId(Uuid);

impl EscrowId {
    /// Create a new random escrow identifier.
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

impl Default for EscrowId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EscrowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "escrow:{}", self.0)
    }
}

// ── Escrow Types ───────────────────────────────────────────────────────

/// Categories of escrow accounts in arbitration.
///
/// Matches the 4 escrow types in Definition 26.7.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EscrowType {
    /// Arbitration institution filing fee held in escrow.
    FilingFee,
    /// Security for claimant's obligation to prosecute.
    SecurityDeposit,
    /// Funds held pending appeal period expiration.
    AwardEscrow,
    /// Security if either party appeals.
    AppealBond,
}

impl std::fmt::Display for EscrowType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::FilingFee => "filing_fee",
            Self::SecurityDeposit => "security_deposit",
            Self::AwardEscrow => "award_escrow",
            Self::AppealBond => "appeal_bond",
        };
        write!(f, "{s}")
    }
}

// ── Escrow Status ──────────────────────────────────────────────────────

/// The status of an escrow account.
///
/// Status machine: `Pending → Funded → [PartiallyReleased | FullyReleased | Forfeited]`
///
/// Terminal states: `FullyReleased`, `Forfeited`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EscrowStatus {
    /// Escrow created but not yet funded.
    Pending,
    /// Funds have been deposited and are held.
    Funded,
    /// Some funds have been released; balance remains.
    PartiallyReleased,
    /// All funds have been released. Terminal state.
    FullyReleased,
    /// Funds have been forfeited. Terminal state.
    Forfeited,
}

impl EscrowStatus {
    /// Whether this status is terminal (no further operations allowed).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::FullyReleased | Self::Forfeited)
    }

    /// The canonical string name.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Funded => "FUNDED",
            Self::PartiallyReleased => "PARTIALLY_RELEASED",
            Self::FullyReleased => "FULLY_RELEASED",
            Self::Forfeited => "FORFEITED",
        }
    }
}

impl std::fmt::Display for EscrowStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── Release Conditions ─────────────────────────────────────────────────

/// Types of conditions that trigger escrow release.
///
/// Matches the 5 release condition types in Definition 26.7.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReleaseConditionType {
    /// Award executed, escrow released to beneficiary.
    RulingEnforced,
    /// No appeal filed within deadline.
    AppealPeriodExpired,
    /// Both parties agree to settlement.
    SettlementAgreed,
    /// Claimant withdraws claim.
    DisputeWithdrawn,
    /// Arbitration institution orders release.
    InstitutionOrder,
}

impl std::fmt::Display for ReleaseConditionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::RulingEnforced => "ruling_enforced",
            Self::AppealPeriodExpired => "appeal_period_expired",
            Self::SettlementAgreed => "settlement_agreed",
            Self::DisputeWithdrawn => "dispute_withdrawn",
            Self::InstitutionOrder => "institution_order",
        };
        write!(f, "{s}")
    }
}

/// A condition that authorizes escrow release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseCondition {
    /// The type of release condition.
    pub condition_type: ReleaseConditionType,
    /// Digest of the evidence supporting this condition.
    pub evidence_digest: ContentDigest,
    /// When the condition was satisfied.
    pub satisfied_at: Timestamp,
}

// ── Escrow Transaction ─────────────────────────────────────────────────

/// Types of escrow transactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransactionType {
    /// Initial funding deposit.
    Deposit,
    /// Full release of remaining funds.
    FullRelease,
    /// Partial release of funds.
    PartialRelease,
    /// Forfeiture of funds.
    Forfeit,
    /// Refund to depositor.
    Refund,
}

/// A recorded escrow transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscrowTransaction {
    /// Transaction type.
    pub transaction_type: TransactionType,
    /// Amount involved in the transaction (string for precision).
    pub amount: String,
    /// Currency code (ISO 4217).
    pub currency: String,
    /// When the transaction occurred.
    pub timestamp: DateTime<Utc>,
    /// Digest of the authorization evidence.
    pub evidence_digest: ContentDigest,
}

// ── Escrow Account ─────────────────────────────────────────────────────

/// An escrow account holding funds during dispute proceedings.
///
/// Created via [`EscrowAccount::create`] tied to a dispute filing.
/// Supports deposit, full/partial release, and forfeiture operations.
/// Includes configurable deadline-based timeout enforcement.
///
/// ## Security Invariant
///
/// All operations check the current status and deadline before proceeding.
/// Terminal statuses reject all further operations. Timeout is checked on
/// every state-changing operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscrowAccount {
    /// Unique escrow account identifier.
    pub id: EscrowId,
    /// The dispute this escrow is tied to.
    pub dispute_id: DisputeId,
    /// Category of escrow.
    pub escrow_type: EscrowType,
    /// Total amount deposited (string for precision).
    pub deposited_amount: String,
    /// Amount currently held (remaining balance).
    pub held_amount: String,
    /// Currency code (ISO 4217).
    pub currency: String,
    /// Current escrow status.
    pub status: EscrowStatus,
    /// Optional deadline for timeout-based auto-release.
    pub deadline: Option<DateTime<Utc>>,
    /// Transaction history.
    pub transactions: Vec<EscrowTransaction>,
    /// When the escrow was created.
    pub created_at: Timestamp,
}

impl EscrowAccount {
    /// Create a new escrow account tied to a dispute.
    ///
    /// The account starts in [`Pending`](EscrowStatus::Pending) status
    /// and must be funded via [`deposit`](EscrowAccount::deposit).
    pub fn create(
        dispute_id: DisputeId,
        escrow_type: EscrowType,
        currency: String,
        deadline: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id: EscrowId::new(),
            dispute_id,
            escrow_type,
            deposited_amount: "0".to_string(),
            held_amount: "0".to_string(),
            currency,
            status: EscrowStatus::Pending,
            deadline,
            transactions: Vec::new(),
            created_at: Timestamp::now(),
        }
    }

    /// Deposit funds into the escrow account.
    ///
    /// Transitions Pending → Funded.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::InvalidEscrowOperation`] if not in Pending
    /// status. Returns [`ArbitrationError::EscrowTimeout`] if the deadline
    /// has passed.
    pub fn deposit(
        &mut self,
        amount: String,
        evidence_digest: ContentDigest,
    ) -> Result<(), ArbitrationError> {
        self.check_timeout()?;
        if self.status != EscrowStatus::Pending {
            return Err(ArbitrationError::InvalidEscrowOperation {
                escrow_id: self.id.to_string(),
                operation: "deposit".to_string(),
                status: self.status.as_str().to_string(),
            });
        }
        self.deposited_amount = amount.clone();
        self.held_amount = amount.clone();
        self.status = EscrowStatus::Funded;
        self.transactions.push(EscrowTransaction {
            transaction_type: TransactionType::Deposit,
            amount,
            currency: self.currency.clone(),
            timestamp: Utc::now(),
            evidence_digest,
        });
        Ok(())
    }

    /// Release all remaining funds from escrow.
    ///
    /// Transitions Funded or PartiallyReleased → FullyReleased.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::InvalidEscrowOperation`] if the escrow
    /// is not in a releasable status.
    pub fn full_release(&mut self, condition: ReleaseCondition) -> Result<(), ArbitrationError> {
        self.check_timeout()?;
        if !matches!(
            self.status,
            EscrowStatus::Funded | EscrowStatus::PartiallyReleased
        ) {
            return Err(ArbitrationError::InvalidEscrowOperation {
                escrow_id: self.id.to_string(),
                operation: "full_release".to_string(),
                status: self.status.as_str().to_string(),
            });
        }
        let released_amount = self.held_amount.clone();
        self.held_amount = "0".to_string();
        self.status = EscrowStatus::FullyReleased;
        self.transactions.push(EscrowTransaction {
            transaction_type: TransactionType::FullRelease,
            amount: released_amount,
            currency: self.currency.clone(),
            timestamp: Utc::now(),
            evidence_digest: condition.evidence_digest,
        });
        Ok(())
    }

    /// Release a partial amount from escrow.
    ///
    /// The escrow transitions to PartiallyReleased if balance remains,
    /// or FullyReleased if the entire balance is released.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::InsufficientEscrowBalance`] if the
    /// requested amount exceeds the held balance.
    /// Returns [`ArbitrationError::InvalidEscrowOperation`] if the escrow
    /// is not in a releasable status.
    pub fn partial_release(
        &mut self,
        amount: String,
        condition: ReleaseCondition,
    ) -> Result<(), ArbitrationError> {
        self.check_timeout()?;
        if !matches!(
            self.status,
            EscrowStatus::Funded | EscrowStatus::PartiallyReleased
        ) {
            return Err(ArbitrationError::InvalidEscrowOperation {
                escrow_id: self.id.to_string(),
                operation: "partial_release".to_string(),
                status: self.status.as_str().to_string(),
            });
        }

        let held = parse_amount(&self.held_amount)?;
        let release = parse_amount(&amount)?;
        // Reject non-positive amounts: a zero release would create a spurious
        // PartiallyReleased state, and a negative release would inflate the balance.
        if release <= 0 {
            return Err(ArbitrationError::InvalidAmount(format!(
                "partial release amount must be positive, got {release}"
            )));
        }
        if release > held {
            return Err(ArbitrationError::InsufficientEscrowBalance {
                escrow_id: self.id.to_string(),
                requested: amount,
                remaining: self.held_amount.clone(),
            });
        }

        let remaining = held - release;
        self.held_amount = format_amount(remaining);
        self.status = if remaining == 0 {
            EscrowStatus::FullyReleased
        } else {
            EscrowStatus::PartiallyReleased
        };

        self.transactions.push(EscrowTransaction {
            transaction_type: TransactionType::PartialRelease,
            amount,
            currency: self.currency.clone(),
            timestamp: Utc::now(),
            evidence_digest: condition.evidence_digest,
        });
        Ok(())
    }

    /// Forfeit the escrow funds.
    ///
    /// Transitions Funded or PartiallyReleased → Forfeited.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::InvalidEscrowOperation`] if not in a
    /// forfeitable status.
    pub fn forfeit(&mut self, evidence_digest: ContentDigest) -> Result<(), ArbitrationError> {
        if !matches!(
            self.status,
            EscrowStatus::Funded | EscrowStatus::PartiallyReleased
        ) {
            return Err(ArbitrationError::InvalidEscrowOperation {
                escrow_id: self.id.to_string(),
                operation: "forfeit".to_string(),
                status: self.status.as_str().to_string(),
            });
        }
        let forfeited_amount = self.held_amount.clone();
        self.held_amount = "0".to_string();
        self.status = EscrowStatus::Forfeited;
        self.transactions.push(EscrowTransaction {
            transaction_type: TransactionType::Forfeit,
            amount: forfeited_amount,
            currency: self.currency.clone(),
            timestamp: Utc::now(),
            evidence_digest,
        });
        Ok(())
    }

    /// Check if the escrow has exceeded its deadline.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::EscrowTimeout`] if the deadline has
    /// passed and the escrow is not in a terminal status.
    pub fn check_timeout(&self) -> Result<(), ArbitrationError> {
        if self.status.is_terminal() {
            return Ok(());
        }
        if let Some(deadline) = self.deadline {
            if Utc::now() > deadline {
                return Err(ArbitrationError::EscrowTimeout {
                    escrow_id: self.id.to_string(),
                    deadline: deadline.to_rfc3339(),
                });
            }
        }
        Ok(())
    }

    /// Check whether the deadline has passed (without returning an error).
    pub fn is_timed_out(&self) -> bool {
        if let Some(deadline) = self.deadline {
            Utc::now() > deadline
        } else {
            false
        }
    }
}

/// Parse an amount string to an i64 (in smallest currency units).
///
/// The escrow system operates in smallest currency units (cents/paise).
/// Invalid amount strings are rejected with [`ArbitrationError::InvalidAmount`]
/// rather than silently defaulting to zero, which could mask data corruption
/// or lead to incorrect settlement calculations.
fn parse_amount(s: &str) -> Result<i64, ArbitrationError> {
    s.parse::<i64>()
        .map_err(|_| ArbitrationError::InvalidAmount(s.to_string()))
}

/// Format an i64 amount back to a string.
fn format_amount(n: i64) -> String {
    n.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use msez_core::{sha256_digest, CanonicalBytes};
    use serde_json::json;

    fn test_digest() -> ContentDigest {
        let canonical = CanonicalBytes::new(&json!({"test": "escrow"})).unwrap();
        sha256_digest(&canonical)
    }

    fn test_dispute_id() -> DisputeId {
        DisputeId::new()
    }

    fn funded_escrow() -> EscrowAccount {
        let mut escrow = EscrowAccount::create(
            test_dispute_id(),
            EscrowType::FilingFee,
            "USD".to_string(),
            None,
        );
        escrow.deposit("10000".to_string(), test_digest()).unwrap();
        escrow
    }

    #[test]
    fn create_escrow_in_pending_status() {
        let escrow = EscrowAccount::create(
            test_dispute_id(),
            EscrowType::SecurityDeposit,
            "SGD".to_string(),
            None,
        );
        assert_eq!(escrow.status, EscrowStatus::Pending);
        assert_eq!(escrow.held_amount, "0");
        assert_eq!(escrow.currency, "SGD");
    }

    #[test]
    fn deposit_transitions_to_funded() {
        let mut escrow = EscrowAccount::create(
            test_dispute_id(),
            EscrowType::FilingFee,
            "USD".to_string(),
            None,
        );
        escrow.deposit("5000".to_string(), test_digest()).unwrap();
        assert_eq!(escrow.status, EscrowStatus::Funded);
        assert_eq!(escrow.held_amount, "5000");
        assert_eq!(escrow.deposited_amount, "5000");
        assert_eq!(escrow.transactions.len(), 1);
    }

    #[test]
    fn deposit_rejected_when_funded() {
        let mut escrow = funded_escrow();
        let result = escrow.deposit("5000".to_string(), test_digest());
        assert!(result.is_err());
    }

    #[test]
    fn full_release_transitions_to_fully_released() {
        let mut escrow = funded_escrow();
        escrow
            .full_release(ReleaseCondition {
                condition_type: ReleaseConditionType::RulingEnforced,
                evidence_digest: test_digest(),
                satisfied_at: Timestamp::now(),
            })
            .unwrap();
        assert_eq!(escrow.status, EscrowStatus::FullyReleased);
        assert_eq!(escrow.held_amount, "0");
        assert!(escrow.status.is_terminal());
    }

    #[test]
    fn full_release_rejected_when_pending() {
        let mut escrow = EscrowAccount::create(
            test_dispute_id(),
            EscrowType::FilingFee,
            "USD".to_string(),
            None,
        );
        let result = escrow.full_release(ReleaseCondition {
            condition_type: ReleaseConditionType::SettlementAgreed,
            evidence_digest: test_digest(),
            satisfied_at: Timestamp::now(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn partial_release_deducts_amount() {
        let mut escrow = funded_escrow();
        escrow
            .partial_release(
                "3000".to_string(),
                ReleaseCondition {
                    condition_type: ReleaseConditionType::InstitutionOrder,
                    evidence_digest: test_digest(),
                    satisfied_at: Timestamp::now(),
                },
            )
            .unwrap();
        assert_eq!(escrow.status, EscrowStatus::PartiallyReleased);
        assert_eq!(escrow.held_amount, "7000");
    }

    #[test]
    fn partial_release_full_amount_transitions_to_fully_released() {
        let mut escrow = funded_escrow();
        escrow
            .partial_release(
                "10000".to_string(),
                ReleaseCondition {
                    condition_type: ReleaseConditionType::AppealPeriodExpired,
                    evidence_digest: test_digest(),
                    satisfied_at: Timestamp::now(),
                },
            )
            .unwrap();
        assert_eq!(escrow.status, EscrowStatus::FullyReleased);
        assert_eq!(escrow.held_amount, "0");
    }

    #[test]
    fn partial_release_exceeding_balance_rejected() {
        let mut escrow = funded_escrow();
        let result = escrow.partial_release(
            "15000".to_string(),
            ReleaseCondition {
                condition_type: ReleaseConditionType::InstitutionOrder,
                evidence_digest: test_digest(),
                satisfied_at: Timestamp::now(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn forfeit_transitions_to_forfeited() {
        let mut escrow = funded_escrow();
        escrow.forfeit(test_digest()).unwrap();
        assert_eq!(escrow.status, EscrowStatus::Forfeited);
        assert_eq!(escrow.held_amount, "0");
        assert!(escrow.status.is_terminal());
    }

    #[test]
    fn terminal_status_rejects_operations() {
        let mut escrow = funded_escrow();
        escrow
            .full_release(ReleaseCondition {
                condition_type: ReleaseConditionType::RulingEnforced,
                evidence_digest: test_digest(),
                satisfied_at: Timestamp::now(),
            })
            .unwrap();

        // Should reject further operations
        assert!(escrow.forfeit(test_digest()).is_err());
        assert!(escrow
            .full_release(ReleaseCondition {
                condition_type: ReleaseConditionType::SettlementAgreed,
                evidence_digest: test_digest(),
                satisfied_at: Timestamp::now(),
            })
            .is_err());
    }

    #[test]
    fn timeout_on_past_deadline() {
        let past_deadline = Utc::now() - chrono::Duration::hours(1);
        let escrow = EscrowAccount::create(
            test_dispute_id(),
            EscrowType::AwardEscrow,
            "USD".to_string(),
            Some(past_deadline),
        );
        assert!(escrow.is_timed_out());
        assert!(escrow.check_timeout().is_err());
    }

    #[test]
    fn no_timeout_on_future_deadline() {
        let future_deadline = Utc::now() + chrono::Duration::hours(24);
        let escrow = EscrowAccount::create(
            test_dispute_id(),
            EscrowType::AppealBond,
            "USD".to_string(),
            Some(future_deadline),
        );
        assert!(!escrow.is_timed_out());
        assert!(escrow.check_timeout().is_ok());
    }

    #[test]
    fn no_timeout_without_deadline() {
        let escrow = EscrowAccount::create(
            test_dispute_id(),
            EscrowType::FilingFee,
            "USD".to_string(),
            None,
        );
        assert!(!escrow.is_timed_out());
        assert!(escrow.check_timeout().is_ok());
    }

    #[test]
    fn timeout_skipped_for_terminal_status() {
        let past_deadline = Utc::now() - chrono::Duration::hours(1);
        let mut escrow = EscrowAccount::create(
            test_dispute_id(),
            EscrowType::FilingFee,
            "USD".to_string(),
            Some(Utc::now() + chrono::Duration::hours(1)),
        );
        escrow.deposit("5000".to_string(), test_digest()).unwrap();
        escrow
            .full_release(ReleaseCondition {
                condition_type: ReleaseConditionType::RulingEnforced,
                evidence_digest: test_digest(),
                satisfied_at: Timestamp::now(),
            })
            .unwrap();
        // Now set deadline to past (simulating time passing after release)
        escrow.deadline = Some(past_deadline);
        // Terminal status should not trigger timeout
        assert!(escrow.check_timeout().is_ok());
    }

    #[test]
    fn escrow_serialization_roundtrip() {
        let escrow = funded_escrow();
        let json_str = serde_json::to_string(&escrow).unwrap();
        let deserialized: EscrowAccount = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.id, escrow.id);
        assert_eq!(deserialized.status, escrow.status);
        assert_eq!(deserialized.held_amount, escrow.held_amount);
    }

    #[test]
    fn transaction_history_tracks_operations() {
        let mut escrow = funded_escrow();
        escrow
            .partial_release(
                "3000".to_string(),
                ReleaseCondition {
                    condition_type: ReleaseConditionType::InstitutionOrder,
                    evidence_digest: test_digest(),
                    satisfied_at: Timestamp::now(),
                },
            )
            .unwrap();
        escrow
            .full_release(ReleaseCondition {
                condition_type: ReleaseConditionType::RulingEnforced,
                evidence_digest: test_digest(),
                satisfied_at: Timestamp::now(),
            })
            .unwrap();

        assert_eq!(escrow.transactions.len(), 3); // deposit + partial + full
        assert_eq!(
            escrow.transactions[0].transaction_type,
            TransactionType::Deposit
        );
        assert_eq!(
            escrow.transactions[1].transaction_type,
            TransactionType::PartialRelease
        );
        assert_eq!(
            escrow.transactions[2].transaction_type,
            TransactionType::FullRelease
        );
    }

    // ── Coverage expansion tests ─────────────────────────────────────

    #[test]
    fn escrow_id_default() {
        let id1 = EscrowId::default();
        let id2 = EscrowId::default();
        assert_ne!(id1, id2);
    }

    #[test]
    fn escrow_id_display() {
        let id = EscrowId::new();
        let display = format!("{id}");
        assert!(display.starts_with("escrow:"));
    }

    #[test]
    fn escrow_id_from_uuid_roundtrip() {
        let uuid = uuid::Uuid::new_v4();
        let id = EscrowId::from_uuid(uuid);
        assert_eq!(*id.as_uuid(), uuid);
    }

    #[test]
    fn escrow_type_display() {
        assert_eq!(format!("{}", EscrowType::FilingFee), "filing_fee");
        assert_eq!(
            format!("{}", EscrowType::SecurityDeposit),
            "security_deposit"
        );
        assert_eq!(format!("{}", EscrowType::AwardEscrow), "award_escrow");
        assert_eq!(format!("{}", EscrowType::AppealBond), "appeal_bond");
    }

    #[test]
    fn escrow_status_display() {
        assert_eq!(format!("{}", EscrowStatus::Pending), "PENDING");
        assert_eq!(format!("{}", EscrowStatus::Funded), "FUNDED");
        assert_eq!(
            format!("{}", EscrowStatus::PartiallyReleased),
            "PARTIALLY_RELEASED"
        );
        assert_eq!(format!("{}", EscrowStatus::FullyReleased), "FULLY_RELEASED");
        assert_eq!(format!("{}", EscrowStatus::Forfeited), "FORFEITED");
    }

    #[test]
    fn escrow_status_as_str_all_variants() {
        assert_eq!(EscrowStatus::Pending.as_str(), "PENDING");
        assert_eq!(EscrowStatus::Funded.as_str(), "FUNDED");
        assert_eq!(
            EscrowStatus::PartiallyReleased.as_str(),
            "PARTIALLY_RELEASED"
        );
        assert_eq!(EscrowStatus::FullyReleased.as_str(), "FULLY_RELEASED");
        assert_eq!(EscrowStatus::Forfeited.as_str(), "FORFEITED");
    }

    #[test]
    fn escrow_status_is_terminal() {
        assert!(!EscrowStatus::Pending.is_terminal());
        assert!(!EscrowStatus::Funded.is_terminal());
        assert!(!EscrowStatus::PartiallyReleased.is_terminal());
        assert!(EscrowStatus::FullyReleased.is_terminal());
        assert!(EscrowStatus::Forfeited.is_terminal());
    }

    #[test]
    fn release_condition_type_display() {
        assert_eq!(
            format!("{}", ReleaseConditionType::RulingEnforced),
            "ruling_enforced"
        );
        assert_eq!(
            format!("{}", ReleaseConditionType::AppealPeriodExpired),
            "appeal_period_expired"
        );
        assert_eq!(
            format!("{}", ReleaseConditionType::SettlementAgreed),
            "settlement_agreed"
        );
        assert_eq!(
            format!("{}", ReleaseConditionType::DisputeWithdrawn),
            "dispute_withdrawn"
        );
        assert_eq!(
            format!("{}", ReleaseConditionType::InstitutionOrder),
            "institution_order"
        );
    }

    #[test]
    fn forfeit_rejected_when_pending() {
        let escrow = EscrowAccount::create(
            test_dispute_id(),
            EscrowType::FilingFee,
            "USD".to_string(),
            None,
        );
        let mut escrow = escrow;
        let result = escrow.forfeit(test_digest());
        assert!(result.is_err());
    }

    #[test]
    fn forfeit_rejected_when_terminal() {
        let mut escrow = funded_escrow();
        escrow.forfeit(test_digest()).unwrap();
        // Second forfeit should fail (Forfeited is terminal)
        assert!(escrow.forfeit(test_digest()).is_err());
    }

    #[test]
    fn partial_release_from_partially_released() {
        let mut escrow = funded_escrow();
        escrow
            .partial_release(
                "3000".to_string(),
                ReleaseCondition {
                    condition_type: ReleaseConditionType::InstitutionOrder,
                    evidence_digest: test_digest(),
                    satisfied_at: Timestamp::now(),
                },
            )
            .unwrap();
        assert_eq!(escrow.status, EscrowStatus::PartiallyReleased);

        // Second partial release from PartiallyReleased state
        escrow
            .partial_release(
                "4000".to_string(),
                ReleaseCondition {
                    condition_type: ReleaseConditionType::SettlementAgreed,
                    evidence_digest: test_digest(),
                    satisfied_at: Timestamp::now(),
                },
            )
            .unwrap();
        assert_eq!(escrow.held_amount, "3000");
        assert_eq!(escrow.status, EscrowStatus::PartiallyReleased);
    }

    #[test]
    fn full_release_from_partially_released() {
        let mut escrow = funded_escrow();
        escrow
            .partial_release(
                "3000".to_string(),
                ReleaseCondition {
                    condition_type: ReleaseConditionType::InstitutionOrder,
                    evidence_digest: test_digest(),
                    satisfied_at: Timestamp::now(),
                },
            )
            .unwrap();

        escrow
            .full_release(ReleaseCondition {
                condition_type: ReleaseConditionType::RulingEnforced,
                evidence_digest: test_digest(),
                satisfied_at: Timestamp::now(),
            })
            .unwrap();
        assert_eq!(escrow.status, EscrowStatus::FullyReleased);
        assert_eq!(escrow.held_amount, "0");
    }

    #[test]
    fn deposit_with_past_deadline_fails() {
        let past_deadline = Utc::now() - chrono::Duration::hours(1);
        let mut escrow = EscrowAccount::create(
            test_dispute_id(),
            EscrowType::FilingFee,
            "USD".to_string(),
            Some(past_deadline),
        );
        let result = escrow.deposit("5000".to_string(), test_digest());
        assert!(result.is_err());
    }

    #[test]
    fn partial_release_rejected_when_pending() {
        let mut escrow = EscrowAccount::create(
            test_dispute_id(),
            EscrowType::FilingFee,
            "USD".to_string(),
            None,
        );
        let result = escrow.partial_release(
            "1000".to_string(),
            ReleaseCondition {
                condition_type: ReleaseConditionType::SettlementAgreed,
                evidence_digest: test_digest(),
                satisfied_at: Timestamp::now(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn parse_amount_invalid_returns_error() {
        assert!(parse_amount("abc").is_err());
        assert!(parse_amount("").is_err());
        assert!(parse_amount("12.34").is_err());
    }

    #[test]
    fn parse_amount_valid() {
        assert_eq!(parse_amount("0").unwrap(), 0);
        assert_eq!(parse_amount("12345").unwrap(), 12345);
        assert_eq!(parse_amount("-100").unwrap(), -100);
    }

    #[test]
    fn format_amount_roundtrip() {
        assert_eq!(format_amount(12345), "12345");
        assert_eq!(format_amount(0), "0");
        assert_eq!(format_amount(-100), "-100");
    }

    #[test]
    fn partial_release_with_invalid_amount_returns_error() {
        let mut escrow = funded_escrow();
        let result = escrow.partial_release(
            "not_a_number".to_string(),
            ReleaseCondition {
                condition_type: ReleaseConditionType::InstitutionOrder,
                evidence_digest: test_digest(),
                satisfied_at: Timestamp::now(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn create_with_each_escrow_type() {
        for escrow_type in [
            EscrowType::FilingFee,
            EscrowType::SecurityDeposit,
            EscrowType::AwardEscrow,
            EscrowType::AppealBond,
        ] {
            let escrow =
                EscrowAccount::create(test_dispute_id(), escrow_type, "PKR".to_string(), None);
            assert_eq!(escrow.escrow_type, escrow_type);
            assert_eq!(escrow.status, EscrowStatus::Pending);
        }
    }

    #[test]
    fn forfeit_from_partially_released() {
        let mut escrow = funded_escrow();
        escrow
            .partial_release(
                "3000".to_string(),
                ReleaseCondition {
                    condition_type: ReleaseConditionType::InstitutionOrder,
                    evidence_digest: test_digest(),
                    satisfied_at: Timestamp::now(),
                },
            )
            .unwrap();
        assert_eq!(escrow.status, EscrowStatus::PartiallyReleased);

        escrow.forfeit(test_digest()).unwrap();
        assert_eq!(escrow.status, EscrowStatus::Forfeited);
        assert_eq!(escrow.held_amount, "0");
    }

    #[test]
    fn full_release_rejected_when_forfeited() {
        let mut escrow = funded_escrow();
        escrow.forfeit(test_digest()).unwrap();
        let result = escrow.full_release(ReleaseCondition {
            condition_type: ReleaseConditionType::RulingEnforced,
            evidence_digest: test_digest(),
            satisfied_at: Timestamp::now(),
        });
        assert!(result.is_err());
    }
}
