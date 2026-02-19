//! # SBP Raast Integration Adapter Interface
//!
//! Defines the adapter interface for SBP Raast, the State Bank of Pakistan's
//! instant payment system. Raast enables real-time retail payments in PKR,
//! operating 24/7 with settlement in central bank money.
//!
//! ## Architecture
//!
//! The `RaastAdapter` trait abstracts over the SBP Raast backend. Production
//! deployments implement it against the live SBP Raast API; test environments
//! use `MockRaastAdapter`. This separation allows the payment rail layer and
//! corridor settlement logic to compose Raast operations without coupling to
//! a specific transport or API version.
//!
//! ## Raast Account IDs (RAIDs)
//!
//! Raast identifies accounts via IBANs (International Bank Account Numbers).
//! Pakistan IBANs follow the format `PK{check}{bank}{account}` where:
//! - `PK` is the country code
//! - `{check}` is a 2-digit check number
//! - `{bank}` is a 4-character bank code
//! - `{account}` is a 16-digit account number
//!
//! Total length: 24 characters. The `validate_iban` helper enforces this format.
//!
//! ## Alias Resolution
//!
//! Raast supports alias-based lookup where a mobile number or CNIC can resolve
//! to a RAID (IBAN). This enables pay-by-phone and pay-by-CNIC flows.
//!
//! ## Settlement
//!
//! Raast settles instantly (< 10 seconds) in PKR. The adapter reports settlement
//! status via [`PaymentStatus`] and fees via [`RaastPaymentResult`].
//!
//! ## Integration Points
//!
//! - **Initiate payment**: Submit a credit transfer instruction via Raast
//! - **Check payment status**: Query the status of a previously initiated payment
//! - **Verify account**: Validate that an IBAN is active and reachable via Raast
//! - **Lookup by alias**: Resolve a phone number or CNIC to a RAID

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// Errors from SBP Raast integration operations.
#[derive(Debug, thiserror::Error)]
pub enum RaastError {
    /// SBP Raast service is unreachable or returned a 5xx status.
    #[error("SBP Raast service unavailable: {reason}")]
    ServiceUnavailable {
        /// Human-readable description of the outage or error.
        reason: String,
    },

    /// IBAN format is invalid (not a well-formed Pakistan IBAN).
    #[error("invalid IBAN: {reason}")]
    InvalidIban {
        /// Description of the validation failure.
        reason: String,
    },

    /// The SBP Raast adapter has not been configured for this deployment.
    #[error("SBP Raast adapter not configured: {reason}")]
    NotConfigured {
        /// Why configuration is missing or incomplete.
        reason: String,
    },

    /// The request to SBP Raast timed out.
    #[error("SBP Raast request timed out after {elapsed_ms}ms")]
    Timeout {
        /// Elapsed time in milliseconds before the timeout triggered.
        elapsed_ms: u64,
    },

    /// SBP Raast rejected the payment instruction.
    #[error("payment rejected by SBP Raast: {reason}")]
    PaymentRejected {
        /// Description of why the payment was rejected.
        reason: String,
    },

    /// The referenced payment was not found on Raast.
    #[error("payment not found: reference {reference}")]
    PaymentNotFound {
        /// The transaction reference that was not found.
        reference: String,
    },

    /// The account could not be verified via Raast.
    #[error("account verification failed: {reason}")]
    AccountVerificationFailed {
        /// Description of the verification failure.
        reason: String,
    },

    /// Alias lookup returned no matching account.
    #[error("alias not found: {alias}")]
    AliasNotFound {
        /// The alias that was looked up.
        alias: String,
    },
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Payment status as reported by SBP Raast.
///
/// Raast settlements are typically instant (< 10 seconds), so the
/// `Processing` state is transient.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaymentStatus {
    /// Payment instruction accepted, awaiting processing.
    Pending,
    /// Payment is being processed by SBP Raast.
    Processing,
    /// Payment settled successfully in central bank money.
    Completed,
    /// Payment failed (insufficient funds, invalid account, etc.).
    Failed,
    /// A previously completed payment has been reversed.
    Reversed,
}

