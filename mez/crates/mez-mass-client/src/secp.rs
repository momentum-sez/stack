//! # SECP Integration Adapter Interface
//!
//! Defines the adapter interface for SECP (Securities and Exchange Commission
//! of Pakistan), the corporate regulator responsible for company registration,
//! licensing, and annual compliance filing.
//!
//! ## Architecture
//!
//! The `SecpAdapter` trait abstracts over the SECP corporate registry backend.
//! Production deployments implement it against the live SECP eServices portal;
//! test environments use `MockSecpAdapter`. This separation allows compliance
//! evaluation and licensepack verification to compose SECP operations without
//! coupling to a specific transport or API version.
//!
//! ## Registration Numbers
//!
//! SECP company registration numbers follow a numeric format (typically 7 digits).
//! The `validate_registration_no` helper enforces this constraint before any
//! request reaches the adapter.
//!
//! ## Integration Points
//!
//! - **Company lookup**: Verify corporate identity and registration status
//! - **License verification**: Confirm validity of SECP-issued licenses
//! - **Filing status**: Check annual compliance (Form A, annual return)
//! - **Directors list**: Retrieve board composition for beneficial ownership checks

use mez_core::Cnic;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Errors from SECP integration operations.
#[derive(Debug, thiserror::Error)]
pub enum SecpError {
    /// SECP service is unreachable or returned a 5xx status.
    #[error("SECP service unavailable: {reason}")]
    ServiceUnavailable {
        /// Human-readable description of the outage or error.
        reason: String,
    },

    /// Company not found in the SECP corporate registry.
    #[error("company not found: {registration_no}")]
    CompanyNotFound {
        /// The registration number that was looked up.
        registration_no: String,
    },

    /// Registration number format is invalid.
    #[error("invalid registration number: {reason}")]
    InvalidRegistrationNumber {
        /// Description of the validation failure.
        reason: String,
    },

    /// The SECP adapter has not been configured for this deployment.
    #[error("SECP adapter not configured: {reason}")]
    NotConfigured {
        /// Why configuration is missing or incomplete.
        reason: String,
    },

    /// The request to SECP timed out.
    #[error("SECP request timed out after {elapsed_ms}ms")]
    Timeout {
        /// Elapsed time in milliseconds before the timeout triggered.
        elapsed_ms: u64,
    },
}

/// Type of company registered with SECP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompanyType {
    /// Private limited company (Pvt. Ltd.).
    Private,
    /// Public limited company (Ltd.).
    Public,
    /// Single Member Company (SMC).
    SingleMember,
    /// Not-for-profit / NGO registered under Section 42.
    Ngo,
    /// Foreign company registered in Pakistan.
    Foreign,
}

impl fmt::Display for CompanyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Private => write!(f, "Private"),
            Self::Public => write!(f, "Public"),
            Self::SingleMember => write!(f, "SingleMember"),
            Self::Ngo => write!(f, "NGO"),
            Self::Foreign => write!(f, "Foreign"),
        }
    }
}

/// Current status of a company in the SECP registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompanyStatus {
    /// Company is active and in good standing.
    Active,
    /// Company is dormant (filed dormancy application).
    Dormant,
    /// Company has been struck off the register.
    StrikeOff,
    /// Company has been formally dissolved.
    Dissolved,
}

impl fmt::Display for CompanyStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "Active"),
            Self::Dormant => write!(f, "Dormant"),
            Self::StrikeOff => write!(f, "StrikeOff"),
            Self::Dissolved => write!(f, "Dissolved"),
        }
    }
}

/// Annual filing/compliance status for a company.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FilingComplianceStatus {
    /// All annual filings are current.
    Current,
    /// One or more required filings are overdue.
    Overdue,
    /// Company is exempt from filing requirements (e.g. newly incorporated).
    Exempt,
}

impl fmt::Display for FilingComplianceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Current => write!(f, "Current"),
            Self::Overdue => write!(f, "Overdue"),
            Self::Exempt => write!(f, "Exempt"),
        }
    }
}

