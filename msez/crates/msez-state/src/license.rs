//! # License Lifecycle State Machine
//!
//! Manages the lifecycle of business licenses, professional certifications,
//! and regulatory authorizations within a jurisdiction.
//!
//! ## Lifecycle
//!
//! ```text
//! Applied ─review()──▶ UnderReview ─issue()──▶ Active
//!    │                     │                     │
//! reject()             reject()         ┌───────┼───────┐
//!    │                     │          suspend() │     expire()
//!    ▼                     ▼             │      │        │
//! Rejected             Rejected     Suspended   │   Expired
//!                                       │       │
//!                                  reinstate() revoke()
//!                                       │       │
//!                                       ▼       ▼
//!                                     Active  Revoked
//! ```
//!
//! Active licenses may also be voluntarily surrendered.

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── License State ────────────────────────────────────────────────────

/// The lifecycle state of a license.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LicenseState {
    /// License application submitted, awaiting review.
    Applied,
    /// License is under review by the licensing authority.
    UnderReview,
    /// License has been granted and is active.
    Active,
    /// License has been suspended pending investigation.
    Suspended,
    /// License has been revoked for cause. Terminal state.
    Revoked,
    /// License has expired and was not renewed. Terminal state.
    Expired,
    /// License was voluntarily surrendered. Terminal state.
    Surrendered,
    /// License application was rejected. Terminal state.
    Rejected,
}

impl LicenseState {
    /// Whether this is a terminal state (no further transitions).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Revoked | Self::Expired | Self::Surrendered | Self::Rejected
        )
    }

    /// The canonical string name of this state.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Applied => "APPLIED",
            Self::UnderReview => "UNDER_REVIEW",
            Self::Active => "ACTIVE",
            Self::Suspended => "SUSPENDED",
            Self::Revoked => "REVOKED",
            Self::Expired => "EXPIRED",
            Self::Surrendered => "SURRENDERED",
            Self::Rejected => "REJECTED",
        }
    }
}

impl std::fmt::Display for LicenseState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── License Error ────────────────────────────────────────────────────

/// Errors during license lifecycle operations.
#[derive(Error, Debug)]
pub enum LicenseError {
    /// Invalid lifecycle state transition.
    #[error("invalid license transition from {from} to {to}: {reason}")]
    InvalidTransition {
        /// Current state.
        from: LicenseState,
        /// Attempted target state.
        to: LicenseState,
        /// Human-readable reason for rejection.
        reason: String,
    },
    /// License is in a terminal state and cannot transition.
    #[error("license is in terminal state {state}")]
    AlreadyTerminal {
        /// The terminal state.
        state: LicenseState,
    },
}

// ── License ──────────────────────────────────────────────────────────

/// A license within the SEZ lifecycle system.
///
/// Tracks the state of a business license, professional certification,
/// or regulatory authorization. Each transition is validated to ensure
/// only legal state progressions occur.
#[derive(Debug)]
pub struct License {
    /// The current lifecycle state.
    pub state: LicenseState,
    /// The license category (e.g., "MANUFACTURING", "TRADING", "PROFESSIONAL").
    pub category: String,
    /// Optional reason for the current state (e.g., suspension reason).
    pub state_reason: Option<String>,
}

impl License {
    /// Create a new license application.
    pub fn new(category: impl Into<String>) -> Self {
        Self {
            state: LicenseState::Applied,
            category: category.into(),
            state_reason: None,
        }
    }

    /// Move the application to review.
    ///
    /// Transitions: Applied → UnderReview.
    pub fn review(&mut self) -> Result<(), LicenseError> {
        if self.state != LicenseState::Applied {
            return Err(LicenseError::InvalidTransition {
                from: self.state,
                to: LicenseState::UnderReview,
                reason: "can only begin review from APPLIED state".to_string(),
            });
        }
        self.state = LicenseState::UnderReview;
        Ok(())
    }

