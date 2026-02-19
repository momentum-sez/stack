//! # FBR IRIS Integration Adapter Interface
//!
//! Defines the adapter interface for FBR IRIS (Inland Revenue Information System),
//! Pakistan's Federal Board of Revenue tax authority integration point.
//!
//! ## Architecture
//!
//! The `FbrIrisAdapter` trait abstracts over the FBR IRIS backend. Production
//! deployments implement it against the live IRIS API; test environments use
//! `MockFbrIrisAdapter`. This separation allows the tax pipeline and compliance
//! evaluation to compose FBR operations without coupling to a specific transport
//! or API version.
//!
//! ## NTN Validation
//!
//! Pakistan NTNs (National Tax Numbers) are 7-digit identifiers issued by FBR.
//! The `validate_ntn` helper delegates to `mez_core::Ntn::new()` for validation
//! before any request reaches the adapter.
//!
//! ## Filer Status
//!
//! Pakistan's tax system differentiates withholding rates based on filer status:
//! - **Filer**: Registered and compliant taxpayer (lower withholding rates)
//! - **NonFiler**: Not registered with FBR (higher withholding rates, typically 2x)
//! - **LateFiler**: Registered but overdue on filings (intermediate rates)
//!
//! This distinction is the core mechanism of Pakistan's tax compliance incentive
//! structure and drives withholding rate differentials across all transaction types.

use mez_core::Ntn;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Errors from FBR IRIS integration operations.
#[derive(Debug, thiserror::Error)]
pub enum FbrError {
    /// FBR IRIS service is unreachable or returned a 5xx status.
    #[error("FBR IRIS service unavailable: {reason}")]
    ServiceUnavailable {
        /// Human-readable description of the outage or error.
        reason: String,
    },

    /// NTN format is invalid (not exactly 7 digits).
    #[error("invalid NTN: {reason}")]
    InvalidNtn {
        /// Description of the validation failure.
        reason: String,
    },

    /// The FBR IRIS adapter has not been configured for this deployment.
    #[error("FBR IRIS adapter not configured: {reason}")]
    NotConfigured {
        /// Why configuration is missing or incomplete.
        reason: String,
    },

    /// The request to FBR IRIS timed out.
    #[error("FBR IRIS request timed out after {elapsed_ms}ms")]
    Timeout {
        /// Elapsed time in milliseconds before the timeout triggered.
        elapsed_ms: u64,
    },

    /// FBR IRIS accepted the request but verification could not be completed.
    #[error("NTN verification failed: {reason}")]
    VerificationFailed {
        /// Description of the failure (e.g. name mismatch, internal error).
        reason: String,
    },

    /// FBR IRIS rejected a tax event submission.
    #[error("tax event submission rejected: {reason}")]
    SubmissionRejected {
        /// Description of why the submission was rejected.
        reason: String,
    },
}

/// Filer status as maintained by FBR.
///
/// Determines withholding rate differentials: filers pay lower rates,
/// non-filers pay approximately double across most transaction categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FilerStatus {
    /// Registered taxpayer with current filings.
    Filer,
    /// Not registered with FBR or registration inactive.
    NonFiler,
    /// Registered but overdue on one or more required filings.
    LateFiler,
}

impl fmt::Display for FilerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Filer => write!(f, "Filer"),
            Self::NonFiler => write!(f, "NonFiler"),
            Self::LateFiler => write!(f, "LateFiler"),
        }
    }
}

/// Taxpayer compliance status as reported by FBR IRIS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComplianceStatus {
    /// All filings current, no outstanding obligations.
    Compliant,
    /// One or more filings overdue or obligations outstanding.
    NonCompliant,
    /// Account suspended by FBR (fraud investigation, court order, etc.).
    Suspended,
}

impl fmt::Display for ComplianceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compliant => write!(f, "Compliant"),
            Self::NonCompliant => write!(f, "NonCompliant"),
            Self::Suspended => write!(f, "Suspended"),
        }
    }
}

