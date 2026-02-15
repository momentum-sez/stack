//! # Payment Rail Adapters (M-010)
//!
//! Defines the [`PaymentRailAdapter`] trait for jurisdiction-specific payment
//! rail integrations. Each adapter handles the specifics of a particular
//! payment network while the corridor orchestration layer remains rail-agnostic.
//!
//! The trait abstracts three operations common to every payment rail:
//! initiating a payment, checking the status of a previously initiated payment,
//! and reporting the rail's human-readable name.
//!
//! ## Implementations
//!
//! | Rail | Status | Jurisdiction | Description |
//! |------|--------|-------------|-------------|
//! | [`RaastAdapter`] | Stub | Pakistan | SBP instant payment system |
//! | [`RtgsAdapter`] | Stub | Multi | Large-value real-time gross settlement |
//! | [`CircleUsdcAdapter`] | Stub | Global | USDC stablecoin settlement for digital corridors |
//!
//! The existing [`super::swift::SwiftPacs008`] adapter covers SWIFT pacs.008
//! and predates this trait. It implements the sealed [`super::swift::SettlementRail`]
//! trait. A future unification pass will bring it under `PaymentRailAdapter`.
//!
//! ## Design Rationale
//!
//! Methods are synchronous in the current stub phase. None of the stub
//! implementations perform I/O, so async would add complexity without benefit.
//! When production integrations land (Raast via SBP, RTGS, Circle API), this
//! trait will migrate to async via boxed futures or native `async fn in trait`
//! once the workspace MSRV supports `dyn`-compatible async trait methods.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// Errors arising from payment rail operations.
///
/// Each variant carries enough context to diagnose the failure without
/// requiring the caller to inspect opaque strings.
#[derive(Error, Debug)]
pub enum PaymentRailError {
    /// The payment rail adapter has not been configured for the target
    /// environment. This is the expected return from all stub adapters
    /// until production credentials and network access are provisioned.
    #[error("payment rail not configured: {0}")]
    NotConfigured(String),

    /// The payment rail rejected the instruction (e.g., insufficient funds,
    /// invalid account, sanctions hit).
    #[error("payment rejected by {rail}: {reason}")]
    Rejected {
        /// Which payment rail rejected the instruction.
        rail: String,
        /// Human-readable rejection reason.
        reason: String,
    },

    /// Network or connectivity error communicating with the payment rail.
    #[error("payment rail network error: {0}")]
    Network(String),

    /// The payment rail returned a response that could not be parsed or
    /// did not conform to the expected schema.
    #[error("unexpected response from {rail}: {details}")]
    UnexpectedResponse {
        /// Which rail returned the unexpected response.
        rail: String,
        /// Details of the unexpected response.
        details: String,
    },

    /// The referenced payment was not found on the rail (e.g., expired
    /// reference, wrong rail queried).
    #[error("payment not found: reference {reference} on rail {rail}")]
    NotFound {
        /// The rail that was queried.
        rail: String,
        /// The reference that was not found.
        reference: String,
    },
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A payment instruction to be submitted to a payment rail.
///
/// All monetary amounts are expressed in the smallest currency unit for the
/// given ISO 4217 code (e.g., paisa for PKR, cents for USD, fils for AED).
/// This avoids floating-point representation issues in financial calculations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentInstruction {
    /// Amount in the smallest currency unit.
    ///
    /// Must be positive. Zero and negative amounts are rejected by adapters.
    pub amount: i64,

    /// ISO 4217 currency code (e.g., "PKR", "USD", "AED").
    pub currency: String,

    /// Source account identifier on the payment rail.
    ///
    /// Format is rail-specific: IBAN for banking rails, wallet address for
    /// Circle USDC, Raast ID for SBP Raast.
    pub from_account: String,

    /// Destination account identifier on the payment rail.
    ///
    /// Same format constraints as `from_account`.
    pub to_account: String,

    /// Payment reference visible to both parties.
    ///
    /// Typically a corridor receipt ID, invoice number, or tax reference.
    pub reference: String,

