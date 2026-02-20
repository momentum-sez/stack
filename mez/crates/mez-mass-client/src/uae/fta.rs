//! # FTA Integration Adapter Interface
//!
//! Defines the adapter interface for the UAE Federal Tax Authority (FTA),
//! responsible for VAT registration, filing, and Economic Substance
//! Regulations (ESR) compliance.
//!
//! ## Architecture
//!
//! The `FtaAdapter` trait abstracts over the FTA backend. Production
//! deployments implement it against the live FTA EmaraTax portal; test
//! environments use `MockFtaAdapter`. This mirrors the FBR IRIS adapter
//! pattern from the Pakistan vertical.
//!
//! ## TRN (Tax Registration Number)
//!
//! UAE TRNs are 15-digit numbers issued by FTA upon VAT registration.
//! The `validate_trn` helper enforces this constraint before any request
//! reaches the adapter.
//!
//! ## Key Integration Points
//!
//! - **TRN verification**: Validate VAT registration status
//! - **VAT filing**: Submit VAT return data (quarterly)
//! - **ESR notification**: Submit Economic Substance Regulations notification
//! - **Withholding status**: UAE has no income tax but has VAT obligations

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::adapter::{AdapterCategory, AdapterHealth, NationalSystemAdapter};

/// Errors from FTA integration operations.
#[derive(Debug, thiserror::Error)]
pub enum FtaError {
    /// FTA service is unreachable or returned a 5xx status.
    #[error("FTA service unavailable: {reason}")]
    ServiceUnavailable {
        /// Human-readable description of the outage or error.
        reason: String,
    },

    /// TRN format is invalid (not exactly 15 digits).
    #[error("invalid TRN: {reason}")]
    InvalidTrn {
        /// Description of the validation failure.
        reason: String,
    },

    /// The FTA adapter has not been configured for this deployment.
    #[error("FTA adapter not configured: {reason}")]
    NotConfigured {
        /// Why configuration is missing or incomplete.
        reason: String,
    },

    /// The request to FTA timed out.
    #[error("FTA request timed out after {elapsed_ms}ms")]
    Timeout {
        /// Elapsed time in milliseconds before the timeout triggered.
        elapsed_ms: u64,
    },

    /// FTA accepted the request but verification could not be completed.
    #[error("TRN verification failed: {reason}")]
    VerificationFailed {
        /// Description of the failure.
        reason: String,
    },

    /// FTA rejected a filing submission.
    #[error("filing rejected: {reason}")]
    FilingRejected {
        /// Description of why the filing was rejected.
        reason: String,
    },
}

/// VAT registration status as reported by FTA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VatRegistrationStatus {
    /// Entity is registered for VAT and filing is current.
    Registered,
    /// Entity is registered but has overdue filings.
    RegisteredOverdue,
    /// Entity has been deregistered from VAT.
    Deregistered,
    /// TRN does not exist in FTA records.
    NotFound,
}

impl fmt::Display for VatRegistrationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Registered => write!(f, "Registered"),
            Self::RegisteredOverdue => write!(f, "RegisteredOverdue"),
            Self::Deregistered => write!(f, "Deregistered"),
            Self::NotFound => write!(f, "NotFound"),
        }
    }
}

/// TRN verification response from FTA.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrnVerificationResponse {
    /// Whether the TRN was successfully verified.
    pub verified: bool,
    /// The TRN that was verified.
    pub trn: String,
    /// Registered entity name as recorded by FTA.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registered_name: Option<String>,
    /// Current VAT registration status.
    pub vat_status: VatRegistrationStatus,
    /// ISO 8601 timestamp when FTA performed the verification.
    pub verification_timestamp: String,
    /// Reference identifier for audit trail.
    pub reference: String,
}

/// VAT return submission to FTA.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VatReturnSubmission {
    /// TRN of the taxpayer (15 digits).
    pub trn: String,
    /// Tax period (e.g. "2026-Q1").
    pub tax_period: String,
    /// Total supplies amount in AED (minor units string).
    pub total_supplies_aed: String,
    /// Total VAT due in AED.
    pub vat_due_aed: String,
    /// Total input VAT recoverable in AED.
    pub input_vat_aed: String,
    /// Net VAT payable (due minus input).
    pub net_vat_aed: String,
    /// Idempotency key.
    pub idempotency_key: String,
}