/// NTN verification response from FBR IRIS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtnVerificationResponse {
    /// Whether the NTN was successfully verified against FBR records.
    pub verified: bool,
    /// The NTN that was verified (canonical 7-digit form).
    pub ntn: String,
    /// Registered entity name as recorded by FBR.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registered_name: Option<String>,
    /// Current filer status of the taxpayer.
    pub filer_status: FilerStatus,
    /// ISO 8601 timestamp when FBR performed the verification.
    pub verification_timestamp: String,
    /// Reference identifier for audit trail.
    pub reference: String,
}

/// A taxable event to be submitted to FBR IRIS for reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxEventSubmission {
    /// Entity ID in the EZ Stack that generated this event.
    pub entity_id: String,
    /// NTN of the taxpayer (7 digits).
    pub ntn: String,
    /// Type of taxable event (e.g. "payment_for_goods", "salary_payment").
    pub event_type: String,
    /// Gross amount of the transaction in minor units string (e.g. "100000.00").
    pub amount: String,
    /// Currency code (ISO 4217, typically "PKR").
    pub currency: String,
    /// Jurisdiction identifier (e.g. "PK").
    pub jurisdiction: String,
    /// Idempotency key to prevent duplicate submissions.
    pub idempotency_key: String,
    /// Tax year (e.g. "2025-2026").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_year: Option<String>,
    /// Statutory section reference (e.g. "S153(1)(a)").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statutory_section: Option<String>,
}

/// Result of a tax event submission to FBR IRIS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxEventResult {
    /// Whether the submission was accepted by FBR IRIS.
    pub accepted: bool,
    /// FBR-assigned reference number for this event.
    pub fbr_reference: String,
    /// ISO 8601 timestamp when FBR recorded the event.
    pub recorded_at: String,
    /// The idempotency key echoed back.
    pub idempotency_key: String,
}

/// Query parameters for looking up applicable withholding rates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithholdingRateQuery {
    /// Type of transaction (e.g. "payment_for_goods", "salary_payment").
    pub transaction_type: String,
    /// Filer status of the taxpayer.
    pub filer_status: FilerStatus,
    /// Jurisdiction (e.g. "PK").
    pub jurisdiction: String,
    /// Tax year for which the rate applies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_year: Option<String>,
}

/// Withholding rate as returned by FBR IRIS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithholdingRate {
    /// Applicable rate as a percentage string (e.g. "4.5").
    pub rate_percent: String,
    /// Statutory section that mandates this rate.
    pub statutory_section: String,
    /// Whether this withholding is treated as final tax.
    pub is_final_tax: bool,
    /// Description of the withholding rule.
    pub description: String,
}

/// Taxpayer profile as maintained by FBR IRIS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxpayerProfile {
    /// NTN of the taxpayer (canonical 7-digit form).
    pub ntn: String,
    /// Current filer status.
    pub filer_status: FilerStatus,
    /// Current compliance status.
    pub compliance_status: ComplianceStatus,
    /// Date since which the NTN has been active (ISO 8601, YYYY-MM-DD).
    pub active_since: String,
    /// Registered name of the taxpayer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registered_name: Option<String>,
}

/// Adapter trait for FBR IRIS tax authority integration.
///
/// Implementations must be `Send + Sync` so they can be shared across
/// async tasks behind an `Arc`. The trait is object-safe to support
/// runtime adapter selection (mock vs. live).
pub trait FbrIrisAdapter: Send + Sync {
    /// Verify an NTN against FBR IRIS records, cross-referencing with the
    /// provided entity name.
    fn verify_ntn(
        &self,
        ntn: &Ntn,
        entity_name: &str,
    ) -> Result<NtnVerificationResponse, FbrError>;

    /// Submit a taxable event to FBR IRIS for recording and reporting.
    fn submit_tax_event(
        &self,
        event: &TaxEventSubmission,
    ) -> Result<TaxEventResult, FbrError>;

    /// Query the applicable withholding rate for a given transaction type
    /// and filer status.
    fn query_withholding_rate(
        &self,
        params: &WithholdingRateQuery,
    ) -> Result<WithholdingRate, FbrError>;

