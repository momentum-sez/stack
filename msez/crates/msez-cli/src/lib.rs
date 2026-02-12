//! # msez-cli — CLI Tool for the SEZ Stack
//!
//! Provides the `msez` command-line interface, replacing the 15,472-line
//! Python `tools/msez.py` monolith with a structured Rust implementation.
//!
//! ## Subcommands
//!
//! - `msez validate` — Zone, module, and profile validation.
//! - `msez lock` — Lockfile generation and deterministic verification.
//! - `msez corridor` — Corridor lifecycle management.
//! - `msez artifact` — Content-addressed storage operations.
//! - `msez signing` — Ed25519 key generation and VC signing.
//!
//! ## Backward Compatibility
//!
//! The CLI interface matches the Python implementation exactly. Every
//! subcommand, every flag, every output format is preserved to ensure
//! CI pipeline compatibility:
//!
//! ```bash
//! msez validate --all-modules
//! msez validate --all-profiles
//! msez validate --all-zones
//! msez lock jurisdictions/_starter/zone.yaml --check
//! ```

pub mod artifact;
pub mod corridor;
pub mod lock;
pub mod signing;
pub mod validate;
