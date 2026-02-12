//! # msez-corridor — Corridor Operations
//!
//! Implements cross-border trade corridor infrastructure:
//!
//! - **Bridge** (`bridge.rs`): Dijkstra-based optimal route computation
//!   across multi-hop corridor networks, with fee computation.
//!
//! - **Receipt** (`receipt.rs`): Corridor receipt chain backed by the
//!   Merkle Mountain Range from `msez-crypto`. Provides append-only
//!   audit trail with inclusion proofs.
//!
//! - **Fork** (`fork.rs`): Fork detection and resolution with three-tier
//!   ordering: (1) timestamp, (2) watcher attestation count, (3) lexicographic
//!   digest tiebreaker. Includes 5-minute clock skew tolerance.
//!
//! - **Anchor** (`anchor.rs`): L1 anchoring for corridors that opt into
//!   blockchain finality. L1-optional by design.
//!
//! - **Netting** (`netting.rs`): Settlement netting engine for bilateral
//!   and multilateral netting of corridor obligations.
//!
//! - **SWIFT** (`swift.rs`): SWIFT pacs.008 adapter for traditional
//!   banking settlement rails.
//!
//! ## Crate Policy
//!
//! - Depends on `msez-core`, `msez-state`, and `msez-crypto` internally.
//! - Uses typestate corridor from `msez-state` for lifecycle management.
//! - Receipt chain uses `MerkleMountainRange` from `msez-crypto`.

pub mod anchor;
pub mod bridge;
pub mod fork;
pub mod netting;
pub mod receipt;
pub mod swift;

pub use bridge::CorridorBridge;
pub use fork::ForkResolver;
pub use netting::NettingEngine;
pub use receipt::ReceiptChain;
