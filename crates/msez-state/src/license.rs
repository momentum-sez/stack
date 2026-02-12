//! # License Lifecycle State Machine
//!
//! Models the lifecycle of business licenses within a jurisdiction.
//!
//! ## States
//!
//! APPLIED → ISSUED → ACTIVE → SUSPENDED → REVOKED | EXPIRED
//!
//! ## Implements
//!
//! Spec §15 — License lifecycle management.

/// The lifecycle state of a license.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LicenseState {
    /// License application submitted.
    Applied,
    /// License has been issued but not yet active.
    Issued,
    /// License is active and valid.
    Active,
    /// License has been temporarily suspended.
    Suspended,
    /// License has been permanently revoked (terminal).
    Revoked,
    /// License has expired (terminal).
    Expired,
}

impl LicenseState {
    /// Whether this state is terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Revoked | Self::Expired)
    }
}

impl std::fmt::Display for LicenseState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Applied => "APPLIED",
            Self::Issued => "ISSUED",
            Self::Active => "ACTIVE",
            Self::Suspended => "SUSPENDED",
            Self::Revoked => "REVOKED",
            Self::Expired => "EXPIRED",
        };
        f.write_str(s)
    }
}

/// A license with its lifecycle state.
///
/// Placeholder — full implementation will enforce state transitions
/// with evidence requirements and expiry tracking.
#[derive(Debug)]
pub struct License {
    /// Current lifecycle state.
    pub state: LicenseState,
}
