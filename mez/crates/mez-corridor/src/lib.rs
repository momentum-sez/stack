//! # mez-corridor — Corridor Operations
//!
//! Provides the operational infrastructure for cross-border trade corridors:
//!
//! - **Bridge** ([`bridge`]): Dijkstra-weighted routing across corridor
//!   graphs, with fee computation per hop.
//!
//! - **Receipt Chain** ([`receipt`]): Append-only corridor receipts backed
//!   by MMR for efficient inclusion proofs.
//!
//! - **Fork Resolution** ([`fork`]): Fork detection with three-level
//!   ordering: timestamp (primary), watcher attestation count (secondary),
//!   and lexicographic digest tiebreaker (tertiary). Includes 5-minute
//!   maximum clock skew tolerance per audit §3.5.
//!
//! - **Anchoring** ([`anchor`]): L1 chain anchoring for corridor checkpoints.
//!   L1 is optional — the system works without blockchain dependencies.
//!
//! - **Netting** ([`netting`]): Settlement netting engine for bilateral
//!   and multilateral obligation compression.
//!
//! - **SWIFT** ([`swift`]): SWIFT pacs.008 payment instruction adapter
//!   for traditional settlement rails.
//!
//! - **Payment Rails** ([`payment_rail`]): Generic [`PaymentRailAdapter`]
//!   trait with stub implementations for SBP Raast, RTGS, and Circle USDC.
//!
//! ## Spec Reference
//!
//! Implements protocols from `spec/40-corridors.md`, including:
//! - Protocol 16.1: Fork resolution with secondary ordering criteria
//! - Receipt chain MMR commitment per Part IV
//! - Dijkstra-weighted corridor routing per bridge protocol

pub mod anchor;
pub mod bridge;
pub mod fork;
pub mod migration;
pub mod netting;
pub mod network;
pub mod payment_rail;
pub mod receipt;
pub mod swift;

// Re-export primary types.
pub use anchor::{AnchorCommitment, AnchorError, AnchorReceipt, AnchorTarget, MockAnchorTarget};
pub use bridge::{BridgeEdge, BridgeRoute, CorridorBridge};
pub use fork::{
    ForkBranch, ForkDetector, ForkError, ForkResolution, ResolutionReason, WatcherAttestation,
    WatcherRegistry, create_attestation, resolve_fork, MAX_CLOCK_SKEW, MAX_FUTURE_DRIFT,
};
pub use migration::{MigrationError, MigrationSaga, MigrationState, SideEffect};
pub use netting::{
    Currency, NetPosition, NettingEngine, NettingError, Obligation, Party, SettlementLeg,
    SettlementPlan,
};
pub use payment_rail::{
    CircleUsdcAdapter, PaymentInstruction, PaymentRailAdapter, PaymentRailError, PaymentResult,
    PaymentStatus, RaastAdapter, RtgsAdapter,
};
pub use receipt::{
    Checkpoint, CorridorReceipt, DigestEntry, MmrCommitment, MmrPeakEntry, ProofObject,
    ReceiptChain, ReceiptError, ReceiptProof, compute_next_root,
};
pub use network::{
    CorridorAcceptance, CorridorNetworkConfig, CorridorPeer, CorridorProposal, CorridorRejection,
    InboundAttestation, InboundReceipt, InboundReceiptResult, NetworkError, PeerEndpoint,
    PeerRegistry, PeerStatus, validate_inbound_receipt, validate_proposal,
};
pub use swift::{SettlementInstruction, SettlementRail, SettlementRailError, SwiftPacs008};

use thiserror::Error;

/// Top-level error type for corridor operations.
#[derive(Error, Debug)]
pub enum CorridorError {
    /// Routing failure in the corridor bridge.
    #[error("routing error: {0}")]
    Routing(String),

    /// Receipt chain integrity violation.
    #[error("receipt error: {0}")]
    Receipt(#[from] ReceiptError),

    /// Fork resolution failure.
    #[error("fork resolution error: {0}")]
    Fork(String),

    /// Anchoring failure.
    #[error("anchor error: {0}")]
    Anchor(#[from] AnchorError),

    /// Netting computation failure.
    #[error("netting error: {0}")]
    Netting(#[from] NettingError),

    /// Inter-zone networking failure.
    #[error("network error: {0}")]
    Network(#[from] NetworkError),

    /// Canonicalization failure.
    #[error("canonicalization error: {0}")]
    Canonicalization(#[from] mez_core::CanonicalizationError),

    /// Crypto operation failure.
    #[error("crypto error: {0}")]
    Crypto(#[from] mez_crypto::CryptoError),
}
