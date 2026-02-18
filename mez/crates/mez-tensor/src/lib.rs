//! # mez-tensor — Compliance Tensor & Manifold
//!
//! The Compliance Tensor is a multi-dimensional evaluation of an entity's
//! compliance status across all 20 `ComplianceDomain`
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
//! where `ComplianceDomain` has 20 variants (defined in `mez-core`) and
//! `ComplianceState` encodes {Compliant, NonCompliant, Pending, Exempt, NotApplicable}.
//!
//! The manifold M is the space of all valid compliance tensor configurations
//! across a corridor, with edges weighted by transition cost (fee, time, risk).
//! Dijkstra optimization finds the minimum-cost compliance path.
//!
//! ## Audit Reference
//!
//! Finding §2.4: The Python codebase had two independent domain enums (8 vs 20).
//! This crate uses the single `ComplianceDomain`
//! enum from `mez-core`. Every `match` is exhaustive — the compiler prevents
//! missing domains.

pub mod commitment;
pub mod evaluation;
pub mod manifold;
pub mod tensor;

// Re-export primary types.
pub use commitment::{commitment_digest, merkle_root, TensorCommitment};
pub use evaluation::{
    evaluate_domain_default, AttestationRef, ComplianceState, DomainEvaluator, EvaluationContext,
};
pub use manifold::{
    ComplianceDistance, ComplianceManifold, CorridorEdge, JurisdictionNode, MigrationHop,
    MigrationPath, PathConstraint,
};
pub use tensor::{
    ComplianceTensor, DefaultJurisdiction, JurisdictionConfig, TensorCell, TensorSlice,
};
