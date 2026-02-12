//! # Watcher Bonding and Slashing State Machine
//!
//! Models the lifecycle of corridor watcher nodes, including bond
//! management and the 6 slashing conditions from the watcher economy.
//!
//! ## States
//!
//! ```text
//! Pending ──▶ Active ──▶ PartiallySlashed ──▶ FullySlashed (terminal)
//!               │              │
//!               │              ├──▶ FullySlashed (terminal)
//!               │              └──▶ Withdrawn (terminal)
//!               │
//!               ├──▶ FullySlashed (terminal)
//!               ├──▶ Withdrawn (terminal)
//!               └──▶ Expired (terminal)
//! ```
//!
//! ## Slashing Conditions
//!
//! | Condition            | Slash % | Description                              |
//! |----------------------|---------|------------------------------------------|
//! | Equivocation         | 100%    | Signing conflicting attestations         |
//! | AvailabilityFailure  |   1%    | Missing required attestation window      |
//! | FalseAttestation     |  50%    | Attesting to an invalid state transition |
//! | Collusion            | 100%    | Coordinated misbehavior with watchers    |
//! | SafetyViolation      |  75%    | Violated protocol safety rules           |
//! | LivenessViolation    |  10%    | Violated protocol liveness rules         |
//!
//! ## Implements
//!
//! Spec §17 — Watcher economy and slashing protocol.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use msez_core::{Timestamp, WatcherId};

// ─── Bond Status ────────────────────────────────────────────────────

/// The bond status of a watcher node.
///
/// Tracks the lifecycle of the watcher's collateral bond from initial
/// posting through active service, possible slashing, and eventual
/// withdrawal or expiry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BondStatus {
    /// Bond submitted, awaiting confirmation.
    Pending,
    /// Bond confirmed and actively backing attestations.
    Active,
    /// Some collateral has been slashed but the watcher can still operate.
    PartiallySlashed,
    /// All collateral has been slashed (terminal).
    FullySlashed,
    /// Bond has been voluntarily withdrawn by the watcher (terminal).
    Withdrawn,
    /// Bond validity period has ended (terminal).
    Expired,
}

impl BondStatus {
    /// Whether this status is terminal (no further transitions possible).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::FullySlashed | Self::Withdrawn | Self::Expired
        )
    }

    /// Whether the watcher can currently attest in this status.
    pub fn can_attest(&self) -> bool {
        matches!(self, Self::Active | Self::PartiallySlashed)
    }
}

impl std::fmt::Display for BondStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Pending => "PENDING",
            Self::Active => "ACTIVE",
            Self::PartiallySlashed => "PARTIALLY_SLASHED",
            Self::FullySlashed => "FULLY_SLASHED",
            Self::Withdrawn => "WITHDRAWN",
            Self::Expired => "EXPIRED",
        };
        f.write_str(s)
    }
}

// ─── Slashing Conditions ────────────────────────────────────────────

/// Conditions that trigger bond slashing.
///
/// Each condition carries a fixed slash percentage defined by the
/// watcher economy protocol (spec §17). The percentage determines
/// what fraction of the *remaining* collateral is confiscated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SlashingCondition {
    /// Signed conflicting attestations for the same height.
    /// Slash: 100% of remaining collateral.
    Equivocation,
    /// Failed to submit attestation within the required window.
    /// Slash: 1% of remaining collateral.
    AvailabilityFailure,
    /// Attested to an invalid state transition.
    /// Slash: 50% of remaining collateral.
    FalseAttestation,
    /// Coordinated false attestation with other watchers.
    /// Slash: 100% of remaining collateral.
    Collusion,
    /// Violated protocol safety rules.
    /// Slash: 75% of remaining collateral.
    SafetyViolation,
    /// Violated protocol liveness rules.
    /// Slash: 10% of remaining collateral.
    LivenessViolation,
}

impl SlashingCondition {
    /// The slash percentage for this condition as basis points (0–10000).
    ///
    /// Using basis points avoids floating-point imprecision in financial
    /// calculations. 10000 = 100%, 5000 = 50%, 100 = 1%, etc.
    pub fn slash_basis_points(&self) -> u64 {
        match self {
            Self::Equivocation => 10_000,       // 100%
            Self::AvailabilityFailure => 100,    //   1%
            Self::FalseAttestation => 5_000,     //  50%
            Self::Collusion => 10_000,           // 100%
            Self::SafetyViolation => 7_500,      //  75%
            Self::LivenessViolation => 1_000,    //  10%
        }
    }

