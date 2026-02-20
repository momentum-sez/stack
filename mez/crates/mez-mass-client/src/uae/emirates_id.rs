//! # ICA Emirates ID Integration Adapter Interface
//!
//! Defines the adapter interface for ICA (Federal Authority for Identity,
//! Citizenship, Customs and Port Security) Emirates ID verification.
//!
//! ## Architecture
//!
//! The `IcaAdapter` trait abstracts over the ICA Emirates ID verification
//! backend. Production deployments implement it against the live ICA API;
//! test environments use `MockIcaAdapter`. This separation allows the
//! identity verification pipeline to compose ICA operations without coupling
//! to a specific transport or API version.
//!
//! ## Emirates ID Validation
//!
//! UAE Emirates IDs are 15-digit numbers prefixed with `784` (ISO 3166
//! country code for UAE). The `validate_emirates_id` helper delegates to
//! `mez_core::EmiratesId::new()` for validation before any request reaches
//! the adapter.

use mez_core::EmiratesId;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::adapter::{AdapterCategory, AdapterHealth, NationalSystemAdapter};

/// Errors from ICA Emirates ID integration operations.
#[derive(Debug, thiserror::Error)]
pub enum IcaError {
    /// ICA service is unreachable or returned a 5xx status.
    #[error("ICA service unavailable: {reason}")]
    ServiceUnavailable {
        /// Human-readable description of the outage or error.
        reason: String,
    },

    /// Emirates ID format is invalid.
    #[error("invalid Emirates ID: {reason}")]
    InvalidEmiratesId {
        /// Description of the validation failure.
        reason: String,
    },

    /// ICA accepted the request but verification could not be completed.
    #[error("Emirates ID verification failed: {reason}")]
    VerificationFailed {
        /// Description of the failure.
        reason: String,
    },

    /// The ICA adapter has not been configured for this deployment.
    #[error("ICA adapter not configured: {reason}")]
    NotConfigured {
        /// Why configuration is missing or incomplete.
        reason: String,
    },

    /// The request to ICA timed out.
    #[error("ICA request timed out after {elapsed_ms}ms")]
    Timeout {
        /// Elapsed time in milliseconds before the timeout triggered.
        elapsed_ms: u64,
    },
}

/// Emirates ID status as reported by ICA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmiratesIdStatus {
    /// Emirates ID is active and valid.
    Active,
    /// Emirates ID has expired and must be renewed.
    Expired,
    /// Emirates ID has been cancelled (visa cancellation, departure, etc.).
    Cancelled,
    /// Emirates ID number does not exist in ICA records.
    NotFound,
}

impl fmt::Display for EmiratesIdStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "Active"),
            Self::Expired => write!(f, "Expired"),
            Self::Cancelled => write!(f, "Cancelled"),
            Self::NotFound => write!(f, "NotFound"),
        }
    }
}

/// Emirates ID verification request sent to ICA.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmiratesIdVerificationRequest {
    /// Emirates ID number (15 digits, with or without dashes).
    pub emirates_id: String,
    /// Full name of the individual for cross-reference.
    pub full_name: String,
    /// Nationality (ISO 3166-1 alpha-2) for additional verification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nationality: Option<String>,
    /// Date of birth in ISO 8601 format (YYYY-MM-DD) for additional verification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_of_birth: Option<String>,
    /// Idempotency key for this request.
    pub request_reference: String,
}

/// Emirates ID verification response from ICA.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmiratesIdVerificationResponse {
    /// Whether the identity was successfully verified.
    pub verified: bool,
    /// Current status of the Emirates ID in ICA records.
    pub emirates_id_status: EmiratesIdStatus,
    /// Visa/residency status (e.g. "residence", "visit", "citizen").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub residency_status: Option<String>,
    /// ISO 8601 timestamp when ICA performed the verification.
    pub verification_timestamp: String,
    /// Reference identifier linking back to the original request.
    pub reference: String,
}