    /// Issue the license after successful review.
    ///
    /// Transitions: UnderReview → Active.
    pub fn issue(&mut self) -> Result<(), LicenseError> {
        if self.state != LicenseState::UnderReview {
            return Err(LicenseError::InvalidTransition {
                from: self.state,
                to: LicenseState::Active,
                reason: "can only issue from UNDER_REVIEW state".to_string(),
            });
        }
        self.state = LicenseState::Active;
        self.state_reason = None;
        Ok(())
    }

    /// Reject the license application. Terminal.
    ///
    /// Transitions: Applied | UnderReview → Rejected.
    pub fn reject(&mut self, reason: impl Into<String>) -> Result<(), LicenseError> {
        if !matches!(
            self.state,
            LicenseState::Applied | LicenseState::UnderReview
        ) {
            return Err(LicenseError::InvalidTransition {
                from: self.state,
                to: LicenseState::Rejected,
                reason: "can only reject from APPLIED or UNDER_REVIEW state".to_string(),
            });
        }
        self.state = LicenseState::Rejected;
        self.state_reason = Some(reason.into());
        Ok(())
    }

    /// Suspend an active license.
    ///
    /// Transitions: Active → Suspended.
    pub fn suspend(&mut self, reason: impl Into<String>) -> Result<(), LicenseError> {
        if self.state != LicenseState::Active {
            return Err(LicenseError::InvalidTransition {
                from: self.state,
                to: LicenseState::Suspended,
                reason: "can only suspend from ACTIVE state".to_string(),
            });
        }
        self.state = LicenseState::Suspended;
        self.state_reason = Some(reason.into());
        Ok(())
    }

    /// Reinstate a suspended license.
    ///
    /// Transitions: Suspended → Active.
    pub fn reinstate(&mut self) -> Result<(), LicenseError> {
        if self.state != LicenseState::Suspended {
            return Err(LicenseError::InvalidTransition {
                from: self.state,
                to: LicenseState::Active,
                reason: "can only reinstate from SUSPENDED state".to_string(),
            });
        }
        self.state = LicenseState::Active;
        self.state_reason = None;
        Ok(())
    }

    /// Revoke an active or suspended license for cause. Terminal.
    ///
    /// Transitions: Active | Suspended → Revoked.
    pub fn revoke(&mut self, reason: impl Into<String>) -> Result<(), LicenseError> {
        if !matches!(self.state, LicenseState::Active | LicenseState::Suspended) {
            return Err(LicenseError::InvalidTransition {
                from: self.state,
                to: LicenseState::Revoked,
                reason: "can only revoke from ACTIVE or SUSPENDED state".to_string(),
            });
        }
        self.state = LicenseState::Revoked;
        self.state_reason = Some(reason.into());
        Ok(())
    }

    /// Mark a license as expired. Terminal.
    ///
    /// Transitions: Active → Expired.
    pub fn expire(&mut self) -> Result<(), LicenseError> {
        if self.state != LicenseState::Active {
            return Err(LicenseError::InvalidTransition {
                from: self.state,
                to: LicenseState::Expired,
                reason: "can only expire from ACTIVE state".to_string(),
            });
        }
        self.state = LicenseState::Expired;
        Ok(())
    }

