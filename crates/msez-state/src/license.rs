//! # License Lifecycle State Machine
//!
//! Models the lifecycle of business licenses within a jurisdiction,
//! covering the full spectrum from application through expiration or revocation.
//!
//! ## States
//!
//! ```text
//! Application ──▶ Review ──▶ Issued ──▶ Active ──▶ Suspended ──▶ Active (reinstatement)
//!                   │                     │            │
//!                   │                     │            └──▶ Revoked (terminal)
//!                   │                     │
//!                   └──▶ Rejected         └──▶ Expired (terminal)
//!                       (terminal)
//! ```
//!
//! ## License Categories
//!
//! Pakistan's deployment requires 15+ license categories. The state machine
//! is generic over license type — the same lifecycle applies to:
//! - Business incorporation licenses
//! - Professional certifications
//! - Import/export permits
//! - Financial services licenses
//! - Tax registrations
//!
//! ## Implements
//!
//! Spec §15 — License lifecycle management.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use msez_core::Timestamp;

// ─── License State ───────────────────────────────────────────────────

/// The lifecycle state of a license.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LicenseState {
    /// License application submitted, awaiting review.
    Application,
    /// Application is under regulatory review.
    Review,
    /// License has been issued but not yet activated.
    Issued,
    /// License is active and valid.
    Active,
    /// License has been temporarily suspended.
    Suspended,
    /// License has been permanently revoked (terminal).
    Revoked,
    /// License has expired (terminal).
    Expired,
    /// License application was rejected (terminal).
    Rejected,
}

impl LicenseState {
    /// Whether this state is terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Revoked | Self::Expired | Self::Rejected)
    }

    /// Whether the license is currently valid for operations.
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Active)
    }
}

impl std::fmt::Display for LicenseState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Application => "APPLICATION",
            Self::Review => "REVIEW",
            Self::Issued => "ISSUED",
            Self::Active => "ACTIVE",
            Self::Suspended => "SUSPENDED",
            Self::Revoked => "REVOKED",
            Self::Expired => "EXPIRED",
            Self::Rejected => "REJECTED",
        };
        f.write_str(s)
    }
}

// ─── Errors ──────────────────────────────────────────────────────────

/// Errors that can occur during license lifecycle transitions.
#[derive(Error, Debug)]
pub enum LicenseError {
    /// Attempted transition is not valid from the current state.
    #[error("invalid license transition: {from} -> {to}")]
    InvalidTransition {
        /// Current state.
        from: String,
        /// Attempted target state.
        to: String,
    },

    /// License is in a terminal state.
    #[error("license is in terminal state {state}")]
    TerminalState {
        /// The terminal state.
        state: String,
    },
}

// ─── Transition Evidence ─────────────────────────────────────────────

/// Evidence for a license lifecycle transition.
#[derive(Debug, Clone)]
pub struct LicenseTransitionEvidence {
    /// Reason for the transition.
    pub reason: String,
    /// Actor who initiated the transition (DID or authority ID).
    pub actor: Option<String>,
}

/// Record of a license state transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseTransitionRecord {
    /// State before the transition.
    pub from_state: LicenseState,
    /// State after the transition.
    pub to_state: LicenseState,
    /// When the transition occurred.
    pub timestamp: Timestamp,
    /// Reason for the transition.
    pub reason: String,
}

// ─── License ─────────────────────────────────────────────────────────

/// A license with its lifecycle state and transition history.
///
/// Enforces valid state transitions with structured error reporting.
/// The license type and category are stored as metadata — the state
/// machine logic is the same regardless of license category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    /// Current lifecycle state.
    pub state: LicenseState,
    /// License type/category (e.g., "business_incorporation", "import_export").
    pub license_type: String,
    /// The entity that holds this license.
    pub holder_id: String,
    /// When the license was created (application submitted).
    pub created_at: Timestamp,
    /// When the license expires (if set).
    pub expires_at: Option<Timestamp>,
    /// Ordered log of all state transitions.
    pub transitions: Vec<LicenseTransitionRecord>,
}

impl License {
    /// Create a new license application.
    pub fn new_application(license_type: String, holder_id: String) -> Self {
        Self {
            state: LicenseState::Application,
            license_type,
            holder_id,
            created_at: Timestamp::now(),
            expires_at: None,
            transitions: Vec::new(),
        }
    }

    /// Submit application for review (APPLICATION → REVIEW).
    pub fn submit_for_review(
        &mut self,
        evidence: LicenseTransitionEvidence,
    ) -> Result<(), LicenseError> {
        self.require_state(LicenseState::Application, "REVIEW")?;
        self.do_transition(LicenseState::Review, &evidence.reason);
        Ok(())
    }

    /// Issue the license after review (REVIEW → ISSUED).
    pub fn issue(
        &mut self,
        evidence: LicenseTransitionEvidence,
        expires_at: Option<Timestamp>,
    ) -> Result<(), LicenseError> {
        self.require_state(LicenseState::Review, "ISSUED")?;
        self.expires_at = expires_at;
        self.do_transition(LicenseState::Issued, &evidence.reason);
        Ok(())
    }

