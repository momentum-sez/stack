//! # IRAS Integration Adapter Interface
//!
//! Defines the adapter interface for IRAS (Inland Revenue Authority of
//! Singapore), responsible for GST collection, corporate tax assessment,
//! and withholding tax administration.
//!
//! ## Architecture
//!
//! The `IrasAdapter` trait abstracts over the IRAS backend. Production
//! deployments implement it against the live IRAS API (myTax Portal);
//! test environments use `MockIrasAdapter`. This mirrors the FBR IRIS
//! adapter from the Pakistan vertical.
//!
//! ## UEN-Based Tax Identification
//!
//! Singapore uses the UEN (Unique Entity Number) as the primary tax
//! identifier for corporate entities. Individual taxpayers use NRIC.
//! The `validate_uen` helper delegates to `mez_core::Uen::new()`.
//!
//! ## Key Tax Rates (as of 2026)
//!
//! - Corporate tax: 17% (partial exemption on first SGD 200K)
//! - GST: 9% (effective 1 Jan 2024)
//! - Withholding tax on interest: 15% (non-resident)
//! - Withholding tax on royalties: 10% (non-resident)

use mez_core::Uen;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::adapter::{AdapterCategory, AdapterHealth, NationalSystemAdapter};

/// Errors from IRAS integration operations.
#[derive(Debug, thiserror::Error)]
pub enum IrasError {
    /// IRAS service is unreachable or returned a 5xx status.
    #[error("IRAS service unavailable: {reason}")]
    ServiceUnavailable {
        /// Human-readable description of the outage or error.
        reason: String,
    },

    /// UEN format is invalid.
    #[error("invalid UEN: {reason}")]
    InvalidUen {
        /// Description of the validation failure.
        reason: String,
    },

    /// The IRAS adapter has not been configured for this deployment.
    #[error("IRAS adapter not configured: {reason}")]
    NotConfigured {
        /// Why configuration is missing or incomplete.
        reason: String,
    },

    /// The request to IRAS timed out.
    #[error("IRAS request timed out after {elapsed_ms}ms")]
    Timeout {
        /// Elapsed time in milliseconds before the timeout triggered.
        elapsed_ms: u64,
    },

    /// IRAS verification could not be completed.
    #[error("UEN verification failed: {reason}")]
    VerificationFailed {
        /// Description of the failure.
        reason: String,
    },

    /// IRAS rejected a filing submission.
    #[error("filing rejected: {reason}")]
    FilingRejected {
        /// Description of why the filing was rejected.
        reason: String,
    },
}

/// GST registration status as reported by IRAS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GstRegistrationStatus {
    /// Entity is registered for GST.
    Registered,
    /// Entity is not registered (below threshold or exempt).
    NotRegistered,
    /// Entity has been deregistered.
    Deregistered,
    /// UEN does not exist in IRAS records.
    NotFound,
}

impl fmt::Display for GstRegistrationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Registered => write!(f, "Registered"),
            Self::NotRegistered => write!(f, "NotRegistered"),
            Self::Deregistered => write!(f, "Deregistered"),
            Self::NotFound => write!(f, "NotFound"),
        }
    }
}

/// Corporate tax filing status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CorporateTaxStatus {
    /// All filings current (ECI + Form C/C-S submitted).
    Current,
    /// One or more filings overdue.
    Overdue,
    /// Entity exempt from filing (e.g. dormant company).
    Exempt,
}

impl fmt::Display for CorporateTaxStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Current => write!(f, "Current"),
            Self::Overdue => write!(f, "Overdue"),
            Self::Exempt => write!(f, "Exempt"),
        }
    }
}

/// UEN verification response from IRAS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UenVerificationResponse {
    /// Whether the UEN was successfully verified.
    pub verified: bool,
    /// The UEN that was verified.
    pub uen: String,
    /// Registered entity name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registered_name: Option<String>,
    /// GST registration status.
    pub gst_status: GstRegistrationStatus,
    /// Corporate tax filing status.
    pub tax_filing_status: CorporateTaxStatus,
    /// ISO 8601 timestamp of the verification.
    pub verification_timestamp: String,
    /// Reference identifier.
    pub reference: String,
}

