//! # Circuit Type Definitions
//!
//! Defines the data models for the 12 circuit types referenced in the
//! specification, organized into 4 categories:
//!
//! - **Compliance**: Attestation circuits for regulatory compliance verification.
//! - **Migration**: Evidence circuits for cross-jurisdiction asset migration.
//! - **Identity**: Verification circuits for KYC/KYB identity proofs.
//! - **Settlement**: Proof circuits for payment and settlement finality.
//!
//! ## Phase 1 Status
//!
//! These are data model stubs — they define the public inputs and witness
//! fields for each circuit type but do not contain real constraint systems.
//! The constraint definitions will be added in Phase 2 when real ZK backends
//! (Groth16, PLONK) are integrated.
//!
//! ## Spec Reference
//!
//! Audit §2.5 and spec chapters on compliance tensor, migration saga,
//! identity verification, and settlement netting define the 12 circuit types.

pub mod compliance;
pub mod identity;
pub mod migration;
pub mod settlement;
