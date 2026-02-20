//! # ACRA Integration Adapter Interface
//!
//! Defines the adapter interface for ACRA (Accounting and Corporate Regulatory
//! Authority of Singapore), responsible for company registration via BizFile+,
//! annual filing, and beneficial ownership disclosure.
//!
//! ## Architecture
//!
//! The `AcraAdapter` trait abstracts over the ACRA BizFile+ backend. Production
//! deployments implement it against the live ACRA API; test environments use
//! `MockAcraAdapter`. This mirrors the SECP adapter from the Pakistan vertical.
//!
//! ## UEN-Based Identification
//!
//! All Singapore entities are identified by UEN. ACRA assigns UENs at
//! incorporation. The `validate_uen` helper delegates to `mez_core::Uen::new()`.
//!
//! ## Integration Points
//!
//! - **Entity profile**: Company name, type, status, registration date
//! - **Filing status**: Annual return, financial statements
//! - **Officers**: Directors, company secretary
//! - **Beneficial ownership**: Register of Registrable Controllers (Part XA, Companies Act)

use mez_core::Uen;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::adapter::{AdapterCategory, AdapterHealth, NationalSystemAdapter};

/// Errors from ACRA integration operations.
#[derive(Debug, thiserror::Error)]
pub enum AcraError {
    /// ACRA service is unreachable or returned a 5xx status.
    #[error("ACRA service unavailable: {reason}")]
    ServiceUnavailable {
        /// Human-readable description of the outage or error.
        reason: String,
    },

    /// Entity not found in ACRA registry.
    #[error("entity not found: {uen}")]
    EntityNotFound {
        /// The UEN that was looked up.
        uen: String,
    },

    /// UEN format is invalid.
    #[error("invalid UEN: {reason}")]
    InvalidUen {
        /// Description of the validation failure.
        reason: String,
    },

    /// The ACRA adapter has not been configured for this deployment.
    #[error("ACRA adapter not configured: {reason}")]
    NotConfigured {
        /// Why configuration is missing or incomplete.
        reason: String,
    },

    /// The request to ACRA timed out.
    #[error("ACRA request timed out after {elapsed_ms}ms")]
    Timeout {
        /// Elapsed time in milliseconds before the timeout triggered.
        elapsed_ms: u64,
    },
}

/// Type of entity registered with ACRA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SgEntityType {
    /// Private company limited by shares (Pte. Ltd.).
    PrivateLimited,
    /// Exempt private company.
    ExemptPrivate,
    /// Public company limited by shares (Ltd.).
    PublicLimited,
    /// Company limited by guarantee (CLG).
    LimitedByGuarantee,
    /// Limited liability partnership (LLP).
    Llp,
    /// Sole proprietorship.
    SoleProprietorship,
    /// General or limited partnership.
    Partnership,
}

impl fmt::Display for SgEntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PrivateLimited => write!(f, "PrivateLimited"),
            Self::ExemptPrivate => write!(f, "ExemptPrivate"),
            Self::PublicLimited => write!(f, "PublicLimited"),
            Self::LimitedByGuarantee => write!(f, "LimitedByGuarantee"),
            Self::Llp => write!(f, "LLP"),
            Self::SoleProprietorship => write!(f, "SoleProprietorship"),
            Self::Partnership => write!(f, "Partnership"),
        }
    }
}

/// Entity status in the ACRA registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SgEntityStatus {
    /// Entity is live/active.
    Live,
    /// Entity has been struck off the register.
    StruckOff,
    /// Entity is in the process of winding up.
    WindingUp,
    /// Entity has been dissolved.
    Dissolved,
    /// Entity has been converted to another entity type.
    Converted,
}

impl fmt::Display for SgEntityStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Live => write!(f, "Live"),
            Self::StruckOff => write!(f, "StruckOff"),
            Self::WindingUp => write!(f, "WindingUp"),
            Self::Dissolved => write!(f, "Dissolved"),
            Self::Converted => write!(f, "Converted"),
        }
    }
}

/// Entity profile from the ACRA registry (BizFile+).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcraEntityProfile {
    /// Unique Entity Number.
    pub uen: String,
    /// Registered entity name.
    pub entity_name: String,
    /// Type of entity.
    pub entity_type: SgEntityType,
    /// Current status.
    pub status: SgEntityStatus,
    /// Date of incorporation/registration (ISO 8601, YYYY-MM-DD).
    pub registration_date: String,
    /// Registered address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registered_address: Option<String>,
    /// Primary SSIC (Singapore Standard Industrial Classification) code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_ssic: Option<String>,
    /// Paid-up capital in SGD.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paid_up_capital_sgd: Option<String>,
}

/// Annual filing status from ACRA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FilingStatus {
    /// All annual filings are current.
    Current,
    /// One or more filings are overdue.
    Overdue,
    /// Entity is exempt from filing (e.g. dormant).
    Exempt,
}

impl fmt::Display for FilingStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Current => write!(f, "Current"),
            Self::Overdue => write!(f, "Overdue"),
            Self::Exempt => write!(f, "Exempt"),
        }
    }
}