/// Company record from the SECP corporate registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyRecord {
    /// SECP registration number.
    pub registration_no: String,
    /// Registered name of the company.
    pub name: String,
    /// Type of company (Private, Public, SMC, etc.).
    pub company_type: CompanyType,
    /// Date of incorporation (ISO 8601, YYYY-MM-DD).
    pub incorporation_date: String,
    /// Current registration status.
    pub status: CompanyStatus,
    /// Registered office address.
    pub registered_address: String,
    /// Authorized share capital in PKR.
    pub authorized_capital: String,
}

/// Result of a license verification against SECP records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseVerification {
    /// The license reference that was verified.
    pub license_ref: String,
    /// Type of license (e.g. "NBFC", "Insurance", "Modaraba").
    pub license_type: String,
    /// Whether the license is currently valid.
    pub valid: bool,
    /// Expiry date of the license (ISO 8601, YYYY-MM-DD), if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_date: Option<String>,
    /// Name of the entity the license was issued to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issued_to: Option<String>,
}

/// Filing status response for a company.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilingStatus {
    /// Registration number of the company.
    pub registration_no: String,
    /// Current compliance status.
    pub compliance_status: FilingComplianceStatus,
    /// Date of last annual return filing (ISO 8601), if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_filing_date: Option<String>,
    /// Next filing deadline (ISO 8601), if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_deadline: Option<String>,
}

/// Director/officer record from the SECP corporate registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorRecord {
    /// Full name of the director.
    pub name: String,
    /// CNIC of the director (for Pakistani nationals).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cnic: Option<Cnic>,
    /// Designation (e.g. "Director", "CEO", "Chairman").
    pub designation: String,
    /// Date of appointment (ISO 8601, YYYY-MM-DD).
    pub appointment_date: String,
}

/// Adapter trait for SECP corporate registry integration.
///
/// Implementations must be `Send + Sync` so they can be shared across
/// async tasks behind an `Arc`. The trait is object-safe to support
/// runtime adapter selection (mock vs. live).
pub trait SecpAdapter: Send + Sync {
    /// Look up a company in the SECP corporate registry by registration number.
    fn lookup_company(&self, registration_no: &str)
        -> Result<CompanyRecord, SecpError>;

    /// Verify a specific license against SECP records.
    fn verify_license(
        &self,
        license_ref: &str,
        license_type: &str,
    ) -> Result<LicenseVerification, SecpError>;

    /// Check the annual compliance filing status for a company.
    fn check_filing_status(
        &self,
        registration_no: &str,
    ) -> Result<FilingStatus, SecpError>;

    /// Retrieve the board of directors for a company, used for beneficial
    /// ownership verification and KYC checks.
    fn get_directors(
        &self,
        registration_no: &str,
    ) -> Result<Vec<DirectorRecord>, SecpError>;

    /// Return the human-readable name of this adapter implementation
    /// (e.g. "MockSecpAdapter", "SecpLiveApiV1").
    fn adapter_name(&self) -> &str;
}

/// Validate that a registration number is well-formed.
///
/// SECP registration numbers are numeric strings, typically 7 digits.
/// This helper enforces: non-empty, all digits, 1-10 characters.
pub fn validate_registration_no(reg_no: &str) -> Result<String, SecpError> {
    let trimmed = reg_no.trim();
    if trimmed.is_empty() {
        return Err(SecpError::InvalidRegistrationNumber {
            reason: "registration number must not be empty".to_string(),
        });
    }
    if !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return Err(SecpError::InvalidRegistrationNumber {
            reason: format!(
                "registration number must contain only digits, got '{}'",
                reg_no
            ),
        });
    }
    if trimmed.len() > 10 {
        return Err(SecpError::InvalidRegistrationNumber {
            reason: format!(
                "registration number must be at most 10 digits, got {} digits",
                trimmed.len()
            ),
        });
    }
    Ok(trimmed.to_string())
}

/// Mock SECP adapter for testing and development.
///
/// Returns deterministic test data based on registration number:
/// - Registration "0000001" returns a Private company
/// - Registration "0000002" returns a Public company
/// - Registration "0000003" returns a SingleMember company
/// - Registration "9999999" returns CompanyNotFound
/// - All other valid registrations return a Private Active company
///
/// Directors are populated with test data for registrations other than "9999999".
#[derive(Debug, Clone)]
pub struct MockSecpAdapter;