    /// Retrieve the taxpayer profile for an NTN, including filer status
    /// and compliance standing.
    fn get_taxpayer_profile(&self, ntn: &Ntn) -> Result<TaxpayerProfile, FbrError>;

    /// Return the human-readable name of this adapter implementation
    /// (e.g. "MockFbrIrisAdapter", "FbrIrisLiveApiV1").
    fn adapter_name(&self) -> &str;
}

/// Validate that an NTN string is well-formed by delegating to
/// `mez_core::Ntn::new()`. Returns the validated `Ntn` on success.
pub fn validate_ntn(ntn: &str) -> Result<Ntn, FbrError> {
    Ntn::new(ntn).map_err(|e| FbrError::InvalidNtn {
        reason: e.to_string(),
    })
}

/// Mock FBR IRIS adapter for testing and development.
///
/// Returns deterministic test data based on NTN prefix conventions:
/// - NTNs starting with "0" are treated as non-filers
/// - NTNs starting with "9" are treated as late filers
/// - All other NTNs are treated as filers
///
/// Withholding rates match Pakistan's Income Tax Ordinance 2001 rates
/// as encoded in the regpacks:
/// - Payment for goods: 4.5% (filer), 9.0% (non-filer)
/// - Payment for services: 8.0% (filer), 16.0% (non-filer)
/// - Salary payment: 12.5% (filer), 20.0% (non-filer)
#[derive(Debug, Clone)]
pub struct MockFbrIrisAdapter;

impl MockFbrIrisAdapter {
    /// Determine filer status from NTN prefix convention.
    fn filer_status_from_ntn(ntn: &Ntn) -> FilerStatus {
        match ntn.as_str().as_bytes().first() {
            Some(b'0') => FilerStatus::NonFiler,
            Some(b'9') => FilerStatus::LateFiler,
            _ => FilerStatus::Filer,
        }
    }
}

impl FbrIrisAdapter for MockFbrIrisAdapter {
    fn verify_ntn(
        &self,
        ntn: &Ntn,
        _entity_name: &str,
    ) -> Result<NtnVerificationResponse, FbrError> {
        let filer_status = Self::filer_status_from_ntn(ntn);

        Ok(NtnVerificationResponse {
            verified: true,
            ntn: ntn.as_str().to_string(),
            registered_name: Some(format!("Mock Entity {}", ntn.as_str())),
            filer_status,
            verification_timestamp: "2026-02-19T12:00:00Z".to_string(),
            reference: format!("MOCK-FBR-{}", ntn.as_str()),
        })
    }

    fn submit_tax_event(
        &self,
        event: &TaxEventSubmission,
    ) -> Result<TaxEventResult, FbrError> {
        // Validate NTN format before accepting submission.
        validate_ntn(&event.ntn)?;

        if event.idempotency_key.is_empty() {
            return Err(FbrError::SubmissionRejected {
                reason: "idempotency_key must not be empty".to_string(),
            });
        }

        Ok(TaxEventResult {
            accepted: true,
            fbr_reference: format!("FBR-MOCK-{}", &event.idempotency_key),
            recorded_at: "2026-02-19T12:00:00Z".to_string(),
            idempotency_key: event.idempotency_key.clone(),
        })
    }

