//! # NADRA Integration Adapter Interface (M-011)
//!
//! Defines the adapter interface for NADRA (National Database and Registration
//! Authority) integration. CNIC verification against NADRA is a legal
//! requirement for KYC in Pakistan's GovOS deployment (P1-005).
//!
//! ## Architecture
//!
//! The `NadraAdapter` trait abstracts over the NADRA verification backend.
//! Production deployments implement it against the live NADRA API; test
//! environments use `MockNadraAdapter`. This separation allows the
//! `IdentityClient` aggregation facade to compose NADRA verification without
//! coupling to a specific transport or API version.
//!
//! ## CNIC Validation
//!
//! Pakistan CNICs are 13-digit numbers (format: `XXXXX-XXXXXXX-X`). The
//! `validate_cnic` helper strips dashes and verifies the digit count before
//! any request reaches the adapter.

use serde::{Deserialize, Serialize};
use std::fmt;

/// CNIC status as reported by NADRA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CnicStatus {
    /// CNIC is active and valid.
    Active,
    /// CNIC has expired and must be renewed.
    Expired,
    /// CNIC has been blocked by NADRA (fraud, court order, etc.).
    Blocked,
    /// CNIC number does not exist in NADRA records.
    NotFound,
}

impl fmt::Display for CnicStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "Active"),
            Self::Expired => write!(f, "Expired"),
            Self::Blocked => write!(f, "Blocked"),
            Self::NotFound => write!(f, "NotFound"),
        }
    }
}

/// Errors from NADRA integration operations.
#[derive(Debug, thiserror::Error)]
pub enum NadraError {
    /// NADRA service is unreachable or returned a 5xx status.
    #[error("NADRA service unavailable: {reason}")]
    ServiceUnavailable {
        /// Human-readable description of the outage or error.
        reason: String,
    },

    /// CNIC format is invalid (not exactly 13 digits after stripping dashes).
    #[error("invalid CNIC: {reason}")]
    InvalidCnic {
        /// Description of the validation failure.
        reason: String,
    },

    /// NADRA accepted the request but verification could not be completed.
    #[error("verification failed: {reason}")]
    VerificationFailed {
        /// Description of the failure (e.g. data mismatch, internal error).
        reason: String,
    },

    /// The NADRA adapter has not been configured for this deployment.
    #[error("NADRA adapter not configured: {reason}")]
    NotConfigured {
        /// Why configuration is missing or incomplete.
        reason: String,
    },

    /// The request to NADRA timed out.
    #[error("NADRA request timed out after {elapsed_ms}ms")]
    Timeout {
        /// Elapsed time in milliseconds before the timeout triggered.
        elapsed_ms: u64,
    },
}

/// CNIC verification request sent to NADRA.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NadraVerificationRequest {
    /// CNIC number (13 digits, with or without dashes).
    pub cnic: String,
    /// Full name of the individual for cross-reference.
    pub full_name: String,
    /// Father's name for additional verification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub father_name: Option<String>,
    /// Date of birth in ISO 8601 format (YYYY-MM-DD) for additional verification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_of_birth: Option<String>,
    /// Idempotency key for this request. Callers must generate a unique value
    /// per logical verification attempt to prevent duplicate charges.
    pub request_reference: String,
}

/// CNIC verification response from NADRA.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NadraVerificationResponse {
    /// Whether the identity was successfully verified.
    pub verified: bool,
    /// Confidence score from NADRA's matching algorithm (0.0 = no match,
    /// 1.0 = exact match). `None` if NADRA does not return a score.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_score: Option<f64>,
    /// Current status of the CNIC in NADRA records.
    pub cnic_status: CnicStatus,
    /// ISO 8601 timestamp when NADRA performed the verification.
    pub verification_timestamp: String,
    /// Reference identifier linking back to the original request.
    pub reference: String,
}

/// Adapter trait for NADRA identity verification.
///
/// Implementations must be `Send + Sync` so they can be shared across
/// async tasks behind an `Arc`. The trait is object-safe to support
/// runtime adapter selection (mock vs. live).
pub trait NadraAdapter: Send + Sync {
    /// Verify an individual's identity against NADRA records using their
    /// CNIC number, name, and optional biographical details.
    fn verify_identity(
        &self,
        request: &NadraVerificationRequest,
    ) -> Result<NadraVerificationResponse, NadraError>;

