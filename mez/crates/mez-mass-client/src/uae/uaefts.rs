//! # CBUAE UAEFTS/IPP Integration Adapter Interface
//!
//! Defines the adapter interface for UAEFTS (UAE Funds Transfer System) and
//! IPP (Instant Payment Platform), the Central Bank of the UAE's payment
//! infrastructure. UAEFTS processes high-value RTGS transfers while IPP
//! enables instant retail payments in AED, operating 24/7.
//!
//! ## Architecture
//!
//! The `UaeftsAdapter` trait abstracts over the CBUAE payment backend.
//! Production deployments implement it against the live CBUAE API; test
//! environments use `MockUaeftsAdapter`. This separation allows the payment
//! rail layer and corridor settlement logic to compose UAEFTS operations
//! without coupling to a specific transport or API version.
//!
//! ## UAE IBANs
//!
//! UAE IBANs follow the format `AE{check}{bank}{account}` where:
//! - `AE` is the country code (ISO 3166)
//! - `{check}` is a 2-digit check number
//! - `{bank}` is a 3-digit bank code
//! - `{account}` is a 16-digit account number
//!
//! Total length: 23 characters. The `validate_uae_iban` helper enforces this format.
//!
//! ## Settlement
//!
//! - **UAEFTS**: RTGS for high-value transfers (AED 500K+), settles within minutes
//! - **IPP**: Instant retail payments (< AED 500K), settles within seconds
//!
//! The adapter automatically selects the appropriate rail based on the transfer amount.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::adapter::{AdapterCategory, AdapterHealth, NationalSystemAdapter};

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// Errors from UAEFTS/IPP integration operations.
#[derive(Debug, thiserror::Error)]
pub enum UaeftsError {
    /// UAEFTS/IPP service is unreachable or returned a 5xx status.
    #[error("UAEFTS/IPP service unavailable: {reason}")]
    ServiceUnavailable {
        /// Human-readable description of the outage or error.
        reason: String,
    },

    /// IBAN format is invalid (not a well-formed UAE IBAN).
    #[error("invalid UAE IBAN: {reason}")]
    InvalidIban {
        /// Description of the validation failure.
        reason: String,
    },

    /// The UAEFTS adapter has not been configured for this deployment.
    #[error("UAEFTS adapter not configured: {reason}")]
    NotConfigured {
        /// Why configuration is missing or incomplete.
        reason: String,
    },

    /// The request to UAEFTS/IPP timed out.
    #[error("UAEFTS/IPP request timed out after {elapsed_ms}ms")]
    Timeout {
        /// Elapsed time in milliseconds before the timeout triggered.
        elapsed_ms: u64,
    },

    /// UAEFTS/IPP rejected the payment instruction.
    #[error("payment rejected by UAEFTS/IPP: {reason}")]
    PaymentRejected {
        /// Description of why the payment was rejected.
        reason: String,
    },

    /// The referenced payment was not found.
    #[error("payment not found: reference {reference}")]
    PaymentNotFound {
        /// The transaction reference that was not found.
        reference: String,
    },

    /// The account could not be verified.
    #[error("account verification failed: {reason}")]
    AccountVerificationFailed {
        /// Description of the verification failure.
        reason: String,
    },
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Payment status as reported by UAEFTS/IPP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UaePaymentStatus {
    /// Payment instruction accepted, awaiting processing.
    Pending,
    /// Payment is being processed.
    Processing,
    /// Payment settled successfully.
    Completed,
    /// Payment failed.
    Failed,
    /// A previously completed payment has been reversed.
    Reversed,
}

impl fmt::Display for UaePaymentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Processing => write!(f, "Processing"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::Reversed => write!(f, "Reversed"),
        }
    }
}

/// Payment rail selected for a given transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UaePaymentRail {
    /// UAEFTS — Real-Time Gross Settlement for high-value transfers.
    Uaefts,
    /// IPP — Instant Payment Platform for retail transfers.
    Ipp,
}

