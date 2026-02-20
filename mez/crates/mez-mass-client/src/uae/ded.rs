//! # DED / ADGM-RA Integration Adapter Interface
//!
//! Defines the adapter interface for commercial registry operations in
//! the UAE. Depending on the free zone:
//! - **DED** (Department of Economic Development): Mainland commercial licenses
//! - **ADGM-RA** (Registration Authority): ADGM free zone company registration
//! - **DIFC-ROC** (Registrar of Companies): DIFC free zone registration
//!
//! ## Architecture
//!
//! The `UaeCorporateRegistryAdapter` trait abstracts over the specific registry
//! backend. This mirrors the SECP adapter from the Pakistan vertical, providing
//! company lookup, license verification, and director/shareholder information.
//!
//! ## License Numbers
//!
//! UAE commercial license numbers vary by authority:
//! - DED: 6-8 digit numeric
//! - ADGM: Alphanumeric (e.g. "000001234")
//! - DIFC: Numeric with CL- prefix
//!
//! The `validate_license_no` helper enforces a basic non-empty alphanumeric check.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::adapter::{AdapterCategory, AdapterHealth, NationalSystemAdapter};

/// Errors from UAE corporate registry operations.
#[derive(Debug, thiserror::Error)]
pub enum UaeCorporateError {
    /// Corporate registry service is unreachable or returned a 5xx status.
    #[error("corporate registry service unavailable: {reason}")]
    ServiceUnavailable {
        /// Human-readable description of the outage or error.
        reason: String,
    },

    /// Company not found in the registry.
    #[error("company not found: {license_no}")]
    CompanyNotFound {
        /// The license number that was looked up.
        license_no: String,
    },

    /// License number format is invalid.
    #[error("invalid license number: {reason}")]
    InvalidLicenseNumber {
        /// Description of the validation failure.
        reason: String,
    },

    /// The adapter has not been configured for this deployment.
    #[error("UAE corporate registry adapter not configured: {reason}")]
    NotConfigured {
        /// Why configuration is missing or incomplete.
        reason: String,
    },

    /// The request timed out.
    #[error("corporate registry request timed out after {elapsed_ms}ms")]
    Timeout {
        /// Elapsed time in milliseconds before the timeout triggered.
        elapsed_ms: u64,
    },
}

/// Type of company entity in the UAE.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UaeCompanyType {
    /// Limited Liability Company (LLC) — mainland.
    Llc,
    /// Free Zone Establishment (FZE) — single shareholder.
    Fze,
    /// Free Zone Company (FZCO) — multiple shareholders.
    Fzco,
    /// Special Purpose Vehicle.
    Spv,
    /// Branch office of a foreign company.
    Branch,
}

impl fmt::Display for UaeCompanyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Llc => write!(f, "LLC"),
            Self::Fze => write!(f, "FZE"),
            Self::Fzco => write!(f, "FZCO"),
            Self::Spv => write!(f, "SPV"),
            Self::Branch => write!(f, "Branch"),
        }
    }
}

/// Company status in the UAE corporate registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UaeCompanyStatus {
    /// Company is active and in good standing.
    Active,
    /// License expired, pending renewal.
    Expired,
    /// Company is in the process of liquidation.
    InLiquidation,
    /// Company has been struck off the register.
    StruckOff,
}

impl fmt::Display for UaeCompanyStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "Active"),
            Self::Expired => write!(f, "Expired"),
            Self::InLiquidation => write!(f, "InLiquidation"),
            Self::StruckOff => write!(f, "StruckOff"),
        }
    }
}

/// Company record from the UAE corporate registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UaeCompanyRecord {
    /// Commercial license number.
    pub license_no: String,
    /// Registered trade name.
    pub trade_name: String,
    /// Type of company.
    pub company_type: UaeCompanyType,
    /// Current status.
    pub status: UaeCompanyStatus,
    /// Free zone or authority that issued the license.
    pub issuing_authority: String,
    /// License issue date (ISO 8601, YYYY-MM-DD).
    pub issue_date: String,
    /// License expiry date (ISO 8601, YYYY-MM-DD).
    pub expiry_date: String,
    /// Registered activities (per ISIC classification).
    pub activities: Vec<String>,
}

/// Shareholder record from the UAE corporate registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UaeShareholderRecord {
    /// Name of the shareholder (individual or corporate).
    pub name: String,
    /// Emirates ID of the shareholder (if an individual).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emirates_id: Option<String>,
    /// Percentage of shares held.
    pub share_percent: String,
    /// Nationality (ISO 3166-1 alpha-2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nationality: Option<String>,
}

/// Adapter trait for UAE corporate registry operations.
///
/// Implementations must be `Send + Sync` so they can be shared across
/// async tasks behind an `Arc`. The trait is object-safe to support
/// runtime adapter selection (mock vs. live).
pub trait UaeCorporateRegistryAdapter: Send + Sync {
    /// Look up a company by its commercial license number.
    fn lookup_company(
        &self,
        license_no: &str,
    ) -> Result<UaeCompanyRecord, UaeCorporateError>;