/// GST return submission to IRAS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GstReturnSubmission {
    /// UEN of the entity.
    pub uen: String,
    /// Accounting period (e.g. "2026-Q1").
    pub accounting_period: String,
    /// Total standard-rated supplies in SGD.
    pub standard_rated_supplies_sgd: String,
    /// Total zero-rated supplies in SGD.
    pub zero_rated_supplies_sgd: String,
    /// Total exempt supplies in SGD.
    pub exempt_supplies_sgd: String,
    /// Output tax (GST charged) in SGD.
    pub output_tax_sgd: String,
    /// Input tax (GST claimable) in SGD.
    pub input_tax_sgd: String,
    /// Net GST payable/refundable in SGD.
    pub net_gst_sgd: String,
    /// Idempotency key.
    pub idempotency_key: String,
}

/// Result of a GST return submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GstReturnResult {
    /// Whether the return was accepted.
    pub accepted: bool,
    /// IRAS-assigned reference number.
    pub iras_reference: String,
    /// ISO 8601 timestamp when IRAS recorded the return.
    pub recorded_at: String,
    /// The idempotency key echoed back.
    pub idempotency_key: String,
}

/// Withholding tax rate query for cross-border payments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithholdingTaxQuery {
    /// Type of payment (e.g. "interest", "royalty", "technical_fee").
    pub payment_type: String,
    /// Country of the non-resident recipient (ISO 3166-1 alpha-2).
    pub recipient_country: String,
    /// Whether a DTA (Double Tax Agreement) applies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dta_applicable: Option<bool>,
}

/// Withholding tax rate response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithholdingTaxRate {
    /// Applicable rate as a percentage string (e.g. "15.0").
    pub rate_percent: String,
    /// Statutory section (e.g. "S45 ITA").
    pub statutory_section: String,
    /// Whether reduced rate applies under DTA.
    pub dta_reduced: bool,
    /// Description of the withholding rule.
    pub description: String,
}

/// Adapter trait for IRAS tax authority integration.
///
/// Implementations must be `Send + Sync` so they can be shared across
/// async tasks behind an `Arc`. The trait is object-safe to support
/// runtime adapter selection (mock vs. live).
pub trait IrasAdapter: Send + Sync {
    /// Verify a UEN against IRAS records.
    fn verify_uen(
        &self,
        uen: &Uen,
        entity_name: &str,
    ) -> Result<UenVerificationResponse, IrasError>;

    /// Submit a GST return to IRAS.
    fn submit_gst_return(
        &self,
        submission: &GstReturnSubmission,
    ) -> Result<GstReturnResult, IrasError>;

    /// Query applicable withholding tax rate for a cross-border payment.
    fn query_withholding_rate(
        &self,
        query: &WithholdingTaxQuery,
    ) -> Result<WithholdingTaxRate, IrasError>;

    /// Return the human-readable name of this adapter implementation.
    fn adapter_name(&self) -> &str;
}

/// Validate that a UEN string is well-formed by delegating to
/// `mez_core::Uen::new()`. Returns the validated `Uen` on success.
pub fn validate_uen(uen: &str) -> Result<Uen, IrasError> {
    Uen::new(uen).map_err(|e| IrasError::InvalidUen {
        reason: e.to_string(),
    })
}

/// Mock IRAS adapter for testing and development.
///
/// Returns deterministic test data:
/// - UENs starting with "0" are treated as not GST-registered
/// - All other valid UENs are GST-registered with current filings
/// - Withholding rates: interest 15%, royalty 10% (standard S45 ITA rates)
#[derive(Debug, Clone)]
pub struct MockIrasAdapter;