impl fmt::Display for PaymentStatus {
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

/// Type of alias used for Raast account lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AliasType {
    /// Mobile phone number (e.g., "03001234567").
    MobileNumber,
    /// CNIC number (13 digits).
    Cnic,
}

impl fmt::Display for AliasType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MobileNumber => write!(f, "MobileNumber"),
            Self::Cnic => write!(f, "CNIC"),
        }
    }
}

/// A Raast credit transfer instruction.
///
/// Amounts are in the smallest currency unit (paisa for PKR).
/// For example, PKR 1,000.00 = 100_000 paisa.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaastPaymentInstruction {
    /// Amount in paisa (smallest PKR unit). Must be positive.
    pub amount: i64,

    /// Source account IBAN (Pakistan format, 24 characters).
    pub from_iban: String,

    /// Destination account IBAN (Pakistan format, 24 characters).
    pub to_iban: String,

    /// Payment reference visible to both parties.
    /// Typically a corridor receipt ID, invoice number, or tax reference.
    pub reference: String,

    /// Purpose of payment code (SBP-defined, e.g., "SALA" for salary,
    /// "SUPP" for supplier payment, "TAXS" for tax payment).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose_code: Option<String>,

    /// Remittance information / description visible in account statements.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remittance_info: Option<String>,

    /// Idempotency key to prevent duplicate payment submissions.
    pub idempotency_key: String,
}

/// Result of a Raast payment initiation or status query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaastPaymentResult {
    /// SBP Raast transaction reference (assigned by Raast).
    pub raast_reference: String,

    /// Current status of the payment.
    pub status: PaymentStatus,

    /// ISO 8601 timestamp of the most recent status change.
    pub timestamp: String,

    /// Fee charged by SBP Raast in paisa. `None` if fee is not yet known
    /// or if the payment rail does not report fees.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee: Option<i64>,

    /// Settlement confirmation identifier (populated after `Completed`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settlement_id: Option<String>,
}

/// Account verification result from SBP Raast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountVerification {
    /// The IBAN that was verified.
    pub iban: String,

    /// Whether the account is active and reachable via Raast.
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

/// Result of an alias-to-RAID lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliasLookupResult {
    /// The alias that was looked up.
    pub alias: String,

    /// Type of alias.
    pub alias_type: AliasType,

    /// Resolved IBAN (RAID).
    pub iban: String,

    /// Account title (holder name), if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_title: Option<String>,

    /// Bank name, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Validate that an IBAN string is a well-formed Pakistan IBAN.
///
/// Pakistan IBANs are 24 characters: `PK` + 2 check digits + 4 bank code + 16 account.
/// This helper checks length, country prefix, and digit constraints.
pub fn validate_iban(iban: &str) -> Result<String, RaastError> {
    let cleaned: String = iban.chars().filter(|c| !c.is_whitespace()).collect();
    let upper = cleaned.to_uppercase();

    if upper.len() != 24 {
        return Err(RaastError::InvalidIban {
            reason: format!(
                "Pakistan IBAN must be exactly 24 characters, got {} from '{}'",
                upper.len(),
                iban
            ),
        });
    }

    if !upper.starts_with("PK") {
        return Err(RaastError::InvalidIban {
            reason: format!(
                "Pakistan IBAN must start with 'PK', got '{}'",
                &upper[..2]
            ),
        });
    }

    // Characters 3-4 must be digits (check digits).
    if !upper[2..4].chars().all(|c| c.is_ascii_digit()) {
        return Err(RaastError::InvalidIban {
            reason: "IBAN check digits (positions 3-4) must be numeric".to_string(),
        });
    }

    // Characters 5-8 are the bank code (alphanumeric).
    if !upper[4..8].chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(RaastError::InvalidIban {
            reason: "IBAN bank code (positions 5-8) must be alphanumeric".to_string(),
        });
    }

    // Characters 9-24 are the account number (digits).
    if !upper[8..].chars().all(|c| c.is_ascii_digit()) {
        return Err(RaastError::InvalidIban {
            reason: "IBAN account number (positions 9-24) must be numeric".to_string(),
        });
    }

    Ok(upper)
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Adapter trait for SBP Raast instant payment system integration.
///
/// Implementations must be `Send + Sync` so they can be shared across
/// async tasks behind an `Arc`. The trait is object-safe to support
/// runtime adapter selection (mock vs. live).
///
/// ## Operations
///
/// - **initiate_payment**: Submit a credit transfer instruction
/// - **check_payment_status**: Query status of a previously initiated payment
/// - **verify_account**: Validate that an IBAN is active and reachable
/// - **lookup_by_alias**: Resolve a phone/CNIC alias to a RAID (IBAN)
pub trait RaastAdapter: Send + Sync {
    /// Submit a credit transfer instruction to SBP Raast.
    ///
    /// On success, returns a [`RaastPaymentResult`] whose `raast_reference`
    /// can be used for subsequent status queries.
    fn initiate_payment(
        &self,
        instruction: &RaastPaymentInstruction,
    ) -> Result<RaastPaymentResult, RaastError>;