    /// Whether this condition results in full slashing (100%).
    pub fn is_total_slash(&self) -> bool {
        self.slash_basis_points() == 10_000
    }

    /// Compute the slash amount for a given remaining collateral.
    ///
    /// Uses integer arithmetic with basis points to avoid floating-point
    /// errors: `slash = remaining * basis_points / 10000`.
    pub fn compute_slash_amount(&self, remaining_collateral: u64) -> u64 {
        let bp = self.slash_basis_points();
        // Use u128 intermediate to avoid overflow on large collateral values.
        let slash = (remaining_collateral as u128 * bp as u128) / 10_000u128;
        slash as u64
    }
}

impl std::fmt::Display for SlashingCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Equivocation => "EQUIVOCATION",
            Self::AvailabilityFailure => "AVAILABILITY_FAILURE",
            Self::FalseAttestation => "FALSE_ATTESTATION",
            Self::Collusion => "COLLUSION",
            Self::SafetyViolation => "SAFETY_VIOLATION",
            Self::LivenessViolation => "LIVENESS_VIOLATION",
        };
        f.write_str(s)
    }
}

// ─── Errors ─────────────────────────────────────────────────────────

/// Errors that can occur during watcher state transitions.
#[derive(Error, Debug)]
pub enum WatcherError {
    /// Attempted transition is not valid from the current status.
    #[error("invalid watcher transition: {from} -> {to}")]
    InvalidTransition {
        /// Current bond status.
        from: String,
        /// Attempted target status.
        to: String,
    },

    /// Bond is in a terminal status.
    #[error("bond is in terminal status {status}")]
    TerminalStatus {
        /// The terminal status.
        status: String,
    },

    /// Insufficient collateral for the requested operation.
    #[error("insufficient collateral: available={available}, required={required}")]
    InsufficientCollateral {
        /// Available collateral.
        available: u64,
        /// Required collateral.
        required: u64,
    },

    /// Attestation value exceeds the watcher's maximum.
    #[error("attestation value {value} exceeds max {max}")]
    AttestationLimitExceeded {
        /// Requested attestation value.
        value: u64,
        /// Maximum allowed value.
        max: u64,
    },
}

// ─── Slashing Evidence ──────────────────────────────────────────────

/// Evidence for a slashing event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingEvidence {
    /// The condition that triggered slashing.
    pub condition: SlashingCondition,
    /// Human-readable description of the violation.
    pub description: String,
    /// The entity that reported the violation (DID or authority ID).
    pub reporter: Option<String>,
}

/// Record of a slashing event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingRecord {
    /// The condition that triggered slashing.
    pub condition: SlashingCondition,
    /// Amount of collateral slashed (in smallest currency unit).
    pub slash_amount: u64,
    /// Remaining collateral after slashing.
    pub remaining_collateral: u64,
    /// Bond status after slashing.
    pub resulting_status: BondStatus,
    /// When the slashing occurred.
    pub timestamp: Timestamp,
    /// Description of the violation.
    pub description: String,
}

// ─── Transition Record ──────────────────────────────────────────────

/// Record of a watcher bond status transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherTransitionRecord {
    /// Status before the transition.
    pub from_status: BondStatus,
    /// Status after the transition.
    pub to_status: BondStatus,
    /// When the transition occurred.
    pub timestamp: Timestamp,
    /// Reason for the transition.
    pub reason: String,
}

// ─── Watcher Bond ───────────────────────────────────────────────────