    /// Retrieve shareholder information for a company.
    fn get_shareholders(
        &self,
        license_no: &str,
    ) -> Result<Vec<UaeShareholderRecord>, UaeCorporateError>;

    /// Return the human-readable name of this adapter implementation.
    fn adapter_name(&self) -> &str;
}

/// Validate that a license number is non-empty and contains only
/// alphanumeric characters, dashes, and slashes.
pub fn validate_license_no(license_no: &str) -> Result<String, UaeCorporateError> {
    let trimmed = license_no.trim();
    if trimmed.is_empty() {
        return Err(UaeCorporateError::InvalidLicenseNumber {
            reason: "license number must not be empty".to_string(),
        });
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '/')
    {
        return Err(UaeCorporateError::InvalidLicenseNumber {
            reason: format!(
                "license number must be alphanumeric (with optional dashes/slashes), got '{}'",
                license_no
            ),
        });
    }
    Ok(trimmed.to_string())
}

/// Mock UAE corporate registry adapter for testing and development.
///
/// Returns deterministic test data based on license number conventions:
/// - License numbers starting with "ADGM" return ADGM free zone entities
/// - License numbers starting with "DIFC" return DIFC entities
/// - License number "9999999" returns CompanyNotFound
/// - All other license numbers return mainland LLC entities
#[derive(Debug, Clone)]
pub struct MockUaeCorporateRegistryAdapter;

impl UaeCorporateRegistryAdapter for MockUaeCorporateRegistryAdapter {
    fn lookup_company(
        &self,
        license_no: &str,
    ) -> Result<UaeCompanyRecord, UaeCorporateError> {
        let canonical = validate_license_no(license_no)?;

        if canonical == "9999999" {
            return Err(UaeCorporateError::CompanyNotFound {
                license_no: canonical,
            });
        }

        let (company_type, authority) = if canonical.starts_with("ADGM") {
            (UaeCompanyType::Fzco, "ADGM Registration Authority")
        } else if canonical.starts_with("DIFC") {
            (UaeCompanyType::Fze, "DIFC Registrar of Companies")
        } else {
            (UaeCompanyType::Llc, "Department of Economic Development")
        };

        Ok(UaeCompanyRecord {
            license_no: canonical.clone(),
            trade_name: format!("Mock Company {}", &canonical[..3.min(canonical.len())]),
            company_type,
            status: UaeCompanyStatus::Active,
            issuing_authority: authority.to_string(),
            issue_date: "2024-01-01".to_string(),
            expiry_date: "2027-01-01".to_string(),
            activities: vec!["General Trading".to_string()],
        })
    }

    fn get_shareholders(
        &self,
        license_no: &str,
    ) -> Result<Vec<UaeShareholderRecord>, UaeCorporateError> {
        let canonical = validate_license_no(license_no)?;

        if canonical == "9999999" {
            return Err(UaeCorporateError::CompanyNotFound {
                license_no: canonical,
            });
        }

        Ok(vec![UaeShareholderRecord {
            name: "Mock Shareholder".to_string(),
            emirates_id: Some("784-1234-1234567-1".to_string()),
            share_percent: "100".to_string(),
            nationality: Some("AE".to_string()),
        }])
    }

    fn adapter_name(&self) -> &str {
        "MockUaeCorporateRegistryAdapter"
    }
}

impl NationalSystemAdapter for MockUaeCorporateRegistryAdapter {
    fn category(&self) -> AdapterCategory {
        AdapterCategory::Corporate
    }

    fn jurisdiction(&self) -> &str {
        "ae-abudhabi-adgm"
    }

    fn health(&self) -> AdapterHealth {
        AdapterHealth::Healthy
    }

    fn adapter_name(&self) -> &str {
        "MockUaeCorporateRegistryAdapter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- validate_license_no ----------------------------------------------------

    #[test]
    fn validate_license_no_accepts_alphanumeric() {
        let result = validate_license_no("ADGM-12345");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "ADGM-12345");
    }

    #[test]
    fn validate_license_no_accepts_numeric() {
        let result = validate_license_no("1234567");
        assert!(result.is_ok());
    }

