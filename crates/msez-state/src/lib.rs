//! # msez-state — Typestate-Encoded State Machines
//!
//! Implements the state machines of the SEZ Stack using the typestate pattern.
//! Each state is a distinct Rust type, and transitions are methods that consume
//! the current state and produce the next state. Invalid transitions are
//! compile errors, not runtime checks.
//!
//! ## State Machines
//!
//! - **Corridor** (`corridor.rs`): `Draft → Pending → Active` with `Halted`,
//!   `Suspended`, and `Deprecated` branches. Implements spec §40.
//!
//! - **Migration** (`migration.rs`): 7-phase migration saga with 2 terminal
//!   states (Completed, Failed). Includes compile-time deadline enforcement
//!   via `MigrationBuilder<NoDeadline>` / `MigrationBuilder<HasDeadline>`.
//!   Implements spec §42.
//!
//! - **Entity** (`entity.rs`): Entity lifecycle with 10-stage dissolution
//!   process. Implements spec §5.
//!
//! - **License** (`license.rs`): License lifecycle
//!   (Application → Review → Issued → Active → Suspended → Revoked/Expired/Rejected).
//!   Implements spec §15.
//!
//! - **Watcher** (`watcher.rs`): Watcher bonding and slashing state machine
//!   with 6 slashing conditions and collateral tracking. Implements spec §17.
//!
//! ## Design
//!
//! The typestate pattern prevents the corridor state machine divergence defect
//! (audit §2.3). There are no string-typed state names — the state is encoded
//! in the Rust type system. `Corridor<Draft>` has a `.submit()` method that
//! returns `Corridor<Pending>`. `Corridor<Draft>` has no `.halt()` method —
//! calling it is a compile error.

pub mod corridor;
pub mod entity;
pub mod license;
pub mod migration;
pub mod watcher;

// ─── Corridor re-exports ────────────────────────────────────────────

pub use corridor::{
    Active, Corridor, CorridorError, CorridorState, Deprecated, Draft, DynCorridor,
    DynCorridorState, Halted, Pending, Suspended, TransitionRecord,
};

// ─── Corridor evidence re-exports ───────────────────────────────────

pub use corridor::{
    ActivationEvidence, DeprecationEvidence, HaltReason, ResumeEvidence, SubmissionEvidence,
    SuspendReason,
};

// ─── Migration re-exports ───────────────────────────────────────────

pub use migration::{
    CompensationAction, CompensationRecord, HasDeadline, MigrationBuilder, MigrationError,
    MigrationPhase, MigrationSaga, MigrationTimeoutError, MigrationTransition, NoDeadline,
};

// ─── Entity re-exports ──────────────────────────────────────────────

pub use entity::{
    DissolutionStage, Entity, EntityError, EntityLifecycleState, EntityTransitionEvidence,
    EntityTransitionRecord,
};

// ─── License re-exports ─────────────────────────────────────────────

pub use license::{License, LicenseError, LicenseState, LicenseTransitionEvidence, LicenseTransitionRecord};

// ─── Watcher re-exports ─────────────────────────────────────────────

pub use watcher::{
    BondStatus, SlashingCondition, SlashingEvidence, SlashingRecord, WatcherBond, WatcherError,
    WatcherTransitionRecord,
};