    fn query_withholding_rate(
        &self,
        params: &WithholdingRateQuery,
    ) -> Result<WithholdingRate, FbrError> {
        // Rates from Pakistan Income Tax Ordinance 2001, matching regpack values.
        let (rate, section, is_final, desc) = match params.transaction_type.as_str() {
            "payment_for_goods" => match params.filer_status {
                FilerStatus::Filer => ("4.5", "S153(1)(a)", true, "WHT on goods — filer rate"),
                FilerStatus::NonFiler => {
                    ("9.0", "S153(1)(a)", true, "WHT on goods — non-filer rate")
                }
                FilerStatus::LateFiler => {
                    ("6.5", "S153(1)(a)", true, "WHT on goods — late filer rate")
                }
            },
            "payment_for_services" => match params.filer_status {
                FilerStatus::Filer => {
                    ("8.0", "S153(1)(b)", true, "WHT on services — filer rate")
                }
                FilerStatus::NonFiler => (
                    "16.0",
                    "S153(1)(b)",
                    true,
                    "WHT on services — non-filer rate",
                ),
                FilerStatus::LateFiler => (
                    "12.0",
                    "S153(1)(b)",
                    true,
                    "WHT on services — late filer rate",
                ),
            },
            "salary_payment" => match params.filer_status {
                FilerStatus::Filer => ("12.5", "S149", false, "WHT on salary — filer rate"),
                FilerStatus::NonFiler => ("20.0", "S149", false, "WHT on salary — non-filer rate"),
                FilerStatus::LateFiler => {
                    ("15.0", "S149", false, "WHT on salary — late filer rate")
                }
            },
            _ => match params.filer_status {
                FilerStatus::Filer => ("4.5", "S153", true, "WHT — default filer rate"),
                FilerStatus::NonFiler => ("9.0", "S153", true, "WHT — default non-filer rate"),
                FilerStatus::LateFiler => ("6.5", "S153", true, "WHT — default late filer rate"),
            },
        };

        Ok(WithholdingRate {
            rate_percent: rate.to_string(),
            statutory_section: section.to_string(),
            is_final_tax: is_final,
            description: desc.to_string(),
        })
    }

    fn get_taxpayer_profile(&self, ntn: &Ntn) -> Result<TaxpayerProfile, FbrError> {
        let filer_status = Self::filer_status_from_ntn(ntn);
        let compliance_status = match filer_status {
            FilerStatus::Filer => ComplianceStatus::Compliant,
            FilerStatus::LateFiler => ComplianceStatus::NonCompliant,
            FilerStatus::NonFiler => ComplianceStatus::NonCompliant,
        };

        Ok(TaxpayerProfile {
            ntn: ntn.as_str().to_string(),
            filer_status,
            compliance_status,
            active_since: "2020-01-01".to_string(),
            registered_name: Some(format!("Mock Entity {}", ntn.as_str())),
        })
    }

    fn adapter_name(&self) -> &str {
        "MockFbrIrisAdapter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- validate_ntn -----------------------------------------------------------

    #[test]
    fn validate_ntn_accepts_7_digits() {
        let result = validate_ntn("1234567");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "1234567");
    }