/// Adapter trait for ICA Emirates ID verification.
///
/// Implementations must be `Send + Sync` so they can be shared across
/// async tasks behind an `Arc`. The trait is object-safe to support
/// runtime adapter selection (mock vs. live).
pub trait IcaAdapter: Send + Sync {
    /// Verify an individual's identity against ICA records using their
    /// Emirates ID number, name, and optional biographical details.
    fn verify_emirates_id(
        &self,
        request: &EmiratesIdVerificationRequest,
    ) -> Result<EmiratesIdVerificationResponse, IcaError>;

    /// Check the current status of an Emirates ID without performing a full
    /// identity verification.
    fn check_emirates_id_status(
        &self,
        emirates_id: &str,
    ) -> Result<EmiratesIdStatus, IcaError>;

    /// Return the human-readable name of this adapter implementation.
    fn adapter_name(&self) -> &str;
}

/// Validate that an Emirates ID string is well-formed by delegating to
/// `mez_core::EmiratesId::new()`. Returns the validated `EmiratesId` on success.
pub fn validate_emirates_id(id: &str) -> Result<EmiratesId, IcaError> {
    EmiratesId::new(id).map_err(|e| IcaError::InvalidEmiratesId {
        reason: e.to_string(),
    })
}

/// Mock ICA adapter for testing and development.
///
/// Returns successful verification for any well-formed Emirates ID.
/// Uses `EmiratesIdStatus::Active` and residency status "residence" by default.
#[derive(Debug, Clone)]
pub struct MockIcaAdapter;

impl IcaAdapter for MockIcaAdapter {
    fn verify_emirates_id(
        &self,
        request: &EmiratesIdVerificationRequest,
    ) -> Result<EmiratesIdVerificationResponse, IcaError> {
        let _validated = validate_emirates_id(&request.emirates_id)?;

        Ok(EmiratesIdVerificationResponse {
            verified: true,
            emirates_id_status: EmiratesIdStatus::Active,
            residency_status: Some("residence".to_string()),
            verification_timestamp: chrono::Utc::now().to_rfc3339(),
            reference: format!("MOCK-{}", request.request_reference),
        })
    }

    fn check_emirates_id_status(
        &self,
        emirates_id: &str,
    ) -> Result<EmiratesIdStatus, IcaError> {
        let _validated = validate_emirates_id(emirates_id)?;
        Ok(EmiratesIdStatus::Active)
    }

    fn adapter_name(&self) -> &str {
        "MockIcaAdapter"
    }
}

impl NationalSystemAdapter for MockIcaAdapter {
    fn category(&self) -> AdapterCategory {
        AdapterCategory::Identity
    }

    fn jurisdiction(&self) -> &str {
        "ae-abudhabi-adgm"
    }

    fn health(&self) -> AdapterHealth {
        AdapterHealth::Healthy
    }