/// Director record from ACRA.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcraDirectorRecord {
    /// Director's name.
    pub name: String,
    /// NRIC or passport number (masked in production).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_no: Option<String>,
    /// Nationality (ISO 3166-1 alpha-2).
    pub nationality: String,
    /// Date of appointment (ISO 8601, YYYY-MM-DD).
    pub appointment_date: String,
}

/// Adapter trait for ACRA corporate registry operations.
///
/// Implementations must be `Send + Sync` so they can be shared across
/// async tasks behind an `Arc`. The trait is object-safe to support
/// runtime adapter selection (mock vs. live).
pub trait AcraAdapter: Send + Sync {
    /// Look up an entity by UEN.
    fn lookup_entity(
        &self,
        uen: &Uen,
    ) -> Result<AcraEntityProfile, AcraError>;

    /// Check the annual filing status for an entity.
    fn check_filing_status(
        &self,
        uen: &Uen,
    ) -> Result<FilingStatus, AcraError>;

    /// Retrieve director records for an entity.
    fn get_directors(
        &self,
        uen: &Uen,
    ) -> Result<Vec<AcraDirectorRecord>, AcraError>;

    /// Return the human-readable name of this adapter implementation.
    fn adapter_name(&self) -> &str;
}

/// Validate that a UEN string is well-formed by delegating to
/// `mez_core::Uen::new()`. Returns the validated `Uen` on success.
pub fn validate_uen(uen: &str) -> Result<Uen, AcraError> {
    Uen::new(uen).map_err(|e| AcraError::InvalidUen {
        reason: e.to_string(),
    })
}

/// Mock ACRA adapter for testing and development.
///
/// Returns deterministic test data based on UEN conventions:
/// - UEN "000000000" returns EntityNotFound
/// - UENs starting with "T" are treated as companies limited by guarantee
/// - All other valid UENs return private limited companies
#[derive(Debug, Clone)]
pub struct MockAcraAdapter;

impl AcraAdapter for MockAcraAdapter {
    fn lookup_entity(
        &self,
        uen: &Uen,
    ) -> Result<AcraEntityProfile, AcraError> {
        if uen.as_str() == "000000000" {
            return Err(AcraError::EntityNotFound {
                uen: uen.as_str().to_string(),
            });
        }

        let entity_type = if uen.as_str().starts_with('T') {
            SgEntityType::LimitedByGuarantee
        } else {
            SgEntityType::PrivateLimited
        };

        Ok(AcraEntityProfile {
            uen: uen.as_str().to_string(),
            entity_name: format!("Mock Entity {} Pte Ltd", &uen.as_str()[..4.min(uen.as_str().len())]),
            entity_type,
            status: SgEntityStatus::Live,
            registration_date: "2020-01-15".to_string(),
            registered_address: Some("1 Raffles Place, Singapore 048616".to_string()),
            primary_ssic: Some("64201".to_string()),
            paid_up_capital_sgd: Some("100000".to_string()),
        })
    }

    fn check_filing_status(
        &self,
        uen: &Uen,
    ) -> Result<FilingStatus, AcraError> {
        if uen.as_str() == "000000000" {
            return Err(AcraError::EntityNotFound {
                uen: uen.as_str().to_string(),
            });
        }
        Ok(FilingStatus::Current)
    }

    fn get_directors(
        &self,
        uen: &Uen,
    ) -> Result<Vec<AcraDirectorRecord>, AcraError> {
        if uen.as_str() == "000000000" {
            return Err(AcraError::EntityNotFound {
                uen: uen.as_str().to_string(),
            });
        }

        Ok(vec![AcraDirectorRecord {
            name: "Mock Director".to_string(),
            id_no: Some("S****567A".to_string()),
            nationality: "SG".to_string(),
            appointment_date: "2020-01-15".to_string(),
        }])
    }

    fn adapter_name(&self) -> &str {
        "MockAcraAdapter"
    }
}

impl NationalSystemAdapter for MockAcraAdapter {
    fn category(&self) -> AdapterCategory {
        AdapterCategory::Corporate
    }

    fn jurisdiction(&self) -> &str {
        "sg"
    }

    fn health(&self) -> AdapterHealth {
        AdapterHealth::Healthy
    }

