//! # MyInfo / Singpass Integration Adapter Interface
//!
//! Defines the adapter interface for Singapore's MyInfo platform (operated by
//! GovTech), which provides verified government-sourced personal data via
//! Singpass authentication.
//!
//! ## Architecture
//!
//! The `MyInfoAdapter` trait abstracts over the MyInfo API backend. Production
//! deployments implement it against the live MyInfo API (v3/v4); test environments
//! use `MockMyInfoAdapter`. This mirrors the NADRA adapter from the Pakistan vertical.
//!
//! ## NRIC Validation
//!
//! Singapore NRICs follow the format [STFGM]XXXXXXX[A-Z] (9 characters total).
//! The `validate_nric` helper delegates to `mez_core::Nric::new()` for validation.
//!
//! ## MyInfo Data Categories
//!
//! MyInfo provides data across categories:
//! - **Person**: Name, DOB, nationality, sex, race, residential status
//! - **Identity**: NRIC, passport details
//! - **Income**: Employment, NOA (Notice of Assessment)
//! - **Address**: Registered address
//!
//! Consent-based access: the individual must authorize data sharing via Singpass.

use mez_core::Nric;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::adapter::{AdapterCategory, AdapterHealth, NationalSystemAdapter};

/// Errors from MyInfo integration operations.
#[derive(Debug, thiserror::Error)]
pub enum MyInfoError {
    /// MyInfo service is unreachable or returned a 5xx status.
    #[error("MyInfo service unavailable: {reason}")]
    ServiceUnavailable {
        /// Human-readable description of the outage or error.
        reason: String,
    },

    /// NRIC format is invalid.
    #[error("invalid NRIC: {reason}")]
    InvalidNric {
        /// Description of the validation failure.
        reason: String,
    },

    /// MyInfo accepted the request but verification could not be completed.
    #[error("identity verification failed: {reason}")]
    VerificationFailed {
        /// Description of the failure.
        reason: String,
    },

    /// The MyInfo adapter has not been configured for this deployment.
    #[error("MyInfo adapter not configured: {reason}")]
    NotConfigured {
        /// Why configuration is missing or incomplete.
        reason: String,
    },

    /// The request to MyInfo timed out.
    #[error("MyInfo request timed out after {elapsed_ms}ms")]
    Timeout {
        /// Elapsed time in milliseconds before the timeout triggered.
        elapsed_ms: u64,
    },

    /// Consent has not been granted for this data request.
    #[error("consent not granted: {reason}")]
    ConsentNotGranted {
        /// Description of the missing consent.
        reason: String,
    },
}

/// NRIC status as reported via MyInfo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NricStatus {
    /// NRIC is active (citizen or permanent resident).
    Active,
    /// NRIC holder's status has lapsed (e.g. PR lapsed).
    Lapsed,
    /// NRIC number does not exist in records.
    NotFound,
}

impl fmt::Display for NricStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "Active"),
            Self::Lapsed => write!(f, "Lapsed"),
            Self::NotFound => write!(f, "NotFound"),
        }
    }
}

/// Residential status categories in Singapore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResidentialStatus {
    /// Singapore citizen.
    Citizen,
    /// Permanent resident.
    PermanentResident,
    /// Foreigner (work pass, dependent pass, etc.).
    Foreigner,
}

impl fmt::Display for ResidentialStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Citizen => write!(f, "Citizen"),
            Self::PermanentResident => write!(f, "PermanentResident"),
            Self::Foreigner => write!(f, "Foreigner"),
        }
    }
}

/// MyInfo person data response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyInfoPersonData {
    /// NRIC (masked except last 4 chars in production, full in sandbox).
    pub nric: String,
    /// Full name as per NRIC.
    pub name: String,
    /// Date of birth (ISO 8601, YYYY-MM-DD).
    pub date_of_birth: String,
    /// Nationality (ISO 3166-1 alpha-2).
    pub nationality: String,
    /// Residential status.
    pub residential_status: ResidentialStatus,
    /// Registered address line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registered_address: Option<String>,
}

