//! # msez CLI Entry Point
//!
//! Assembles subcommands and dispatches to handler modules.

use clap::Parser;

/// SEZ Stack CLI — Sovereign Economic Zone toolchain.
///
/// Validates zone configurations, manages lockfiles, operates corridors,
/// and performs cryptographic signing for the SEZ Stack.
#[derive(Parser, Debug)]
#[command(name = "msez", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Validate zones, modules, and profiles.
    Validate(msez_cli::validate::ValidateArgs),
    /// Generate or verify lockfiles.
    Lock(msez_cli::lock::LockArgs),
    /// Corridor lifecycle management.
    Corridor(msez_cli::corridor::CorridorArgs),
    /// Content-addressed storage operations.
    Artifact(msez_cli::artifact::ArtifactArgs),
    /// Ed25519 and VC signing operations.
    Sign(msez_cli::signing::SignArgs),
}

fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Validate(_args) => {
            // TODO: Dispatch to validate handler
            tracing::info!("Validate command — not yet implemented");
        }
        Commands::Lock(_args) => {
            // TODO: Dispatch to lock handler
            tracing::info!("Lock command — not yet implemented");
        }
        Commands::Corridor(_args) => {
            // TODO: Dispatch to corridor handler
            tracing::info!("Corridor command — not yet implemented");
        }
        Commands::Artifact(_args) => {
            // TODO: Dispatch to artifact handler
            tracing::info!("Artifact command — not yet implemented");
        }
        Commands::Sign(_args) => {
            // TODO: Dispatch to sign handler
            tracing::info!("Sign command — not yet implemented");
        }
    }

    Ok(())
}
