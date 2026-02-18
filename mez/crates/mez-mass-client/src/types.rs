//! # Shared Identifier Types
//!
//! Re-exports identifier newtypes from [`mez_core`] so that downstream
//! consumers of `mez-mass-client` can use the same typed identifiers
//! that the rest of the EZ Stack uses.
//!
//! ## Dependency Invariant (CLAUDE.md §V.2)
//!
//! `mez-mass-client` depends on `mez-core` ONLY for these identifier
//! newtypes. It must never import EZ Stack domain logic (compliance
//! tensors, corridors, packs, VCs, etc.).

pub use mez_core::EntityId;
pub use mez_core::JurisdictionId;

/// Type alias for entity identifiers originating from or destined for
/// Mass API calls. Structurally identical to [`EntityId`] — the alias
/// carries semantic intent ("this ID came from / is going to Mass").
pub type MassEntityId = EntityId;