/// NRIC verification response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NricVerificationResponse {
    /// Whether the identity was successfully verified.
    pub verified: bool,
    /// Current status of the NRIC.
    pub nric_status: NricStatus,
    /// ISO 8601 timestamp of the verification.
    pub verification_timestamp: String,
    /// Reference identifier.
    pub reference: String,
}

/// Adapter trait for MyInfo / Singpass identity verification.
///
/// Implementations must be `Send + Sync` so they can be shared across
/// async tasks behind an `Arc`. The trait is object-safe to support
/// runtime adapter selection (mock vs. live).
pub trait MyInfoAdapter: Send + Sync {
    /// Verify an individual's NRIC status via MyInfo.
    fn verify_nric(
        &self,
        nric: &Nric,
    ) -> Result<NricVerificationResponse, MyInfoError>;

    /// Retrieve person data from MyInfo (requires prior consent).
    fn get_person_data(
        &self,
        nric: &Nric,
        consent_token: &str,
    ) -> Result<MyInfoPersonData, MyInfoError>;

    /// Return the human-readable name of this adapter implementation.
    fn adapter_name(&self) -> &str;
}

/// Validate that an NRIC string is well-formed by delegating to
/// `mez_core::Nric::new()`. Returns the validated `Nric` on success.
pub fn validate_nric(nric: &str) -> Result<Nric, MyInfoError> {
    Nric::new(nric).map_err(|e| MyInfoError::InvalidNric {
        reason: e.to_string(),
    })
}

/// Mock MyInfo adapter for testing and development.
///
/// Returns deterministic test data for any valid NRIC:
/// - NRICs starting with "S" or "T" are treated as citizens
/// - NRICs starting with "F" or "G" are treated as permanent residents
/// - NRICs starting with "M" are treated as foreigners
#[derive(Debug, Clone)]
pub struct MockMyInfoAdapter;

impl MyInfoAdapter for MockMyInfoAdapter {
    fn verify_nric(
        &self,
        nric: &Nric,
    ) -> Result<NricVerificationResponse, MyInfoError> {
        Ok(NricVerificationResponse {
            verified: true,
            nric_status: NricStatus::Active,
            verification_timestamp: chrono::Utc::now().to_rfc3339(),
            reference: format!("MOCK-MYINFO-{}", &nric.as_str()[..4]),
        })
    }

    fn get_person_data(
        &self,
        nric: &Nric,
        _consent_token: &str,
    ) -> Result<MyInfoPersonData, MyInfoError> {
        let residential_status = match nric.as_str().as_bytes().first() {
            Some(b'S' | b'T') => ResidentialStatus::Citizen,
            Some(b'F' | b'G') => ResidentialStatus::PermanentResident,
            _ => ResidentialStatus::Foreigner,
        };

        Ok(MyInfoPersonData {
            nric: nric.as_str().to_string(),
            name: "Mock Person".to_string(),
            date_of_birth: "1990-01-15".to_string(),
            nationality: "SG".to_string(),
            residential_status,
            registered_address: Some("1 Test Street, Singapore 123456".to_string()),
        })
    }

    fn adapter_name(&self) -> &str {
        "MockMyInfoAdapter"
    }
}

impl NationalSystemAdapter for MockMyInfoAdapter {
    fn category(&self) -> AdapterCategory {
        AdapterCategory::Identity
    }

    fn jurisdiction(&self) -> &str {
        "sg"
    }

    fn health(&self) -> AdapterHealth {
        AdapterHealth::Healthy
    }