impl SecpAdapter for MockSecpAdapter {
    fn lookup_company(
        &self,
        registration_no: &str,
    ) -> Result<CompanyRecord, SecpError> {
        let reg = validate_registration_no(registration_no)?;

        if reg == "9999999" {
            return Err(SecpError::CompanyNotFound {
                registration_no: reg,
            });
        }

        let (company_type, name) = match reg.as_str() {
            "0000002" => (CompanyType::Public, "Mock Public Ltd."),
            "0000003" => (CompanyType::SingleMember, "Mock SMC (Pvt.) Ltd."),
            "0000004" => (CompanyType::Ngo, "Mock Foundation"),
            "0000005" => (CompanyType::Foreign, "Mock Foreign Corp."),
            _ => (CompanyType::Private, "Mock Private (Pvt.) Ltd."),
        };

        Ok(CompanyRecord {
            registration_no: reg,
            name: name.to_string(),
            company_type,
            incorporation_date: "2020-06-15".to_string(),
            status: CompanyStatus::Active,
            registered_address: "123 Mock Street, Islamabad".to_string(),
            authorized_capital: "10000000".to_string(),
        })
    }

    fn verify_license(
        &self,
        license_ref: &str,
        license_type: &str,
    ) -> Result<LicenseVerification, SecpError> {
        if license_ref.is_empty() {
            return Err(SecpError::InvalidRegistrationNumber {
                reason: "license_ref must not be empty".to_string(),
            });
        }

        Ok(LicenseVerification {
            license_ref: license_ref.to_string(),
            license_type: license_type.to_string(),
            valid: true,
            expiry_date: Some("2027-12-31".to_string()),
            issued_to: Some("Mock Licensed Entity".to_string()),
        })
    }

    fn check_filing_status(
        &self,
        registration_no: &str,
    ) -> Result<FilingStatus, SecpError> {
        let reg = validate_registration_no(registration_no)?;

        if reg == "9999999" {
            return Err(SecpError::CompanyNotFound {
                registration_no: reg,
            });
        }

        let compliance_status = match reg.as_str() {
            "0000003" => FilingComplianceStatus::Exempt,
            "0000002" => FilingComplianceStatus::Overdue,
            _ => FilingComplianceStatus::Current,
        };

        Ok(FilingStatus {
            registration_no: reg,
            compliance_status,
            last_filing_date: Some("2025-10-01".to_string()),
            next_deadline: Some("2026-10-01".to_string()),
        })
    }

    fn get_directors(
        &self,
        registration_no: &str,
    ) -> Result<Vec<DirectorRecord>, SecpError> {
        let reg = validate_registration_no(registration_no)?;

        if reg == "9999999" {
            return Err(SecpError::CompanyNotFound {
                registration_no: reg,
            });
        }

        Ok(vec![
            DirectorRecord {
                name: "Ali Khan".to_string(),
                cnic: Some(Cnic::new("42101-1234567-1").expect("test CNIC")),
                designation: "CEO".to_string(),
                appointment_date: "2020-06-15".to_string(),
            },
            DirectorRecord {
                name: "Fatima Ahmed".to_string(),
                cnic: Some(Cnic::new("42201-7654321-2").expect("test CNIC")),
                designation: "Director".to_string(),
                appointment_date: "2021-01-10".to_string(),
            },
        ])
    }

    fn adapter_name(&self) -> &str {
        "MockSecpAdapter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- validate_registration_no -----------------------------------------------

    #[test]
    fn validate_registration_no_accepts_digits() {
        let result = validate_registration_no("0000001");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "0000001");
    }