    /// Arbitrary key-value metadata attached to the payment.
    ///
    /// Rails that support metadata fields (e.g., Circle transfer metadata)
    /// will forward these. Rails that do not will silently ignore them.
    pub metadata: HashMap<String, String>,
}

/// Result of initiating a payment or querying its current state.
///
/// Returned by both [`PaymentRailAdapter::initiate_payment`] and
/// [`PaymentRailAdapter::check_status`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResult {
    /// Rail-specific transaction reference.
    ///
    /// This is the identifier the rail assigned to the payment. Use it
    /// for subsequent [`PaymentRailAdapter::check_status`] calls.
    pub rail_reference: String,

    /// Current status of the payment on the rail.
    pub status: PaymentStatus,

    /// Timestamp of the most recent status change, as reported by the rail.
    pub timestamp: DateTime<Utc>,

    /// Fee charged by the rail for this payment, in the same minor-unit
    /// denomination as the instruction amount.
    ///
    /// `None` when the fee is not yet known (e.g., payment is still pending)
    /// or when the rail does not report fees.
    pub fee: Option<i64>,
}

/// Payment lifecycle status as reported by the rail.
///
/// Variants are ordered by typical lifecycle progression:
/// `Pending` -> `Processing` -> `Completed` (or `Failed` / `Reversed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    /// Payment has been accepted but processing has not started.
    Pending,

    /// Payment is actively being processed by the rail.
    Processing,

    /// Payment settled successfully.
    Completed,

    /// Payment failed (insufficient funds, network timeout, etc.).
    Failed,

    /// A previously completed payment has been reversed (chargeback,
    /// recall, or regulatory clawback).
    Reversed,
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Abstraction over jurisdiction-specific payment rails.
///
/// Each concrete adapter (Raast, RTGS, Circle USDC) implements this trait,
/// allowing the corridor orchestration layer to initiate payments and track
/// their status without coupling to any single rail's API.
///
/// ## Object Safety
///
/// The trait is object-safe (`Send + Sync`) so that adapters can be stored
/// as `Box<dyn PaymentRailAdapter>` in corridor configuration.
///
/// ## Stub Contract
///
/// All current implementations are stubs that return
/// [`PaymentRailError::NotConfigured`]. Production implementations will
/// be added as SBP, central bank, and Circle API access is provisioned.
pub trait PaymentRailAdapter: Send + Sync {
    /// Human-readable name of the payment rail (e.g., "SBP Raast", "RTGS",
    /// "Circle USDC").
    fn rail_name(&self) -> &str;

    /// Initiate a payment on this rail.
    ///
    /// On success, returns a [`PaymentResult`] whose `rail_reference` can be
    /// used in subsequent [`check_status`](Self::check_status) calls.
    ///
    /// # Errors
    ///
    /// Returns [`PaymentRailError::NotConfigured`] for stub adapters.
    /// Production adapters may return `Rejected`, `Network`, or
    /// `UnexpectedResponse` depending on the failure mode.
    fn initiate_payment(
        &self,
        instruction: &PaymentInstruction,
    ) -> Result<PaymentResult, PaymentRailError>;

    /// Query the current status of a previously initiated payment.
    ///
    /// `rail_reference` is the identifier returned by a prior
    /// [`initiate_payment`](Self::initiate_payment) call.
    ///
    /// # Errors
    ///
    /// Returns [`PaymentRailError::NotConfigured`] for stub adapters.
    /// Production adapters may return `NotFound`, `Network`, or
    /// `UnexpectedResponse`.
    fn check_status(
        &self,
        rail_reference: &str,
    ) -> Result<PaymentStatus, PaymentRailError>;
}

// ---------------------------------------------------------------------------
// Stub: SBP Raast (Pakistan instant payment system)
// ---------------------------------------------------------------------------