/// A watcher bond with collateral tracking and slashing history.
///
/// The bond amount determines the maximum value the watcher can attest
/// to (typically 10x the bond amount). Slashing reduces the available
/// collateral, which in turn reduces the watcher's attestation capacity.
///
/// All monetary amounts are in the smallest unit of the collateral
/// currency (e.g., wei for ETH, micro-units for USDC) to avoid
/// floating-point arithmetic entirely.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherBond {
    /// Unique watcher identifier.
    pub watcher_id: WatcherId,
    /// Current bond status.
    pub status: BondStatus,
    /// Total collateral posted (in smallest currency unit).
    pub collateral_amount: u64,
    /// Collateral currency identifier (e.g., "USDC", "ETH").
    pub collateral_currency: String,
    /// Total amount slashed so far.
    pub slashed_amount: u64,
    /// Number of slashing incidents.
    pub slash_count: u32,
    /// Maximum attestation value in USD (smallest unit).
    /// Default: 10x the collateral amount.
    pub max_attestation_value: u64,
    /// When the bond was created.
    pub created_at: Timestamp,
    /// When the bond expires (if set).
    pub valid_until: Option<Timestamp>,
    /// Ordered history of all slashing events.
    pub slashing_history: Vec<SlashingRecord>,
    /// Ordered log of all status transitions.
    pub transitions: Vec<WatcherTransitionRecord>,
}

impl WatcherBond {
    /// Create a new watcher bond in `Pending` status.
    ///
    /// The `max_attestation_value` defaults to 10x the collateral amount,
    /// following the watcher economy protocol.
    pub fn new(
        watcher_id: WatcherId,
        collateral_amount: u64,
        collateral_currency: String,
        valid_until: Option<Timestamp>,
    ) -> Self {
        Self {
            watcher_id,
            status: BondStatus::Pending,
            collateral_amount,
            collateral_currency,
            slashed_amount: 0,
            slash_count: 0,
            max_attestation_value: collateral_amount.saturating_mul(10),
            created_at: Timestamp::now(),
            valid_until,
            slashing_history: Vec::new(),
            transitions: Vec::new(),
        }
    }

    /// Available (unslashed) collateral.
    pub fn available_collateral(&self) -> u64 {
        self.collateral_amount.saturating_sub(self.slashed_amount)
    }

    /// Whether the bond is currently valid for attestation.
    pub fn is_valid(&self) -> bool {
        self.status.can_attest()
    }