impl fmt::Display for UaePaymentRail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uaefts => write!(f, "UAEFTS"),
            Self::Ipp => write!(f, "IPP"),
        }
    }
}

/// A UAE credit transfer instruction.
///
/// Amounts are in fils (smallest AED unit: 1 AED = 100 fils).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UaePaymentInstruction {
    /// Amount in fils (smallest AED unit). Must be positive.
    pub amount: i64,

    /// Source account IBAN (UAE format, 23 characters).
    pub from_iban: String,

    /// Destination account IBAN (UAE format, 23 characters).
    pub to_iban: String,

    /// Payment reference visible to both parties.
    pub reference: String,

    /// Purpose of payment code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose_code: Option<String>,

    /// Remittance information / description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remittance_info: Option<String>,

    /// Idempotency key to prevent duplicate payment submissions.
    pub idempotency_key: String,
}

/// Result of a UAEFTS/IPP payment initiation or status query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UaePaymentResult {
    /// CBUAE transaction reference.
    pub cbuae_reference: String,

    /// Current status of the payment.
    pub status: UaePaymentStatus,

    /// Which payment rail was used.
    pub rail: UaePaymentRail,

    /// ISO 8601 timestamp of the most recent status change.
    pub timestamp: String,

    /// Fee charged in fils. `None` if fee is not yet known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee: Option<i64>,

    /// Settlement confirmation identifier (populated after `Completed`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settlement_id: Option<String>,
}

/// Account verification result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UaeAccountVerification {
    /// The IBAN that was verified.
    pub iban: String,

    /// Whether the account is active and reachable.
    pub active: bool,

    /// Account holder name as registered with the bank.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_title: Option<String>,

    /// Bank name (from the IBAN bank code).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_name: Option<String>,

    /// ISO 8601 timestamp when the verification was performed.
    pub verification_timestamp: String,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Validate that an IBAN string is a well-formed UAE IBAN.
///
/// UAE IBANs are 23 characters: `AE` + 2 check digits + 3 bank code + 16 account.
pub fn validate_uae_iban(iban: &str) -> Result<String, UaeftsError> {
    let cleaned: String = iban.chars().filter(|c| !c.is_whitespace()).collect();
    let upper = cleaned.to_uppercase();

    if upper.len() != 23 {
        return Err(UaeftsError::InvalidIban {
            reason: format!(
                "UAE IBAN must be exactly 23 characters, got {} from '{}'",
                upper.len(),
                iban
            ),
        });
    }

    if !upper.starts_with("AE") {
        return Err(UaeftsError::InvalidIban {
            reason: format!(
                "UAE IBAN must start with 'AE', got '{}'",
                &upper[..2]
            ),
        });
    }

    // Characters 3-4 must be digits (check digits).
    if !upper[2..4].chars().all(|c| c.is_ascii_digit()) {
        return Err(UaeftsError::InvalidIban {
            reason: "IBAN check digits (positions 3-4) must be numeric".to_string(),
        });
    }

    // Characters 5-7 are the bank code (digits).
    if !upper[4..7].chars().all(|c| c.is_ascii_digit()) {
        return Err(UaeftsError::InvalidIban {
            reason: "IBAN bank code (positions 5-7) must be numeric".to_string(),
        });
    }

    // Characters 8-23 are the account number (digits).
    if !upper[7..].chars().all(|c| c.is_ascii_digit()) {
        return Err(UaeftsError::InvalidIban {
            reason: "IBAN account number (positions 8-23) must be numeric".to_string(),
        });
    }

    Ok(upper)
}