    /// Check the current status of a CNIC number without performing a full
    /// identity verification. Useful for pre-flight checks before initiating
    /// costly verification workflows.
    fn check_cnic_status(&self, cnic: &str) -> Result<CnicStatus, NadraError>;

    /// Return the human-readable name of this adapter implementation
    /// (e.g. "MockNadraAdapter", "NadraLiveApiV2").
    fn adapter_name(&self) -> &str;
}

/// Validate that a CNIC string contains exactly 13 digits after stripping
/// dashes. Returns the canonical 13-digit form on success.
pub fn validate_cnic(cnic: &str) -> Result<String, NadraError> {
    let digits: String = cnic.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 13 {
        return Err(NadraError::InvalidCnic {
            reason: format!(
                "CNIC must be exactly 13 digits, got {} digits from input '{}'",
                digits.len(),
                cnic
            ),
        });
    }
    Ok(digits)
}

/// Mock NADRA adapter for testing and development.
///
/// Returns successful verification for any well-formed 13-digit CNIC.
/// Uses a fixed match score of 0.95 and `CnicStatus::Active`.
#[derive(Debug, Clone)]
pub struct MockNadraAdapter;

impl NadraAdapter for MockNadraAdapter {
    fn verify_identity(
        &self,
        request: &NadraVerificationRequest,
    ) -> Result<NadraVerificationResponse, NadraError> {
        let _canonical_cnic = validate_cnic(&request.cnic)?;

        Ok(NadraVerificationResponse {
            verified: true,
            match_score: Some(0.95),
            cnic_status: CnicStatus::Active,
            verification_timestamp: chrono::Utc::now().to_rfc3339(),
            reference: format!("MOCK-{}", request.request_reference),
        })
    }

    fn check_cnic_status(&self, cnic: &str) -> Result<CnicStatus, NadraError> {
        let _canonical = validate_cnic(cnic)?;
        Ok(CnicStatus::Active)
    }

