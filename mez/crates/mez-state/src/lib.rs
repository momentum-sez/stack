//! # mez-state — Typestate-Encoded State Machines
//!
//! This crate encodes all lifecycle state machines using the Rust typestate
//! pattern: each state is a distinct type, and only valid transitions exist
//! as methods. Invalid transitions are compile errors, not runtime checks.
//!
//! ## State Machines
//!
//! - **Corridor** ([`corridor`]): `Draft → Pending → Active` with `Halted`
//!   and `Suspended` branches. Matches spec §40-corridors exactly. There
//!   is no string `"OPERATIONAL"` anywhere — the audit finding §2.3 is
//!   structurally prevented.
//!
//! - **Migration** ([`migration`]): 8 phases + 3 terminal states with
//!   compile-time deadline enforcement via the builder pattern.
//!
//! - **Entity** ([`entity`]): Formation through 10-stage dissolution.
//!
//! - **License** ([`license`]): License lifecycle management.
//!
//! - **Watcher** ([`watcher`]): Bonding, active watching, slashing, and
//!   unbonding with 4 slashing conditions.
//!
//! ## Design Principle
//!
//! ```text
//! // Calling .halt() on Corridor<Draft> is a COMPILE ERROR:
//! let draft = Corridor::<Draft>::new(id, jurisdiction_a, jurisdiction_b);
//! draft.halt(reason); // ERROR: no method named `halt` on `Corridor<Draft>`
//! ```

pub mod corridor;
pub mod entity;
pub mod license;
pub mod migration;
pub mod watcher;

// Re-export primary types.
pub use corridor::{
    Active, Corridor, CorridorState, Deprecated, Draft, DynCorridorData, DynCorridorState, Halted,
    Pending, Suspended, TransitionRecord,
};
pub use entity::{DissolutionStage, Entity, EntityLifecycleState};
pub use license::{License, LicenseState};
pub use migration::{MigrationBuilder, MigrationSaga, MigrationState, NoDeadline};
pub use watcher::{SlashingCondition, Watcher, WatcherState};