/// IPP threshold in fils — transfers above this use UAEFTS RTGS.
/// AED 500,000 = 50,000,000 fils.
const IPP_THRESHOLD_FILS: i64 = 50_000_000;

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Adapter trait for CBUAE UAEFTS/IPP payment system integration.
///
/// Implementations must be `Send + Sync` so they can be shared across
/// async tasks behind an `Arc`. The trait is object-safe to support
/// runtime adapter selection (mock vs. live).
pub trait UaeftsAdapter: Send + Sync {
    /// Submit a credit transfer instruction to UAEFTS or IPP.
    ///
    /// The rail is selected automatically based on the transfer amount:
    /// amounts below AED 500K use IPP; amounts at or above use UAEFTS.
    fn initiate_payment(
        &self,
        instruction: &UaePaymentInstruction,
    ) -> Result<UaePaymentResult, UaeftsError>;

    /// Query the current status of a previously initiated payment.
    fn check_payment_status(
        &self,
        cbuae_reference: &str,
    ) -> Result<UaePaymentResult, UaeftsError>;

    /// Verify that a UAE IBAN is active and reachable.
    fn verify_account(&self, iban: &str) -> Result<UaeAccountVerification, UaeftsError>;

    /// Return the human-readable name of this adapter implementation.
    fn adapter_name(&self) -> &str;
}

// ---------------------------------------------------------------------------
// Mock adapter
// ---------------------------------------------------------------------------

/// Mock UAEFTS/IPP adapter for testing and development.
///
/// Returns deterministic test data based on IBAN conventions:
/// - IBANs ending in "0000" are treated as inactive accounts
/// - IBANs ending in "9999" trigger payment rejection (insufficient funds)
/// - All other valid IBANs succeed with instant settlement
///
/// Mock fee: AED 1.00 (100 fils) for IPP, AED 25.00 (2500 fils) for UAEFTS.
#[derive(Debug, Clone)]
pub struct MockUaeftsAdapter;

impl MockUaeftsAdapter {
    /// Fixed mock fee for IPP: AED 1.00 = 100 fils.
    const MOCK_FEE_IPP: i64 = 100;

    /// Fixed mock fee for UAEFTS: AED 25.00 = 2500 fils.
    const MOCK_FEE_UAEFTS: i64 = 2500;

    fn is_inactive_account(iban: &str) -> bool {
        iban.ends_with("0000")
    }

    fn should_reject(iban: &str) -> bool {
        iban.ends_with("9999")
    }

    fn select_rail(amount: i64) -> UaePaymentRail {
        if amount >= IPP_THRESHOLD_FILS {
            UaePaymentRail::Uaefts
        } else {
            UaePaymentRail::Ipp
        }
    }

    fn fee_for_rail(rail: UaePaymentRail) -> i64 {
        match rail {
            UaePaymentRail::Ipp => Self::MOCK_FEE_IPP,
            UaePaymentRail::Uaefts => Self::MOCK_FEE_UAEFTS,
        }
    }
}

impl UaeftsAdapter for MockUaeftsAdapter {
    fn initiate_payment(
        &self,
        instruction: &UaePaymentInstruction,
    ) -> Result<UaePaymentResult, UaeftsError> {
        let from = validate_uae_iban(&instruction.from_iban)?;
        let _to = validate_uae_iban(&instruction.to_iban)?;

        if instruction.amount <= 0 {
            return Err(UaeftsError::PaymentRejected {
                reason: "amount must be positive".to_string(),
            });
        }

        if instruction.idempotency_key.is_empty() {
            return Err(UaeftsError::PaymentRejected {
                reason: "idempotency_key must not be empty".to_string(),
            });
        }

        if Self::should_reject(&from) {
            return Err(UaeftsError::PaymentRejected {
                reason: "insufficient funds (mock: IBAN ends in 9999)".to_string(),
            });
        }

        let rail = Self::select_rail(instruction.amount);
        Ok(UaePaymentResult {
            cbuae_reference: format!("UAEFTS-MOCK-{}", &instruction.idempotency_key),
            status: UaePaymentStatus::Completed,
            rail,
            timestamp: "2026-02-20T12:00:00Z".to_string(),
            fee: Some(Self::fee_for_rail(rail)),
            settlement_id: Some(format!("STL-{}", &instruction.idempotency_key)),
        })
    }