    #[test]
    fn validate_license_no_rejects_empty() {
        let result = validate_license_no("");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UaeCorporateError::InvalidLicenseNumber { .. }
        ));
    }

    #[test]
    fn validate_license_no_rejects_special_chars() {
        let result = validate_license_no("ABC@123");
        assert!(result.is_err());
    }

    // -- UaeCompanyType ---------------------------------------------------------

    #[test]
    fn company_type_display() {
        assert_eq!(format!("{}", UaeCompanyType::Llc), "LLC");
        assert_eq!(format!("{}", UaeCompanyType::Fze), "FZE");
        assert_eq!(format!("{}", UaeCompanyType::Fzco), "FZCO");
        assert_eq!(format!("{}", UaeCompanyType::Spv), "SPV");
        assert_eq!(format!("{}", UaeCompanyType::Branch), "Branch");
    }

    #[test]
    fn company_type_serde_roundtrip() {
        let ct = UaeCompanyType::Fzco;
        let json = serde_json::to_string(&ct).expect("serialize");
        let back: UaeCompanyType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, UaeCompanyType::Fzco);
    }

    // -- UaeCompanyStatus -------------------------------------------------------

    #[test]
    fn company_status_display() {
        assert_eq!(format!("{}", UaeCompanyStatus::Active), "Active");
        assert_eq!(format!("{}", UaeCompanyStatus::Expired), "Expired");
        assert_eq!(
            format!("{}", UaeCompanyStatus::InLiquidation),
            "InLiquidation"
        );
        assert_eq!(format!("{}", UaeCompanyStatus::StruckOff), "StruckOff");
    }

    // -- UaeCorporateError ------------------------------------------------------

    #[test]
    fn error_display() {
        let err = UaeCorporateError::CompanyNotFound {
            license_no: "ADGM-999".to_string(),
        };
        assert!(format!("{err}").contains("ADGM-999"));

        let err = UaeCorporateError::Timeout { elapsed_ms: 5000 };
        assert!(format!("{err}").contains("5000"));
    }

    // -- MockUaeCorporateRegistryAdapter ----------------------------------------

    #[test]
    fn mock_lookup_adgm_company() {
        let adapter = MockUaeCorporateRegistryAdapter;
        let record = adapter.lookup_company("ADGM-12345").expect("should succeed");
        assert_eq!(record.company_type, UaeCompanyType::Fzco);
        assert_eq!(record.status, UaeCompanyStatus::Active);
        assert!(record.issuing_authority.contains("ADGM"));
    }

    #[test]
    fn mock_lookup_difc_company() {
        let adapter = MockUaeCorporateRegistryAdapter;
        let record = adapter.lookup_company("DIFC-001").expect("should succeed");
        assert_eq!(record.company_type, UaeCompanyType::Fze);
        assert!(record.issuing_authority.contains("DIFC"));
    }

    #[test]
    fn mock_lookup_mainland_company() {
        let adapter = MockUaeCorporateRegistryAdapter;
        let record = adapter.lookup_company("1234567").expect("should succeed");
        assert_eq!(record.company_type, UaeCompanyType::Llc);
        assert!(record.issuing_authority.contains("Economic Development"));
    }

    #[test]
    fn mock_lookup_not_found() {
        let adapter = MockUaeCorporateRegistryAdapter;
        let result = adapter.lookup_company("9999999");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UaeCorporateError::CompanyNotFound { .. }
        ));
    }

    #[test]
    fn mock_get_shareholders() {
        let adapter = MockUaeCorporateRegistryAdapter;
        let shareholders = adapter.get_shareholders("ADGM-12345").expect("should succeed");
        assert_eq!(shareholders.len(), 1);
        assert!(shareholders[0].emirates_id.is_some());
    }

    #[test]
    fn mock_adapter_name() {
        let adapter = MockUaeCorporateRegistryAdapter;
        assert_eq!(
            UaeCorporateRegistryAdapter::adapter_name(&adapter),
            "MockUaeCorporateRegistryAdapter"
        );
    }

    #[test]
    fn mock_national_system_adapter() {
        let adapter = MockUaeCorporateRegistryAdapter;
        assert_eq!(
            NationalSystemAdapter::category(&adapter),
            AdapterCategory::Corporate
        );
        assert_eq!(
            NationalSystemAdapter::jurisdiction(&adapter),
            "ae-abudhabi-adgm"
        );
    }

    #[test]
    fn trait_object_safety() {
        let adapter: Box<dyn UaeCorporateRegistryAdapter> =
            Box::new(MockUaeCorporateRegistryAdapter);
        assert_eq!(adapter.adapter_name(), "MockUaeCorporateRegistryAdapter");
    }

    #[test]
    fn company_record_serde_roundtrip() {
        let record = UaeCompanyRecord {
            license_no: "ADGM-12345".to_string(),
            trade_name: "Test Co".to_string(),
            company_type: UaeCompanyType::Fzco,
            status: UaeCompanyStatus::Active,
            issuing_authority: "ADGM RA".to_string(),
            issue_date: "2024-01-01".to_string(),
            expiry_date: "2027-01-01".to_string(),
            activities: vec!["Trading".to_string()],
        };
        let json = serde_json::to_string(&record).expect("serialize");
        let back: UaeCompanyRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.license_no, "ADGM-12345");
        assert_eq!(back.company_type, UaeCompanyType::Fzco);
    }
}