/// SBP Raast adapter for Pakistan's instant payment system.
///
/// Raast is the State Bank of Pakistan's instant payment system, supporting
/// real-time retail payments in PKR. It operates 24/7 and settles in central
/// bank money.
///
/// **Status: STUB** -- awaiting SBP Raast API integration specification
/// and production credentials from SBP.
///
/// When implemented, this adapter will:
/// - Submit payment instructions via the SBP Raast API
/// - Poll transaction status via the SBP Raast status endpoint
/// - Map SBP Raast status codes to [`PaymentStatus`] variants
/// - Report SBP-assessed fees in [`PaymentResult::fee`]
pub struct RaastAdapter;

impl PaymentRailAdapter for RaastAdapter {
    fn rail_name(&self) -> &str {
        "SBP Raast"
    }

    fn initiate_payment(
        &self,
        _instruction: &PaymentInstruction,
    ) -> Result<PaymentResult, PaymentRailError> {
        Err(PaymentRailError::NotConfigured(
            "SBP Raast adapter requires SBP API credentials and network access; \
             contact SBP for integration onboarding"
                .into(),
        ))
    }

    fn check_status(
        &self,
        _rail_reference: &str,
    ) -> Result<PaymentStatus, PaymentRailError> {
        Err(PaymentRailError::NotConfigured(
            "SBP Raast status query requires SBP API credentials and network access"
                .into(),
        ))
    }
}

// ---------------------------------------------------------------------------
// Stub: RTGS (Real-Time Gross Settlement)
// ---------------------------------------------------------------------------

/// RTGS adapter for large-value real-time gross settlement.
///
/// RTGS systems are operated by central banks for high-value, time-critical
/// interbank transfers. Each jurisdiction has its own RTGS system (e.g.,
/// PRISM in Pakistan, Fedwire in the US, CHAPS in the UK).
///
/// **Status: STUB** -- awaiting RTGS gateway API specification from the
/// relevant central bank.
///
/// When implemented, this adapter will:
/// - Submit large-value payment instructions via the RTGS gateway
/// - Track settlement finality through the RTGS status interface
/// - Map RTGS settlement states to [`PaymentStatus`] variants
/// - Report RTGS processing fees in [`PaymentResult::fee`]
pub struct RtgsAdapter;

impl PaymentRailAdapter for RtgsAdapter {
    fn rail_name(&self) -> &str {
        "RTGS"
    }

    fn initiate_payment(
        &self,
        _instruction: &PaymentInstruction,
    ) -> Result<PaymentResult, PaymentRailError> {
        Err(PaymentRailError::NotConfigured(
            "RTGS adapter requires central bank gateway credentials and \
             network access; contact the relevant central bank for integration"
                .into(),
        ))
    }

    fn check_status(
        &self,
        _rail_reference: &str,
    ) -> Result<PaymentStatus, PaymentRailError> {
        Err(PaymentRailError::NotConfigured(
            "RTGS status query requires central bank gateway credentials and network access"
                .into(),
        ))
    }
}

// ---------------------------------------------------------------------------
// Stub: Circle USDC (stablecoin settlement)
// ---------------------------------------------------------------------------

/// Circle USDC adapter for stablecoin settlement on digital corridors.
///
/// Circle's USDC is a fully-reserved dollar stablecoin used for instant,
/// low-cost cross-border settlement. The adapter targets the Circle
/// Payments API for programmatic USDC transfers between Circle wallets.
///
/// **Status: STUB** -- awaiting Circle API key provisioning and
/// sandbox environment access.
///
/// When implemented, this adapter will:
/// - Submit USDC transfer instructions via the Circle Payments API
/// - Poll transfer status via Circle's transfer status endpoint
/// - Map Circle transfer states to [`PaymentStatus`] variants
/// - Report Circle processing fees in [`PaymentResult::fee`]
pub struct CircleUsdcAdapter;

impl PaymentRailAdapter for CircleUsdcAdapter {
    fn rail_name(&self) -> &str {
        "Circle USDC"
    }

    fn initiate_payment(
        &self,
        _instruction: &PaymentInstruction,
    ) -> Result<PaymentResult, PaymentRailError> {
        Err(PaymentRailError::NotConfigured(
            "Circle USDC adapter requires Circle API key and sandbox/production \
             environment configuration"
                .into(),
        ))
    }