    fn check_payment_status(
        &self,
        cbuae_reference: &str,
    ) -> Result<UaePaymentResult, UaeftsError> {
        if cbuae_reference.is_empty() {
            return Err(UaeftsError::PaymentNotFound {
                reference: cbuae_reference.to_string(),
            });
        }

        Ok(UaePaymentResult {
            cbuae_reference: cbuae_reference.to_string(),
            status: UaePaymentStatus::Completed,
            rail: UaePaymentRail::Ipp,
            timestamp: "2026-02-20T12:00:01Z".to_string(),
            fee: Some(Self::MOCK_FEE_IPP),
            settlement_id: Some(format!("STL-{cbuae_reference}")),
        })
    }

    fn verify_account(&self, iban: &str) -> Result<UaeAccountVerification, UaeftsError> {
        let validated = validate_uae_iban(iban)?;

        if Self::is_inactive_account(&validated) {
            return Ok(UaeAccountVerification {
                iban: validated,
                active: false,
                account_title: None,
                bank_name: Some("Mock Bank".to_string()),
                verification_timestamp: "2026-02-20T12:00:00Z".to_string(),
            });
        }

        let bank_code = validated[4..7].to_string();
        Ok(UaeAccountVerification {
            iban: validated,
            active: true,
            account_title: Some("Mock Account Holder".to_string()),
            bank_name: Some(format!("Bank {bank_code}")),
            verification_timestamp: "2026-02-20T12:00:00Z".to_string(),
        })
    }

    fn adapter_name(&self) -> &str {
        "MockUaeftsAdapter"
    }
}

impl NationalSystemAdapter for MockUaeftsAdapter {
    fn category(&self) -> AdapterCategory {
        AdapterCategory::Payments
    }

    fn jurisdiction(&self) -> &str {
        "ae"
    }

    fn health(&self) -> AdapterHealth {
        AdapterHealth::Healthy
    }

    fn adapter_name(&self) -> &str {
        "MockUaeftsAdapter"
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- validate_uae_iban -------------------------------------------------------

    #[test]
    fn validate_uae_iban_accepts_valid() {
        let result = validate_uae_iban("AE070331234567890123456");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "AE070331234567890123456");
    }

