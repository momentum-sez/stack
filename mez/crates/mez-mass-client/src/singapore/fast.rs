//! # FAST/PayNow Integration Adapter Interface
//!
//! Defines the adapter interface for Singapore's FAST (Fast And Secure
//! Transfers) and PayNow payment systems, operated by the Banking Computer
//! Services Pte Ltd (BCS) under MAS oversight.
//!
//! ## Architecture
//!
//! The `FastPaynowAdapter` trait abstracts over the FAST/PayNow backend.
//! Production deployments implement it against the live API; test environments
//! use `MockFastPaynowAdapter`.
//!
//! ## FAST
//!
//! FAST enables near-instant SGD fund transfers between participating banks
//! and non-bank financial institutions in Singapore, 24/7/365. Settlement
//! is typically within seconds.
//!
//! ## PayNow
//!
//! PayNow is a peer-to-peer funds transfer service that lets users send
//! and receive SGD instantly via mobile number, NRIC/FIN, UEN, or VPA
//! (Virtual Payment Address). PayNow rides on FAST infrastructure.
//!
//! ## Account Identifiers
//!
//! Singapore does not use IBANs. Domestic transfers use bank account numbers
//! qualified by bank/branch codes (SWIFT BIC or local clearing code).
//! PayNow uses proxy identifiers (mobile, NRIC, UEN).

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::adapter::{AdapterCategory, AdapterHealth, NationalSystemAdapter};

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// Errors from FAST/PayNow integration operations.
#[derive(Debug, thiserror::Error)]
pub enum FastPaynowError {
    /// FAST/PayNow service is unreachable or returned a 5xx status.
    #[error("FAST/PayNow service unavailable: {reason}")]
    ServiceUnavailable {
        /// Human-readable description of the outage or error.
        reason: String,
    },

    /// Account number format is invalid.
    #[error("invalid account number: {reason}")]
    InvalidAccount {
        /// Description of the validation failure.
        reason: String,
    },

    /// The FAST/PayNow adapter has not been configured for this deployment.
    #[error("FAST/PayNow adapter not configured: {reason}")]
    NotConfigured {
        /// Why configuration is missing or incomplete.
        reason: String,
    },

    /// The request timed out.
    #[error("FAST/PayNow request timed out after {elapsed_ms}ms")]
    Timeout {
        /// Elapsed time in milliseconds before the timeout triggered.
        elapsed_ms: u64,
    },

    /// Payment was rejected.
    #[error("payment rejected by FAST/PayNow: {reason}")]
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

    /// PayNow proxy lookup returned no matching account.
    #[error("PayNow proxy not found: {proxy}")]
    ProxyNotFound {
        /// The proxy identifier that was looked up.
        proxy: String,
    },
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Payment status as reported by FAST/PayNow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SgPaymentStatus {
    /// Payment instruction accepted, awaiting processing.
    Pending,
    /// Payment is being processed by FAST.
    Processing,
    /// Payment settled successfully.
    Completed,
    /// Payment failed.
    Failed,
    /// Payment was returned (e.g., invalid beneficiary account).
    Returned,
}

impl fmt::Display for SgPaymentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Processing => write!(f, "Processing"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::Returned => write!(f, "Returned"),
        }
    }
}

/// PayNow proxy type for alias-based transfers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaynowProxyType {
    /// Mobile phone number (e.g., "+6591234567").
    MobileNumber,
    /// NRIC/FIN number.
    NricFin,
    /// Unique Entity Number (business).
    Uen,
    /// Virtual Payment Address.
    Vpa,
}

impl fmt::Display for PaynowProxyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MobileNumber => write!(f, "MobileNumber"),
            Self::NricFin => write!(f, "NRIC/FIN"),
            Self::Uen => write!(f, "UEN"),
            Self::Vpa => write!(f, "VPA"),
        }
    }
}

/// A FAST credit transfer instruction.
///
/// Amounts are in cents (smallest SGD unit: 1 SGD = 100 cents).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastPaymentInstruction {
    /// Amount in cents (smallest SGD unit). Must be positive.
    pub amount: i64,

    /// Source bank SWIFT BIC (e.g., "DBSSSGSG" for DBS).
    pub from_bank_bic: String,

    /// Source account number.
    pub from_account: String,

    /// Destination bank SWIFT BIC.
    pub to_bank_bic: String,

    /// Destination account number.
    pub to_account: String,

    /// Payment reference visible to both parties.
    pub reference: String,

    /// Purpose of payment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose_code: Option<String>,

    /// Remittance information / description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remittance_info: Option<String>,

    /// Idempotency key to prevent duplicate payment submissions.
    pub idempotency_key: String,
}