    fn check_status(
        &self,
        _rail_reference: &str,
    ) -> Result<PaymentStatus, PaymentRailError> {
        Err(PaymentRailError::NotConfigured(
            "Circle USDC status query requires Circle API key and environment configuration"
                .into(),
        ))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    /// Build a sample payment instruction for test use.
    fn sample_instruction() -> PaymentInstruction {
        let mut metadata = HashMap::new();
        metadata.insert("corridor".into(), "PK-UAE".into());
        metadata.insert("receipt_id".into(), Uuid::new_v4().to_string());

        PaymentInstruction {
            amount: 500_000,
            currency: "PKR".into(),
            from_account: "PK36HABB0000001123456702".into(),
            to_account: "PK36HABB0000009876543210".into(),
            reference: "MSEZ-TEST-001".into(),
            metadata,
        }
    }

    // -- RaastAdapter -------------------------------------------------------

    #[test]
    fn raast_rail_name() {
        let adapter = RaastAdapter;
        assert_eq!(adapter.rail_name(), "SBP Raast");
    }

    #[test]
    fn raast_initiate_payment_returns_not_configured() {
        let adapter = RaastAdapter;
        let result = adapter.initiate_payment(&sample_instruction());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, PaymentRailError::NotConfigured(_)),
            "expected NotConfigured, got: {err:?}"
        );
    }

    #[test]
    fn raast_check_status_returns_not_configured() {
        let adapter = RaastAdapter;
        let result = adapter.check_status("RAAST-REF-001");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, PaymentRailError::NotConfigured(_)),
            "expected NotConfigured, got: {err:?}"
        );
    }

    // -- RtgsAdapter --------------------------------------------------------

    #[test]
    fn rtgs_rail_name() {
        let adapter = RtgsAdapter;
        assert_eq!(adapter.rail_name(), "RTGS");
    }

    #[test]
    fn rtgs_initiate_payment_returns_not_configured() {
        let adapter = RtgsAdapter;
        let result = adapter.initiate_payment(&sample_instruction());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, PaymentRailError::NotConfigured(_)),
            "expected NotConfigured, got: {err:?}"
        );
    }

    #[test]
    fn rtgs_check_status_returns_not_configured() {
        let adapter = RtgsAdapter;
        let result = adapter.check_status("RTGS-REF-001");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, PaymentRailError::NotConfigured(_)),
            "expected NotConfigured, got: {err:?}"
        );
    }

    // -- CircleUsdcAdapter --------------------------------------------------

    #[test]
    fn circle_rail_name() {
        let adapter = CircleUsdcAdapter;
        assert_eq!(adapter.rail_name(), "Circle USDC");
    }

    #[test]
    fn circle_initiate_payment_returns_not_configured() {
        let adapter = CircleUsdcAdapter;
        let mut instr = sample_instruction();
        instr.currency = "USD".into();
        instr.from_account = "circle-wallet-001".into();
        instr.to_account = "circle-wallet-002".into();
        instr.amount = 100_00;

        let result = adapter.initiate_payment(&instr);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, PaymentRailError::NotConfigured(_)),
            "expected NotConfigured, got: {err:?}"
        );
    }

    #[test]
    fn circle_check_status_returns_not_configured() {
        let adapter = CircleUsdcAdapter;
        let result = adapter.check_status("CIRCLE-REF-001");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, PaymentRailError::NotConfigured(_)),
            "expected NotConfigured, got: {err:?}"
        );
    }

    // -- Trait object safety ------------------------------------------------

    #[test]
    fn payment_rail_adapter_is_object_safe() {
        // Verify the trait can be used as a trait object, which is required
        // for dynamic dispatch in corridor configuration.
        let adapters: Vec<Box<dyn PaymentRailAdapter>> = vec![
            Box::new(RaastAdapter),
            Box::new(RtgsAdapter),
            Box::new(CircleUsdcAdapter),
        ];

        let expected_names = ["SBP Raast", "RTGS", "Circle USDC"];
        for (adapter, expected) in adapters.iter().zip(expected_names.iter()) {
            assert_eq!(adapter.rail_name(), *expected);
        }
    }

    // -- PaymentInstruction serde round-trip ---------------------------------

    #[test]
    fn payment_instruction_serde_roundtrip() {
        let original = sample_instruction();
        let json = serde_json::to_string(&original).expect("serialize PaymentInstruction");
        let recovered: PaymentInstruction =
            serde_json::from_str(&json).expect("deserialize PaymentInstruction");

        assert_eq!(recovered.amount, original.amount);
        assert_eq!(recovered.currency, original.currency);
        assert_eq!(recovered.from_account, original.from_account);
        assert_eq!(recovered.to_account, original.to_account);
        assert_eq!(recovered.reference, original.reference);
        assert_eq!(recovered.metadata.len(), original.metadata.len());
        assert_eq!(
            recovered.metadata.get("corridor"),
            original.metadata.get("corridor")
        );
    }

    // -- PaymentResult serde round-trip -------------------------------------

    #[test]
    fn payment_result_serde_roundtrip() {
        let original = PaymentResult {
            rail_reference: "RAAST-2026-001".into(),
            status: PaymentStatus::Completed,
            timestamp: Utc::now(),
            fee: Some(150),
        };
        let json = serde_json::to_string(&original).expect("serialize PaymentResult");
        let recovered: PaymentResult =
            serde_json::from_str(&json).expect("deserialize PaymentResult");

        assert_eq!(recovered.rail_reference, original.rail_reference);
        assert_eq!(recovered.status, PaymentStatus::Completed);
        assert_eq!(recovered.fee, Some(150));
    }

    #[test]
    fn payment_result_with_no_fee() {
        let result = PaymentResult {
            rail_reference: "RTGS-2026-001".into(),
            status: PaymentStatus::Pending,
            timestamp: Utc::now(),
            fee: None,
        };
        let json = serde_json::to_string(&result).expect("serialize PaymentResult");
        let recovered: PaymentResult =
            serde_json::from_str(&json).expect("deserialize PaymentResult");

        assert_eq!(recovered.fee, None);
        assert_eq!(recovered.status, PaymentStatus::Pending);
    }

    // -- PaymentStatus serde ------------------------------------------------

    #[test]
    fn payment_status_serde_all_variants() {
        let variants = [
            (PaymentStatus::Pending, "\"pending\""),
            (PaymentStatus::Processing, "\"processing\""),
            (PaymentStatus::Completed, "\"completed\""),
            (PaymentStatus::Failed, "\"failed\""),
            (PaymentStatus::Reversed, "\"reversed\""),
        ];
        for (status, expected_json) in &variants {
            let json = serde_json::to_string(status).expect("serialize PaymentStatus");
            assert_eq!(&json, expected_json, "serialization mismatch for {status:?}");
            let recovered: PaymentStatus =
                serde_json::from_str(&json).expect("deserialize PaymentStatus");
            assert_eq!(&recovered, status, "round-trip mismatch for {status:?}");
        }
    }

    // -- PaymentRailError display -------------------------------------------

    #[test]
    fn error_display_not_configured() {
        let err = PaymentRailError::NotConfigured("test rail".into());
        let msg = err.to_string();
        assert!(
            msg.contains("not configured"),
            "expected 'not configured' in: {msg}"
        );
    }

    #[test]
    fn error_display_rejected() {
        let err = PaymentRailError::Rejected {
            rail: "RTGS".into(),
            reason: "insufficient funds".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("RTGS"), "expected rail name in: {msg}");
        assert!(
            msg.contains("insufficient funds"),
            "expected reason in: {msg}"
        );
    }

    #[test]
    fn error_display_not_found() {
        let err = PaymentRailError::NotFound {
            rail: "Raast".into(),
            reference: "REF-999".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("REF-999"), "expected reference in: {msg}");
        assert!(msg.contains("Raast"), "expected rail name in: {msg}");
    }
}
