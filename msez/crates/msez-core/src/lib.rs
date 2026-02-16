#![deny(missing_docs)]

//! # msez-core — Foundational Types for the Momentum SEZ Stack
//!
//! This crate defines the foundational types that every other crate in the
//! workspace depends on. It has no internal crate dependencies — only `serde`,
//! `serde_json`, `thiserror`, `chrono`, `uuid`, and `sha2` from the external ecosystem.
//!
//! ## Design Principles
//!
//! 1. **Newtype wrappers for domain primitives.** Every identifier is a distinct
//!    type. You cannot pass a [`JurisdictionId`] where an [`EntityId`] is expected.
//!
//! 2. **[`CanonicalBytes`] is the sole path to digest computation.** All content-
//!    addressed digests in the entire stack flow through `CanonicalBytes::new()`,
//!    which applies JCS-compatible canonicalization with Momentum-specific type
//!    coercion rules (float rejection, datetime normalization, key coercion).
//!
//! 3. **Single [`ComplianceDomain`] enum.** One definition, 20 variants, exhaustive
//!    `match` everywhere. No independent domain lists that can diverge.
//!
//! 4. **[`MsezError`] hierarchy.** Structured errors with `thiserror` — no
//!    `Box<dyn Error>`, no `.unwrap()` outside tests.

pub mod canonical;
pub mod digest;
pub mod domain;
pub mod error;
pub mod identity;
pub mod jurisdiction;
pub mod sovereignty;
pub mod temporal;

// Re-export primary types at crate root for ergonomic imports.
pub use canonical::CanonicalBytes;
pub use digest::{
    sha256_bytes, sha256_digest, sha256_raw, ContentDigest, DigestAlgorithm, Sha256Accumulator,
};
pub use domain::ComplianceDomain;
pub use error::{CanonicalizationError, MsezError, ValidationError};
pub use identity::{Cnic, Did, EntityId, MigrationId, Ntn, PassportNumber, WatcherId};
pub use jurisdiction::{CorridorId, JurisdictionId};
pub use sovereignty::{DataCategory, SovereigntyEnforcer, SovereigntyPolicy, SovereigntyVerdict};
pub use temporal::Timestamp;
