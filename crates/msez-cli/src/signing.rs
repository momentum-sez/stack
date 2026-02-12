//! # Sign Subcommand
//!
//! Ed25519 key generation, document signing, and VC proof creation.

use clap::Args;

/// Arguments for the sign subcommand.
#[derive(Args, Debug)]
pub struct SignArgs {
    /// Signing operation to perform.
    #[arg(long)]
    pub operation: Option<String>,
}