    fn adapter_name(&self) -> &str {
        "MockNadraAdapter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- validate_cnic -----------------------------------------------------------

    #[test]
    fn validate_cnic_accepts_13_digits() {
        let result = validate_cnic("1234567890123");
        assert!(result.is_ok());
        assert_eq!(result.expect("valid CNIC"), "1234567890123");
    }

    #[test]
    fn validate_cnic_strips_dashes() {
        let result = validate_cnic("12345-6789012-3");
        assert!(result.is_ok());
        assert_eq!(result.expect("valid dashed CNIC"), "1234567890123");
    }

    #[test]
    fn validate_cnic_rejects_too_few_digits() {
        let result = validate_cnic("12345");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, NadraError::InvalidCnic { .. }));
    }

    #[test]
    fn validate_cnic_rejects_too_many_digits() {
        let result = validate_cnic("12345678901234");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, NadraError::InvalidCnic { .. }));
    }

    #[test]
    fn validate_cnic_rejects_empty_string() {
        let result = validate_cnic("");
        assert!(result.is_err());
    }

    #[test]
    fn validate_cnic_rejects_non_digit_input() {
        let result = validate_cnic("abcdefghijklm");
        assert!(result.is_err());
    }

    // -- CnicStatus Display ------------------------------------------------------

    #[test]
    fn cnic_status_display() {
        assert_eq!(CnicStatus::Active.to_string(), "Active");
        assert_eq!(CnicStatus::Expired.to_string(), "Expired");
        assert_eq!(CnicStatus::Blocked.to_string(), "Blocked");
        assert_eq!(CnicStatus::NotFound.to_string(), "NotFound");
    }

    // -- CnicStatus serde round-trip ---------------------------------------------

    #[test]
    fn cnic_status_serde_round_trip() {
        for status in [
            CnicStatus::Active,
            CnicStatus::Expired,
            CnicStatus::Blocked,
            CnicStatus::NotFound,
        ] {
            let json = serde_json::to_string(&status).expect("serialize CnicStatus");
            let back: CnicStatus = serde_json::from_str(&json).expect("deserialize CnicStatus");
            assert_eq!(status, back);
        }
    }

    // -- NadraError Display ------------------------------------------------------

    #[test]
    fn nadra_error_display_messages() {
        let err = NadraError::ServiceUnavailable {
            reason: "connection refused".into(),
        };
        assert!(err.to_string().contains("connection refused"));

        let err = NadraError::InvalidCnic {
            reason: "too short".into(),
        };
        assert!(err.to_string().contains("too short"));

        let err = NadraError::VerificationFailed {
            reason: "name mismatch".into(),
        };
        assert!(err.to_string().contains("name mismatch"));

        let err = NadraError::NotConfigured {
            reason: "missing API key".into(),
        };
        assert!(err.to_string().contains("missing API key"));

        let err = NadraError::Timeout { elapsed_ms: 5000 };
        assert!(err.to_string().contains("5000"));
    }

    // -- NadraVerificationRequest serde ------------------------------------------

    #[test]
    fn verification_request_serde_round_trip() {
        let req = NadraVerificationRequest {
            cnic: "1234567890123".into(),
            full_name: "Ali Khan".into(),
            father_name: Some("Ahmed Khan".into()),
            date_of_birth: Some("1990-06-15".into()),
            request_reference: "REQ-001".into(),
        };
        let json = serde_json::to_string(&req).expect("serialize request");
        let back: NadraVerificationRequest =
            serde_json::from_str(&json).expect("deserialize request");
        assert_eq!(back.cnic, "1234567890123");
        assert_eq!(back.full_name, "Ali Khan");
        assert_eq!(back.father_name.as_deref(), Some("Ahmed Khan"));
        assert_eq!(back.date_of_birth.as_deref(), Some("1990-06-15"));
        assert_eq!(back.request_reference, "REQ-001");
    }

    #[test]
    fn verification_request_optional_fields_absent() {
        let json = r#"{"cnic":"1234567890123","full_name":"Ali","request_reference":"R1"}"#;
        let req: NadraVerificationRequest =
            serde_json::from_str(json).expect("deserialize minimal request");
        assert!(req.father_name.is_none());
        assert!(req.date_of_birth.is_none());
    }

    #[test]
    fn verification_request_skips_none_on_serialize() {
        let req = NadraVerificationRequest {
            cnic: "1234567890123".into(),
            full_name: "Ali".into(),
            father_name: None,
            date_of_birth: None,
            request_reference: "R1".into(),
        };
        let json = serde_json::to_string(&req).expect("serialize");
        assert!(!json.contains("father_name"));
        assert!(!json.contains("date_of_birth"));
    }

    // -- NadraVerificationResponse serde -----------------------------------------

    #[test]
    fn verification_response_serde_round_trip() {
        let resp = NadraVerificationResponse {
            verified: true,
            match_score: Some(0.95),
            cnic_status: CnicStatus::Active,
            verification_timestamp: "2026-02-15T12:00:00Z".into(),
            reference: "MOCK-REQ-001".into(),
        };
        let json = serde_json::to_string(&resp).expect("serialize response");
        let back: NadraVerificationResponse =
            serde_json::from_str(&json).expect("deserialize response");
        assert!(back.verified);
        assert_eq!(back.match_score, Some(0.95));
        assert_eq!(back.cnic_status, CnicStatus::Active);
        assert_eq!(back.verification_timestamp, "2026-02-15T12:00:00Z");
        assert_eq!(back.reference, "MOCK-REQ-001");
    }

    #[test]
    fn verification_response_match_score_absent() {
        let json = r#"{"verified":false,"match_score":null,"cnic_status":"NotFound","verification_timestamp":"2026-02-15T00:00:00Z","reference":"R1"}"#;
        let resp: NadraVerificationResponse =
            serde_json::from_str(json).expect("deserialize with null score");
        assert!(!resp.verified);
        assert!(resp.match_score.is_none());
        assert_eq!(resp.cnic_status, CnicStatus::NotFound);
    }

    // -- MockNadraAdapter --------------------------------------------------------

    #[test]
    fn mock_adapter_verifies_valid_cnic() {
        let adapter = MockNadraAdapter;
        let req = NadraVerificationRequest {
            cnic: "1234567890123".into(),
            full_name: "Ali Khan".into(),
            father_name: Some("Ahmed Khan".into()),
            date_of_birth: Some("1990-06-15".into()),
            request_reference: "REQ-001".into(),
        };
        let resp = adapter.verify_identity(&req).expect("should verify valid CNIC");
        assert!(resp.verified);
        assert_eq!(resp.match_score, Some(0.95));
        assert_eq!(resp.cnic_status, CnicStatus::Active);
        assert!(resp.reference.starts_with("MOCK-"));
        assert!(resp.reference.contains("REQ-001"));
        assert!(!resp.verification_timestamp.is_empty());
    }

    #[test]
    fn mock_adapter_rejects_invalid_cnic() {
        let adapter = MockNadraAdapter;
        let req = NadraVerificationRequest {
            cnic: "12345".into(),
            full_name: "Ali".into(),
            father_name: None,
            date_of_birth: None,
            request_reference: "REQ-002".into(),
        };
        let result = adapter.verify_identity(&req);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            NadraError::InvalidCnic { .. }
        ));
    }

    #[test]
    fn mock_adapter_accepts_dashed_cnic() {
        let adapter = MockNadraAdapter;
        let req = NadraVerificationRequest {
            cnic: "12345-6789012-3".into(),
            full_name: "Fatima Bibi".into(),
            father_name: None,
            date_of_birth: None,
            request_reference: "REQ-003".into(),
        };
        let resp = adapter.verify_identity(&req).expect("dashed CNIC should pass");
        assert!(resp.verified);
    }

    #[test]
    fn mock_adapter_check_cnic_status_valid() {
        let adapter = MockNadraAdapter;
        let status = adapter
            .check_cnic_status("1234567890123")
            .expect("status check should succeed");
        assert_eq!(status, CnicStatus::Active);
    }

    #[test]
    fn mock_adapter_check_cnic_status_dashed() {
        let adapter = MockNadraAdapter;
        let status = adapter
            .check_cnic_status("12345-6789012-3")
            .expect("dashed CNIC status check should succeed");
        assert_eq!(status, CnicStatus::Active);
    }

    #[test]
    fn mock_adapter_check_cnic_status_rejects_invalid() {
        let adapter = MockNadraAdapter;
        let result = adapter.check_cnic_status("999");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            NadraError::InvalidCnic { .. }
        ));
    }

    #[test]
    fn mock_adapter_name() {
        let adapter = MockNadraAdapter;
        assert_eq!(adapter.adapter_name(), "MockNadraAdapter");
    }

    #[test]
    fn adapter_trait_is_object_safe() {
        let adapter: Box<dyn NadraAdapter> = Box::new(MockNadraAdapter);
        assert_eq!(adapter.adapter_name(), "MockNadraAdapter");
        let status = adapter
            .check_cnic_status("1234567890123")
            .expect("trait object status check");
        assert_eq!(status, CnicStatus::Active);
    }

    #[test]
    fn adapter_trait_behind_arc() {
        let adapter: std::sync::Arc<dyn NadraAdapter> = std::sync::Arc::new(MockNadraAdapter);
        let req = NadraVerificationRequest {
            cnic: "1234567890123".into(),
            full_name: "Arc Test".into(),
            father_name: None,
            date_of_birth: None,
            request_reference: "ARC-001".into(),
        };
        let resp = adapter.verify_identity(&req).expect("Arc adapter should work");
        assert!(resp.verified);
    }

    #[test]
    fn match_score_boundary_values() {
        // Verify that 0.0 and 1.0 round-trip correctly through serde.
        for score in [0.0_f64, 0.5, 1.0] {
            let resp = NadraVerificationResponse {
                verified: score >= 0.5,
                match_score: Some(score),
                cnic_status: CnicStatus::Active,
                verification_timestamp: "2026-01-01T00:00:00Z".into(),
                reference: "BOUNDARY".into(),
            };
            let json = serde_json::to_string(&resp).expect("serialize");
            let back: NadraVerificationResponse =
                serde_json::from_str(&json).expect("deserialize");
            let diff = (back.match_score.expect("score present") - score).abs();
            assert!(diff < f64::EPSILON, "score {score} did not round-trip");
        }
    }
}
