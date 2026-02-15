//! # Watcher Bonding State Machine
//!
//! Manages the lifecycle of watcher nodes in the corridor economy.
//! Watchers post bonds, observe corridor activity, and can be slashed
//! for 4 defined conditions.
//!
//! ## Lifecycle
//!
//! ```text
//! Registered ─bond()──▶ Bonded ─activate()──▶ Active
//!                                                │
//!                                       ┌────────┴────────┐
//!                                    slash()          unbond()
//!                                       │                 │
//!                                       ▼                 ▼
//!                                    Slashed          Unbonding
//!                                       │                 │
//!                                  rebond()       complete_unbond()
//!                                       │                 │
//!                                       ▼                 ▼
//!                                    Bonded          Deactivated
//! ```
//!
//! ## Slashing Conditions
//!
//! 1. **Equivocation**: Signing conflicting attestations (100% slash)
//! 2. **Availability Failure**: Missing required attestations (1% per incident)
//! 3. **False Attestation**: Attesting to invalid state (50% slash)
//! 4. **Collusion**: Coordinated false attestation (100% slash + permanent ban)

use serde::{Deserialize, Serialize};
use thiserror::Error;

use msez_core::WatcherId;

// ── Watcher State ────────────────────────────────────────────────────

/// The lifecycle state of a watcher node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WatcherState {
    /// Watcher has registered but not yet posted a bond.
    Registered,
    /// Bond has been posted and confirmed.
    Bonded,
    /// Watcher is actively monitoring corridor activity.
    Active,
    /// Watcher has been slashed for a protocol violation.
    Slashed,
    /// Watcher is in the unbonding period (cooldown before withdrawal).
    Unbonding,
    /// Bond has been returned; watcher is deactivated. Terminal state.
    Deactivated,
    /// Watcher permanently banned (collusion). Terminal state.
    Banned,
}

impl WatcherState {
    /// Whether this is a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Deactivated | Self::Banned)
    }

    /// The canonical string name of this state.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Registered => "REGISTERED",
            Self::Bonded => "BONDED",
            Self::Active => "ACTIVE",
            Self::Slashed => "SLASHED",
            Self::Unbonding => "UNBONDING",
            Self::Deactivated => "DEACTIVATED",
            Self::Banned => "BANNED",
        }
    }
}

impl std::fmt::Display for WatcherState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── Slashing Conditions ──────────────────────────────────────────────

/// The 4 slashing conditions for watcher nodes.
///
/// Each condition has a defined slash percentage applied to the
/// watcher's bonded collateral.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SlashingCondition {
    /// Watcher signed conflicting attestations for the same state/height.
    /// Slash: 100% of bond.
    Equivocation,
    /// Watcher failed to attest within the required SLA window.
    /// Slash: 1% of bond per incident.
    AvailabilityFailure,
    /// Watcher attested to an invalid state transition.
    /// Slash: 50% of bond.
    FalseAttestation,
    /// Coordinated false attestation detected via quorum analysis.
    /// Slash: 100% of bond + permanent ban.
    Collusion,
}

impl SlashingCondition {
    /// The slash percentage for this condition (0.0 to 1.0).
    pub fn slash_percentage(&self) -> f64 {
        match self {
            Self::Equivocation => 1.00,
            Self::AvailabilityFailure => 0.01,
            Self::FalseAttestation => 0.50,
            Self::Collusion => 1.00,
        }
    }

    /// Whether this condition results in a permanent ban.
    pub fn causes_ban(&self) -> bool {
        matches!(self, Self::Collusion)
    }

    /// The canonical string name.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Equivocation => "EQUIVOCATION",
            Self::AvailabilityFailure => "AVAILABILITY_FAILURE",
            Self::FalseAttestation => "FALSE_ATTESTATION",
            Self::Collusion => "COLLUSION",
        }
    }
}

impl std::fmt::Display for SlashingCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── Watcher Error ────────────────────────────────────────────────────

/// Errors during watcher lifecycle operations.
#[derive(Error, Debug)]
pub enum WatcherError {
    /// Invalid lifecycle state transition.
    #[error("invalid watcher transition from {from} to {to}: {reason}")]
    InvalidTransition {
        /// Current state.
        from: WatcherState,
        /// Attempted target state.
        to: WatcherState,
        /// Human-readable reason.
        reason: String,
    },
    /// Insufficient stake for the requested operation.
    #[error("insufficient stake: required {required}, available {available}")]
    InsufficientStake {
        /// Required stake amount.
        required: u64,
        /// Available stake amount.
        available: u64,
    },
    /// Watcher is in a terminal state.
    #[error("watcher {id} is in terminal state {state}")]
    AlreadyTerminal {
        /// Watcher identifier.
        id: WatcherId,
        /// The terminal state.
        state: WatcherState,
    },
}

