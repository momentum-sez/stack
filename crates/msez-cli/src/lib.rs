//! # msez-cli — SEZ Stack Command-Line Interface
//!
//! Replaces the 15,472-line Python monolith `tools/msez.py` with a
//! structured clap-based CLI. Every subcommand, flag, and output format
//! from the Python original is preserved for backward compatibility.
//!
//! ## Subcommands
//!
//! - `validate` — Zone, module, and profile validation
//! - `lock` — Lockfile generation and deterministic verification
//! - `corridor` — Corridor lifecycle management
//! - `artifact` — CAS store, resolve, verify, and graph operations
//! - `sign` — Ed25519 and VC signing operations
//!
//! ## Crate Policy
//!
//! - CLI construction (argument parsing) is separated from business logic.
//! - Handler functions delegate to domain crates — no business logic here.
//! - Output format must match the Python CLI exactly for CI compatibility.

pub mod artifact;
pub mod corridor;
pub mod lock;
pub mod signing;
pub mod validate;