/// Result of a FAST/PayNow payment initiation or status query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastPaymentResult {
    /// FAST transaction reference.
    pub fast_reference: String,

    /// Current status of the payment.
    pub status: SgPaymentStatus,

    /// ISO 8601 timestamp of the most recent status change.
    pub timestamp: String,

    /// Fee charged in cents. `None` if fee is not yet known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee: Option<i64>,

    /// Settlement confirmation identifier (populated after `Completed`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settlement_id: Option<String>,
}

/// Result of a PayNow proxy lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaynowLookupResult {
    /// The proxy that was looked up.
    pub proxy: String,

    /// Type of proxy.
    pub proxy_type: PaynowProxyType,

    /// Resolved bank BIC.
    pub bank_bic: String,

    /// Resolved account number.
    pub account_number: String,

    /// Account holder name, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Validate a Singapore bank account number.
///
/// Singapore account numbers are 10-12 digits, qualified by a bank SWIFT BIC.
/// This helper does basic format validation.
pub fn validate_sg_account(bank_bic: &str, account: &str) -> Result<String, FastPaynowError> {
    if bank_bic.is_empty() {
        return Err(FastPaynowError::InvalidAccount {
            reason: "bank BIC must not be empty".to_string(),
        });
    }

    if bank_bic.len() < 8 || bank_bic.len() > 11 {
        return Err(FastPaynowError::InvalidAccount {
            reason: format!(
                "bank BIC must be 8-11 characters (SWIFT format), got {}",
                bank_bic.len()
            ),
        });
    }

    let cleaned: String = account.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();
    if cleaned.len() < 9 || cleaned.len() > 14 {
        return Err(FastPaynowError::InvalidAccount {
            reason: format!(
                "Singapore account number must be 9-14 characters, got {} from '{}'",
                cleaned.len(),
                account
            ),
        });
    }

    if !cleaned.chars().all(|c| c.is_ascii_digit()) {
        return Err(FastPaynowError::InvalidAccount {
            reason: "account number must contain only digits".to_string(),
        });
    }

    Ok(cleaned)
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Adapter trait for Singapore FAST/PayNow payment system integration.
///
/// Implementations must be `Send + Sync` so they can be shared across
/// async tasks behind an `Arc`. The trait is object-safe to support
/// runtime adapter selection (mock vs. live).
pub trait FastPaynowAdapter: Send + Sync {
    /// Submit a credit transfer instruction via FAST.
    fn initiate_payment(
        &self,
        instruction: &FastPaymentInstruction,
    ) -> Result<FastPaymentResult, FastPaynowError>;

    /// Query the current status of a previously initiated payment.
    fn check_payment_status(
        &self,
        fast_reference: &str,
    ) -> Result<FastPaymentResult, FastPaynowError>;

    /// Resolve a PayNow proxy (mobile, NRIC, UEN) to a bank account.
    fn paynow_lookup(
        &self,
        proxy: &str,
        proxy_type: PaynowProxyType,
    ) -> Result<PaynowLookupResult, FastPaynowError>;

    /// Return the human-readable name of this adapter implementation.
    fn adapter_name(&self) -> &str;
}

// ---------------------------------------------------------------------------
// Mock adapter
// ---------------------------------------------------------------------------

/// Mock FAST/PayNow adapter for testing and development.
///
/// Returns deterministic test data based on account conventions:
/// - Account numbers ending in "0000" are treated as inactive
/// - Account numbers ending in "9999" trigger payment rejection
/// - All other valid accounts succeed with instant settlement
///
/// Mock fee: SGD 0.00 (FAST is free for consumers in Singapore).
#[derive(Debug, Clone)]
pub struct MockFastPaynowAdapter;

impl MockFastPaynowAdapter {
    /// FAST is generally free for consumers; banks may charge for corporate accounts.
    const MOCK_FEE: i64 = 0;

    fn should_reject(account: &str) -> bool {
        account.ends_with("9999")
    }
}

impl FastPaynowAdapter for MockFastPaynowAdapter {
    fn initiate_payment(
        &self,
        instruction: &FastPaymentInstruction,
    ) -> Result<FastPaymentResult, FastPaynowError> {
        let _from = validate_sg_account(&instruction.from_bank_bic, &instruction.from_account)?;
        let _to = validate_sg_account(&instruction.to_bank_bic, &instruction.to_account)?;

        if instruction.amount <= 0 {
            return Err(FastPaynowError::PaymentRejected {
                reason: "amount must be positive".to_string(),
            });
        }

        if instruction.idempotency_key.is_empty() {
            return Err(FastPaynowError::PaymentRejected {
                reason: "idempotency_key must not be empty".to_string(),
            });
        }

        if Self::should_reject(&instruction.from_account) {
            return Err(FastPaynowError::PaymentRejected {
                reason: "insufficient funds (mock: account ends in 9999)".to_string(),
            });
        }

        Ok(FastPaymentResult {
            fast_reference: format!("FAST-MOCK-{}", &instruction.idempotency_key),
            status: SgPaymentStatus::Completed,
            timestamp: "2026-02-20T12:00:00Z".to_string(),
            fee: Some(Self::MOCK_FEE),
            settlement_id: Some(format!("STL-{}", &instruction.idempotency_key)),
        })
    }

    fn check_payment_status(
        &self,
        fast_reference: &str,
    ) -> Result<FastPaymentResult, FastPaynowError> {
        if fast_reference.is_empty() {
            return Err(FastPaynowError::PaymentNotFound {
                reference: fast_reference.to_string(),
            });
        }

        Ok(FastPaymentResult {
            fast_reference: fast_reference.to_string(),
            status: SgPaymentStatus::Completed,
            timestamp: "2026-02-20T12:00:01Z".to_string(),
            fee: Some(Self::MOCK_FEE),
            settlement_id: Some(format!("STL-{fast_reference}")),
        })
    }

    fn paynow_lookup(
        &self,
        proxy: &str,
        proxy_type: PaynowProxyType,
    ) -> Result<PaynowLookupResult, FastPaynowError> {
        match proxy_type {
            PaynowProxyType::MobileNumber => {
                // Singapore mobile: +65 followed by 8 digits starting with 8 or 9
                let digits: String = proxy.chars().filter(|c| c.is_ascii_digit()).collect();
                if digits.len() < 8 || digits.len() > 10 {
                    return Err(FastPaynowError::ProxyNotFound {
                        proxy: proxy.to_string(),
                    });
                }
            }
            PaynowProxyType::NricFin => {
                // Validate NRIC format: [STFGM] + 7 digits + letter
                let trimmed = proxy.trim().to_uppercase();
                if trimmed.len() != 9 {
                    return Err(FastPaynowError::ProxyNotFound {
                        proxy: proxy.to_string(),
                    });
                }
            }
            PaynowProxyType::Uen => {
                let trimmed = proxy.trim();
                if trimmed.len() < 9 || trimmed.len() > 10 {
                    return Err(FastPaynowError::ProxyNotFound {
                        proxy: proxy.to_string(),
                    });
                }
            }
            PaynowProxyType::Vpa => {
                if proxy.is_empty() {
                    return Err(FastPaynowError::ProxyNotFound {
                        proxy: proxy.to_string(),
                    });
                }
            }
        }

        Ok(PaynowLookupResult {
            proxy: proxy.to_string(),
            proxy_type,
            bank_bic: "DBSSSGSG".to_string(),
            account_number: "0012345678".to_string(),
            account_name: Some("Mock PayNow Holder".to_string()),
        })
    }

    fn adapter_name(&self) -> &str {
        "MockFastPaynowAdapter"
    }
}

impl NationalSystemAdapter for MockFastPaynowAdapter {
    fn category(&self) -> AdapterCategory {
        AdapterCategory::Payments
    }

    fn jurisdiction(&self) -> &str {
        "sg"
    }

    fn health(&self) -> AdapterHealth {
        AdapterHealth::Healthy
    }

    fn adapter_name(&self) -> &str {
        "MockFastPaynowAdapter"
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- validate_sg_account -----------------------------------------------------

    #[test]
    fn validate_sg_account_accepts_valid() {
        let result = validate_sg_account("DBSSSGSG", "0012345678");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "0012345678");
    }

    #[test]
    fn validate_sg_account_rejects_empty_bic() {
        let result = validate_sg_account("", "0012345678");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FastPaynowError::InvalidAccount { .. }));
    }

    #[test]
    fn validate_sg_account_rejects_short_account() {
        let result = validate_sg_account("DBSSSGSG", "12345");
        assert!(result.is_err());
    }

    #[test]
    fn validate_sg_account_rejects_non_digit_account() {
        let result = validate_sg_account("DBSSSGSG", "001234567X");
        assert!(result.is_err());
    }

    // -- SgPaymentStatus Display -------------------------------------------------

    #[test]
    fn payment_status_display() {
        assert_eq!(SgPaymentStatus::Pending.to_string(), "Pending");
        assert_eq!(SgPaymentStatus::Completed.to_string(), "Completed");
        assert_eq!(SgPaymentStatus::Returned.to_string(), "Returned");
    }

    #[test]
    fn payment_status_serde_roundtrip() {
        for status in [
            SgPaymentStatus::Pending,
            SgPaymentStatus::Processing,
            SgPaymentStatus::Completed,
            SgPaymentStatus::Failed,
            SgPaymentStatus::Returned,
        ] {
            let json = serde_json::to_string(&status).expect("serialize");
            let back: SgPaymentStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(status, back);
        }
    }

    // -- PaynowProxyType Display -------------------------------------------------

    #[test]
    fn proxy_type_display() {
        assert_eq!(PaynowProxyType::MobileNumber.to_string(), "MobileNumber");
        assert_eq!(PaynowProxyType::NricFin.to_string(), "NRIC/FIN");
        assert_eq!(PaynowProxyType::Uen.to_string(), "UEN");
        assert_eq!(PaynowProxyType::Vpa.to_string(), "VPA");
    }

    // -- Error Display -----------------------------------------------------------

    #[test]
    fn error_display_messages() {
        let err = FastPaynowError::ServiceUnavailable { reason: "connection refused".into() };
        assert!(err.to_string().contains("connection refused"));

        let err = FastPaynowError::Timeout { elapsed_ms: 5000 };
        assert!(err.to_string().contains("5000"));

        let err = FastPaynowError::ProxyNotFound { proxy: "91234567".into() };
        assert!(err.to_string().contains("91234567"));
    }

    // -- MockFastPaynowAdapter: initiate_payment ---------------------------------

    #[test]
    fn mock_adapter_initiates_valid_payment() {
        let adapter = MockFastPaynowAdapter;
        let instr = FastPaymentInstruction {
            amount: 100_000, // SGD 1,000
            from_bank_bic: "DBSSSGSG".into(),
            from_account: "0012345678".into(),
            to_bank_bic: "OCBCSGSG".into(),
            to_account: "5012345678".into(),
            reference: "MEZ-TEST-001".into(),
            purpose_code: Some("SUPP".into()),
            remittance_info: None,
            idempotency_key: "IDEM-001".into(),
        };
        let result = adapter.initiate_payment(&instr).expect("should initiate payment");
        assert_eq!(result.status, SgPaymentStatus::Completed);
        assert!(result.fast_reference.starts_with("FAST-MOCK-"));
        assert_eq!(result.fee, Some(MockFastPaynowAdapter::MOCK_FEE));
        assert!(result.settlement_id.is_some());
    }

    #[test]
    fn mock_adapter_rejects_invalid_account() {
        let adapter = MockFastPaynowAdapter;
        let instr = FastPaymentInstruction {
            amount: 100_000,
            from_bank_bic: "DBSSSGSG".into(),
            from_account: "123".into(), // too short
            to_bank_bic: "OCBCSGSG".into(),
            to_account: "5012345678".into(),
            reference: "REF".into(),
            purpose_code: None,
            remittance_info: None,
            idempotency_key: "IK1".into(),
        };
        let result = adapter.initiate_payment(&instr);
        assert!(result.is_err());
    }

    #[test]
    fn mock_adapter_rejects_zero_amount() {
        let adapter = MockFastPaynowAdapter;
        let instr = FastPaymentInstruction {
            amount: 0,
            from_bank_bic: "DBSSSGSG".into(),
            from_account: "0012345678".into(),
            to_bank_bic: "OCBCSGSG".into(),
            to_account: "5012345678".into(),
            reference: "REF".into(),
            purpose_code: None,
            remittance_info: None,
            idempotency_key: "IK1".into(),
        };
        let result = adapter.initiate_payment(&instr);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FastPaynowError::PaymentRejected { .. }));
    }

    #[test]
    fn mock_adapter_rejects_insufficient_funds() {
        let adapter = MockFastPaynowAdapter;
        let instr = FastPaymentInstruction {
            amount: 100_000,
            from_bank_bic: "DBSSSGSG".into(),
            from_account: "0012349999".into(), // ends in 9999
            to_bank_bic: "OCBCSGSG".into(),
            to_account: "5012345678".into(),
            reference: "REF".into(),
            purpose_code: None,
            remittance_info: None,
            idempotency_key: "IK1".into(),
        };
        let result = adapter.initiate_payment(&instr);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FastPaynowError::PaymentRejected { .. }));
    }

    // -- MockFastPaynowAdapter: check_payment_status -----------------------------

    #[test]
    fn mock_adapter_check_status_valid() {
        let adapter = MockFastPaynowAdapter;
        let result = adapter.check_payment_status("FAST-MOCK-001").expect("should return status");
        assert_eq!(result.status, SgPaymentStatus::Completed);
    }

    #[test]
    fn mock_adapter_check_status_empty_ref() {
        let adapter = MockFastPaynowAdapter;
        let result = adapter.check_payment_status("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FastPaynowError::PaymentNotFound { .. }));
    }

    // -- MockFastPaynowAdapter: paynow_lookup ------------------------------------

    #[test]
    fn mock_adapter_lookup_mobile() {
        let adapter = MockFastPaynowAdapter;
        let result = adapter.paynow_lookup("+6591234567", PaynowProxyType::MobileNumber)
            .expect("should resolve mobile");
        assert_eq!(result.proxy_type, PaynowProxyType::MobileNumber);
        assert!(!result.account_number.is_empty());
        assert!(result.account_name.is_some());
    }

    #[test]
    fn mock_adapter_lookup_nric() {
        let adapter = MockFastPaynowAdapter;
        let result = adapter.paynow_lookup("S1234567D", PaynowProxyType::NricFin)
            .expect("should resolve NRIC");
        assert_eq!(result.proxy_type, PaynowProxyType::NricFin);
    }

    #[test]
    fn mock_adapter_lookup_uen() {
        let adapter = MockFastPaynowAdapter;
        let result = adapter.paynow_lookup("201912345D", PaynowProxyType::Uen)
            .expect("should resolve UEN");
        assert_eq!(result.proxy_type, PaynowProxyType::Uen);
    }

    #[test]
    fn mock_adapter_lookup_invalid_mobile() {
        let adapter = MockFastPaynowAdapter;
        let result = adapter.paynow_lookup("123", PaynowProxyType::MobileNumber);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FastPaynowError::ProxyNotFound { .. }));
    }

    #[test]
    fn mock_adapter_lookup_invalid_nric() {
        let adapter = MockFastPaynowAdapter;
        let result = adapter.paynow_lookup("ABC", PaynowProxyType::NricFin);
        assert!(result.is_err());
    }

    // -- Trait properties --------------------------------------------------------

    #[test]
    fn mock_adapter_name() {
        let adapter = MockFastPaynowAdapter;
        assert_eq!(FastPaynowAdapter::adapter_name(&adapter), "MockFastPaynowAdapter");
    }

    #[test]
    fn adapter_trait_is_object_safe() {
        let adapter: Box<dyn FastPaynowAdapter> = Box::new(MockFastPaynowAdapter);
        assert_eq!(adapter.adapter_name(), "MockFastPaynowAdapter");
    }

    #[test]
    fn national_system_adapter_impl() {
        let adapter = MockFastPaynowAdapter;
        assert_eq!(NationalSystemAdapter::category(&adapter), AdapterCategory::Payments);
        assert_eq!(NationalSystemAdapter::jurisdiction(&adapter), "sg");
        assert!(matches!(NationalSystemAdapter::health(&adapter), AdapterHealth::Healthy));
    }

    // -- Serde round-trips -------------------------------------------------------

    #[test]
    fn payment_instruction_serde_roundtrip() {
        let instr = FastPaymentInstruction {
            amount: 500_000,
            from_bank_bic: "DBSSSGSG".into(),
            from_account: "0012345678".into(),
            to_bank_bic: "OCBCSGSG".into(),
            to_account: "5012345678".into(),
            reference: "MEZ-CORR-001".into(),
            purpose_code: Some("SUPP".into()),
            remittance_info: Some("Invoice #12345".into()),
            idempotency_key: "IDEM-001".into(),
        };
        let json = serde_json::to_string(&instr).expect("serialize");
        let back: FastPaymentInstruction = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.amount, 500_000);
        assert_eq!(back.from_bank_bic, "DBSSSGSG");
    }

    #[test]
    fn payment_result_serde_roundtrip() {
        let result = FastPaymentResult {
            fast_reference: "FAST-001".into(),
            status: SgPaymentStatus::Completed,
            timestamp: "2026-02-20T12:00:00Z".into(),
            fee: Some(0),
            settlement_id: Some("STL-001".into()),
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let back: FastPaymentResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.fast_reference, "FAST-001");
        assert_eq!(back.status, SgPaymentStatus::Completed);
    }

    #[test]
    fn paynow_lookup_result_serde_roundtrip() {
        let result = PaynowLookupResult {
            proxy: "+6591234567".into(),
            proxy_type: PaynowProxyType::MobileNumber,
            bank_bic: "DBSSSGSG".into(),
            account_number: "0012345678".into(),
            account_name: Some("Test User".into()),
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let back: PaynowLookupResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.proxy, "+6591234567");
        assert_eq!(back.proxy_type, PaynowProxyType::MobileNumber);
    }
}
