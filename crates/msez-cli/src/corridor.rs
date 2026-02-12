//! # Corridor Subcommand
//!
//! Corridor lifecycle management: propose, activate, halt, suspend, resume.

use clap::Args;

/// Arguments for the corridor subcommand.
#[derive(Args, Debug)]
pub struct CorridorArgs {
    /// Corridor operation to perform.
    #[arg(long)]
    pub operation: Option<String>,
}