impl IrasAdapter for MockIrasAdapter {
    fn verify_uen(
        &self,
        uen: &Uen,
        _entity_name: &str,
    ) -> Result<UenVerificationResponse, IrasError> {
        let gst_status = if uen.as_str().starts_with('0') {
            GstRegistrationStatus::NotRegistered
        } else {
            GstRegistrationStatus::Registered
        };

        Ok(UenVerificationResponse {
            verified: true,
            uen: uen.as_str().to_string(),
            registered_name: Some("Mock Pte Ltd".to_string()),
            gst_status,
            tax_filing_status: CorporateTaxStatus::Current,
            verification_timestamp: chrono::Utc::now().to_rfc3339(),
            reference: format!("MOCK-IRAS-{}", &uen.as_str()[..4.min(uen.as_str().len())]),
        })
    }

    fn submit_gst_return(
        &self,
        submission: &GstReturnSubmission,
    ) -> Result<GstReturnResult, IrasError> {
        let _validated = validate_uen(&submission.uen)?;

        Ok(GstReturnResult {
            accepted: true,
            iras_reference: format!("MOCK-GST-{}", submission.accounting_period),
            recorded_at: chrono::Utc::now().to_rfc3339(),
            idempotency_key: submission.idempotency_key.clone(),
        })
    }

    fn query_withholding_rate(
        &self,
        query: &WithholdingTaxQuery,
    ) -> Result<WithholdingTaxRate, IrasError> {
        // Standard S45 ITA rates for non-residents.
        let (rate, section, desc) = match query.payment_type.as_str() {
            "interest" => ("15.0", "S45(1) ITA", "Withholding tax on interest paid to non-resident"),
            "royalty" => ("10.0", "S45(1) ITA", "Withholding tax on royalty paid to non-resident"),
            "technical_fee" => (
                "15.0",
                "S45(1) ITA",
                "Withholding tax on technical service fee paid to non-resident",
            ),
            _ => ("15.0", "S45(1) ITA", "Default withholding rate for non-resident payments"),
        };

        let dta_reduced = query.dta_applicable.unwrap_or(false);

        Ok(WithholdingTaxRate {
            rate_percent: rate.to_string(),
            statutory_section: section.to_string(),
            dta_reduced,
            description: desc.to_string(),
        })
    }

    fn adapter_name(&self) -> &str {
        "MockIrasAdapter"
    }
}

impl NationalSystemAdapter for MockIrasAdapter {
    fn category(&self) -> AdapterCategory {
        AdapterCategory::Tax
    }

    fn jurisdiction(&self) -> &str {
        "sg"
    }

    fn health(&self) -> AdapterHealth {
        AdapterHealth::Healthy
    }

    fn adapter_name(&self) -> &str {
        "MockIrasAdapter"
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
        assert!(matches!(result.unwrap_err(), IrasError::InvalidUen { .. }));
    }

    // -- GstRegistrationStatus --------------------------------------------------

    #[test]
    fn gst_registration_status_display() {
        assert_eq!(format!("{}", GstRegistrationStatus::Registered), "Registered");
        assert_eq!(
            format!("{}", GstRegistrationStatus::NotRegistered),
            "NotRegistered"
        );
        assert_eq!(
            format!("{}", GstRegistrationStatus::Deregistered),
            "Deregistered"
        );
        assert_eq!(format!("{}", GstRegistrationStatus::NotFound), "NotFound");
    }