    /// Whether the bond is in a terminal status.
    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }

    /// Check if the watcher can attest to a value.
    pub fn can_attest(&self, value: u64) -> Result<(), WatcherError> {
        if !self.status.can_attest() {
            return Err(WatcherError::InvalidTransition {
                from: self.status.to_string(),
                to: "attest".to_string(),
            });
        }
        // Max attestation scales with available collateral.
        let effective_max = self.effective_max_attestation();
        if value > effective_max {
            return Err(WatcherError::AttestationLimitExceeded {
                value,
                max: effective_max,
            });
        }
        Ok(())
    }

    /// Effective maximum attestation value, scaled by available collateral.
    ///
    /// If the watcher has been partially slashed, the effective maximum
    /// is reduced proportionally.
    pub fn effective_max_attestation(&self) -> u64 {
        if self.collateral_amount == 0 {
            return 0;
        }
        let available = self.available_collateral();
        // Scale: max_value * (available / total)
        // Use u128 to avoid overflow.
        ((self.max_attestation_value as u128 * available as u128)
            / self.collateral_amount as u128) as u64
    }

    // ── Status Transitions ──────────────────────────────────────────

    /// Activate the bond (PENDING → ACTIVE).
    pub fn activate(&mut self, reason: &str) -> Result<(), WatcherError> {
        self.require_status(BondStatus::Pending, "ACTIVE")?;
        self.do_transition(BondStatus::Active, reason);
        Ok(())
    }

    /// Slash the watcher's collateral for a violation.
    ///
    /// Applies the slash percentage for the given condition to the
    /// remaining collateral. If all collateral is consumed, transitions
    /// to `FullySlashed` (terminal). Otherwise transitions to
    /// `PartiallySlashed`.
    ///
    /// Can be called from `Active` or `PartiallySlashed` status.
    pub fn slash(&mut self, evidence: SlashingEvidence) -> Result<SlashingRecord, WatcherError> {
        if self.status.is_terminal() {
            return Err(WatcherError::TerminalStatus {
                status: self.status.to_string(),
            });
        }
        if !matches!(self.status, BondStatus::Active | BondStatus::PartiallySlashed) {
            return Err(WatcherError::InvalidTransition {
                from: self.status.to_string(),
                to: "SLASHED".to_string(),
            });
        }

        let remaining = self.available_collateral();
        let slash_amount = evidence.condition.compute_slash_amount(remaining);
        // Ensure we slash at least 1 unit if remaining > 0 and percentage > 0.
        let slash_amount = if slash_amount == 0 && remaining > 0 && evidence.condition.slash_basis_points() > 0 {
            1
        } else {
            slash_amount
        };

        self.slashed_amount = self.slashed_amount.saturating_add(slash_amount);
        self.slash_count += 1;

        let new_remaining = self.available_collateral();
        let new_status = if new_remaining == 0 {
            BondStatus::FullySlashed
        } else {
            BondStatus::PartiallySlashed
        };

        let record = SlashingRecord {
            condition: evidence.condition,
            slash_amount,
            remaining_collateral: new_remaining,
            resulting_status: new_status,
            timestamp: Timestamp::now(),
            description: evidence.description.clone(),
        };

        self.slashing_history.push(record.clone());
        self.do_transition(new_status, &evidence.description);

        Ok(record)
    }

    /// Voluntarily withdraw the bond (ACTIVE or PARTIALLY_SLASHED → WITHDRAWN).
    ///
    /// The watcher ceases attesting and reclaims any remaining collateral.
    pub fn withdraw(&mut self, reason: &str) -> Result<(), WatcherError> {
        if self.status.is_terminal() {
            return Err(WatcherError::TerminalStatus {
                status: self.status.to_string(),
            });
        }
        if !matches!(self.status, BondStatus::Active | BondStatus::PartiallySlashed) {
            return Err(WatcherError::InvalidTransition {
                from: self.status.to_string(),
                to: "WITHDRAWN".to_string(),
            });
        }
        self.do_transition(BondStatus::Withdrawn, reason);
        Ok(())
    }

    /// Mark the bond as expired (ACTIVE or PARTIALLY_SLASHED → EXPIRED).
    ///
    /// Typically triggered by a deadline-based system check when the
    /// bond validity period has ended.
    pub fn expire(&mut self, reason: &str) -> Result<(), WatcherError> {
        if self.status.is_terminal() {
            return Err(WatcherError::TerminalStatus {
                status: self.status.to_string(),
            });
        }
        if !matches!(self.status, BondStatus::Active | BondStatus::PartiallySlashed) {
            return Err(WatcherError::InvalidTransition {
                from: self.status.to_string(),
                to: "EXPIRED".to_string(),
            });
        }
        self.do_transition(BondStatus::Expired, reason);
        Ok(())
    }

    // ── Internal ────────────────────────────────────────────────────

    /// Validate that the bond is in the expected status.
    fn require_status(&self, expected: BondStatus, target: &str) -> Result<(), WatcherError> {
        if self.status.is_terminal() {
            return Err(WatcherError::TerminalStatus {
                status: self.status.to_string(),
            });
        }
        if self.status != expected {
            return Err(WatcherError::InvalidTransition {
                from: self.status.to_string(),
                to: target.to_string(),
            });
        }
        Ok(())
    }

    /// Record a status transition.
    fn do_transition(&mut self, to: BondStatus, reason: &str) {
        self.transitions.push(WatcherTransitionRecord {
            from_status: self.status,
            to_status: to,
            timestamp: Timestamp::now(),
            reason: reason.to_string(),
        });
        self.status = to;
    }
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_watcher_id() -> WatcherId {
        WatcherId::new()
    }

    fn make_pending_bond() -> WatcherBond {
        WatcherBond::new(
            make_watcher_id(),
            1_000_000, // 1M units
            "USDC".to_string(),
            None,
        )
    }

    fn make_active_bond() -> WatcherBond {
        let mut bond = make_pending_bond();
        bond.activate("Bond confirmed on-chain").unwrap();
        bond
    }

    fn slash_evidence(condition: SlashingCondition) -> SlashingEvidence {
        SlashingEvidence {
            condition,
            description: format!("Test slashing: {condition}"),
            reporter: Some("test-reporter".to_string()),
        }
    }

    // ── Construction tests ──────────────────────────────────────────

    #[test]
    fn test_new_bond_pending() {
        let bond = make_pending_bond();
        assert_eq!(bond.status, BondStatus::Pending);
        assert_eq!(bond.collateral_amount, 1_000_000);
        assert_eq!(bond.slashed_amount, 0);
        assert_eq!(bond.slash_count, 0);
        assert_eq!(bond.max_attestation_value, 10_000_000); // 10x
        assert_eq!(bond.available_collateral(), 1_000_000);
        assert!(!bond.is_valid());
        assert!(!bond.is_terminal());
    }

    // ── Activation tests ────────────────────────────────────────────

    #[test]
    fn test_activate() {
        let mut bond = make_pending_bond();
        bond.activate("Confirmed").unwrap();
        assert_eq!(bond.status, BondStatus::Active);
        assert!(bond.is_valid());
        assert_eq!(bond.transitions.len(), 1);
    }

    #[test]
    fn test_cannot_activate_from_active() {
        let mut bond = make_active_bond();
        let result = bond.activate("Double activate");
        assert!(result.is_err());
    }

    // ── Slashing tests ──────────────────────────────────────────────

    #[test]
    fn test_slash_equivocation_full() {
        let mut bond = make_active_bond();
        let record = bond.slash(slash_evidence(SlashingCondition::Equivocation)).unwrap();

        assert_eq!(record.slash_amount, 1_000_000); // 100%
        assert_eq!(record.remaining_collateral, 0);
        assert_eq!(record.resulting_status, BondStatus::FullySlashed);
        assert_eq!(bond.status, BondStatus::FullySlashed);
        assert!(bond.is_terminal());
        assert_eq!(bond.slash_count, 1);
    }

    #[test]
    fn test_slash_availability_failure_partial() {
        let mut bond = make_active_bond();
        let record = bond
            .slash(slash_evidence(SlashingCondition::AvailabilityFailure))
            .unwrap();

        // 1% of 1,000,000 = 10,000
        assert_eq!(record.slash_amount, 10_000);
        assert_eq!(record.remaining_collateral, 990_000);
        assert_eq!(record.resulting_status, BondStatus::PartiallySlashed);
        assert_eq!(bond.status, BondStatus::PartiallySlashed);
        assert!(!bond.is_terminal());
        assert!(bond.is_valid()); // Can still attest
    }

    #[test]
    fn test_slash_false_attestation() {
        let mut bond = make_active_bond();
        let record = bond
            .slash(slash_evidence(SlashingCondition::FalseAttestation))
            .unwrap();

        // 50% of 1,000,000 = 500,000
        assert_eq!(record.slash_amount, 500_000);
        assert_eq!(record.remaining_collateral, 500_000);
        assert_eq!(bond.status, BondStatus::PartiallySlashed);
    }

    #[test]
    fn test_slash_collusion_full() {
        let mut bond = make_active_bond();
        let record = bond.slash(slash_evidence(SlashingCondition::Collusion)).unwrap();

        assert_eq!(record.slash_amount, 1_000_000); // 100%
        assert_eq!(bond.status, BondStatus::FullySlashed);
        assert!(bond.is_terminal());
    }

    #[test]
    fn test_slash_safety_violation() {
        let mut bond = make_active_bond();
        let record = bond
            .slash(slash_evidence(SlashingCondition::SafetyViolation))
            .unwrap();

        // 75% of 1,000,000 = 750,000
        assert_eq!(record.slash_amount, 750_000);
        assert_eq!(record.remaining_collateral, 250_000);
        assert_eq!(bond.status, BondStatus::PartiallySlashed);
    }

    #[test]
    fn test_slash_liveness_violation() {
        let mut bond = make_active_bond();
        let record = bond
            .slash(slash_evidence(SlashingCondition::LivenessViolation))
            .unwrap();

        // 10% of 1,000,000 = 100,000
        assert_eq!(record.slash_amount, 100_000);
        assert_eq!(record.remaining_collateral, 900_000);
        assert_eq!(bond.status, BondStatus::PartiallySlashed);
    }

    #[test]
    fn test_multiple_slashes_accumulate() {
        let mut bond = make_active_bond();

        // First: availability failure → 1% of 1,000,000 = 10,000
        bond.slash(slash_evidence(SlashingCondition::AvailabilityFailure))
            .unwrap();
        assert_eq!(bond.available_collateral(), 990_000);
        assert_eq!(bond.slash_count, 1);

        // Second: liveness violation → 10% of 990,000 = 99,000
        bond.slash(slash_evidence(SlashingCondition::LivenessViolation))
            .unwrap();
        assert_eq!(bond.available_collateral(), 891_000);
        assert_eq!(bond.slash_count, 2);

        // Third: false attestation → 50% of 891,000 = 445,500
        bond.slash(slash_evidence(SlashingCondition::FalseAttestation))
            .unwrap();
        assert_eq!(bond.available_collateral(), 445_500);
        assert_eq!(bond.slash_count, 3);
        assert_eq!(bond.status, BondStatus::PartiallySlashed);
        assert_eq!(bond.slashing_history.len(), 3);
    }

    #[test]
    fn test_partial_then_full_slash() {
        let mut bond = make_active_bond();

        // Partial: false attestation → 50%
        bond.slash(slash_evidence(SlashingCondition::FalseAttestation))
            .unwrap();
        assert_eq!(bond.status, BondStatus::PartiallySlashed);

        // Full: equivocation → 100% of remaining
        bond.slash(slash_evidence(SlashingCondition::Equivocation))
            .unwrap();
        assert_eq!(bond.status, BondStatus::FullySlashed);
        assert!(bond.is_terminal());
        assert_eq!(bond.available_collateral(), 0);
    }

    #[test]
    fn test_cannot_slash_from_pending() {
        let mut bond = make_pending_bond();
        let result = bond.slash(slash_evidence(SlashingCondition::Equivocation));
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_slash_from_fully_slashed() {
        let mut bond = make_active_bond();
        bond.slash(slash_evidence(SlashingCondition::Equivocation))
            .unwrap();
        assert_eq!(bond.status, BondStatus::FullySlashed);

        let result = bond.slash(slash_evidence(SlashingCondition::AvailabilityFailure));
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_slash_from_withdrawn() {
        let mut bond = make_active_bond();
        bond.withdraw("Leaving network").unwrap();
        let result = bond.slash(slash_evidence(SlashingCondition::Equivocation));
        assert!(result.is_err());
    }

    // ── Withdrawal tests ────────────────────────────────────────────

    #[test]
    fn test_withdraw_from_active() {
        let mut bond = make_active_bond();
        bond.withdraw("Voluntary exit").unwrap();
        assert_eq!(bond.status, BondStatus::Withdrawn);
        assert!(bond.is_terminal());
    }

    #[test]
    fn test_withdraw_from_partially_slashed() {
        let mut bond = make_active_bond();
        bond.slash(slash_evidence(SlashingCondition::AvailabilityFailure))
            .unwrap();
        bond.withdraw("Exit after warning").unwrap();
        assert_eq!(bond.status, BondStatus::Withdrawn);
        assert!(bond.is_terminal());
    }

    #[test]
    fn test_cannot_withdraw_from_pending() {
        let mut bond = make_pending_bond();
        let result = bond.withdraw("test");
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_withdraw_from_fully_slashed() {
        let mut bond = make_active_bond();
        bond.slash(slash_evidence(SlashingCondition::Equivocation))
            .unwrap();
        let result = bond.withdraw("test");
        assert!(result.is_err());
    }

    // ── Expiry tests ────────────────────────────────────────────────

    #[test]
    fn test_expire_from_active() {
        let mut bond = make_active_bond();
        bond.expire("Validity period ended").unwrap();
        assert_eq!(bond.status, BondStatus::Expired);
        assert!(bond.is_terminal());
    }

    #[test]
    fn test_expire_from_partially_slashed() {
        let mut bond = make_active_bond();
        bond.slash(slash_evidence(SlashingCondition::AvailabilityFailure))
            .unwrap();
        bond.expire("Validity period ended").unwrap();
        assert_eq!(bond.status, BondStatus::Expired);
        assert!(bond.is_terminal());
    }

    #[test]
    fn test_cannot_expire_from_pending() {
        let mut bond = make_pending_bond();
        let result = bond.expire("test");
        assert!(result.is_err());
    }

    // ── Attestation limit tests ─────────────────────────────────────

    #[test]
    fn test_attestation_limit_active() {
        let bond = make_active_bond();
        // Max attestation: 10x collateral = 10,000,000
        assert!(bond.can_attest(10_000_000).is_ok());
        assert!(bond.can_attest(10_000_001).is_err());
    }

    #[test]
    fn test_attestation_limit_reduced_after_slash() {
        let mut bond = make_active_bond();
        // Slash 50% → available collateral = 500,000
        bond.slash(slash_evidence(SlashingCondition::FalseAttestation))
            .unwrap();

        // Max attestation should be halved: 5,000,000
        assert_eq!(bond.effective_max_attestation(), 5_000_000);
        assert!(bond.can_attest(5_000_000).is_ok());
        assert!(bond.can_attest(5_000_001).is_err());
    }

    #[test]
    fn test_attestation_fails_when_not_active() {
        let bond = make_pending_bond();
        let result = bond.can_attest(1);
        assert!(result.is_err());
    }

    // ── Slashing condition computation tests ────────────────────────

    #[test]
    fn test_slash_basis_points() {
        assert_eq!(SlashingCondition::Equivocation.slash_basis_points(), 10_000);
        assert_eq!(SlashingCondition::AvailabilityFailure.slash_basis_points(), 100);
        assert_eq!(SlashingCondition::FalseAttestation.slash_basis_points(), 5_000);
        assert_eq!(SlashingCondition::Collusion.slash_basis_points(), 10_000);
        assert_eq!(SlashingCondition::SafetyViolation.slash_basis_points(), 7_500);
        assert_eq!(SlashingCondition::LivenessViolation.slash_basis_points(), 1_000);
    }

    #[test]
    fn test_compute_slash_amount() {
        assert_eq!(
            SlashingCondition::Equivocation.compute_slash_amount(1_000_000),
            1_000_000
        );
        assert_eq!(
            SlashingCondition::AvailabilityFailure.compute_slash_amount(1_000_000),
            10_000
        );
        assert_eq!(
            SlashingCondition::FalseAttestation.compute_slash_amount(1_000_000),
            500_000
        );
        assert_eq!(
            SlashingCondition::SafetyViolation.compute_slash_amount(1_000_000),
            750_000
        );
        assert_eq!(
            SlashingCondition::LivenessViolation.compute_slash_amount(1_000_000),
            100_000
        );
    }

    #[test]
    fn test_is_total_slash() {
        assert!(SlashingCondition::Equivocation.is_total_slash());
        assert!(SlashingCondition::Collusion.is_total_slash());
        assert!(!SlashingCondition::AvailabilityFailure.is_total_slash());
        assert!(!SlashingCondition::FalseAttestation.is_total_slash());
        assert!(!SlashingCondition::SafetyViolation.is_total_slash());
        assert!(!SlashingCondition::LivenessViolation.is_total_slash());
    }

    // ── Display tests ───────────────────────────────────────────────

    #[test]
    fn test_bond_status_display() {
        assert_eq!(BondStatus::Pending.to_string(), "PENDING");
        assert_eq!(BondStatus::Active.to_string(), "ACTIVE");
        assert_eq!(BondStatus::PartiallySlashed.to_string(), "PARTIALLY_SLASHED");
        assert_eq!(BondStatus::FullySlashed.to_string(), "FULLY_SLASHED");
        assert_eq!(BondStatus::Withdrawn.to_string(), "WITHDRAWN");
        assert_eq!(BondStatus::Expired.to_string(), "EXPIRED");
    }

    #[test]
    fn test_slashing_condition_display() {
        assert_eq!(SlashingCondition::Equivocation.to_string(), "EQUIVOCATION");
        assert_eq!(
            SlashingCondition::AvailabilityFailure.to_string(),
            "AVAILABILITY_FAILURE"
        );
        assert_eq!(
            SlashingCondition::FalseAttestation.to_string(),
            "FALSE_ATTESTATION"
        );
        assert_eq!(SlashingCondition::Collusion.to_string(), "COLLUSION");
        assert_eq!(
            SlashingCondition::SafetyViolation.to_string(),
            "SAFETY_VIOLATION"
        );
        assert_eq!(
            SlashingCondition::LivenessViolation.to_string(),
            "LIVENESS_VIOLATION"
        );
    }

    // ── Transition log tests ────────────────────────────────────────

    #[test]
    fn test_transition_log_records_all_changes() {
        let mut bond = make_pending_bond();
        bond.activate("Activated").unwrap();
        bond.slash(slash_evidence(SlashingCondition::AvailabilityFailure))
            .unwrap();
        bond.withdraw("Exiting").unwrap();

        assert_eq!(bond.transitions.len(), 3);
        assert_eq!(bond.transitions[0].from_status, BondStatus::Pending);
        assert_eq!(bond.transitions[0].to_status, BondStatus::Active);
        assert_eq!(bond.transitions[1].from_status, BondStatus::Active);
        assert_eq!(bond.transitions[1].to_status, BondStatus::PartiallySlashed);
        assert_eq!(bond.transitions[2].from_status, BondStatus::PartiallySlashed);
        assert_eq!(bond.transitions[2].to_status, BondStatus::Withdrawn);
    }

    // ── Serialization tests ─────────────────────────────────────────

    #[test]
    fn test_bond_serialization() {
        let mut bond = make_active_bond();
        bond.slash(slash_evidence(SlashingCondition::AvailabilityFailure))
            .unwrap();

        let json = serde_json::to_string(&bond).unwrap();
        let parsed: WatcherBond = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, bond.status);
        assert_eq!(parsed.collateral_amount, bond.collateral_amount);
        assert_eq!(parsed.slashed_amount, bond.slashed_amount);
        assert_eq!(parsed.slash_count, bond.slash_count);
        assert_eq!(parsed.slashing_history.len(), 1);
    }

    #[test]
    fn test_slashing_condition_serialization() {
        let evidence = slash_evidence(SlashingCondition::FalseAttestation);
        let json = serde_json::to_string(&evidence).unwrap();
        let parsed: SlashingEvidence = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.condition, SlashingCondition::FalseAttestation);
    }

    // ── Edge case tests ─────────────────────────────────────────────

    #[test]
    fn test_zero_collateral_bond() {
        let mut bond = WatcherBond::new(
            make_watcher_id(),
            0,
            "USDC".to_string(),
            None,
        );
        bond.activate("Activated").unwrap();
        assert_eq!(bond.effective_max_attestation(), 0);
        assert!(bond.can_attest(1).is_err());
    }

    #[test]
    fn test_full_lifecycle_activate_slash_withdraw() {
        let mut bond = make_pending_bond();

        // Pending → Active
        bond.activate("Bond confirmed").unwrap();
        assert!(bond.is_valid());

        // Active → PartiallySlashed (availability failure: 1%)
        let record = bond
            .slash(slash_evidence(SlashingCondition::AvailabilityFailure))
            .unwrap();
        assert_eq!(record.slash_amount, 10_000);
        assert!(bond.is_valid()); // Still valid

        // PartiallySlashed → Withdrawn
        bond.withdraw("Voluntary exit after warning").unwrap();
        assert!(bond.is_terminal());
        assert!(!bond.is_valid());

        assert_eq!(bond.transitions.len(), 3);
        assert_eq!(bond.slashing_history.len(), 1);
    }

    #[test]
    fn test_full_lifecycle_activate_expire() {
        let mut bond = make_pending_bond();
        bond.activate("Confirmed").unwrap();
        bond.expire("Validity period ended").unwrap();
        assert_eq!(bond.status, BondStatus::Expired);
        assert!(bond.is_terminal());
    }

    #[test]
    fn test_no_defective_state_names() {
        // Verify none of the old placeholder state names leak through.
        let all_statuses = [
            BondStatus::Pending,
            BondStatus::Active,
            BondStatus::PartiallySlashed,
            BondStatus::FullySlashed,
            BondStatus::Withdrawn,
            BondStatus::Expired,
        ];
        for status in &all_statuses {
            let name = status.to_string();
            assert!(!name.contains("PROPOSED"), "Found defective name PROPOSED");
            assert!(
                !name.contains("OPERATIONAL"),
                "Found defective name OPERATIONAL"
            );
            assert!(!name.contains("UNBONDED"), "Found old placeholder name UNBONDED");
            assert!(!name.contains("UNBONDING"), "Found old placeholder name UNBONDING");
        }
    }
}
