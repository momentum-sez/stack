//! # Artifact Subcommand
//!
//! Content-addressed storage operations: store, resolve, verify, graph.

use clap::Args;

/// Arguments for the artifact subcommand.
#[derive(Args, Debug)]
pub struct ArtifactArgs {
    /// Artifact operation to perform.
    #[arg(long)]
    pub operation: Option<String>,
}
