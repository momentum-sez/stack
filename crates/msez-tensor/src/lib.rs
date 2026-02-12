//! # msez-tensor — Compliance Tensor & Manifold
//!
//! Implements the mathematical framework for multi-domain compliance
//! evaluation:
//!
//! - **Tensor** (`tensor.rs`): The compliance tensor `T: J × D → S` maps
//!   a (jurisdiction, domain) pair to a compliance state. Generic over
//!   jurisdiction type `J`.
//!
//! - **Manifold** (`manifold.rs`): The compliance manifold provides path
//!   optimization over the tensor space — finding the optimal compliance
//!   path for cross-border operations.
//!
//! - **Commitment** (`commitment.rs`): Tensor commitment generation using
//!   `CanonicalBytes` → SHA-256 digest for content-addressed storage.
//!
//! - **Evaluation** (`evaluation.rs`): Domain evaluation logic that maps
//!   regulatory rules to compliance states across all 20 domains.
//!
//! ## Mathematical Definition
//!
//! Let D = {d_1, d_2, ..., d_20} be the set of compliance domains
//! (from `msez_core::ComplianceDomain`). Let J be the set of jurisdictions.
//! The compliance tensor is a mapping T: J × D → S where S is the set
//! of compliance states {Compliant, NonCompliant, Pending, Exempt, Unknown}.
//!
//! ## Security Invariant
//!
//! Tensor commitments are computed exclusively from `CanonicalBytes` via
//! `msez-crypto::sha256_digest()`. The canonicalization split defect
//! (audit §2.1) is prevented by the type system.
//!
//! ## Implements
//!
//! Spec §12 — Compliance tensor structure and evaluation.

pub mod commitment;
pub mod evaluation;
pub mod manifold;
pub mod tensor;

pub use commitment::TensorCommitment;
pub use tensor::{ComplianceState, ComplianceTensor};