// ── Watcher ──────────────────────────────────────────────────────────

/// A watcher node in the corridor economy.
///
/// Tracks the watcher's lifecycle state, bonded stake, and slashing
/// history. Stake-based transitions enforce that bonding requires
/// sufficient collateral and slashing reduces the available stake.
#[derive(Debug, Clone, PartialEq)]
pub struct Watcher {
    /// Unique watcher identifier.
    pub id: WatcherId,
    /// Current watcher lifecycle state.
    pub state: WatcherState,
    /// Total bonded stake (in smallest currency unit).
    pub bonded_stake: u64,
    /// Amount slashed from the bond.
    pub slashed_amount: u64,
    /// Number of slashing incidents.
    pub slash_count: u32,
    /// Number of successful attestations.
    pub attestation_count: u64,
}

impl Watcher {
    /// Create a new watcher in the Registered state.
    pub fn new(id: WatcherId) -> Self {
        Self {
            id,
            state: WatcherState::Registered,
            bonded_stake: 0,
            slashed_amount: 0,
            slash_count: 0,
            attestation_count: 0,
        }
    }

    /// Available (unslashed) stake.
    pub fn available_stake(&self) -> u64 {
        self.bonded_stake.saturating_sub(self.slashed_amount)
    }

    /// Post a bond with the given stake amount.
    ///
    /// Transitions: Registered → Bonded.
    pub fn bond(&mut self, stake: u64) -> Result<(), WatcherError> {
        if self.state != WatcherState::Registered {
            return Err(WatcherError::InvalidTransition {
                from: self.state,
                to: WatcherState::Bonded,
                reason: "can only bond from REGISTERED state".to_string(),
            });
        }
        if stake == 0 {
            return Err(WatcherError::InsufficientStake {
                required: 1,
                available: 0,
            });
        }
        self.bonded_stake = stake;
        self.state = WatcherState::Bonded;
        Ok(())
    }

    /// Activate the watcher for corridor monitoring.
    ///
    /// Transitions: Bonded → Active.
    pub fn activate(&mut self) -> Result<(), WatcherError> {
        if self.state != WatcherState::Bonded {
            return Err(WatcherError::InvalidTransition {
                from: self.state,
                to: WatcherState::Active,
                reason: "can only activate from BONDED state".to_string(),
            });
        }
        self.state = WatcherState::Active;
        Ok(())
    }

    /// Slash the watcher for a protocol violation.
    ///
    /// Transitions: Active → Slashed.
    /// If the condition is Collusion, transitions to Banned (terminal).
    ///
    /// Returns the amount actually slashed.
    pub fn slash(&mut self, condition: SlashingCondition) -> Result<u64, WatcherError> {
        if self.state != WatcherState::Active {
            return Err(WatcherError::InvalidTransition {
                from: self.state,
                to: WatcherState::Slashed,
                reason: "can only slash from ACTIVE state".to_string(),
            });
        }

        let slash_amount = ((self.bonded_stake as f64) * condition.slash_percentage()) as u64;
        let actual_slash = slash_amount.min(self.available_stake());
        self.slashed_amount += actual_slash;
        self.slash_count += 1;

        if condition.causes_ban() {
            self.state = WatcherState::Banned;
        } else {
            self.state = WatcherState::Slashed;
        }

        Ok(actual_slash)
    }

    /// Re-bond after being slashed (top up stake and return to Bonded).
    ///
    /// Transitions: Slashed → Bonded.
    pub fn rebond(&mut self, additional_stake: u64) -> Result<(), WatcherError> {
        if self.state != WatcherState::Slashed {
            return Err(WatcherError::InvalidTransition {
                from: self.state,
                to: WatcherState::Bonded,
                reason: "can only rebond from SLASHED state".to_string(),
            });
        }
        self.bonded_stake += additional_stake;
        self.state = WatcherState::Bonded;
        Ok(())
    }

    /// Begin the unbonding process (voluntary withdrawal).
    ///
    /// Transitions: Active → Unbonding.
    pub fn unbond(&mut self) -> Result<(), WatcherError> {
        if self.state != WatcherState::Active {
            return Err(WatcherError::InvalidTransition {
                from: self.state,
                to: WatcherState::Unbonding,
                reason: "can only unbond from ACTIVE state".to_string(),
            });
        }
        self.state = WatcherState::Unbonding;
        Ok(())
    }