    /// Voluntarily surrender a license. Terminal.
    ///
    /// Transitions: Active → Surrendered.
    pub fn surrender(&mut self) -> Result<(), LicenseError> {
        if self.state != LicenseState::Active {
            return Err(LicenseError::InvalidTransition {
                from: self.state,
                to: LicenseState::Surrendered,
                reason: "can only surrender from ACTIVE state".to_string(),
            });
        }
        self.state = LicenseState::Surrendered;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_license() -> License {
        License::new("MANUFACTURING")
    }

    #[test]
    fn new_license_is_applied() {
        let lic = test_license();
        assert_eq!(lic.state, LicenseState::Applied);
        assert_eq!(lic.category, "MANUFACTURING");
    }

    #[test]
    fn full_happy_path() {
        let mut lic = test_license();
        lic.review().unwrap();
        assert_eq!(lic.state, LicenseState::UnderReview);

        lic.issue().unwrap();
        assert_eq!(lic.state, LicenseState::Active);
        assert!(!lic.state.is_terminal());
    }

    #[test]
    fn reject_from_applied() {
        let mut lic = test_license();
        lic.reject("Incomplete documentation").unwrap();
        assert_eq!(lic.state, LicenseState::Rejected);
        assert!(lic.state.is_terminal());
        assert_eq!(
            lic.state_reason.as_deref(),
            Some("Incomplete documentation")
        );
    }

    #[test]
    fn reject_from_under_review() {
        let mut lic = test_license();
        lic.review().unwrap();
        lic.reject("Failed compliance check").unwrap();
        assert_eq!(lic.state, LicenseState::Rejected);
    }

    #[test]
    fn suspend_and_reinstate() {
        let mut lic = test_license();
        lic.review().unwrap();
        lic.issue().unwrap();

        lic.suspend("Investigation pending").unwrap();
        assert_eq!(lic.state, LicenseState::Suspended);
        assert_eq!(lic.state_reason.as_deref(), Some("Investigation pending"));

        lic.reinstate().unwrap();
        assert_eq!(lic.state, LicenseState::Active);
        assert!(lic.state_reason.is_none());
    }

    #[test]
    fn revoke_from_active() {
        let mut lic = test_license();
        lic.review().unwrap();
        lic.issue().unwrap();
        lic.revoke("Regulatory violation").unwrap();
        assert_eq!(lic.state, LicenseState::Revoked);
        assert!(lic.state.is_terminal());
    }

    #[test]
    fn revoke_from_suspended() {
        let mut lic = test_license();
        lic.review().unwrap();
        lic.issue().unwrap();
        lic.suspend("Under investigation").unwrap();
        lic.revoke("Investigation confirmed violation").unwrap();
        assert_eq!(lic.state, LicenseState::Revoked);
    }

    #[test]
    fn expire_from_active() {
        let mut lic = test_license();
        lic.review().unwrap();
        lic.issue().unwrap();
        lic.expire().unwrap();
        assert_eq!(lic.state, LicenseState::Expired);
        assert!(lic.state.is_terminal());
    }

    #[test]
    fn surrender_from_active() {
        let mut lic = test_license();
        lic.review().unwrap();
        lic.issue().unwrap();
        lic.surrender().unwrap();
        assert_eq!(lic.state, LicenseState::Surrendered);
        assert!(lic.state.is_terminal());
    }

    #[test]
    fn cannot_issue_from_applied() {
        let mut lic = test_license();
        let err = lic.issue().unwrap_err();
        assert!(matches!(err, LicenseError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_suspend_from_applied() {
        let mut lic = test_license();
        let err = lic.suspend("reason").unwrap_err();
        assert!(matches!(err, LicenseError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_reinstate_from_active() {
        let mut lic = test_license();
        lic.review().unwrap();
        lic.issue().unwrap();
        let err = lic.reinstate().unwrap_err();
        assert!(matches!(err, LicenseError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_expire_from_suspended() {
        let mut lic = test_license();
        lic.review().unwrap();
        lic.issue().unwrap();
        lic.suspend("reason").unwrap();
        let err = lic.expire().unwrap_err();
        assert!(matches!(err, LicenseError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_reject_from_active() {
        let mut lic = test_license();
        lic.review().unwrap();
        lic.issue().unwrap();
        let err = lic.reject("reason").unwrap_err();
        assert!(matches!(err, LicenseError::InvalidTransition { .. }));
    }

    #[test]
    fn state_display_names() {
        assert_eq!(LicenseState::Applied.as_str(), "APPLIED");
        assert_eq!(LicenseState::UnderReview.as_str(), "UNDER_REVIEW");
        assert_eq!(LicenseState::Active.as_str(), "ACTIVE");
        assert_eq!(LicenseState::Suspended.as_str(), "SUSPENDED");
        assert_eq!(LicenseState::Revoked.as_str(), "REVOKED");
        assert_eq!(LicenseState::Expired.as_str(), "EXPIRED");
        assert_eq!(LicenseState::Surrendered.as_str(), "SURRENDERED");
        assert_eq!(LicenseState::Rejected.as_str(), "REJECTED");
    }

    #[test]
    fn all_terminal_states() {
        assert!(LicenseState::Revoked.is_terminal());
        assert!(LicenseState::Expired.is_terminal());
        assert!(LicenseState::Surrendered.is_terminal());
        assert!(LicenseState::Rejected.is_terminal());

        assert!(!LicenseState::Applied.is_terminal());
        assert!(!LicenseState::UnderReview.is_terminal());
        assert!(!LicenseState::Active.is_terminal());
        assert!(!LicenseState::Suspended.is_terminal());
    }

    // ── Additional coverage tests ────────────────────────────────────

    #[test]
    fn license_state_display_all_variants() {
        assert_eq!(format!("{}", LicenseState::Applied), "APPLIED");
        assert_eq!(format!("{}", LicenseState::UnderReview), "UNDER_REVIEW");
        assert_eq!(format!("{}", LicenseState::Active), "ACTIVE");
        assert_eq!(format!("{}", LicenseState::Suspended), "SUSPENDED");
        assert_eq!(format!("{}", LicenseState::Revoked), "REVOKED");
        assert_eq!(format!("{}", LicenseState::Expired), "EXPIRED");
        assert_eq!(format!("{}", LicenseState::Surrendered), "SURRENDERED");
        assert_eq!(format!("{}", LicenseState::Rejected), "REJECTED");
    }

    #[test]
    fn cannot_review_from_under_review() {
        let mut lic = test_license();
        lic.review().unwrap();
        let err = lic.review().unwrap_err();
        assert!(matches!(err, LicenseError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_review_from_active() {
        let mut lic = test_license();
        lic.review().unwrap();
        lic.issue().unwrap();
        let err = lic.review().unwrap_err();
        assert!(matches!(err, LicenseError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_issue_from_active() {
        let mut lic = test_license();
        lic.review().unwrap();
        lic.issue().unwrap();
        let err = lic.issue().unwrap_err();
        assert!(matches!(err, LicenseError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_suspend_from_under_review() {
        let mut lic = test_license();
        lic.review().unwrap();
        let err = lic.suspend("reason").unwrap_err();
        assert!(matches!(err, LicenseError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_reinstate_from_applied() {
        let mut lic = test_license();
        let err = lic.reinstate().unwrap_err();
        assert!(matches!(err, LicenseError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_reinstate_from_under_review() {
        let mut lic = test_license();
        lic.review().unwrap();
        let err = lic.reinstate().unwrap_err();
        assert!(matches!(err, LicenseError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_revoke_from_applied() {
        let mut lic = test_license();
        let err = lic.revoke("reason").unwrap_err();
        assert!(matches!(err, LicenseError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_revoke_from_under_review() {
        let mut lic = test_license();
        lic.review().unwrap();
        let err = lic.revoke("reason").unwrap_err();
        assert!(matches!(err, LicenseError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_expire_from_applied() {
        let mut lic = test_license();
        let err = lic.expire().unwrap_err();
        assert!(matches!(err, LicenseError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_expire_from_under_review() {
        let mut lic = test_license();
        lic.review().unwrap();
        let err = lic.expire().unwrap_err();
        assert!(matches!(err, LicenseError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_surrender_from_applied() {
        let mut lic = test_license();
        let err = lic.surrender().unwrap_err();
        assert!(matches!(err, LicenseError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_surrender_from_suspended() {
        let mut lic = test_license();
        lic.review().unwrap();
        lic.issue().unwrap();
        lic.suspend("reason").unwrap();
        let err = lic.surrender().unwrap_err();
        assert!(matches!(err, LicenseError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_transition_from_rejected() {
        let mut lic = test_license();
        lic.reject("bad application").unwrap();
        assert!(lic.review().is_err());
        assert!(lic.issue().is_err());
        assert!(lic.suspend("reason").is_err());
        assert!(lic.reinstate().is_err());
        assert!(lic.revoke("reason").is_err());
        assert!(lic.expire().is_err());
        assert!(lic.surrender().is_err());
        assert!(lic.reject("again").is_err());
    }

    #[test]
    fn cannot_transition_from_revoked() {
        let mut lic = test_license();
        lic.review().unwrap();
        lic.issue().unwrap();
        lic.revoke("violation").unwrap();
        assert!(lic.review().is_err());
        assert!(lic.issue().is_err());
        assert!(lic.suspend("reason").is_err());
        assert!(lic.reinstate().is_err());
        assert!(lic.expire().is_err());
        assert!(lic.surrender().is_err());
    }

    #[test]
    fn cannot_transition_from_expired() {
        let mut lic = test_license();
        lic.review().unwrap();
        lic.issue().unwrap();
        lic.expire().unwrap();
        assert!(lic.review().is_err());
        assert!(lic.issue().is_err());
        assert!(lic.reinstate().is_err());
        assert!(lic.revoke("reason").is_err());
        assert!(lic.surrender().is_err());
    }

    #[test]
    fn cannot_transition_from_surrendered() {
        let mut lic = test_license();
        lic.review().unwrap();
        lic.issue().unwrap();
        lic.surrender().unwrap();
        assert!(lic.review().is_err());
        assert!(lic.issue().is_err());
        assert!(lic.reinstate().is_err());
        assert!(lic.revoke("reason").is_err());
        assert!(lic.expire().is_err());
    }

    #[test]
    fn license_error_invalid_transition_display() {
        let err = LicenseError::InvalidTransition {
            from: LicenseState::Applied,
            to: LicenseState::Active,
            reason: "must review first".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("APPLIED"));
        assert!(msg.contains("ACTIVE"));
        assert!(msg.contains("must review first"));
    }

    #[test]
    fn license_error_already_terminal_display() {
        let err = LicenseError::AlreadyTerminal {
            state: LicenseState::Revoked,
        };
        let msg = format!("{err}");
        assert!(msg.contains("terminal state"));
        assert!(msg.contains("REVOKED"));
    }

    #[test]
    fn license_state_reason_cleared_on_issue() {
        let mut lic = test_license();
        lic.review().unwrap();
        // Manually set a state_reason to verify issue() clears it
        lic.state_reason = Some("under review notes".to_string());
        lic.issue().unwrap();
        assert!(lic.state_reason.is_none());
    }

    #[test]
    fn license_state_reason_set_on_suspend() {
        let mut lic = test_license();
        lic.review().unwrap();
        lic.issue().unwrap();
        lic.suspend("regulatory investigation").unwrap();
        assert_eq!(lic.state_reason.as_deref(), Some("regulatory investigation"));
    }

    #[test]
    fn license_state_reason_set_on_revoke() {
        let mut lic = test_license();
        lic.review().unwrap();
        lic.issue().unwrap();
        lic.revoke("fraud detected").unwrap();
        assert_eq!(lic.state_reason.as_deref(), Some("fraud detected"));
    }

    #[test]
    fn license_state_reason_cleared_on_reinstate() {
        let mut lic = test_license();
        lic.review().unwrap();
        lic.issue().unwrap();
        lic.suspend("pending investigation").unwrap();
        assert!(lic.state_reason.is_some());
        lic.reinstate().unwrap();
        assert!(lic.state_reason.is_none());
    }

    #[test]
    fn license_category_preserved() {
        let lic = License::new("PROFESSIONAL");
        assert_eq!(lic.category, "PROFESSIONAL");
    }
}
