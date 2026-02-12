//! # msez-core — Foundational Types for the SEZ Stack
//!
//! This crate is the bedrock of the Momentum SEZ Stack. It defines the core
//! type-system primitives that enforce correctness guarantees at compile time.
//! Every other crate in the workspace depends on `msez-core`; it depends on
//! nothing internal.
//!
//! ## Key Design Principles
//!
//! 1. **Newtype wrappers for domain primitives.** `JurisdictionId`, `EntityId`,
//!    `CorridorId`, `NTN`, `CNIC` — all newtypes. No bare strings for identifiers.
//!
//! 2. **`CanonicalBytes` newtype.** ALL digest computation flows through
//!    `CanonicalBytes::new()`. No raw `serde_json::to_vec()` for digests. Ever.
//!    This prevents the canonicalization split defect class by construction.
//!
//! 3. **Single `ComplianceDomain` enum.** One definition, 20 variants, exhaustive
//!    `match` everywhere. Adding a domain forces every consumer to handle it.
//!
//! 4. **UTC-only timestamps.** The `Timestamp` type enforces UTC with Z suffix
//!    and seconds precision — matching the JCS canonicalization rules.
//!
//! ## Crate Policy
//!
//! - No dependencies on other `msez-*` crates (this is the leaf of the DAG).
//! - No `unsafe` code.
//! - No `panic!()` or `.unwrap()` outside tests.
//! - All public types derive `Debug`, `Clone`, and implement `Serialize`/`Deserialize`.

pub mod canonical;
pub mod digest;
pub mod domain;
pub mod error;
pub mod identity;
pub mod jurisdiction;
pub mod temporal;

// Re-export primary types for ergonomic imports.
pub use canonical::CanonicalBytes;
pub use digest::{ContentDigest, DigestAlgorithm};
pub use domain::ComplianceDomain;
pub use error::MsezError;
pub use identity::{EntityId, CorridorId, MigrationId, WatcherId, CNIC, NTN, PassportNumber};
pub use jurisdiction::JurisdictionId;
pub use temporal::Timestamp;
