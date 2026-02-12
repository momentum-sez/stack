//! # msez-tensor — Compliance Tensor & Manifold
//!
//! The Compliance Tensor is a multi-dimensional evaluation of an entity's
//! compliance status across all [`ComplianceDomain`](msez_core::ComplianceDomain)
//! variants for a given jurisdiction. The Compliance Manifold provides
//! path optimization over the tensor space.
//!
//! ## Mathematical Model
//!
//! A compliance tensor T for entity E in jurisdiction J is a function:
//!
//! ```text
//! T(E, J) : ComplianceDomain → ComplianceState
//! ```
//!
//! where `ComplianceDomain` has 20 variants (defined in `msez-core`) and
//! `ComplianceState` encodes {Compliant, NonCompliant, Pending, NotApplicable}.
//!
//! The manifold M is the space of all valid compliance tensor configurations
//! across a corridor, with edges weighted by transition cost (fee, time, risk).
//! Dijkstra optimization finds the minimum-cost compliance path.
//!
//! ## Audit Reference
//!
//! Finding §2.4: The Python codebase had two independent domain enums (8 vs 20).
//! This crate uses the single [`ComplianceDomain`](msez_core::ComplianceDomain)
//! enum from `msez-core`. Every `match` is exhaustive — the compiler prevents
//! missing domains.

pub mod commitment;
pub mod evaluation;
pub mod manifold;
pub mod tensor;

// Re-export primary types.
pub use commitment::TensorCommitment;
pub use evaluation::ComplianceState;
pub use manifold::ComplianceManifold;
pub use tensor::ComplianceTensor;