    /// Reject the application (REVIEW → REJECTED).
    pub fn reject(&mut self, evidence: LicenseTransitionEvidence) -> Result<(), LicenseError> {
        self.require_state(LicenseState::Review, "REJECTED")?;
        self.do_transition(LicenseState::Rejected, &evidence.reason);
        Ok(())
    }

    /// Activate the license (ISSUED → ACTIVE).
    pub fn activate(&mut self, evidence: LicenseTransitionEvidence) -> Result<(), LicenseError> {
        self.require_state(LicenseState::Issued, "ACTIVE")?;
        self.do_transition(LicenseState::Active, &evidence.reason);
        Ok(())
    }

    /// Suspend the license (ACTIVE → SUSPENDED).
    pub fn suspend(&mut self, evidence: LicenseTransitionEvidence) -> Result<(), LicenseError> {
        self.require_state(LicenseState::Active, "SUSPENDED")?;
        self.do_transition(LicenseState::Suspended, &evidence.reason);
        Ok(())
    }

    /// Reinstate a suspended license (SUSPENDED → ACTIVE).
    pub fn reinstate(&mut self, evidence: LicenseTransitionEvidence) -> Result<(), LicenseError> {
        self.require_state(LicenseState::Suspended, "ACTIVE")?;
        self.do_transition(LicenseState::Active, &evidence.reason);
        Ok(())
    }

    /// Revoke the license permanently (ACTIVE or SUSPENDED → REVOKED).
    pub fn revoke(&mut self, evidence: LicenseTransitionEvidence) -> Result<(), LicenseError> {
        if self.state.is_terminal() {
            return Err(LicenseError::TerminalState {
                state: self.state.to_string(),
            });
        }
        if !matches!(self.state, LicenseState::Active | LicenseState::Suspended) {
            return Err(LicenseError::InvalidTransition {
                from: self.state.to_string(),
                to: "REVOKED".to_string(),
            });
        }
        self.do_transition(LicenseState::Revoked, &evidence.reason);
        Ok(())
    }

    /// Expire the license (ACTIVE → EXPIRED).
    ///
    /// Typically triggered by a deadline-based system check.
    pub fn expire(&mut self, evidence: LicenseTransitionEvidence) -> Result<(), LicenseError> {
        self.require_state(LicenseState::Active, "EXPIRED")?;
        self.do_transition(LicenseState::Expired, &evidence.reason);
        Ok(())
    }

    /// Whether the license is currently valid for operations.
    pub fn is_valid(&self) -> bool {
        self.state.is_valid()
    }

    /// Whether the license is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    /// Validate that the license is in the expected state.
    fn require_state(&self, expected: LicenseState, target: &str) -> Result<(), LicenseError> {
        if self.state.is_terminal() {
            return Err(LicenseError::TerminalState {
                state: self.state.to_string(),
            });
        }
        if self.state != expected {
            return Err(LicenseError::InvalidTransition {
                from: self.state.to_string(),
                to: target.to_string(),
            });
        }
        Ok(())
    }