/// Result of a VAT return submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VatReturnResult {
    /// Whether the return was accepted.
    pub accepted: bool,
    /// FTA-assigned reference number.
    pub fta_reference: String,
    /// ISO 8601 timestamp when FTA recorded the return.
    pub recorded_at: String,
    /// The idempotency key echoed back.
    pub idempotency_key: String,
}

/// Adapter trait for FTA tax authority integration.
///
/// Implementations must be `Send + Sync` so they can be shared across
/// async tasks behind an `Arc`. The trait is object-safe to support
/// runtime adapter selection (mock vs. live).
pub trait FtaAdapter: Send + Sync {
    /// Verify a TRN against FTA records.
    fn verify_trn(
        &self,
        trn: &str,
        entity_name: &str,
    ) -> Result<TrnVerificationResponse, FtaError>;

    /// Submit a VAT return to FTA.
    fn submit_vat_return(
        &self,
        submission: &VatReturnSubmission,
    ) -> Result<VatReturnResult, FtaError>;

    /// Return the human-readable name of this adapter implementation.
    fn adapter_name(&self) -> &str;
}

/// Validate that a TRN string contains exactly 15 digits.
/// Returns the canonical form on success.
pub fn validate_trn(trn: &str) -> Result<String, FtaError> {
    let digits: String = trn.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 15 {
        return Err(FtaError::InvalidTrn {
            reason: format!(
                "TRN must be exactly 15 digits, got {} digits from input '{}'",
                digits.len(),
                trn
            ),
        });
    }
    Ok(digits)
}

/// Mock FTA adapter for testing and development.
///
/// Returns deterministic test data:
/// - TRNs starting with "000" are treated as deregistered
/// - All other valid TRNs are treated as registered
/// - VAT rate is fixed at 5% per Federal Decree-Law No. 8/2017
#[derive(Debug, Clone)]
pub struct MockFtaAdapter;

impl FtaAdapter for MockFtaAdapter {
    fn verify_trn(
        &self,
        trn: &str,
        _entity_name: &str,
    ) -> Result<TrnVerificationResponse, FtaError> {
        let canonical = validate_trn(trn)?;

        let vat_status = if canonical.starts_with("000") {
            VatRegistrationStatus::Deregistered
        } else {
            VatRegistrationStatus::Registered
        };

        Ok(TrnVerificationResponse {
            verified: true,
            trn: canonical,
            registered_name: Some("Mock Entity LLC".to_string()),
            vat_status,
            verification_timestamp: chrono::Utc::now().to_rfc3339(),
            reference: format!("MOCK-FTA-{}", &trn[..7.min(trn.len())]),
        })
    }

    fn submit_vat_return(
        &self,
        submission: &VatReturnSubmission,
    ) -> Result<VatReturnResult, FtaError> {
        let _canonical = validate_trn(&submission.trn)?;

        Ok(VatReturnResult {
            accepted: true,
            fta_reference: format!("MOCK-VAT-{}", submission.tax_period),
            recorded_at: chrono::Utc::now().to_rfc3339(),
            idempotency_key: submission.idempotency_key.clone(),
        })
    }

    fn adapter_name(&self) -> &str {
        "MockFtaAdapter"
    }
}

impl NationalSystemAdapter for MockFtaAdapter {
    fn category(&self) -> AdapterCategory {
        AdapterCategory::Tax
    }

    fn jurisdiction(&self) -> &str {
        "ae-abudhabi-adgm"
    }

    fn health(&self) -> AdapterHealth {
        AdapterHealth::Healthy
    }

    fn adapter_name(&self) -> &str {
        "MockFtaAdapter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- validate_trn -----------------------------------------------------------

    #[test]
    fn validate_trn_accepts_15_digits() {
        let result = validate_trn("123456789012345");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "123456789012345");
    }