    #[test]
    fn validate_ntn_accepts_leading_zeros() {
        let result = validate_ntn("0012345");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "0012345");
    }

    #[test]
    fn validate_ntn_rejects_too_few_digits() {
        let result = validate_ntn("123456");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FbrError::InvalidNtn { .. }));
    }

    #[test]
    fn validate_ntn_rejects_too_many_digits() {
        let result = validate_ntn("12345678");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FbrError::InvalidNtn { .. }));
    }

    #[test]
    fn validate_ntn_rejects_non_digits() {
        let result = validate_ntn("123456a");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FbrError::InvalidNtn { .. }));
    }

    #[test]
    fn validate_ntn_rejects_empty_string() {
        let result = validate_ntn("");
        assert!(result.is_err());
    }

    // -- FilerStatus Display ----------------------------------------------------

    #[test]
    fn filer_status_display() {
        assert_eq!(FilerStatus::Filer.to_string(), "Filer");
        assert_eq!(FilerStatus::NonFiler.to_string(), "NonFiler");
        assert_eq!(FilerStatus::LateFiler.to_string(), "LateFiler");
    }

    // -- ComplianceStatus Display -----------------------------------------------

    #[test]
    fn compliance_status_display() {
        assert_eq!(ComplianceStatus::Compliant.to_string(), "Compliant");
        assert_eq!(ComplianceStatus::NonCompliant.to_string(), "NonCompliant");
        assert_eq!(ComplianceStatus::Suspended.to_string(), "Suspended");
    }

    // -- FbrError Display -------------------------------------------------------

    #[test]
    fn fbr_error_display_messages() {
        let err = FbrError::ServiceUnavailable {
            reason: "connection refused".into(),
        };
        assert!(err.to_string().contains("connection refused"));

        let err = FbrError::InvalidNtn {
            reason: "too short".into(),
        };
        assert!(err.to_string().contains("too short"));

        let err = FbrError::NotConfigured {
            reason: "missing API key".into(),
        };
        assert!(err.to_string().contains("missing API key"));

        let err = FbrError::Timeout { elapsed_ms: 5000 };
        assert!(err.to_string().contains("5000"));

        let err = FbrError::VerificationFailed {
            reason: "name mismatch".into(),
        };
        assert!(err.to_string().contains("name mismatch"));

        let err = FbrError::SubmissionRejected {
            reason: "duplicate".into(),
        };
        assert!(err.to_string().contains("duplicate"));
    }

    // -- Serde round-trips ------------------------------------------------------

    #[test]
    fn filer_status_serde_round_trip() {
        for status in [FilerStatus::Filer, FilerStatus::NonFiler, FilerStatus::LateFiler] {
            let json = serde_json::to_string(&status).expect("serialize FilerStatus");
            let back: FilerStatus = serde_json::from_str(&json).expect("deserialize FilerStatus");
            assert_eq!(status, back);
        }
    }

    #[test]
    fn compliance_status_serde_round_trip() {
        for status in [
            ComplianceStatus::Compliant,
            ComplianceStatus::NonCompliant,
            ComplianceStatus::Suspended,
        ] {
            let json = serde_json::to_string(&status).expect("serialize ComplianceStatus");
            let back: ComplianceStatus =
                serde_json::from_str(&json).expect("deserialize ComplianceStatus");
            assert_eq!(status, back);
        }
    }

    #[test]
    fn ntn_verification_response_serde_round_trip() {
        let resp = NtnVerificationResponse {
            verified: true,
            ntn: "1234567".into(),
            registered_name: Some("Test Corp".into()),
            filer_status: FilerStatus::Filer,
            verification_timestamp: "2026-02-19T12:00:00Z".into(),
            reference: "MOCK-FBR-1234567".into(),
        };
        let json = serde_json::to_string(&resp).expect("serialize response");
        let back: NtnVerificationResponse =
            serde_json::from_str(&json).expect("deserialize response");
        assert!(back.verified);
        assert_eq!(back.ntn, "1234567");
        assert_eq!(back.registered_name.as_deref(), Some("Test Corp"));
        assert_eq!(back.filer_status, FilerStatus::Filer);
    }

    #[test]
    fn ntn_verification_response_optional_name_absent() {
        let resp = NtnVerificationResponse {
            verified: true,
            ntn: "1234567".into(),
            registered_name: None,
            filer_status: FilerStatus::NonFiler,
            verification_timestamp: "2026-02-19T12:00:00Z".into(),
            reference: "REF-1".into(),
        };
        let json = serde_json::to_string(&resp).expect("serialize");
        assert!(!json.contains("registered_name"));
    }

    #[test]
    fn tax_event_submission_serde_round_trip() {
        let event = TaxEventSubmission {
            entity_id: "entity-001".into(),
            ntn: "1234567".into(),
            event_type: "payment_for_goods".into(),
            amount: "100000.00".into(),
            currency: "PKR".into(),
            jurisdiction: "PK".into(),
            idempotency_key: "IDEM-001".into(),
            tax_year: Some("2025-2026".into()),
            statutory_section: Some("S153(1)(a)".into()),
        };
        let json = serde_json::to_string(&event).expect("serialize");
        let back: TaxEventSubmission =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.entity_id, "entity-001");
        assert_eq!(back.ntn, "1234567");
        assert_eq!(back.idempotency_key, "IDEM-001");
        assert_eq!(back.tax_year.as_deref(), Some("2025-2026"));
        assert_eq!(back.statutory_section.as_deref(), Some("S153(1)(a)"));
    }

    #[test]
    fn tax_event_submission_optional_fields_absent() {
        let event = TaxEventSubmission {
            entity_id: "e1".into(),
            ntn: "1234567".into(),
            event_type: "payment_for_goods".into(),
            amount: "1000".into(),
            currency: "PKR".into(),
            jurisdiction: "PK".into(),
            idempotency_key: "IK1".into(),
            tax_year: None,
            statutory_section: None,
        };
        let json = serde_json::to_string(&event).expect("serialize");
        assert!(!json.contains("tax_year"));
        assert!(!json.contains("statutory_section"));
    }

    #[test]
    fn tax_event_result_serde_round_trip() {
        let result = TaxEventResult {
            accepted: true,
            fbr_reference: "FBR-MOCK-001".into(),
            recorded_at: "2026-02-19T12:00:00Z".into(),
            idempotency_key: "IDEM-001".into(),
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let back: TaxEventResult = serde_json::from_str(&json).expect("deserialize");
        assert!(back.accepted);
        assert_eq!(back.fbr_reference, "FBR-MOCK-001");
    }

    #[test]
    fn withholding_rate_query_serde_round_trip() {
        let query = WithholdingRateQuery {
            transaction_type: "payment_for_goods".into(),
            filer_status: FilerStatus::Filer,
            jurisdiction: "PK".into(),
            tax_year: Some("2025-2026".into()),
        };
        let json = serde_json::to_string(&query).expect("serialize");
        let back: WithholdingRateQuery =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.transaction_type, "payment_for_goods");
        assert_eq!(back.filer_status, FilerStatus::Filer);
    }

    #[test]
    fn withholding_rate_serde_round_trip() {
        let rate = WithholdingRate {
            rate_percent: "4.5".into(),
            statutory_section: "S153(1)(a)".into(),
            is_final_tax: true,
            description: "WHT on goods".into(),
        };
        let json = serde_json::to_string(&rate).expect("serialize");
        let back: WithholdingRate = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.rate_percent, "4.5");
        assert!(back.is_final_tax);
    }

    #[test]
    fn taxpayer_profile_serde_round_trip() {
        let profile = TaxpayerProfile {
            ntn: "1234567".into(),
            filer_status: FilerStatus::Filer,
            compliance_status: ComplianceStatus::Compliant,
            active_since: "2020-01-01".into(),
            registered_name: Some("Test Corp".into()),
        };
        let json = serde_json::to_string(&profile).expect("serialize");
        let back: TaxpayerProfile = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.ntn, "1234567");
        assert_eq!(back.filer_status, FilerStatus::Filer);
        assert_eq!(back.compliance_status, ComplianceStatus::Compliant);
        assert_eq!(back.active_since, "2020-01-01");
    }

    // -- MockFbrIrisAdapter: verify_ntn -----------------------------------------

    #[test]
    fn mock_adapter_verifies_valid_ntn() {
        let adapter = MockFbrIrisAdapter;
        let ntn = Ntn::new("1234567").unwrap();
        let resp = adapter
            .verify_ntn(&ntn, "Test Entity")
            .expect("should verify valid NTN");
        assert!(resp.verified);
        assert_eq!(resp.ntn, "1234567");
        assert_eq!(resp.filer_status, FilerStatus::Filer);
        assert!(resp.registered_name.is_some());
        assert!(resp.reference.starts_with("MOCK-FBR-"));
        assert!(!resp.verification_timestamp.is_empty());
    }

    #[test]
    fn mock_adapter_nonfiler_ntn_prefix() {
        let adapter = MockFbrIrisAdapter;
        let ntn = Ntn::new("0123456").unwrap();
        let resp = adapter
            .verify_ntn(&ntn, "Non-Filer Entity")
            .expect("should verify NTN");
        assert_eq!(resp.filer_status, FilerStatus::NonFiler);
    }

    #[test]
    fn mock_adapter_latefiler_ntn_prefix() {
        let adapter = MockFbrIrisAdapter;
        let ntn = Ntn::new("9123456").unwrap();
        let resp = adapter
            .verify_ntn(&ntn, "Late Filer Entity")
            .expect("should verify NTN");
        assert_eq!(resp.filer_status, FilerStatus::LateFiler);
    }

    // -- MockFbrIrisAdapter: submit_tax_event -----------------------------------

    #[test]
    fn mock_adapter_submit_tax_event() {
        let adapter = MockFbrIrisAdapter;
        let event = TaxEventSubmission {
            entity_id: "entity-001".into(),
            ntn: "1234567".into(),
            event_type: "payment_for_goods".into(),
            amount: "100000.00".into(),
            currency: "PKR".into(),
            jurisdiction: "PK".into(),
            idempotency_key: "IDEM-001".into(),
            tax_year: Some("2025-2026".into()),
            statutory_section: None,
        };
        let result = adapter
            .submit_tax_event(&event)
            .expect("should accept event");
        assert!(result.accepted);
        assert!(result.fbr_reference.contains("IDEM-001"));
        assert_eq!(result.idempotency_key, "IDEM-001");
    }

    #[test]
    fn mock_adapter_submit_rejects_invalid_ntn() {
        let adapter = MockFbrIrisAdapter;
        let event = TaxEventSubmission {
            entity_id: "e1".into(),
            ntn: "123".into(),
            event_type: "payment_for_goods".into(),
            amount: "1000".into(),
            currency: "PKR".into(),
            jurisdiction: "PK".into(),
            idempotency_key: "IK1".into(),
            tax_year: None,
            statutory_section: None,
        };
        let result = adapter.submit_tax_event(&event);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FbrError::InvalidNtn { .. }));
    }

    #[test]
    fn mock_adapter_submit_rejects_empty_idempotency_key() {
        let adapter = MockFbrIrisAdapter;
        let event = TaxEventSubmission {
            entity_id: "e1".into(),
            ntn: "1234567".into(),
            event_type: "payment_for_goods".into(),
            amount: "1000".into(),
            currency: "PKR".into(),
            jurisdiction: "PK".into(),
            idempotency_key: "".into(),
            tax_year: None,
            statutory_section: None,
        };
        let result = adapter.submit_tax_event(&event);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FbrError::SubmissionRejected { .. }
        ));
    }

    // -- MockFbrIrisAdapter: query_withholding_rate -----------------------------

    #[test]
    fn mock_adapter_withholding_rate_filer_goods() {
        let adapter = MockFbrIrisAdapter;
        let query = WithholdingRateQuery {
            transaction_type: "payment_for_goods".into(),
            filer_status: FilerStatus::Filer,
            jurisdiction: "PK".into(),
            tax_year: None,
        };
        let rate = adapter.query_withholding_rate(&query).expect("should return rate");
        assert_eq!(rate.rate_percent, "4.5");
        assert_eq!(rate.statutory_section, "S153(1)(a)");
        assert!(rate.is_final_tax);
    }

    #[test]
    fn mock_adapter_withholding_rate_nonfiler_goods() {
        let adapter = MockFbrIrisAdapter;
        let query = WithholdingRateQuery {
            transaction_type: "payment_for_goods".into(),
            filer_status: FilerStatus::NonFiler,
            jurisdiction: "PK".into(),
            tax_year: None,
        };
        let rate = adapter.query_withholding_rate(&query).expect("should return rate");
        assert_eq!(rate.rate_percent, "9.0");
    }

    #[test]
    fn mock_adapter_withholding_rate_filer_services() {
        let adapter = MockFbrIrisAdapter;
        let query = WithholdingRateQuery {
            transaction_type: "payment_for_services".into(),
            filer_status: FilerStatus::Filer,
            jurisdiction: "PK".into(),
            tax_year: None,
        };
        let rate = adapter.query_withholding_rate(&query).expect("should return rate");
        assert_eq!(rate.rate_percent, "8.0");
        assert_eq!(rate.statutory_section, "S153(1)(b)");
    }

    #[test]
    fn mock_adapter_withholding_rate_nonfiler_services() {
        let adapter = MockFbrIrisAdapter;
        let query = WithholdingRateQuery {
            transaction_type: "payment_for_services".into(),
            filer_status: FilerStatus::NonFiler,
            jurisdiction: "PK".into(),
            tax_year: None,
        };
        let rate = adapter.query_withholding_rate(&query).expect("should return rate");
        assert_eq!(rate.rate_percent, "16.0");
    }

    #[test]
    fn mock_adapter_withholding_rate_salary() {
        let adapter = MockFbrIrisAdapter;
        let query = WithholdingRateQuery {
            transaction_type: "salary_payment".into(),
            filer_status: FilerStatus::Filer,
            jurisdiction: "PK".into(),
            tax_year: None,
        };
        let rate = adapter.query_withholding_rate(&query).expect("should return rate");
        assert_eq!(rate.rate_percent, "12.5");
        assert!(!rate.is_final_tax);
    }

    #[test]
    fn mock_adapter_withholding_rate_unknown_type_defaults() {
        let adapter = MockFbrIrisAdapter;
        let query = WithholdingRateQuery {
            transaction_type: "unknown_event".into(),
            filer_status: FilerStatus::Filer,
            jurisdiction: "PK".into(),
            tax_year: None,
        };
        let rate = adapter.query_withholding_rate(&query).expect("should return default rate");
        assert_eq!(rate.rate_percent, "4.5");
    }

    // -- MockFbrIrisAdapter: get_taxpayer_profile -------------------------------

    #[test]
    fn mock_adapter_taxpayer_profile_filer() {
        let adapter = MockFbrIrisAdapter;
        let ntn = Ntn::new("1234567").unwrap();
        let profile = adapter
            .get_taxpayer_profile(&ntn)
            .expect("should return profile");
        assert_eq!(profile.ntn, "1234567");
        assert_eq!(profile.filer_status, FilerStatus::Filer);
        assert_eq!(profile.compliance_status, ComplianceStatus::Compliant);
        assert_eq!(profile.active_since, "2020-01-01");
        assert!(profile.registered_name.is_some());
    }

    #[test]
    fn mock_adapter_taxpayer_profile_nonfiler() {
        let adapter = MockFbrIrisAdapter;
        let ntn = Ntn::new("0123456").unwrap();
        let profile = adapter
            .get_taxpayer_profile(&ntn)
            .expect("should return profile");
        assert_eq!(profile.filer_status, FilerStatus::NonFiler);
        assert_eq!(profile.compliance_status, ComplianceStatus::NonCompliant);
    }

    #[test]
    fn mock_adapter_taxpayer_profile_latefiler() {
        let adapter = MockFbrIrisAdapter;
        let ntn = Ntn::new("9123456").unwrap();
        let profile = adapter
            .get_taxpayer_profile(&ntn)
            .expect("should return profile");
        assert_eq!(profile.filer_status, FilerStatus::LateFiler);
        assert_eq!(profile.compliance_status, ComplianceStatus::NonCompliant);
    }

    // -- Trait properties -------------------------------------------------------

    #[test]
    fn mock_adapter_name() {
        let adapter = MockFbrIrisAdapter;
        assert_eq!(adapter.adapter_name(), "MockFbrIrisAdapter");
    }

    #[test]
    fn adapter_trait_is_object_safe() {
        let adapter: Box<dyn FbrIrisAdapter> = Box::new(MockFbrIrisAdapter);
        assert_eq!(adapter.adapter_name(), "MockFbrIrisAdapter");
        let ntn = Ntn::new("1234567").unwrap();
        let resp = adapter
            .verify_ntn(&ntn, "Object Safe Test")
            .expect("trait object verify");
        assert!(resp.verified);
    }

    #[test]
    fn adapter_trait_behind_arc() {
        let adapter: std::sync::Arc<dyn FbrIrisAdapter> =
            std::sync::Arc::new(MockFbrIrisAdapter);
        let ntn = Ntn::new("1234567").unwrap();
        let resp = adapter
            .verify_ntn(&ntn, "Arc Test")
            .expect("Arc adapter should work");
        assert!(resp.verified);
    }
}