    /// Complete the unbonding process. Terminal.
    ///
    /// Transitions: Unbonding → Deactivated.
    ///
    /// Returns the stake that was returned (available stake after any slashing).
    pub fn complete_unbond(&mut self) -> Result<u64, WatcherError> {
        if self.state != WatcherState::Unbonding {
            return Err(WatcherError::InvalidTransition {
                from: self.state,
                to: WatcherState::Deactivated,
                reason: "can only complete unbonding from UNBONDING state".to_string(),
            });
        }
        let returned = self.available_stake();
        self.state = WatcherState::Deactivated;
        Ok(returned)
    }

    /// Record a successful attestation.
    pub fn record_attestation(&mut self) {
        self.attestation_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_watcher() -> Watcher {
        Watcher::new(WatcherId::new())
    }

    #[test]
    fn new_watcher_is_registered() {
        let w = test_watcher();
        assert_eq!(w.state, WatcherState::Registered);
        assert_eq!(w.bonded_stake, 0);
    }

    #[test]
    fn bond_and_activate() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();
        assert_eq!(w.state, WatcherState::Bonded);
        assert_eq!(w.bonded_stake, 1_000_000);

        w.activate().unwrap();
        assert_eq!(w.state, WatcherState::Active);
    }

    #[test]
    fn zero_stake_bond_rejected() {
        let mut w = test_watcher();
        let err = w.bond(0).unwrap_err();
        assert!(matches!(err, WatcherError::InsufficientStake { .. }));
    }

    #[test]
    fn slash_equivocation_100_percent() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();
        w.activate().unwrap();

