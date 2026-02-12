//! # msez-vc — Verifiable Credentials for the SEZ Stack
//!
//! Implements the W3C Verifiable Credentials Data Model adapted for
//! sovereign economic zone infrastructure. Provides:
//!
//! - **Credential structure** ([`VerifiableCredential`]) with typed envelope,
//!   credential subject, and proof array.
//! - **Ed25519 proof generation and verification** using the cryptographic
//!   primitives from `msez-crypto`.
//! - **Smart Asset Registry credentials** ([`SmartAssetRegistryVc`]) for
//!   registering and transferring compliance-evaluated smart assets.
//!
//! ## Security Invariants
//!
//! - All proof computation uses [`CanonicalBytes`](msez_core::CanonicalBytes)
//!   for payload canonicalization — never raw `serde_json::to_vec()`.
//! - Proof objects have rigid structure (`additionalProperties: false` at
//!   the schema level) to prevent injection attacks.
//! - BBS+ selective disclosure is Phase 2 (feature-gated in msez-crypto).

pub mod credential;
pub mod proof;
pub mod registry;

// Re-export primary types.
pub use credential::{
    ContextValue, CredentialTypeValue, ProofResult, ProofValue, VcError, VerifiableCredential,
};
pub use proof::{Proof, ProofPurpose, ProofType};
pub use registry::{
    ArtifactRef, BindingComplianceResult, ComplianceProfile, EnforcementProfile,
    JurisdictionBinding, LawpackRef, SmartAssetRegistrySubject, SmartAssetRegistryVc,
    REGISTRY_SCHEMA_ID,
};
