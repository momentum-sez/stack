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
//!    `CorridorId`, `DID`, `NTN`, `CNIC` — all newtypes with validated constructors.
//!    No bare strings for identifiers.
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
//! 5. **`sha256_digest()` accepts only `&CanonicalBytes`.** Compile-time enforcement
//!    that all digest paths flow through canonicalization. Poseidon2 behind feature flag.
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
pub use digest::{sha256_digest, sha256_hex, ContentDigest, DigestAlgorithm};
pub use domain::{ComplianceDomain, COMPLIANCE_DOMAIN_COUNT};
pub use error::MsezError;
pub use identity::{
    CorridorId, DID, EntityId, JurisdictionId, MigrationId, PassportNumber, WatcherId, CNIC, NTN,
};
pub use temporal::Timestamp;