    #[test]
    fn validate_trn_rejects_too_few_digits() {
        let result = validate_trn("12345");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FtaError::InvalidTrn { .. }));
    }

    #[test]
    fn validate_trn_rejects_too_many_digits() {
        let result = validate_trn("1234567890123456");
        assert!(result.is_err());
    }

    #[test]
    fn validate_trn_rejects_empty() {
        let result = validate_trn("");
        assert!(result.is_err());
    }

    // -- VatRegistrationStatus --------------------------------------------------

    #[test]
    fn vat_registration_status_display() {
        assert_eq!(format!("{}", VatRegistrationStatus::Registered), "Registered");
        assert_eq!(
            format!("{}", VatRegistrationStatus::RegisteredOverdue),
            "RegisteredOverdue"
        );
        assert_eq!(
            format!("{}", VatRegistrationStatus::Deregistered),
            "Deregistered"
        );
        assert_eq!(format!("{}", VatRegistrationStatus::NotFound), "NotFound");
    }

    #[test]
    fn vat_registration_status_serde_roundtrip() {
        let status = VatRegistrationStatus::Registered;
        let json = serde_json::to_string(&status).expect("serialize");
        let back: VatRegistrationStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, VatRegistrationStatus::Registered);
    }

    // -- FtaError ---------------------------------------------------------------

    #[test]
    fn fta_error_display() {
        let err = FtaError::ServiceUnavailable {
            reason: "maintenance".to_string(),
        };
        assert!(format!("{err}").contains("maintenance"));

        let err = FtaError::Timeout { elapsed_ms: 3000 };
        assert!(format!("{err}").contains("3000"));
    }

    // -- MockFtaAdapter ---------------------------------------------------------

    #[test]
    fn mock_fta_verify_trn_registered() {
        let adapter = MockFtaAdapter;
        let resp = adapter
            .verify_trn("100000000000000", "Test LLC")
            .expect("valid TRN");
        assert!(resp.verified);
        assert_eq!(resp.vat_status, VatRegistrationStatus::Registered);
    }

    #[test]
    fn mock_fta_verify_trn_deregistered() {
        let adapter = MockFtaAdapter;
        let resp = adapter
            .verify_trn("000000000000000", "Test LLC")
            .expect("valid TRN");
        assert_eq!(resp.vat_status, VatRegistrationStatus::Deregistered);
    }

    #[test]
    fn mock_fta_rejects_invalid_trn() {
        let adapter = MockFtaAdapter;
        let result = adapter.verify_trn("invalid", "Test");
        assert!(result.is_err());
    }

    #[test]
    fn mock_fta_submit_vat_return() {
        let adapter = MockFtaAdapter;
        let submission = VatReturnSubmission {
            trn: "100000000000000".to_string(),
            tax_period: "2026-Q1".to_string(),
            total_supplies_aed: "1000000.00".to_string(),
            vat_due_aed: "50000.00".to_string(),
            input_vat_aed: "20000.00".to_string(),
            net_vat_aed: "30000.00".to_string(),
            idempotency_key: "IDK-001".to_string(),
        };
        let result = adapter.submit_vat_return(&submission).expect("should succeed");
        assert!(result.accepted);
        assert_eq!(result.idempotency_key, "IDK-001");
    }

    #[test]
    fn mock_fta_adapter_name() {
        let adapter = MockFtaAdapter;
        assert_eq!(FtaAdapter::adapter_name(&adapter), "MockFtaAdapter");
    }

    #[test]
    fn mock_fta_national_system_adapter() {
        let adapter = MockFtaAdapter;
        assert_eq!(NationalSystemAdapter::category(&adapter), AdapterCategory::Tax);
        assert_eq!(NationalSystemAdapter::jurisdiction(&adapter), "ae-abudhabi-adgm");
    }

    #[test]
    fn trait_object_safety() {
        let adapter: Box<dyn FtaAdapter> = Box::new(MockFtaAdapter);
        assert_eq!(adapter.adapter_name(), "MockFtaAdapter");
    }

    #[test]
    fn trn_verification_response_serde_roundtrip() {
        let resp = TrnVerificationResponse {
            verified: true,
            trn: "100000000000000".to_string(),
            registered_name: Some("Test LLC".to_string()),
            vat_status: VatRegistrationStatus::Registered,
            verification_timestamp: "2026-01-15T12:00:00Z".to_string(),
            reference: "REF-001".to_string(),
        };
        let json = serde_json::to_string(&resp).expect("serialize");
        let back: TrnVerificationResponse = serde_json::from_str(&json).expect("deserialize");
        assert!(back.verified);
        assert_eq!(back.vat_status, VatRegistrationStatus::Registered);
    }
}