    fn adapter_name(&self) -> &str {
        "MockMyInfoAdapter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- validate_nric ----------------------------------------------------------

    #[test]
    fn validate_nric_accepts_valid() {
        let result = validate_nric("S1234567A");
        assert!(result.is_ok());
    }

    #[test]
    fn validate_nric_rejects_invalid_prefix() {
        let result = validate_nric("X1234567A");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MyInfoError::InvalidNric { .. }));
    }

    #[test]
    fn validate_nric_rejects_too_short() {
        let result = validate_nric("S123");
        assert!(result.is_err());
    }

    // -- NricStatus -------------------------------------------------------------

    #[test]
    fn nric_status_display() {
        assert_eq!(format!("{}", NricStatus::Active), "Active");
        assert_eq!(format!("{}", NricStatus::Lapsed), "Lapsed");
        assert_eq!(format!("{}", NricStatus::NotFound), "NotFound");
    }

    #[test]
    fn nric_status_serde_roundtrip() {
        let status = NricStatus::Active;
        let json = serde_json::to_string(&status).expect("serialize");
        let back: NricStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, NricStatus::Active);
    }

    // -- ResidentialStatus ------------------------------------------------------

    #[test]
    fn residential_status_display() {
        assert_eq!(format!("{}", ResidentialStatus::Citizen), "Citizen");
        assert_eq!(
            format!("{}", ResidentialStatus::PermanentResident),
            "PermanentResident"
        );
        assert_eq!(format!("{}", ResidentialStatus::Foreigner), "Foreigner");
    }

    // -- MyInfoError ------------------------------------------------------------

    #[test]
    fn myinfo_error_display() {
        let err = MyInfoError::ServiceUnavailable {
            reason: "API down".to_string(),
        };
        assert!(format!("{err}").contains("API down"));

        let err = MyInfoError::ConsentNotGranted {
            reason: "user declined".to_string(),
        };
        assert!(format!("{err}").contains("user declined"));
    }

    // -- MockMyInfoAdapter ------------------------------------------------------

    #[test]
    fn mock_myinfo_verify_nric() {
        let adapter = MockMyInfoAdapter;
        let nric = Nric::new("S1234567A").expect("valid NRIC");
        let resp = adapter.verify_nric(&nric).expect("should succeed");
        assert!(resp.verified);
        assert_eq!(resp.nric_status, NricStatus::Active);
    }

    #[test]
    fn mock_myinfo_get_person_data_citizen() {
        let adapter = MockMyInfoAdapter;
        let nric = Nric::new("S1234567A").expect("valid NRIC");
        let data = adapter
            .get_person_data(&nric, "mock-consent-token")
            .expect("should succeed");
        assert_eq!(data.residential_status, ResidentialStatus::Citizen);
        assert_eq!(data.nationality, "SG");
    }

    #[test]
    fn mock_myinfo_get_person_data_pr() {
        let adapter = MockMyInfoAdapter;
        let nric = Nric::new("F1234567A").expect("valid NRIC");
        let data = adapter
            .get_person_data(&nric, "mock-consent-token")
            .expect("should succeed");
        assert_eq!(data.residential_status, ResidentialStatus::PermanentResident);
    }

    #[test]
    fn mock_myinfo_get_person_data_foreigner() {
        let adapter = MockMyInfoAdapter;
        let nric = Nric::new("M1234567A").expect("valid NRIC");
        let data = adapter
            .get_person_data(&nric, "mock-consent-token")
            .expect("should succeed");
        assert_eq!(data.residential_status, ResidentialStatus::Foreigner);
    }

    #[test]
    fn mock_myinfo_adapter_name() {
        let adapter = MockMyInfoAdapter;
        assert_eq!(MyInfoAdapter::adapter_name(&adapter), "MockMyInfoAdapter");
    }

    #[test]
    fn mock_myinfo_national_system_adapter() {
        let adapter = MockMyInfoAdapter;
        assert_eq!(
            NationalSystemAdapter::category(&adapter),
            AdapterCategory::Identity
        );
        assert_eq!(NationalSystemAdapter::jurisdiction(&adapter), "sg");
    }

    #[test]
    fn trait_object_safety() {
        let adapter: Box<dyn MyInfoAdapter> = Box::new(MockMyInfoAdapter);
        assert_eq!(adapter.adapter_name(), "MockMyInfoAdapter");
    }

    #[test]
    fn person_data_serde_roundtrip() {
        let data = MyInfoPersonData {
            nric: "S1234567A".to_string(),
            name: "Test".to_string(),
            date_of_birth: "1990-01-15".to_string(),
            nationality: "SG".to_string(),
            residential_status: ResidentialStatus::Citizen,
            registered_address: None,
        };
        let json = serde_json::to_string(&data).expect("serialize");
        let back: MyInfoPersonData = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.nric, "S1234567A");
        // address was None, should not appear in JSON
        assert!(!json.contains("registered_address"));
    }
}
