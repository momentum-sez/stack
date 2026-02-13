//! # Arbitration Error Types
//!
//! Structured error hierarchy for the arbitration subsystem.
//! Every error variant carries diagnostic context: the operation that failed,
//! the state at the time of failure, and actionable information for operators.
//!
//! ## Spec Reference
//!
//! Error handling follows Definition 26 error reporting requirements:
//! state machine rejections include current state, attempted transition,
//! and rejection reason.

use thiserror::Error;

/// Errors arising from arbitration operations.
///
/// Each variant carries enough context for operators to diagnose the failure
/// without inspecting logs. State machine errors include the current and
/// target states. Escrow errors include the escrow ID and current status.
#[derive(Error, Debug)]
pub enum ArbitrationError {
    /// Attempted state transition is not valid from the current dispute state.
    #[error("invalid dispute transition from {from} to {to}: {reason}")]
    InvalidTransition {
        /// The current state name.
        from: String,
        /// The attempted target state name.
        to: String,
        /// Human-readable reason for the rejection.
        reason: String,
    },

    /// Dispute is in a terminal state and cannot accept further transitions.
    #[error("dispute {dispute_id} is in terminal state {state}")]
    TerminalState {
        /// The dispute identifier.
        dispute_id: String,
        /// The terminal state name.
        state: String,
    },

    /// Escrow operation violated status preconditions.
    #[error("escrow {escrow_id} cannot perform {operation} in status {status}")]
    InvalidEscrowOperation {
        /// The escrow account identifier.
        escrow_id: String,
        /// The attempted operation (e.g., "release", "deposit").
        operation: String,
        /// The current escrow status.
        status: String,
    },

    /// Escrow has exceeded its configured deadline.
    #[error("escrow {escrow_id} exceeded deadline {deadline}")]
    EscrowTimeout {
        /// The escrow account identifier.
        escrow_id: String,
        /// The deadline that was exceeded (ISO 8601).
        deadline: String,
    },

    /// Partial release amount exceeds remaining escrow balance.
    #[error(
        "partial release of {requested} exceeds remaining balance {remaining} for escrow {escrow_id}"
    )]
    InsufficientEscrowBalance {
        /// The escrow account identifier.
        escrow_id: String,
        /// The requested release amount.
        requested: String,
        /// The remaining escrow balance.
        remaining: String,
    },

    /// Evidence integrity verification failed — stored digest does not match
    /// recomputed digest.
    #[error(
        "evidence integrity violation for {evidence_id}: expected digest {expected}, got {actual}"
    )]
    EvidenceIntegrityViolation {
        /// The evidence item identifier.
        evidence_id: String,
        /// The expected digest (from the evidence record).
        expected: String,
        /// The actual recomputed digest.
        actual: String,
    },

    /// Enforcement precondition not met (e.g., appeal period not expired).
    #[error("enforcement precondition not met for order {order_id}: {reason}")]
    EnforcementPreconditionFailed {
        /// The order identifier.
        order_id: String,
        /// Why the precondition failed.
        reason: String,
    },

    /// Canonicalization error during digest computation.
    #[error("canonicalization error: {0}")]
    Canonicalization(#[from] msez_core::CanonicalizationError),

    /// Invalid monetary amount string.
    #[error("invalid monetary amount: \"{0}\"")]
    InvalidAmount(String),

    /// Invalid dispute type string.
    #[error("unsupported dispute type: \"{0}\"")]
    InvalidDisputeType(String),

    /// Invalid evidence type string.
    #[error("unsupported evidence type: \"{0}\"")]
    InvalidEvidenceType(String),

    /// Invalid escrow type string.
    #[error("unsupported escrow type: \"{0}\"")]
    InvalidEscrowType(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_transition_display() {
        let err = ArbitrationError::InvalidTransition {
            from: "Filed".to_string(),
            to: "Awarded".to_string(),
            reason: "must go through hearing".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("Filed"));
        assert!(msg.contains("Awarded"));
        assert!(msg.contains("must go through hearing"));
    }

    #[test]
    fn terminal_state_display() {
        let err = ArbitrationError::TerminalState {
            dispute_id: "disp-001".to_string(),
            state: "Settled".to_string(),
        };
        assert!(format!("{err}").contains("disp-001"));
        assert!(format!("{err}").contains("Settled"));
    }

    #[test]
    fn invalid_escrow_operation_display() {
        let err = ArbitrationError::InvalidEscrowOperation {
            escrow_id: "esc-001".to_string(),
            operation: "release".to_string(),
            status: "frozen".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("esc-001"));
        assert!(msg.contains("release"));
        assert!(msg.contains("frozen"));
    }

    #[test]
    fn escrow_timeout_display() {
        let err = ArbitrationError::EscrowTimeout {
            escrow_id: "esc-002".to_string(),
            deadline: "2026-01-15T00:00:00Z".to_string(),
        };
        assert!(format!("{err}").contains("esc-002"));
    }

    #[test]
    fn insufficient_escrow_balance_display() {
        let err = ArbitrationError::InsufficientEscrowBalance {
            escrow_id: "esc-003".to_string(),
            requested: "1000".to_string(),
            remaining: "500".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("1000"));
        assert!(msg.contains("500"));
    }

    #[test]
    fn evidence_integrity_violation_display() {
        let err = ArbitrationError::EvidenceIntegrityViolation {
            evidence_id: "ev-001".to_string(),
            expected: "aabb".to_string(),
            actual: "ccdd".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("ev-001"));
        assert!(msg.contains("aabb"));
        assert!(msg.contains("ccdd"));
    }

    #[test]
    fn enforcement_precondition_failed_display() {
        let err = ArbitrationError::EnforcementPreconditionFailed {
            order_id: "ord-001".to_string(),
            reason: "appeal period active".to_string(),
        };
        assert!(format!("{err}").contains("appeal period active"));
    }

    #[test]
    fn invalid_amount_display() {
        let err = ArbitrationError::InvalidAmount("NaN".to_string());
        assert!(format!("{err}").contains("NaN"));
    }

    #[test]
    fn invalid_dispute_type_display() {
        let err = ArbitrationError::InvalidDisputeType("unknown".to_string());
        assert!(format!("{err}").contains("unknown"));
    }

    #[test]
    fn invalid_evidence_type_display() {
        let err = ArbitrationError::InvalidEvidenceType("bad".to_string());
        assert!(format!("{err}").contains("bad"));
    }

    #[test]
    fn invalid_escrow_type_display() {
        let err = ArbitrationError::InvalidEscrowType("foo".to_string());
        assert!(format!("{err}").contains("foo"));
    }

    #[test]
    fn all_variants_are_debug() {
        let err = ArbitrationError::InvalidAmount("test".to_string());
        assert!(!format!("{err:?}").is_empty());
    }
}
