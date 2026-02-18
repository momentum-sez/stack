//! Licensepack enum types.
//!
//! Core enumerations for license status, domain classification, and
//! compliance tensor state mapping.

use serde::{Deserialize, Serialize};

/// License status enumeration.
///
/// Represents the lifecycle state of a business license. Terminal states
/// (Revoked, Expired, Surrendered) cannot transition back to Active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseStatus {
    /// License is currently active and valid.
    Active,
    /// License is temporarily suspended.
    Suspended,
    /// License has been revoked for cause (terminal).
    Revoked,
    /// License has expired (terminal).
    Expired,
    /// License application is pending.
    Pending,
    /// License was voluntarily surrendered (terminal).
    Surrendered,
}

impl LicenseStatus {
    /// Whether this status represents a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Revoked | Self::Expired | Self::Surrendered)
    }

    /// String representation matching Python enum values.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Revoked => "revoked",
            Self::Expired => "expired",
            Self::Pending => "pending",
            Self::Surrendered => "surrendered",
        }
    }
}

impl std::fmt::Display for LicenseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// License domain categories.
///
/// Classifies licenses by the regulatory domain they cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseDomain {
    /// Financial services licenses.
    Financial,
    /// Corporate services licenses.
    Corporate,
    /// Professional certifications.
    Professional,
    /// Trade licenses.
    Trade,
    /// Insurance licenses.
    Insurance,
    /// Mixed/multi-domain licenses.
    Mixed,
}

impl LicenseDomain {
    /// String representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Financial => "financial",
            Self::Corporate => "corporate",
            Self::Professional => "professional",
            Self::Trade => "trade",
            Self::Insurance => "insurance",
            Self::Mixed => "mixed",
        }
    }
}

impl std::fmt::Display for LicenseDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Compliance tensor states for the LICENSING domain.
///
/// Maps license status to the compliance tensor lattice:
/// `NON_COMPLIANT < SUSPENDED < UNKNOWN < PENDING < COMPLIANT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LicenseComplianceState {
    /// License is valid and permits the activity.
    Compliant,
    /// No valid license, expired, or activity not permitted.
    NonCompliant,
    /// License application is pending.
    Pending,
    /// License is temporarily suspended.
    Suspended,
    /// License state is unknown.
    Unknown,
}

impl LicenseComplianceState {
    /// String representation matching Python enum values.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Compliant => "COMPLIANT",
            Self::NonCompliant => "NON_COMPLIANT",
            Self::Pending => "PENDING",
            Self::Suspended => "SUSPENDED",
            Self::Unknown => "UNKNOWN",
        }
    }
}

impl std::fmt::Display for LicenseComplianceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_license_status_terminal() {
        assert!(!LicenseStatus::Active.is_terminal());
        assert!(!LicenseStatus::Suspended.is_terminal());
        assert!(!LicenseStatus::Pending.is_terminal());
        assert!(LicenseStatus::Revoked.is_terminal());
        assert!(LicenseStatus::Expired.is_terminal());
        assert!(LicenseStatus::Surrendered.is_terminal());
    }

    #[test]
    fn test_license_status_serialization() {
        let json = serde_json::to_string(&LicenseStatus::Active).unwrap();
        assert_eq!(json, "\"active\"");
        let parsed: LicenseStatus = serde_json::from_str("\"suspended\"").unwrap();
        assert_eq!(parsed, LicenseStatus::Suspended);
    }

    #[test]
    fn test_license_domain_serialization() {
        let json = serde_json::to_string(&LicenseDomain::Financial).unwrap();
        assert_eq!(json, "\"financial\"");
    }

    #[test]
    fn test_compliance_state_values() {
        assert_eq!(LicenseComplianceState::Compliant.as_str(), "COMPLIANT");
        assert_eq!(
            LicenseComplianceState::NonCompliant.as_str(),
            "NON_COMPLIANT"
        );
        assert_eq!(LicenseComplianceState::Pending.as_str(), "PENDING");
        assert_eq!(LicenseComplianceState::Suspended.as_str(), "SUSPENDED");
        assert_eq!(LicenseComplianceState::Unknown.as_str(), "UNKNOWN");
    }

    #[test]
    fn test_license_status_display() {
        assert_eq!(format!("{}", LicenseStatus::Active), "active");
        assert_eq!(format!("{}", LicenseStatus::Suspended), "suspended");
        assert_eq!(format!("{}", LicenseStatus::Revoked), "revoked");
        assert_eq!(format!("{}", LicenseStatus::Expired), "expired");
        assert_eq!(format!("{}", LicenseStatus::Pending), "pending");
        assert_eq!(format!("{}", LicenseStatus::Surrendered), "surrendered");
    }

    #[test]
    fn test_license_domain_display() {
        assert_eq!(format!("{}", LicenseDomain::Financial), "financial");
        assert_eq!(format!("{}", LicenseDomain::Corporate), "corporate");
        assert_eq!(format!("{}", LicenseDomain::Professional), "professional");
        assert_eq!(format!("{}", LicenseDomain::Trade), "trade");
        assert_eq!(format!("{}", LicenseDomain::Insurance), "insurance");
        assert_eq!(format!("{}", LicenseDomain::Mixed), "mixed");
    }

    #[test]
    fn test_license_domain_serialization_roundtrip() {
        for domain in [
            LicenseDomain::Financial,
            LicenseDomain::Corporate,
            LicenseDomain::Professional,
            LicenseDomain::Trade,
            LicenseDomain::Insurance,
            LicenseDomain::Mixed,
        ] {
            let json = serde_json::to_string(&domain).unwrap();
            let deserialized: LicenseDomain = serde_json::from_str(&json).unwrap();
            assert_eq!(domain, deserialized);
        }
    }

    #[test]
    fn test_license_compliance_state_display() {
        assert_eq!(
            format!("{}", LicenseComplianceState::Compliant),
            "COMPLIANT"
        );
        assert_eq!(
            format!("{}", LicenseComplianceState::NonCompliant),
            "NON_COMPLIANT"
        );
        assert_eq!(format!("{}", LicenseComplianceState::Pending), "PENDING");
        assert_eq!(
            format!("{}", LicenseComplianceState::Suspended),
            "SUSPENDED"
        );
        assert_eq!(format!("{}", LicenseComplianceState::Unknown), "UNKNOWN");
    }
}
