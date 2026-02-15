//! # Shared Identifier Types
//!
//! Re-exports identifier newtypes from [`msez_core`] so that downstream
//! consumers of `msez-mass-client` can use the same typed identifiers
//! that the rest of the SEZ Stack uses.
//!
//! ## Dependency Invariant (CLAUDE.md §V.2)
//!
//! `msez-mass-client` depends on `msez-core` ONLY for these identifier
//! newtypes. It must never import SEZ Stack domain logic (compliance
//! tensors, corridors, packs, VCs, etc.).

pub use msez_core::EntityId;
pub use msez_core::JurisdictionId;

/// Type alias for entity identifiers originating from or destined for
/// Mass API calls. Structurally identical to [`EntityId`] — the alias
/// carries semantic intent ("this ID came from / is going to Mass").
pub type MassEntityId = EntityId;
