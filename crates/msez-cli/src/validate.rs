//! # Validate Subcommand
//!
//! Zone, module, and profile validation commands.
//! Preserves the interface of `python -m tools.msez validate --all-modules`.

use clap::Args;

/// Arguments for the validate subcommand.
#[derive(Args, Debug)]
pub struct ValidateArgs {
    /// Validate all modules.
    #[arg(long)]
    pub all_modules: bool,

    /// Validate all profiles.
    #[arg(long)]
    pub all_profiles: bool,

    /// Validate all zones.
    #[arg(long)]
    pub all_zones: bool,
}