    #[test]
    fn validate_uae_iban_accepts_lowercase() {
        let result = validate_uae_iban("ae070331234567890123456");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "AE070331234567890123456");
    }

    #[test]
    fn validate_uae_iban_rejects_wrong_country() {
        let result = validate_uae_iban("PK070331234567890123456");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), UaeftsError::InvalidIban { .. }));
    }

    #[test]
    fn validate_uae_iban_rejects_too_short() {
        let result = validate_uae_iban("AE07033123456789012");
        assert!(result.is_err());
    }

    #[test]
    fn validate_uae_iban_rejects_too_long() {
        let result = validate_uae_iban("AE07033123456789012345600");
        assert!(result.is_err());
    }

    #[test]
    fn validate_uae_iban_rejects_empty() {
        let result = validate_uae_iban("");
        assert!(result.is_err());
    }

    // -- UaePaymentStatus Display ------------------------------------------------

    #[test]
    fn payment_status_display() {
        assert_eq!(UaePaymentStatus::Pending.to_string(), "Pending");
        assert_eq!(UaePaymentStatus::Completed.to_string(), "Completed");
        assert_eq!(UaePaymentStatus::Failed.to_string(), "Failed");
    }

    #[test]
    fn payment_status_serde_roundtrip() {
        for status in [
            UaePaymentStatus::Pending,
            UaePaymentStatus::Processing,
            UaePaymentStatus::Completed,
            UaePaymentStatus::Failed,
            UaePaymentStatus::Reversed,
        ] {
            let json = serde_json::to_string(&status).expect("serialize");
            let back: UaePaymentStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(status, back);
        }
    }

    // -- UaePaymentRail Display --------------------------------------------------

    #[test]
    fn payment_rail_display() {
        assert_eq!(UaePaymentRail::Uaefts.to_string(), "UAEFTS");
        assert_eq!(UaePaymentRail::Ipp.to_string(), "IPP");
    }

    // -- Error Display -----------------------------------------------------------

    #[test]
    fn error_display_messages() {
        let err = UaeftsError::ServiceUnavailable { reason: "connection refused".into() };
        assert!(err.to_string().contains("connection refused"));

        let err = UaeftsError::InvalidIban { reason: "too short".into() };
        assert!(err.to_string().contains("too short"));

        let err = UaeftsError::Timeout { elapsed_ms: 5000 };
        assert!(err.to_string().contains("5000"));

        let err = UaeftsError::PaymentRejected { reason: "insufficient funds".into() };
        assert!(err.to_string().contains("insufficient funds"));

        let err = UaeftsError::PaymentNotFound { reference: "REF-999".into() };
        assert!(err.to_string().contains("REF-999"));
    }

    // -- MockUaeftsAdapter: initiate_payment -------------------------------------

    #[test]
    fn mock_adapter_initiates_ipp_payment() {
        let adapter = MockUaeftsAdapter;
        let instr = UaePaymentInstruction {
            amount: 100_000, // AED 1,000 — below IPP threshold
            from_iban: "AE070331234567890123456".into(),
            to_iban: "AE070331234567890654321".into(),
            reference: "MEZ-TEST-001".into(),
            purpose_code: Some("SUPP".into()),
            remittance_info: None,
            idempotency_key: "IDEM-001".into(),
        };
        let result = adapter.initiate_payment(&instr).expect("should initiate payment");
        assert_eq!(result.status, UaePaymentStatus::Completed);
        assert_eq!(result.rail, UaePaymentRail::Ipp);
        assert!(result.cbuae_reference.starts_with("UAEFTS-MOCK-"));
        assert_eq!(result.fee, Some(MockUaeftsAdapter::MOCK_FEE_IPP));
        assert!(result.settlement_id.is_some());
    }

    #[test]
    fn mock_adapter_initiates_uaefts_payment() {
        let adapter = MockUaeftsAdapter;
        let instr = UaePaymentInstruction {
            amount: 50_000_000, // AED 500,000 — at UAEFTS threshold
            from_iban: "AE070331234567890123456".into(),
            to_iban: "AE070331234567890654321".into(),
            reference: "MEZ-TEST-002".into(),
            purpose_code: None,
            remittance_info: None,
            idempotency_key: "IDEM-002".into(),
        };
        let result = adapter.initiate_payment(&instr).expect("should initiate UAEFTS");
        assert_eq!(result.rail, UaePaymentRail::Uaefts);
        assert_eq!(result.fee, Some(MockUaeftsAdapter::MOCK_FEE_UAEFTS));
    }

    #[test]
    fn mock_adapter_rejects_invalid_iban() {
        let adapter = MockUaeftsAdapter;
        let instr = UaePaymentInstruction {
            amount: 100_000,
            from_iban: "INVALID".into(),
            to_iban: "AE070331234567890654321".into(),
            reference: "REF".into(),
            purpose_code: None,
            remittance_info: None,
            idempotency_key: "IK1".into(),
        };
        let result = adapter.initiate_payment(&instr);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), UaeftsError::InvalidIban { .. }));
    }

    #[test]
    fn mock_adapter_rejects_zero_amount() {
        let adapter = MockUaeftsAdapter;
        let instr = UaePaymentInstruction {
            amount: 0,
            from_iban: "AE070331234567890123456".into(),
            to_iban: "AE070331234567890654321".into(),
            reference: "REF".into(),
            purpose_code: None,
            remittance_info: None,
            idempotency_key: "IK1".into(),
        };
        let result = adapter.initiate_payment(&instr);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), UaeftsError::PaymentRejected { .. }));
    }

    #[test]
    fn mock_adapter_rejects_insufficient_funds() {
        let adapter = MockUaeftsAdapter;
        let instr = UaePaymentInstruction {
            amount: 100_000,
            from_iban: "AE070331234567890129999".into(),
            to_iban: "AE070331234567890654321".into(),
            reference: "REF".into(),
            purpose_code: None,
            remittance_info: None,
            idempotency_key: "IK1".into(),
        };
        let result = adapter.initiate_payment(&instr);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), UaeftsError::PaymentRejected { .. }));
    }

    // -- MockUaeftsAdapter: check_payment_status ---------------------------------

    #[test]
    fn mock_adapter_check_status_valid() {
        let adapter = MockUaeftsAdapter;
        let result = adapter.check_payment_status("UAEFTS-MOCK-001").expect("should return status");
        assert_eq!(result.status, UaePaymentStatus::Completed);
        assert_eq!(result.cbuae_reference, "UAEFTS-MOCK-001");
    }

    #[test]
    fn mock_adapter_check_status_empty_ref() {
        let adapter = MockUaeftsAdapter;
        let result = adapter.check_payment_status("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), UaeftsError::PaymentNotFound { .. }));
    }

    // -- MockUaeftsAdapter: verify_account ---------------------------------------

    #[test]
    fn mock_adapter_verify_active_account() {
        let adapter = MockUaeftsAdapter;
        let result = adapter.verify_account("AE070331234567890123456").expect("should verify");
        assert!(result.active);
        assert_eq!(result.iban, "AE070331234567890123456");
        assert!(result.account_title.is_some());
    }

    #[test]
    fn mock_adapter_verify_inactive_account() {
        let adapter = MockUaeftsAdapter;
        let result = adapter.verify_account("AE070331234567890120000").expect("should verify");
        assert!(!result.active);
        assert!(result.account_title.is_none());
    }

    // -- Trait properties --------------------------------------------------------

    #[test]
    fn mock_adapter_name() {
        let adapter = MockUaeftsAdapter;
        assert_eq!(UaeftsAdapter::adapter_name(&adapter), "MockUaeftsAdapter");
    }

    #[test]
    fn adapter_trait_is_object_safe() {
        let adapter: Box<dyn UaeftsAdapter> = Box::new(MockUaeftsAdapter);
        assert_eq!(adapter.adapter_name(), "MockUaeftsAdapter");
    }

    #[test]
    fn national_system_adapter_impl() {
        let adapter = MockUaeftsAdapter;
        assert_eq!(NationalSystemAdapter::category(&adapter), AdapterCategory::Payments);
        assert_eq!(NationalSystemAdapter::jurisdiction(&adapter), "ae");
        assert!(matches!(NationalSystemAdapter::health(&adapter), AdapterHealth::Healthy));
    }

    // -- Serde round-trips -------------------------------------------------------

    #[test]
    fn payment_instruction_serde_roundtrip() {
        let instr = UaePaymentInstruction {
            amount: 500_000,
            from_iban: "AE070331234567890123456".into(),
            to_iban: "AE070331234567890654321".into(),
            reference: "MEZ-CORR-001".into(),
            purpose_code: Some("SUPP".into()),
            remittance_info: Some("Invoice #12345".into()),
            idempotency_key: "IDEM-001".into(),
        };
        let json = serde_json::to_string(&instr).expect("serialize");
        let back: UaePaymentInstruction = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.amount, 500_000);
        assert_eq!(back.from_iban, "AE070331234567890123456");
    }

    #[test]
    fn payment_result_serde_roundtrip() {
        let result = UaePaymentResult {
            cbuae_reference: "UAEFTS-001".into(),
            status: UaePaymentStatus::Completed,
            rail: UaePaymentRail::Ipp,
            timestamp: "2026-02-20T12:00:00Z".into(),
            fee: Some(100),
            settlement_id: Some("STL-001".into()),
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let back: UaePaymentResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.cbuae_reference, "UAEFTS-001");
        assert_eq!(back.rail, UaePaymentRail::Ipp);
    }
}