    #[test]
    fn gst_registration_status_serde_roundtrip() {
        let status = GstRegistrationStatus::Registered;
        let json = serde_json::to_string(&status).expect("serialize");
        let back: GstRegistrationStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, GstRegistrationStatus::Registered);
    }

    // -- CorporateTaxStatus -----------------------------------------------------

    #[test]
    fn corporate_tax_status_display() {
        assert_eq!(format!("{}", CorporateTaxStatus::Current), "Current");
        assert_eq!(format!("{}", CorporateTaxStatus::Overdue), "Overdue");
        assert_eq!(format!("{}", CorporateTaxStatus::Exempt), "Exempt");
    }

    // -- IrasError --------------------------------------------------------------

    #[test]
    fn iras_error_display() {
        let err = IrasError::ServiceUnavailable {
            reason: "maintenance window".to_string(),
        };
        assert!(format!("{err}").contains("maintenance window"));

        let err = IrasError::Timeout { elapsed_ms: 8000 };
        assert!(format!("{err}").contains("8000"));
    }

    // -- MockIrasAdapter --------------------------------------------------------

    #[test]
    fn mock_iras_verify_uen_registered() {
        let adapter = MockIrasAdapter;
        let uen = Uen::new("200012345A").expect("valid UEN");
        let resp = adapter.verify_uen(&uen, "Test Pte Ltd").expect("should succeed");
        assert!(resp.verified);
        assert_eq!(resp.gst_status, GstRegistrationStatus::Registered);
        assert_eq!(resp.tax_filing_status, CorporateTaxStatus::Current);
    }

    #[test]
    fn mock_iras_verify_uen_not_registered() {
        let adapter = MockIrasAdapter;
        let uen = Uen::new("012345678A").expect("valid UEN");
        let resp = adapter.verify_uen(&uen, "Test").expect("should succeed");
        assert_eq!(resp.gst_status, GstRegistrationStatus::NotRegistered);
    }

    #[test]
    fn mock_iras_submit_gst_return() {
        let adapter = MockIrasAdapter;
        let submission = GstReturnSubmission {
            uen: "200012345A".to_string(),
            accounting_period: "2026-Q1".to_string(),
            standard_rated_supplies_sgd: "500000.00".to_string(),
            zero_rated_supplies_sgd: "100000.00".to_string(),
            exempt_supplies_sgd: "0.00".to_string(),
            output_tax_sgd: "45000.00".to_string(),
            input_tax_sgd: "20000.00".to_string(),
            net_gst_sgd: "25000.00".to_string(),
            idempotency_key: "GST-001".to_string(),
        };
        let result = adapter.submit_gst_return(&submission).expect("should succeed");
        assert!(result.accepted);
        assert_eq!(result.idempotency_key, "GST-001");
    }

    #[test]
    fn mock_iras_query_withholding_interest() {
        let adapter = MockIrasAdapter;
        let query = WithholdingTaxQuery {
            payment_type: "interest".to_string(),
            recipient_country: "US".to_string(),
            dta_applicable: None,
        };
        let rate = adapter.query_withholding_rate(&query).expect("should succeed");
        assert_eq!(rate.rate_percent, "15.0");
        assert!(rate.statutory_section.contains("S45"));
    }

    #[test]
    fn mock_iras_query_withholding_royalty() {
        let adapter = MockIrasAdapter;
        let query = WithholdingTaxQuery {
            payment_type: "royalty".to_string(),
            recipient_country: "JP".to_string(),
            dta_applicable: Some(true),
        };
        let rate = adapter.query_withholding_rate(&query).expect("should succeed");
        assert_eq!(rate.rate_percent, "10.0");
        assert!(rate.dta_reduced);
    }

    #[test]
    fn mock_iras_adapter_name() {
        let adapter = MockIrasAdapter;
        assert_eq!(IrasAdapter::adapter_name(&adapter), "MockIrasAdapter");
    }

    #[test]
    fn mock_iras_national_system_adapter() {
        let adapter = MockIrasAdapter;
        assert_eq!(NationalSystemAdapter::category(&adapter), AdapterCategory::Tax);
        assert_eq!(NationalSystemAdapter::jurisdiction(&adapter), "sg");
    }

    #[test]
    fn trait_object_safety() {
        let adapter: Box<dyn IrasAdapter> = Box::new(MockIrasAdapter);
        assert_eq!(adapter.adapter_name(), "MockIrasAdapter");
    }

    #[test]
    fn uen_verification_response_serde_roundtrip() {
        let resp = UenVerificationResponse {
            verified: true,
            uen: "200012345A".to_string(),
            registered_name: Some("Test Pte Ltd".to_string()),
            gst_status: GstRegistrationStatus::Registered,
            tax_filing_status: CorporateTaxStatus::Current,
            verification_timestamp: "2026-01-15T12:00:00Z".to_string(),
            reference: "REF-001".to_string(),
        };
        let json = serde_json::to_string(&resp).expect("serialize");
        let back: UenVerificationResponse = serde_json::from_str(&json).expect("deserialize");
        assert!(back.verified);
        assert_eq!(back.gst_status, GstRegistrationStatus::Registered);
    }
}