    #[test]
    fn validate_registration_no_trims_whitespace() {
        let result = validate_registration_no("  0000001  ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "0000001");
    }

    #[test]
    fn validate_registration_no_rejects_empty() {
        let result = validate_registration_no("");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SecpError::InvalidRegistrationNumber { .. }
        ));
    }

    #[test]
    fn validate_registration_no_rejects_non_digits() {
        let result = validate_registration_no("ABC1234");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SecpError::InvalidRegistrationNumber { .. }
        ));
    }

    #[test]
    fn validate_registration_no_rejects_too_long() {
        let result = validate_registration_no("12345678901");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SecpError::InvalidRegistrationNumber { .. }
        ));
    }

    #[test]
    fn validate_registration_no_accepts_boundary_length() {
        assert!(validate_registration_no("1").is_ok());
        assert!(validate_registration_no("1234567890").is_ok());
    }

    // -- CompanyType Display ----------------------------------------------------

    #[test]
    fn company_type_display() {
        assert_eq!(CompanyType::Private.to_string(), "Private");
        assert_eq!(CompanyType::Public.to_string(), "Public");
        assert_eq!(CompanyType::SingleMember.to_string(), "SingleMember");
        assert_eq!(CompanyType::Ngo.to_string(), "NGO");
        assert_eq!(CompanyType::Foreign.to_string(), "Foreign");
    }

    // -- CompanyStatus Display --------------------------------------------------

    #[test]
    fn company_status_display() {
        assert_eq!(CompanyStatus::Active.to_string(), "Active");
        assert_eq!(CompanyStatus::Dormant.to_string(), "Dormant");
        assert_eq!(CompanyStatus::StrikeOff.to_string(), "StrikeOff");
        assert_eq!(CompanyStatus::Dissolved.to_string(), "Dissolved");
    }

    // -- FilingComplianceStatus Display ------------------------------------------

    #[test]
    fn filing_compliance_status_display() {
        assert_eq!(FilingComplianceStatus::Current.to_string(), "Current");
        assert_eq!(FilingComplianceStatus::Overdue.to_string(), "Overdue");
        assert_eq!(FilingComplianceStatus::Exempt.to_string(), "Exempt");
    }

    // -- SecpError Display ------------------------------------------------------

    #[test]
    fn secp_error_display_messages() {
        let err = SecpError::ServiceUnavailable {
            reason: "connection refused".into(),
        };
        assert!(err.to_string().contains("connection refused"));

        let err = SecpError::CompanyNotFound {
            registration_no: "9999999".into(),
        };
        assert!(err.to_string().contains("9999999"));

        let err = SecpError::InvalidRegistrationNumber {
            reason: "not digits".into(),
        };
        assert!(err.to_string().contains("not digits"));

        let err = SecpError::NotConfigured {
            reason: "missing cert".into(),
        };
        assert!(err.to_string().contains("missing cert"));

        let err = SecpError::Timeout { elapsed_ms: 3000 };
        assert!(err.to_string().contains("3000"));
    }

    // -- Serde round-trips ------------------------------------------------------

    #[test]
    fn company_type_serde_round_trip() {
        for ct in [
            CompanyType::Private,
            CompanyType::Public,
            CompanyType::SingleMember,
            CompanyType::Ngo,
            CompanyType::Foreign,
        ] {
            let json = serde_json::to_string(&ct).expect("serialize CompanyType");
            let back: CompanyType = serde_json::from_str(&json).expect("deserialize CompanyType");
            assert_eq!(ct, back);
        }
    }

    #[test]
    fn company_status_serde_round_trip() {
        for cs in [
            CompanyStatus::Active,
            CompanyStatus::Dormant,
            CompanyStatus::StrikeOff,
            CompanyStatus::Dissolved,
        ] {
            let json = serde_json::to_string(&cs).expect("serialize CompanyStatus");
            let back: CompanyStatus =
                serde_json::from_str(&json).expect("deserialize CompanyStatus");
            assert_eq!(cs, back);
        }
    }

    #[test]
    fn filing_compliance_status_serde_round_trip() {
        for fs in [
            FilingComplianceStatus::Current,
            FilingComplianceStatus::Overdue,
            FilingComplianceStatus::Exempt,
        ] {
            let json =
                serde_json::to_string(&fs).expect("serialize FilingComplianceStatus");
            let back: FilingComplianceStatus =
                serde_json::from_str(&json).expect("deserialize FilingComplianceStatus");
            assert_eq!(fs, back);
        }
    }

    #[test]
    fn company_record_serde_round_trip() {
        let record = CompanyRecord {
            registration_no: "0000001".into(),
            name: "Test Corp (Pvt.) Ltd.".into(),
            company_type: CompanyType::Private,
            incorporation_date: "2020-06-15".into(),
            status: CompanyStatus::Active,
            registered_address: "123 Test St, Islamabad".into(),
            authorized_capital: "10000000".into(),
        };
        let json = serde_json::to_string(&record).expect("serialize");
        let back: CompanyRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.registration_no, "0000001");
        assert_eq!(back.name, "Test Corp (Pvt.) Ltd.");
        assert_eq!(back.company_type, CompanyType::Private);
        assert_eq!(back.status, CompanyStatus::Active);
    }

    #[test]
    fn license_verification_serde_round_trip() {
        let lv = LicenseVerification {
            license_ref: "LIC-001".into(),
            license_type: "NBFC".into(),
            valid: true,
            expiry_date: Some("2027-12-31".into()),
            issued_to: Some("Test Entity".into()),
        };
        let json = serde_json::to_string(&lv).expect("serialize");
        let back: LicenseVerification = serde_json::from_str(&json).expect("deserialize");
        assert!(back.valid);
        assert_eq!(back.license_ref, "LIC-001");
        assert_eq!(back.expiry_date.as_deref(), Some("2027-12-31"));
    }

    #[test]
    fn license_verification_optional_fields_absent() {
        let lv = LicenseVerification {
            license_ref: "LIC-002".into(),
            license_type: "Insurance".into(),
            valid: false,
            expiry_date: None,
            issued_to: None,
        };
        let json = serde_json::to_string(&lv).expect("serialize");
        assert!(!json.contains("expiry_date"));
        assert!(!json.contains("issued_to"));
    }

    #[test]
    fn filing_status_serde_round_trip() {
        let fs = FilingStatus {
            registration_no: "0000001".into(),
            compliance_status: FilingComplianceStatus::Current,
            last_filing_date: Some("2025-10-01".into()),
            next_deadline: Some("2026-10-01".into()),
        };
        let json = serde_json::to_string(&fs).expect("serialize");
        let back: FilingStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.compliance_status, FilingComplianceStatus::Current);
        assert_eq!(back.last_filing_date.as_deref(), Some("2025-10-01"));
    }

    #[test]
    fn director_record_serde_round_trip() {
        let cnic = Cnic::new("42101-1234567-1").unwrap();
        let dr = DirectorRecord {
            name: "Ali Khan".into(),
            cnic: Some(cnic),
            designation: "CEO".into(),
            appointment_date: "2020-06-15".into(),
        };
        let json = serde_json::to_string(&dr).expect("serialize");
        let back: DirectorRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.name, "Ali Khan");
        assert!(back.cnic.is_some());
        assert_eq!(back.designation, "CEO");
    }

    #[test]
    fn director_record_without_cnic() {
        let dr = DirectorRecord {
            name: "Foreign Director".into(),
            cnic: None,
            designation: "Director".into(),
            appointment_date: "2022-03-01".into(),
        };
        let json = serde_json::to_string(&dr).expect("serialize");
        assert!(!json.contains("cnic"));
    }

    // -- MockSecpAdapter: lookup_company ----------------------------------------

    #[test]
    fn mock_adapter_lookup_private_company() {
        let adapter = MockSecpAdapter;
        let record = adapter
            .lookup_company("0000001")
            .expect("should find company");
        assert_eq!(record.registration_no, "0000001");
        assert_eq!(record.company_type, CompanyType::Private);
        assert_eq!(record.status, CompanyStatus::Active);
        assert!(!record.name.is_empty());
        assert!(!record.registered_address.is_empty());
    }

    #[test]
    fn mock_adapter_lookup_public_company() {
        let adapter = MockSecpAdapter;
        let record = adapter
            .lookup_company("0000002")
            .expect("should find company");
        assert_eq!(record.company_type, CompanyType::Public);
    }

    #[test]
    fn mock_adapter_lookup_smc() {
        let adapter = MockSecpAdapter;
        let record = adapter
            .lookup_company("0000003")
            .expect("should find company");
        assert_eq!(record.company_type, CompanyType::SingleMember);
    }

    #[test]
    fn mock_adapter_lookup_ngo() {
        let adapter = MockSecpAdapter;
        let record = adapter
            .lookup_company("0000004")
            .expect("should find company");
        assert_eq!(record.company_type, CompanyType::Ngo);
    }

    #[test]
    fn mock_adapter_lookup_foreign() {
        let adapter = MockSecpAdapter;
        let record = adapter
            .lookup_company("0000005")
            .expect("should find company");
        assert_eq!(record.company_type, CompanyType::Foreign);
    }

    #[test]
    fn mock_adapter_lookup_not_found() {
        let adapter = MockSecpAdapter;
        let result = adapter.lookup_company("9999999");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SecpError::CompanyNotFound { .. }
        ));
    }

    #[test]
    fn mock_adapter_lookup_invalid_registration() {
        let adapter = MockSecpAdapter;
        let result = adapter.lookup_company("ABC");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SecpError::InvalidRegistrationNumber { .. }
        ));
    }

    // -- MockSecpAdapter: verify_license ----------------------------------------

    #[test]
    fn mock_adapter_verify_license() {
        let adapter = MockSecpAdapter;
        let result = adapter
            .verify_license("LIC-001", "NBFC")
            .expect("should verify license");
        assert!(result.valid);
        assert_eq!(result.license_ref, "LIC-001");
        assert_eq!(result.license_type, "NBFC");
        assert!(result.expiry_date.is_some());
        assert!(result.issued_to.is_some());
    }

    #[test]
    fn mock_adapter_verify_license_empty_ref() {
        let adapter = MockSecpAdapter;
        let result = adapter.verify_license("", "NBFC");
        assert!(result.is_err());
    }

    // -- MockSecpAdapter: check_filing_status -----------------------------------

    #[test]
    fn mock_adapter_filing_status_current() {
        let adapter = MockSecpAdapter;
        let status = adapter
            .check_filing_status("0000001")
            .expect("should return status");
        assert_eq!(status.compliance_status, FilingComplianceStatus::Current);
        assert!(status.last_filing_date.is_some());
        assert!(status.next_deadline.is_some());
    }

    #[test]
    fn mock_adapter_filing_status_overdue() {
        let adapter = MockSecpAdapter;
        let status = adapter
            .check_filing_status("0000002")
            .expect("should return status");
        assert_eq!(status.compliance_status, FilingComplianceStatus::Overdue);
    }

    #[test]
    fn mock_adapter_filing_status_exempt() {
        let adapter = MockSecpAdapter;
        let status = adapter
            .check_filing_status("0000003")
            .expect("should return status");
        assert_eq!(status.compliance_status, FilingComplianceStatus::Exempt);
    }

    #[test]
    fn mock_adapter_filing_status_not_found() {
        let adapter = MockSecpAdapter;
        let result = adapter.check_filing_status("9999999");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SecpError::CompanyNotFound { .. }
        ));
    }

    // -- MockSecpAdapter: get_directors -----------------------------------------

    #[test]
    fn mock_adapter_get_directors() {
        let adapter = MockSecpAdapter;
        let directors = adapter
            .get_directors("0000001")
            .expect("should return directors");
        assert_eq!(directors.len(), 2);
        assert_eq!(directors[0].name, "Ali Khan");
        assert_eq!(directors[0].designation, "CEO");
        assert!(directors[0].cnic.is_some());
        assert_eq!(directors[1].name, "Fatima Ahmed");
        assert_eq!(directors[1].designation, "Director");
    }

    #[test]
    fn mock_adapter_get_directors_not_found() {
        let adapter = MockSecpAdapter;
        let result = adapter.get_directors("9999999");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SecpError::CompanyNotFound { .. }
        ));
    }

    #[test]
    fn mock_adapter_get_directors_invalid_reg() {
        let adapter = MockSecpAdapter;
        let result = adapter.get_directors("XYZ");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SecpError::InvalidRegistrationNumber { .. }
        ));
    }

    // -- Trait properties -------------------------------------------------------

    #[test]
    fn mock_adapter_name() {
        let adapter = MockSecpAdapter;
        assert_eq!(adapter.adapter_name(), "MockSecpAdapter");
    }

    #[test]
    fn adapter_trait_is_object_safe() {
        let adapter: Box<dyn SecpAdapter> = Box::new(MockSecpAdapter);
        assert_eq!(adapter.adapter_name(), "MockSecpAdapter");
        let record = adapter
            .lookup_company("0000001")
            .expect("trait object lookup");
        assert_eq!(record.registration_no, "0000001");
    }

    #[test]
    fn adapter_trait_behind_arc() {
        let adapter: std::sync::Arc<dyn SecpAdapter> =
            std::sync::Arc::new(MockSecpAdapter);
        let record = adapter
            .lookup_company("0000001")
            .expect("Arc adapter should work");
        assert_eq!(record.company_type, CompanyType::Private);
    }
}