        let slashed = w.slash(SlashingCondition::Equivocation).unwrap();
        assert_eq!(slashed, 1_000_000);
        assert_eq!(w.state, WatcherState::Slashed);
        assert_eq!(w.available_stake(), 0);
        assert_eq!(w.slash_count, 1);
    }

    #[test]
    fn slash_availability_1_percent() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();
        w.activate().unwrap();

        let slashed = w.slash(SlashingCondition::AvailabilityFailure).unwrap();
        assert_eq!(slashed, 10_000); // 1% of 1M
        assert_eq!(w.available_stake(), 990_000);
    }

    #[test]
    fn slash_false_attestation_50_percent() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();
        w.activate().unwrap();

        let slashed = w.slash(SlashingCondition::FalseAttestation).unwrap();
        assert_eq!(slashed, 500_000);
        assert_eq!(w.available_stake(), 500_000);
    }

    #[test]
    fn slash_collusion_bans_permanently() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();
        w.activate().unwrap();

        let slashed = w.slash(SlashingCondition::Collusion).unwrap();
        assert_eq!(slashed, 1_000_000);
        assert_eq!(w.state, WatcherState::Banned);
        assert!(w.state.is_terminal());
    }

    #[test]
    fn rebond_after_slash() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();
        w.activate().unwrap();
        w.slash(SlashingCondition::AvailabilityFailure).unwrap();

        assert_eq!(w.state, WatcherState::Slashed);
        w.rebond(50_000).unwrap();
        assert_eq!(w.state, WatcherState::Bonded);
        assert_eq!(w.bonded_stake, 1_050_000);
    }

    #[test]
    fn unbond_and_deactivate() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();
        w.activate().unwrap();

        w.unbond().unwrap();
        assert_eq!(w.state, WatcherState::Unbonding);

        let returned = w.complete_unbond().unwrap();
        assert_eq!(returned, 1_000_000);
        assert_eq!(w.state, WatcherState::Deactivated);
        assert!(w.state.is_terminal());
    }

    #[test]
    fn cannot_bond_from_active() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();
        w.activate().unwrap();

        let err = w.bond(500_000).unwrap_err();
        assert!(matches!(err, WatcherError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_activate_from_registered() {
        let mut w = test_watcher();
        let err = w.activate().unwrap_err();
        assert!(matches!(err, WatcherError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_slash_from_bonded() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();

        let err = w.slash(SlashingCondition::Equivocation).unwrap_err();
        assert!(matches!(err, WatcherError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_unbond_from_bonded() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();

        let err = w.unbond().unwrap_err();
        assert!(matches!(err, WatcherError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_rebond_from_active() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();
        w.activate().unwrap();

        let err = w.rebond(500_000).unwrap_err();
        assert!(matches!(err, WatcherError::InvalidTransition { .. }));
    }

    #[test]
    fn attestation_tracking() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();
        w.activate().unwrap();

        w.record_attestation();
        w.record_attestation();
        w.record_attestation();
        assert_eq!(w.attestation_count, 3);
    }

    #[test]
    fn state_display_names() {
        assert_eq!(WatcherState::Registered.as_str(), "REGISTERED");
        assert_eq!(WatcherState::Bonded.as_str(), "BONDED");
        assert_eq!(WatcherState::Active.as_str(), "ACTIVE");
        assert_eq!(WatcherState::Slashed.as_str(), "SLASHED");
        assert_eq!(WatcherState::Unbonding.as_str(), "UNBONDING");
        assert_eq!(WatcherState::Deactivated.as_str(), "DEACTIVATED");
        assert_eq!(WatcherState::Banned.as_str(), "BANNED");
    }

    #[test]
    fn slashing_condition_percentages() {
        // These are exact literal returns, so assert_eq! is correct.
        assert_eq!(SlashingCondition::Equivocation.slash_percentage(), 1.00);
        assert_eq!(SlashingCondition::AvailabilityFailure.slash_percentage(), 0.01);
        assert_eq!(SlashingCondition::FalseAttestation.slash_percentage(), 0.50);
        assert_eq!(SlashingCondition::Collusion.slash_percentage(), 1.00);
    }

    #[test]
    fn only_collusion_causes_ban() {
        assert!(!SlashingCondition::Equivocation.causes_ban());
        assert!(!SlashingCondition::AvailabilityFailure.causes_ban());
        assert!(!SlashingCondition::FalseAttestation.causes_ban());
        assert!(SlashingCondition::Collusion.causes_ban());
    }

    #[test]
    fn all_terminal_states() {
        assert!(WatcherState::Deactivated.is_terminal());
        assert!(WatcherState::Banned.is_terminal());

        assert!(!WatcherState::Registered.is_terminal());
        assert!(!WatcherState::Bonded.is_terminal());
        assert!(!WatcherState::Active.is_terminal());
        assert!(!WatcherState::Slashed.is_terminal());
        assert!(!WatcherState::Unbonding.is_terminal());
    }

    // ── Additional coverage tests ────────────────────────────────────

    #[test]
    fn watcher_state_display_all_variants() {
        assert_eq!(format!("{}", WatcherState::Registered), "REGISTERED");
        assert_eq!(format!("{}", WatcherState::Bonded), "BONDED");
        assert_eq!(format!("{}", WatcherState::Active), "ACTIVE");
        assert_eq!(format!("{}", WatcherState::Slashed), "SLASHED");
        assert_eq!(format!("{}", WatcherState::Unbonding), "UNBONDING");
        assert_eq!(format!("{}", WatcherState::Deactivated), "DEACTIVATED");
        assert_eq!(format!("{}", WatcherState::Banned), "BANNED");
    }

    #[test]
    fn slashing_condition_display_all_variants() {
        assert_eq!(
            format!("{}", SlashingCondition::Equivocation),
            "EQUIVOCATION"
        );
        assert_eq!(
            format!("{}", SlashingCondition::AvailabilityFailure),
            "AVAILABILITY_FAILURE"
        );
        assert_eq!(
            format!("{}", SlashingCondition::FalseAttestation),
            "FALSE_ATTESTATION"
        );
        assert_eq!(format!("{}", SlashingCondition::Collusion), "COLLUSION");
    }

    #[test]
    fn slashing_condition_as_str_all_variants() {
        assert_eq!(SlashingCondition::Equivocation.as_str(), "EQUIVOCATION");
        assert_eq!(
            SlashingCondition::AvailabilityFailure.as_str(),
            "AVAILABILITY_FAILURE"
        );
        assert_eq!(
            SlashingCondition::FalseAttestation.as_str(),
            "FALSE_ATTESTATION"
        );
        assert_eq!(SlashingCondition::Collusion.as_str(), "COLLUSION");
    }

    #[test]
    fn cannot_slash_from_registered() {
        let mut w = test_watcher();
        let err = w.slash(SlashingCondition::Equivocation).unwrap_err();
        assert!(matches!(err, WatcherError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_slash_from_slashed() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();
        w.activate().unwrap();
        w.slash(SlashingCondition::AvailabilityFailure).unwrap();
        assert_eq!(w.state, WatcherState::Slashed);
        let err = w.slash(SlashingCondition::Equivocation).unwrap_err();
        assert!(matches!(err, WatcherError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_slash_from_unbonding() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();
        w.activate().unwrap();
        w.unbond().unwrap();
        let err = w.slash(SlashingCondition::FalseAttestation).unwrap_err();
        assert!(matches!(err, WatcherError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_activate_from_active() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();
        w.activate().unwrap();
        let err = w.activate().unwrap_err();
        assert!(matches!(err, WatcherError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_activate_from_slashed() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();
        w.activate().unwrap();
        w.slash(SlashingCondition::AvailabilityFailure).unwrap();
        let err = w.activate().unwrap_err();
        assert!(matches!(err, WatcherError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_unbond_from_registered() {
        let mut w = test_watcher();
        let err = w.unbond().unwrap_err();
        assert!(matches!(err, WatcherError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_unbond_from_slashed() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();
        w.activate().unwrap();
        w.slash(SlashingCondition::AvailabilityFailure).unwrap();
        let err = w.unbond().unwrap_err();
        assert!(matches!(err, WatcherError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_complete_unbond_from_active() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();
        w.activate().unwrap();
        let err = w.complete_unbond().unwrap_err();
        assert!(matches!(err, WatcherError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_complete_unbond_from_registered() {
        let mut w = test_watcher();
        let err = w.complete_unbond().unwrap_err();
        assert!(matches!(err, WatcherError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_rebond_from_registered() {
        let mut w = test_watcher();
        let err = w.rebond(500_000).unwrap_err();
        assert!(matches!(err, WatcherError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_rebond_from_bonded() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();
        let err = w.rebond(500_000).unwrap_err();
        assert!(matches!(err, WatcherError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_bond_from_bonded() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();
        let err = w.bond(500_000).unwrap_err();
        assert!(matches!(err, WatcherError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_operate_after_ban() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();
        w.activate().unwrap();
        w.slash(SlashingCondition::Collusion).unwrap();
        assert_eq!(w.state, WatcherState::Banned);
        assert!(w.state.is_terminal());

        // All transitions should fail from banned state
        assert!(w.bond(500_000).is_err());
        assert!(w.activate().is_err());
        assert!(w.unbond().is_err());
        assert!(w.rebond(500_000).is_err());
        assert!(w.slash(SlashingCondition::Equivocation).is_err());
        assert!(w.complete_unbond().is_err());
    }

    #[test]
    fn cannot_operate_after_deactivated() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();
        w.activate().unwrap();
        w.unbond().unwrap();
        w.complete_unbond().unwrap();
        assert_eq!(w.state, WatcherState::Deactivated);
        assert!(w.state.is_terminal());

        assert!(w.bond(500_000).is_err());
        assert!(w.activate().is_err());
        assert!(w.unbond().is_err());
    }

    #[test]
    fn available_stake_after_multiple_slashes() {
        let mut w = test_watcher();
        w.bond(1_000_000).unwrap();
        w.activate().unwrap();
        // First slash: 1% = 10,000
        w.slash(SlashingCondition::AvailabilityFailure).unwrap();
        assert_eq!(w.available_stake(), 990_000);
        assert_eq!(w.slash_count, 1);

        // Rebond and reactivate
        w.rebond(0).unwrap();
        w.activate().unwrap();
        // Second slash: 1% of original bond = 10,000
        w.slash(SlashingCondition::AvailabilityFailure).unwrap();
        assert_eq!(w.slash_count, 2);
    }

    #[test]
    fn available_stake_saturates_at_zero() {
        let mut w = test_watcher();
        w.bond(100).unwrap();
        w.activate().unwrap();
        // 100% slash removes all stake
        w.slash(SlashingCondition::Equivocation).unwrap();
        assert_eq!(w.available_stake(), 0);
    }

    #[test]
    fn watcher_error_invalid_transition_display() {
        let err = WatcherError::InvalidTransition {
            from: WatcherState::Registered,
            to: WatcherState::Active,
            reason: "must bond first".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("REGISTERED"));
        assert!(msg.contains("ACTIVE"));
        assert!(msg.contains("must bond first"));
    }

    #[test]
    fn watcher_error_insufficient_stake_display() {
        let err = WatcherError::InsufficientStake {
            required: 1000,
            available: 500,
        };
        let msg = format!("{err}");
        assert!(msg.contains("1000"));
        assert!(msg.contains("500"));
    }

    #[test]
    fn watcher_error_already_terminal_display() {
        let err = WatcherError::AlreadyTerminal {
            id: WatcherId::new(),
            state: WatcherState::Banned,
        };
        let msg = format!("{err}");
        assert!(msg.contains("terminal state"));
        assert!(msg.contains("BANNED"));
    }
}
