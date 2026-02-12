//! # Lock Subcommand
//!
//! Lockfile generation and deterministic verification.
//! Preserves the interface of `python -m tools.msez lock <zone.yaml> --check`.

use clap::Args;

/// Arguments for the lock subcommand.
#[derive(Args, Debug)]
pub struct LockArgs {
    /// Path to the zone YAML file.
    pub zone_file: String,

    /// Verify lockfile without regenerating.
    #[arg(long)]
    pub check: bool,
}