    /// Record a state transition.
    fn do_transition(&mut self, to: LicenseState, reason: &str) {
        self.transitions.push(LicenseTransitionRecord {
            from_state: self.state,
            to_state: to,
            timestamp: Timestamp::now(),
            reason: reason.to_string(),
        });
        self.state = to;
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(reason: &str) -> LicenseTransitionEvidence {
        LicenseTransitionEvidence {
            reason: reason.to_string(),
            actor: Some("test-authority".to_string()),
        }
    }

    fn make_application() -> License {
        License::new_application("business_incorporation".to_string(), "entity-001".to_string())
    }

    fn make_active_license() -> License {
        let mut lic = make_application();
        lic.submit_for_review(evidence("Application complete")).unwrap();
        lic.issue(evidence("Approved"), None).unwrap();
        lic.activate(evidence("Activated")).unwrap();
        lic
    }

    // ── Happy-path lifecycle tests ───────────────────────────────────

    #[test]
    fn test_new_application() {
        let lic = make_application();
        assert_eq!(lic.state, LicenseState::Application);
        assert_eq!(lic.license_type, "business_incorporation");
        assert!(!lic.is_valid());
        assert!(!lic.is_terminal());
    }

    #[test]
    fn test_application_to_review() {
        let mut lic = make_application();
        lic.submit_for_review(evidence("Complete")).unwrap();
        assert_eq!(lic.state, LicenseState::Review);
        assert_eq!(lic.transitions.len(), 1);
    }

    #[test]
    fn test_review_to_issued() {
        let mut lic = make_application();
        lic.submit_for_review(evidence("complete")).unwrap();
        lic.issue(evidence("approved"), None).unwrap();
        assert_eq!(lic.state, LicenseState::Issued);
    }

    #[test]
    fn test_issued_to_active() {
        let mut lic = make_application();
        lic.submit_for_review(evidence("complete")).unwrap();
        lic.issue(evidence("approved"), None).unwrap();
        lic.activate(evidence("activated")).unwrap();
        assert_eq!(lic.state, LicenseState::Active);
        assert!(lic.is_valid());
    }

    #[test]
    fn test_active_to_suspended() {
        let mut lic = make_active_license();
        lic.suspend(evidence("Compliance violation")).unwrap();
        assert_eq!(lic.state, LicenseState::Suspended);
        assert!(!lic.is_valid());
    }

    #[test]
    fn test_suspended_to_active_reinstatement() {
        let mut lic = make_active_license();
        lic.suspend(evidence("Suspended")).unwrap();
        lic.reinstate(evidence("Reinstated")).unwrap();
        assert_eq!(lic.state, LicenseState::Active);
        assert!(lic.is_valid());
    }

    #[test]
    fn test_active_to_revoked() {
        let mut lic = make_active_license();
        lic.revoke(evidence("Fraud detected")).unwrap();
        assert_eq!(lic.state, LicenseState::Revoked);
        assert!(lic.is_terminal());
    }

    #[test]
    fn test_suspended_to_revoked() {
        let mut lic = make_active_license();
        lic.suspend(evidence("suspended")).unwrap();
        lic.revoke(evidence("Revoked during suspension")).unwrap();
        assert_eq!(lic.state, LicenseState::Revoked);
        assert!(lic.is_terminal());
    }

    #[test]
    fn test_active_to_expired() {
        let mut lic = make_active_license();
        lic.expire(evidence("License period ended")).unwrap();
        assert_eq!(lic.state, LicenseState::Expired);
        assert!(lic.is_terminal());
    }

    #[test]
    fn test_review_to_rejected() {
        let mut lic = make_application();
        lic.submit_for_review(evidence("complete")).unwrap();
        lic.reject(evidence("Does not meet requirements")).unwrap();
        assert_eq!(lic.state, LicenseState::Rejected);
        assert!(lic.is_terminal());
    }

    // ── Full lifecycle test ──────────────────────────────────────────

    #[test]
    fn test_full_lifecycle_application_through_expiry() {
        let mut lic = make_application();
        lic.submit_for_review(evidence("Submitted")).unwrap();
        lic.issue(evidence("Approved"), None).unwrap();
        lic.activate(evidence("Activated")).unwrap();
        lic.suspend(evidence("Audit")).unwrap();
        lic.reinstate(evidence("Audit passed")).unwrap();
        lic.expire(evidence("Term ended")).unwrap();

        assert!(lic.is_terminal());
        assert_eq!(lic.transitions.len(), 6);
    }

    // ── Invalid transition tests ─────────────────────────────────────

    #[test]
    fn test_cannot_activate_from_application() {
        let mut lic = make_application();
        let result = lic.activate(evidence("test"));
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_suspend_from_issued() {
        let mut lic = make_application();
        lic.submit_for_review(evidence("complete")).unwrap();
        lic.issue(evidence("approved"), None).unwrap();
        let result = lic.suspend(evidence("test"));
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_expire_from_suspended() {
        let mut lic = make_active_license();
        lic.suspend(evidence("suspended")).unwrap();
        let result = lic.expire(evidence("test"));
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_transition_from_revoked() {
        let mut lic = make_active_license();
        lic.revoke(evidence("revoked")).unwrap();
        let result = lic.activate(evidence("test"));
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_transition_from_expired() {
        let mut lic = make_active_license();
        lic.expire(evidence("expired")).unwrap();
        let result = lic.reinstate(evidence("test"));
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_revoke_from_review() {
        let mut lic = make_application();
        lic.submit_for_review(evidence("complete")).unwrap();
        let result = lic.revoke(evidence("test"));
        assert!(result.is_err());
    }

    // ── Expiry tracking ──────────────────────────────────────────────

    #[test]
    fn test_issue_with_expiry() {
        let mut lic = make_application();
        lic.submit_for_review(evidence("complete")).unwrap();
        let expiry = Timestamp::now();
        lic.issue(evidence("approved"), Some(expiry)).unwrap();
        assert_eq!(lic.expires_at, Some(expiry));
    }

    // ── Display tests ────────────────────────────────────────────────

    #[test]
    fn test_license_state_display() {
        assert_eq!(LicenseState::Application.to_string(), "APPLICATION");
        assert_eq!(LicenseState::Review.to_string(), "REVIEW");
        assert_eq!(LicenseState::Issued.to_string(), "ISSUED");
        assert_eq!(LicenseState::Active.to_string(), "ACTIVE");
        assert_eq!(LicenseState::Suspended.to_string(), "SUSPENDED");
        assert_eq!(LicenseState::Revoked.to_string(), "REVOKED");
        assert_eq!(LicenseState::Expired.to_string(), "EXPIRED");
        assert_eq!(LicenseState::Rejected.to_string(), "REJECTED");
    }

    // ── Serialization tests ──────────────────────────────────────────

    #[test]
    fn test_license_serialization() {
        let lic = make_active_license();
        let json = serde_json::to_string(&lic).unwrap();
        let parsed: License = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.state, lic.state);
        assert_eq!(parsed.license_type, lic.license_type);
    }
}