    fn adapter_name(&self) -> &str {
        "MockIcaAdapter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- validate_emirates_id ---------------------------------------------------

    #[test]
    fn validate_emirates_id_accepts_valid() {
        let result = validate_emirates_id("784-1234-1234567-1");
        assert!(result.is_ok());
    }

    #[test]
    fn validate_emirates_id_rejects_wrong_prefix() {
        let result = validate_emirates_id("123456789012345");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, IcaError::InvalidEmiratesId { .. }));
    }

    #[test]
    fn validate_emirates_id_rejects_too_short() {
        let result = validate_emirates_id("78412341234");
        assert!(result.is_err());
    }

    // -- EmiratesIdStatus -------------------------------------------------------

    #[test]
    fn emirates_id_status_display() {
        assert_eq!(format!("{}", EmiratesIdStatus::Active), "Active");
        assert_eq!(format!("{}", EmiratesIdStatus::Expired), "Expired");
        assert_eq!(format!("{}", EmiratesIdStatus::Cancelled), "Cancelled");
        assert_eq!(format!("{}", EmiratesIdStatus::NotFound), "NotFound");
    }

    #[test]
    fn emirates_id_status_serde_roundtrip() {
        let status = EmiratesIdStatus::Active;
        let json = serde_json::to_string(&status).expect("serialize");
        let back: EmiratesIdStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, EmiratesIdStatus::Active);
    }

    // -- IcaError ---------------------------------------------------------------

    #[test]
    fn ica_error_display() {
        let err = IcaError::ServiceUnavailable {
            reason: "connection refused".to_string(),
        };
        assert!(format!("{err}").contains("connection refused"));

        let err = IcaError::Timeout { elapsed_ms: 5000 };
        assert!(format!("{err}").contains("5000"));
    }

    // -- MockIcaAdapter ---------------------------------------------------------

    #[test]
    fn mock_ica_verify_valid_id() {
        let adapter = MockIcaAdapter;
        let req = EmiratesIdVerificationRequest {
            emirates_id: "784-1234-1234567-1".to_string(),
            full_name: "Test Person".to_string(),
            nationality: Some("PK".to_string()),
            date_of_birth: None,
            request_reference: "REQ-001".to_string(),
        };
        let resp = adapter.verify_emirates_id(&req).expect("should succeed");
        assert!(resp.verified);
        assert_eq!(resp.emirates_id_status, EmiratesIdStatus::Active);
        assert!(resp.reference.contains("MOCK"));
    }

    #[test]
    fn mock_ica_rejects_invalid_id() {
        let adapter = MockIcaAdapter;
        let req = EmiratesIdVerificationRequest {
            emirates_id: "invalid".to_string(),
            full_name: "Test".to_string(),
            nationality: None,
            date_of_birth: None,
            request_reference: "REQ-002".to_string(),
        };
        assert!(adapter.verify_emirates_id(&req).is_err());
    }

    #[test]
    fn mock_ica_check_status() {
        let adapter = MockIcaAdapter;
        let status = adapter
            .check_emirates_id_status("784-1234-1234567-1")
            .expect("valid");
        assert_eq!(status, EmiratesIdStatus::Active);
    }

    #[test]
    fn mock_ica_adapter_name() {
        let adapter = MockIcaAdapter;
        assert_eq!(IcaAdapter::adapter_name(&adapter), "MockIcaAdapter");
    }

    #[test]
    fn mock_ica_national_system_adapter() {
        let adapter = MockIcaAdapter;
        assert_eq!(NationalSystemAdapter::category(&adapter), AdapterCategory::Identity);
        assert_eq!(NationalSystemAdapter::jurisdiction(&adapter), "ae-abudhabi-adgm");
        assert!(matches!(
            NationalSystemAdapter::health(&adapter),
            AdapterHealth::Healthy
        ));
    }

    #[test]
    fn trait_object_safety() {
        let adapter: Box<dyn IcaAdapter> = Box::new(MockIcaAdapter);
        assert_eq!(adapter.adapter_name(), "MockIcaAdapter");
    }

    #[test]
    fn arc_safety() {
        use std::sync::Arc;
        let adapter: Arc<dyn IcaAdapter> = Arc::new(MockIcaAdapter);
        assert_eq!(adapter.adapter_name(), "MockIcaAdapter");
    }

    #[test]
    fn verification_request_serde_roundtrip() {
        let req = EmiratesIdVerificationRequest {
            emirates_id: "784-1234-1234567-1".to_string(),
            full_name: "Test".to_string(),
            nationality: None,
            date_of_birth: Some("1990-01-01".to_string()),
            request_reference: "REQ-003".to_string(),
        };
        let json = serde_json::to_string(&req).expect("serialize");
        let back: EmiratesIdVerificationRequest =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.emirates_id, "784-1234-1234567-1");
        // nationality was None, should not appear in JSON
        assert!(!json.contains("nationality"));
    }

    #[test]
    fn verification_response_serde_roundtrip() {
        let resp = EmiratesIdVerificationResponse {
            verified: true,
            emirates_id_status: EmiratesIdStatus::Active,
            residency_status: Some("residence".to_string()),
            verification_timestamp: "2026-01-15T12:00:00Z".to_string(),
            reference: "REF-001".to_string(),
        };
        let json = serde_json::to_string(&resp).expect("serialize");
        let back: EmiratesIdVerificationResponse =
            serde_json::from_str(&json).expect("deserialize");
        assert!(back.verified);
        assert_eq!(back.emirates_id_status, EmiratesIdStatus::Active);
    }
}