    /// Query the current status of a previously initiated payment.
    ///
    /// `raast_reference` is the identifier returned by a prior
    /// [`initiate_payment`](Self::initiate_payment) call.
    fn check_payment_status(
        &self,
        raast_reference: &str,
    ) -> Result<RaastPaymentResult, RaastError>;

    /// Verify that an IBAN is active and reachable via Raast.
    ///
    /// Used for pre-flight validation before initiating payments.
    fn verify_account(&self, iban: &str) -> Result<AccountVerification, RaastError>;

    /// Resolve a phone number or CNIC alias to a RAID (IBAN).
    ///
    /// Enables pay-by-phone and pay-by-CNIC flows.
    fn lookup_by_alias(
        &self,
        alias: &str,
        alias_type: AliasType,
    ) -> Result<AliasLookupResult, RaastError>;

    /// Return the human-readable name of this adapter implementation
    /// (e.g. "MockRaastAdapter", "SbpRaastLiveApiV1").
    fn adapter_name(&self) -> &str;
}

// ---------------------------------------------------------------------------
// Mock adapter
// ---------------------------------------------------------------------------

/// Mock SBP Raast adapter for testing and development.
///
/// Returns deterministic test data based on IBAN conventions:
/// - IBANs ending in "0000" are treated as inactive accounts
/// - IBANs ending in "9999" trigger payment rejection (insufficient funds)
/// - All other valid IBANs succeed with instant settlement
///
/// Alias lookups succeed for mobile numbers starting with "03" and
/// 13-digit CNIC numbers.
///
/// Mock fee is fixed at PKR 0.50 (50 paisa) per transaction.
#[derive(Debug, Clone)]
pub struct MockRaastAdapter;

impl MockRaastAdapter {
    /// Fixed mock fee: PKR 0.50 = 50 paisa.
    const MOCK_FEE: i64 = 50;

    /// Determine if an IBAN represents an inactive account (test convention).
    fn is_inactive_account(iban: &str) -> bool {
        iban.ends_with("0000")
    }

    /// Determine if a payment should be rejected (test convention).
    fn should_reject(iban: &str) -> bool {
        iban.ends_with("9999")
    }
}

impl RaastAdapter for MockRaastAdapter {
    fn initiate_payment(
        &self,
        instruction: &RaastPaymentInstruction,
    ) -> Result<RaastPaymentResult, RaastError> {
        // Validate IBANs.
        let from = validate_iban(&instruction.from_iban)?;
        let _to = validate_iban(&instruction.to_iban)?;

        // Validate amount.
        if instruction.amount <= 0 {
            return Err(RaastError::PaymentRejected {
                reason: "amount must be positive".to_string(),
            });
        }

        // Validate idempotency key.
        if instruction.idempotency_key.is_empty() {
            return Err(RaastError::PaymentRejected {
                reason: "idempotency_key must not be empty".to_string(),
            });
        }

        // Check for rejection convention.
        if Self::should_reject(&from) {
            return Err(RaastError::PaymentRejected {
                reason: "insufficient funds (mock: IBAN ends in 9999)".to_string(),
            });
        }

        Ok(RaastPaymentResult {
            raast_reference: format!("RAAST-MOCK-{}", &instruction.idempotency_key),
            status: PaymentStatus::Completed,
            timestamp: "2026-02-19T12:00:00Z".to_string(),
            fee: Some(Self::MOCK_FEE),
            settlement_id: Some(format!("STL-{}", &instruction.idempotency_key)),
        })
    }