    fn adapter_name(&self) -> &str {
        "MockAcraAdapter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- validate_uen -----------------------------------------------------------

    #[test]
    fn validate_uen_accepts_valid() {
        let result = validate_uen("200012345A");
        assert!(result.is_ok());
    }

    #[test]
    fn validate_uen_rejects_too_short() {
        let result = validate_uen("12");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AcraError::InvalidUen { .. }));
    }

    // -- SgEntityType -----------------------------------------------------------

    #[test]
    fn entity_type_display() {
        assert_eq!(format!("{}", SgEntityType::PrivateLimited), "PrivateLimited");
        assert_eq!(format!("{}", SgEntityType::Llp), "LLP");
        assert_eq!(
            format!("{}", SgEntityType::SoleProprietorship),
            "SoleProprietorship"
        );
    }

    #[test]
    fn entity_type_serde_roundtrip() {
        let et = SgEntityType::PrivateLimited;
        let json = serde_json::to_string(&et).expect("serialize");
        let back: SgEntityType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, SgEntityType::PrivateLimited);
    }

    // -- SgEntityStatus ---------------------------------------------------------

    #[test]
    fn entity_status_display() {
        assert_eq!(format!("{}", SgEntityStatus::Live), "Live");
        assert_eq!(format!("{}", SgEntityStatus::StruckOff), "StruckOff");
        assert_eq!(format!("{}", SgEntityStatus::WindingUp), "WindingUp");
        assert_eq!(format!("{}", SgEntityStatus::Dissolved), "Dissolved");
        assert_eq!(format!("{}", SgEntityStatus::Converted), "Converted");
    }

    // -- FilingStatus -----------------------------------------------------------

    #[test]
    fn filing_status_display() {
        assert_eq!(format!("{}", FilingStatus::Current), "Current");
        assert_eq!(format!("{}", FilingStatus::Overdue), "Overdue");
        assert_eq!(format!("{}", FilingStatus::Exempt), "Exempt");
    }

    // -- AcraError --------------------------------------------------------------

    #[test]
    fn acra_error_display() {
        let err = AcraError::EntityNotFound {
            uen: "200012345A".to_string(),
        };
        assert!(format!("{err}").contains("200012345A"));

        let err = AcraError::Timeout { elapsed_ms: 10000 };
        assert!(format!("{err}").contains("10000"));
    }

    // -- MockAcraAdapter --------------------------------------------------------

    #[test]
    fn mock_acra_lookup_entity() {
        let adapter = MockAcraAdapter;
        let uen = Uen::new("200012345A").expect("valid UEN");
        let profile = adapter.lookup_entity(&uen).expect("should succeed");
        assert_eq!(profile.entity_type, SgEntityType::PrivateLimited);
        assert_eq!(profile.status, SgEntityStatus::Live);
        assert!(profile.registered_address.is_some());
    }

    #[test]
    fn mock_acra_lookup_clg() {
        let adapter = MockAcraAdapter;
        let uen = Uen::new("T12345678A").expect("valid UEN");
        let profile = adapter.lookup_entity(&uen).expect("should succeed");
        assert_eq!(profile.entity_type, SgEntityType::LimitedByGuarantee);
    }

    #[test]
    fn mock_acra_lookup_not_found() {
        let adapter = MockAcraAdapter;
        let uen = Uen::new("000000000").expect("valid UEN");
        let result = adapter.lookup_entity(&uen);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AcraError::EntityNotFound { .. }));
    }

    #[test]
    fn mock_acra_check_filing_status() {
        let adapter = MockAcraAdapter;
        let uen = Uen::new("200012345A").expect("valid UEN");
        let status = adapter.check_filing_status(&uen).expect("should succeed");
        assert_eq!(status, FilingStatus::Current);
    }

    #[test]
    fn mock_acra_get_directors() {
        let adapter = MockAcraAdapter;
        let uen = Uen::new("200012345A").expect("valid UEN");
        let directors = adapter.get_directors(&uen).expect("should succeed");
        assert_eq!(directors.len(), 1);
        assert_eq!(directors[0].nationality, "SG");
    }

    #[test]
    fn mock_acra_adapter_name() {
        let adapter = MockAcraAdapter;
        assert_eq!(AcraAdapter::adapter_name(&adapter), "MockAcraAdapter");
    }

    #[test]
    fn mock_acra_national_system_adapter() {
        let adapter = MockAcraAdapter;
        assert_eq!(
            NationalSystemAdapter::category(&adapter),
            AdapterCategory::Corporate
        );
        assert_eq!(NationalSystemAdapter::jurisdiction(&adapter), "sg");
    }

    #[test]
    fn trait_object_safety() {
        let adapter: Box<dyn AcraAdapter> = Box::new(MockAcraAdapter);
        assert_eq!(adapter.adapter_name(), "MockAcraAdapter");
    }

    #[test]
    fn entity_profile_serde_roundtrip() {
        let profile = AcraEntityProfile {
            uen: "200012345A".to_string(),
            entity_name: "Test Pte Ltd".to_string(),
            entity_type: SgEntityType::PrivateLimited,
            status: SgEntityStatus::Live,
            registration_date: "2020-01-15".to_string(),
            registered_address: Some("1 Test Street".to_string()),
            primary_ssic: Some("64201".to_string()),
            paid_up_capital_sgd: None,
        };
        let json = serde_json::to_string(&profile).expect("serialize");
        let back: AcraEntityProfile = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.uen, "200012345A");
        assert_eq!(back.entity_type, SgEntityType::PrivateLimited);
        // paid_up_capital_sgd was None, should not appear
        assert!(!json.contains("paid_up_capital_sgd"));
    }

    #[test]
    fn director_record_serde_roundtrip() {
        let record = AcraDirectorRecord {
            name: "Test Director".to_string(),
            id_no: Some("S****567A".to_string()),
            nationality: "SG".to_string(),
            appointment_date: "2020-01-15".to_string(),
        };
        let json = serde_json::to_string(&record).expect("serialize");
        let back: AcraDirectorRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.name, "Test Director");
    }
}
