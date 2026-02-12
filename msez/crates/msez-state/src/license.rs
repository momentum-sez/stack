//! # License Lifecycle State Machine
//!
//! Manages the lifecycle of business licenses, professional certifications,
//! and regulatory authorizations within a jurisdiction.

use serde::{Deserialize, Serialize};

/// The lifecycle state of a license.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LicenseState {
    /// License application submitted.
    Applied,
    /// License is under review.
    UnderReview,
    /// License has been granted and is active.
    Active,
    /// License has been suspended pending investigation.
    Suspended,
    /// License has been revoked. Terminal state.
    Revoked,
    /// License has expired and was not renewed. Terminal state.
    Expired,
    /// License was voluntarily surrendered. Terminal state.
    Surrendered,
}

/// A license within the SEZ lifecycle system.
#[derive(Debug)]
pub struct License {
    /// The current lifecycle state.
    pub state: LicenseState,
    /// The license category (e.g., "MANUFACTURING", "TRADING", "PROFESSIONAL").
    pub category: String,
}