    fn check_payment_status(
        &self,
        raast_reference: &str,
    ) -> Result<RaastPaymentResult, RaastError> {
        if raast_reference.is_empty() {
            return Err(RaastError::PaymentNotFound {
                reference: raast_reference.to_string(),
            });
        }

        Ok(RaastPaymentResult {
            raast_reference: raast_reference.to_string(),
            status: PaymentStatus::Completed,
            timestamp: "2026-02-19T12:00:01Z".to_string(),
            fee: Some(Self::MOCK_FEE),
            settlement_id: Some(format!("STL-{raast_reference}")),
        })
    }

    fn verify_account(&self, iban: &str) -> Result<AccountVerification, RaastError> {
        let validated = validate_iban(iban)?;

        if Self::is_inactive_account(&validated) {
            return Ok(AccountVerification {
                iban: validated,
                active: false,
                account_title: None,
                bank_name: Some("Mock Bank".to_string()),
                verification_timestamp: "2026-02-19T12:00:00Z".to_string(),
            });
        }

        // Extract bank code from IBAN (positions 5-8).
        let bank_code = validated[4..8].to_string();
        Ok(AccountVerification {
            iban: validated,
            active: true,
            account_title: Some("Mock Account Holder".to_string()),
            bank_name: Some(format!("Bank {bank_code}")),
            verification_timestamp: "2026-02-19T12:00:00Z".to_string(),
        })
    }

    fn lookup_by_alias(
        &self,
        alias: &str,
        alias_type: AliasType,
    ) -> Result<AliasLookupResult, RaastError> {
        match alias_type {
            AliasType::MobileNumber => {
                if !alias.starts_with("03") || alias.len() != 11 {
                    return Err(RaastError::AliasNotFound {
                        alias: alias.to_string(),
                    });
                }
            }
            AliasType::Cnic => {
                let digits: String = alias.chars().filter(|c| c.is_ascii_digit()).collect();
                if digits.len() != 13 {
                    return Err(RaastError::AliasNotFound {
                        alias: alias.to_string(),
                    });
                }
            }
        }

        // Return a deterministic mock IBAN.
        Ok(AliasLookupResult {
            alias: alias.to_string(),
            alias_type,
            iban: "PK36HABB0000001123456702".to_string(),
            account_title: Some("Mock Alias Holder".to_string()),
            bank_name: Some("HBL".to_string()),
        })
    }

    fn adapter_name(&self) -> &str {
        "MockRaastAdapter"
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- validate_iban -----------------------------------------------------------

    #[test]
    fn validate_iban_accepts_valid_pk_iban() {
        let result = validate_iban("PK36HABB0000001123456702");
        assert!(result.is_ok());
        assert_eq!(result.expect("valid IBAN"), "PK36HABB0000001123456702");
    }

    #[test]
    fn validate_iban_accepts_lowercase() {
        let result = validate_iban("pk36habb0000001123456702");
        assert!(result.is_ok());
        assert_eq!(result.expect("valid IBAN"), "PK36HABB0000001123456702");
    }

    #[test]
    fn validate_iban_strips_whitespace() {
        let result = validate_iban("PK36 HABB 0000 0011 2345 6702");
        assert!(result.is_ok());
        assert_eq!(result.expect("valid IBAN"), "PK36HABB0000001123456702");
    }

    #[test]
    fn validate_iban_rejects_wrong_country() {
        let result = validate_iban("GB36HABB0000001123456702");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RaastError::InvalidIban { .. }));
    }

    #[test]
    fn validate_iban_rejects_too_short() {
        let result = validate_iban("PK36HABB000000112345");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RaastError::InvalidIban { .. }));
    }

    #[test]
    fn validate_iban_rejects_too_long() {
        let result = validate_iban("PK36HABB00000011234567020000");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RaastError::InvalidIban { .. }));
    }

    #[test]
    fn validate_iban_rejects_empty() {
        let result = validate_iban("");
        assert!(result.is_err());
    }

    #[test]
    fn validate_iban_rejects_non_digit_check() {
        let result = validate_iban("PKXXHABB0000001123456702");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RaastError::InvalidIban { .. }));
    }

    #[test]
    fn validate_iban_rejects_non_digit_account() {
        let result = validate_iban("PK36HABB000000112345670X");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RaastError::InvalidIban { .. }));
    }

    // -- PaymentStatus Display ---------------------------------------------------

    #[test]
    fn payment_status_display() {
        assert_eq!(PaymentStatus::Pending.to_string(), "Pending");
        assert_eq!(PaymentStatus::Processing.to_string(), "Processing");
        assert_eq!(PaymentStatus::Completed.to_string(), "Completed");
        assert_eq!(PaymentStatus::Failed.to_string(), "Failed");
        assert_eq!(PaymentStatus::Reversed.to_string(), "Reversed");
    }

    // -- AliasType Display -------------------------------------------------------

    #[test]
    fn alias_type_display() {
        assert_eq!(AliasType::MobileNumber.to_string(), "MobileNumber");
        assert_eq!(AliasType::Cnic.to_string(), "CNIC");
    }

    // -- RaastError Display ------------------------------------------------------

    #[test]
    fn raast_error_display_messages() {
        let err = RaastError::ServiceUnavailable {
            reason: "connection refused".into(),
        };
        assert!(err.to_string().contains("connection refused"));

        let err = RaastError::InvalidIban {
            reason: "too short".into(),
        };
        assert!(err.to_string().contains("too short"));

        let err = RaastError::NotConfigured {
            reason: "missing API key".into(),
        };
        assert!(err.to_string().contains("missing API key"));

        let err = RaastError::Timeout { elapsed_ms: 5000 };
        assert!(err.to_string().contains("5000"));

        let err = RaastError::PaymentRejected {
            reason: "insufficient funds".into(),
        };
        assert!(err.to_string().contains("insufficient funds"));

        let err = RaastError::PaymentNotFound {
            reference: "REF-999".into(),
        };
        assert!(err.to_string().contains("REF-999"));

        let err = RaastError::AccountVerificationFailed {
            reason: "bank offline".into(),
        };
        assert!(err.to_string().contains("bank offline"));

        let err = RaastError::AliasNotFound {
            alias: "03001234567".into(),
        };
        assert!(err.to_string().contains("03001234567"));
    }

    // -- Serde round-trips -------------------------------------------------------

    #[test]
    fn payment_status_serde_round_trip() {
        for status in [
            PaymentStatus::Pending,
            PaymentStatus::Processing,
            PaymentStatus::Completed,
            PaymentStatus::Failed,
            PaymentStatus::Reversed,
        ] {
            let json = serde_json::to_string(&status).expect("serialize PaymentStatus");
            let back: PaymentStatus =
                serde_json::from_str(&json).expect("deserialize PaymentStatus");
            assert_eq!(status, back);
        }
    }

    #[test]
    fn alias_type_serde_round_trip() {
        for alias_type in [AliasType::MobileNumber, AliasType::Cnic] {
            let json = serde_json::to_string(&alias_type).expect("serialize AliasType");
            let back: AliasType = serde_json::from_str(&json).expect("deserialize AliasType");
            assert_eq!(alias_type, back);
        }
    }

    #[test]
    fn raast_payment_instruction_serde_round_trip() {
        let instr = RaastPaymentInstruction {
            amount: 500_000,
            from_iban: "PK36HABB0000001123456702".into(),
            to_iban: "PK36HABB0000009876543210".into(),
            reference: "MEZ-CORR-001".into(),
            purpose_code: Some("SUPP".into()),
            remittance_info: Some("Invoice #12345".into()),
            idempotency_key: "IDEM-001".into(),
        };
        let json = serde_json::to_string(&instr).expect("serialize");
        let back: RaastPaymentInstruction = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.amount, 500_000);
        assert_eq!(back.from_iban, "PK36HABB0000001123456702");
        assert_eq!(back.to_iban, "PK36HABB0000009876543210");
        assert_eq!(back.reference, "MEZ-CORR-001");
        assert_eq!(back.purpose_code.as_deref(), Some("SUPP"));
        assert_eq!(back.remittance_info.as_deref(), Some("Invoice #12345"));
        assert_eq!(back.idempotency_key, "IDEM-001");
    }

    #[test]
    fn raast_payment_instruction_optional_fields_absent() {
        let instr = RaastPaymentInstruction {
            amount: 100_000,
            from_iban: "PK36HABB0000001123456702".into(),
            to_iban: "PK36HABB0000009876543210".into(),
            reference: "REF".into(),
            purpose_code: None,
            remittance_info: None,
            idempotency_key: "IK1".into(),
        };
        let json = serde_json::to_string(&instr).expect("serialize");
        assert!(!json.contains("purpose_code"));
        assert!(!json.contains("remittance_info"));
    }

    #[test]
    fn raast_payment_result_serde_round_trip() {
        let result = RaastPaymentResult {
            raast_reference: "RAAST-2026-001".into(),
            status: PaymentStatus::Completed,
            timestamp: "2026-02-19T12:00:00Z".into(),
            fee: Some(50),
            settlement_id: Some("STL-001".into()),
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let back: RaastPaymentResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.raast_reference, "RAAST-2026-001");
        assert_eq!(back.status, PaymentStatus::Completed);
        assert_eq!(back.fee, Some(50));
        assert_eq!(back.settlement_id.as_deref(), Some("STL-001"));
    }

    #[test]
    fn raast_payment_result_optional_fields_absent() {
        let result = RaastPaymentResult {
            raast_reference: "RAAST-2026-002".into(),
            status: PaymentStatus::Pending,
            timestamp: "2026-02-19T12:00:00Z".into(),
            fee: None,
            settlement_id: None,
        };
        let json = serde_json::to_string(&result).expect("serialize");
        assert!(!json.contains("fee"));
        assert!(!json.contains("settlement_id"));
    }

    #[test]
    fn account_verification_serde_round_trip() {
        let av = AccountVerification {
            iban: "PK36HABB0000001123456702".into(),
            active: true,
            account_title: Some("Test Account".into()),
            bank_name: Some("HBL".into()),
            verification_timestamp: "2026-02-19T12:00:00Z".into(),
        };
        let json = serde_json::to_string(&av).expect("serialize");
        let back: AccountVerification = serde_json::from_str(&json).expect("deserialize");
        assert!(back.active);
        assert_eq!(back.iban, "PK36HABB0000001123456702");
        assert_eq!(back.account_title.as_deref(), Some("Test Account"));
    }

    #[test]
    fn account_verification_optional_fields_absent() {
        let av = AccountVerification {
            iban: "PK36HABB0000001123456702".into(),
            active: false,
            account_title: None,
            bank_name: None,
            verification_timestamp: "2026-02-19T12:00:00Z".into(),
        };
        let json = serde_json::to_string(&av).expect("serialize");
        assert!(!json.contains("account_title"));
        assert!(!json.contains("bank_name"));
    }

    #[test]
    fn alias_lookup_result_serde_round_trip() {
        let result = AliasLookupResult {
            alias: "03001234567".into(),
            alias_type: AliasType::MobileNumber,
            iban: "PK36HABB0000001123456702".into(),
            account_title: Some("Ali Khan".into()),
            bank_name: Some("HBL".into()),
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let back: AliasLookupResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.alias, "03001234567");
        assert_eq!(back.alias_type, AliasType::MobileNumber);
        assert_eq!(back.iban, "PK36HABB0000001123456702");
    }

    // -- MockRaastAdapter: initiate_payment --------------------------------------

    #[test]
    fn mock_adapter_initiates_valid_payment() {
        let adapter = MockRaastAdapter;
        let instr = RaastPaymentInstruction {
            amount: 500_000,
            from_iban: "PK36HABB0000001123456702".into(),
            to_iban: "PK36HABB0000009876543210".into(),
            reference: "MEZ-TEST-001".into(),
            purpose_code: Some("SUPP".into()),
            remittance_info: None,
            idempotency_key: "IDEM-001".into(),
        };
        let result = adapter
            .initiate_payment(&instr)
            .expect("should initiate payment");
        assert_eq!(result.status, PaymentStatus::Completed);
        assert!(result.raast_reference.starts_with("RAAST-MOCK-"));
        assert!(result.raast_reference.contains("IDEM-001"));
        assert_eq!(result.fee, Some(MockRaastAdapter::MOCK_FEE));
        assert!(result.settlement_id.is_some());
        assert!(!result.timestamp.is_empty());
    }

    #[test]
    fn mock_adapter_rejects_invalid_from_iban() {
        let adapter = MockRaastAdapter;
        let instr = RaastPaymentInstruction {
            amount: 100_000,
            from_iban: "INVALID".into(),
            to_iban: "PK36HABB0000009876543210".into(),
            reference: "REF".into(),
            purpose_code: None,
            remittance_info: None,
            idempotency_key: "IK1".into(),
        };
        let result = adapter.initiate_payment(&instr);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RaastError::InvalidIban { .. }));
    }

    #[test]
    fn mock_adapter_rejects_invalid_to_iban() {
        let adapter = MockRaastAdapter;
        let instr = RaastPaymentInstruction {
            amount: 100_000,
            from_iban: "PK36HABB0000001123456702".into(),
            to_iban: "INVALID".into(),
            reference: "REF".into(),
            purpose_code: None,
            remittance_info: None,
            idempotency_key: "IK1".into(),
        };
        let result = adapter.initiate_payment(&instr);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RaastError::InvalidIban { .. }));
    }

    #[test]
    fn mock_adapter_rejects_zero_amount() {
        let adapter = MockRaastAdapter;
        let instr = RaastPaymentInstruction {
            amount: 0,
            from_iban: "PK36HABB0000001123456702".into(),
            to_iban: "PK36HABB0000009876543210".into(),
            reference: "REF".into(),
            purpose_code: None,
            remittance_info: None,
            idempotency_key: "IK1".into(),
        };
        let result = adapter.initiate_payment(&instr);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RaastError::PaymentRejected { .. }
        ));
    }

    #[test]
    fn mock_adapter_rejects_negative_amount() {
        let adapter = MockRaastAdapter;
        let instr = RaastPaymentInstruction {
            amount: -100,
            from_iban: "PK36HABB0000001123456702".into(),
            to_iban: "PK36HABB0000009876543210".into(),
            reference: "REF".into(),
            purpose_code: None,
            remittance_info: None,
            idempotency_key: "IK1".into(),
        };
        let result = adapter.initiate_payment(&instr);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RaastError::PaymentRejected { .. }
        ));
    }

    #[test]
    fn mock_adapter_rejects_empty_idempotency_key() {
        let adapter = MockRaastAdapter;
        let instr = RaastPaymentInstruction {
            amount: 100_000,
            from_iban: "PK36HABB0000001123456702".into(),
            to_iban: "PK36HABB0000009876543210".into(),
            reference: "REF".into(),
            purpose_code: None,
            remittance_info: None,
            idempotency_key: "".into(),
        };
        let result = adapter.initiate_payment(&instr);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RaastError::PaymentRejected { .. }
        ));
    }

    #[test]
    fn mock_adapter_rejects_insufficient_funds_convention() {
        let adapter = MockRaastAdapter;
        let instr = RaastPaymentInstruction {
            amount: 100_000,
            from_iban: "PK36HABB0000001123459999".into(),
            to_iban: "PK36HABB0000009876543210".into(),
            reference: "REF".into(),
            purpose_code: None,
            remittance_info: None,
            idempotency_key: "IK1".into(),
        };
        let result = adapter.initiate_payment(&instr);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RaastError::PaymentRejected { .. }
        ));
    }

    // -- MockRaastAdapter: check_payment_status ----------------------------------

    #[test]
    fn mock_adapter_check_payment_status_valid() {
        let adapter = MockRaastAdapter;
        let result = adapter
            .check_payment_status("RAAST-MOCK-001")
            .expect("should return status");
        assert_eq!(result.status, PaymentStatus::Completed);
        assert_eq!(result.raast_reference, "RAAST-MOCK-001");
        assert_eq!(result.fee, Some(MockRaastAdapter::MOCK_FEE));
        assert!(result.settlement_id.is_some());
    }

    #[test]
    fn mock_adapter_check_payment_status_empty_ref() {
        let adapter = MockRaastAdapter;
        let result = adapter.check_payment_status("");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RaastError::PaymentNotFound { .. }
        ));
    }

    // -- MockRaastAdapter: verify_account ----------------------------------------

    #[test]
    fn mock_adapter_verify_active_account() {
        let adapter = MockRaastAdapter;
        let result = adapter
            .verify_account("PK36HABB0000001123456702")
            .expect("should verify account");
        assert!(result.active);
        assert_eq!(result.iban, "PK36HABB0000001123456702");
        assert!(result.account_title.is_some());
        assert!(result.bank_name.is_some());
        assert!(!result.verification_timestamp.is_empty());
    }

    #[test]
    fn mock_adapter_verify_inactive_account() {
        let adapter = MockRaastAdapter;
        let result = adapter
            .verify_account("PK36HABB0000001123450000")
            .expect("should verify account");
        assert!(!result.active);
        assert!(result.account_title.is_none());
    }

    #[test]
    fn mock_adapter_verify_invalid_iban() {
        let adapter = MockRaastAdapter;
        let result = adapter.verify_account("INVALID");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RaastError::InvalidIban { .. }));
    }

    // -- MockRaastAdapter: lookup_by_alias ---------------------------------------

    #[test]
    fn mock_adapter_lookup_mobile_alias() {
        let adapter = MockRaastAdapter;
        let result = adapter
            .lookup_by_alias("03001234567", AliasType::MobileNumber)
            .expect("should resolve alias");
        assert_eq!(result.alias, "03001234567");
        assert_eq!(result.alias_type, AliasType::MobileNumber);
        assert!(!result.iban.is_empty());
        assert!(result.account_title.is_some());
        assert!(result.bank_name.is_some());
    }

    #[test]
    fn mock_adapter_lookup_cnic_alias() {
        let adapter = MockRaastAdapter;
        let result = adapter
            .lookup_by_alias("4210112345671", AliasType::Cnic)
            .expect("should resolve CNIC alias");
        assert_eq!(result.alias_type, AliasType::Cnic);
        assert!(!result.iban.is_empty());
    }

    #[test]
    fn mock_adapter_lookup_cnic_with_dashes() {
        let adapter = MockRaastAdapter;
        let result = adapter
            .lookup_by_alias("42101-1234567-1", AliasType::Cnic)
            .expect("should resolve dashed CNIC");
        assert_eq!(result.alias_type, AliasType::Cnic);
    }

    #[test]
    fn mock_adapter_lookup_invalid_mobile() {
        let adapter = MockRaastAdapter;
        let result = adapter.lookup_by_alias("12345", AliasType::MobileNumber);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RaastError::AliasNotFound { .. }
        ));
    }

    #[test]
    fn mock_adapter_lookup_invalid_cnic() {
        let adapter = MockRaastAdapter;
        let result = adapter.lookup_by_alias("12345", AliasType::Cnic);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RaastError::AliasNotFound { .. }
        ));
    }

    // -- Trait properties -------------------------------------------------------

    #[test]
    fn mock_adapter_name() {
        let adapter = MockRaastAdapter;
        assert_eq!(adapter.adapter_name(), "MockRaastAdapter");
    }

    #[test]
    fn adapter_trait_is_object_safe() {
        let adapter: Box<dyn RaastAdapter> = Box::new(MockRaastAdapter);
        assert_eq!(adapter.adapter_name(), "MockRaastAdapter");
        let result = adapter
            .verify_account("PK36HABB0000001123456702")
            .expect("trait object verify");
        assert!(result.active);
    }

    #[test]
    fn adapter_trait_behind_arc() {
        let adapter: std::sync::Arc<dyn RaastAdapter> = std::sync::Arc::new(MockRaastAdapter);
        let instr = RaastPaymentInstruction {
            amount: 100_000,
            from_iban: "PK36HABB0000001123456702".into(),
            to_iban: "PK36HABB0000009876543210".into(),
            reference: "ARC-TEST".into(),
            purpose_code: None,
            remittance_info: None,
            idempotency_key: "ARC-IK1".into(),
        };
        let result = adapter
            .initiate_payment(&instr)
            .expect("Arc adapter should work");
        assert_eq!(result.status, PaymentStatus::Completed);
    }
}
