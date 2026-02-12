//! # msez CLI entry point
//!
//! Parses command-line arguments and dispatches to subcommand handlers.
//! Uses clap derive macros for argument parsing with backward-compatible
//! subcommand structure matching the Python `tools/msez.py` CLI.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use msez_cli::artifact::{run_artifact, ArtifactArgs};
use msez_cli::corridor::{run_corridor, CorridorArgs};
use msez_cli::lock::{run_lock, LockArgs};
use msez_cli::signing::{run_signing, SigningArgs};
use msez_cli::validate::{run_validate, ValidateArgs};

/// MSEZ Stack CLI — v0.4.44 GENESIS
///
/// Reference implementation of the SEZ Stack toolchain. Provides zone/module
/// validation, deterministic lockfile generation, corridor lifecycle management,
/// content-addressed artifact storage, and Ed25519/VC signing.
#[derive(Parser, Debug)]
#[command(name = "msez", version = "0.4.44", about, long_about = None)]
struct Cli {
    /// Enable verbose output. Repeat for more verbosity (-v, -vv, -vvv).
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Path to configuration file.
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    /// Output directory for generated artifacts.
    #[arg(long, global = true)]
    output_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Validate modules, profiles, and zones against their schemas.
    Validate(ValidateArgs),

    /// Generate or verify a deterministic lockfile for a zone configuration.
    Lock(LockArgs),

    /// Corridor lifecycle management (create, submit, activate, halt, etc.).
    Corridor(CorridorArgs),

    /// Content-addressed storage operations (store, resolve, verify).
    Artifact(ArtifactArgs),

    /// Ed25519 key generation, VC signing, and signature verification.
    #[command(name = "vc")]
    Signing(SigningArgs),
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Initialize tracing based on verbosity level.
    let filter = match cli.verbose {
        0 => EnvFilter::new("warn"),
        1 => EnvFilter::new("info"),
        2 => EnvFilter::new("debug"),
        _ => EnvFilter::new("trace"),
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    tracing::debug!("msez CLI v0.4.44 starting");

    // Resolve the repository root: walk up from CWD looking for `schemas/` and `modules/`.
    let repo_root = resolve_repo_root().unwrap_or_else(|| {
        tracing::warn!("Could not locate repository root; using current directory");
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    });

    tracing::debug!(repo_root = %repo_root.display(), "resolved repository root");

    let result = match cli.command {
        Commands::Validate(args) => run_validate(&args, &repo_root),
        Commands::Lock(args) => run_lock(&args, &repo_root),
        Commands::Corridor(args) => run_corridor(&args, &repo_root),
        Commands::Artifact(args) => run_artifact(&args, &repo_root),
        Commands::Signing(args) => run_signing(&args, &repo_root),
    };

    match result {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            tracing::error!("{e:#}");
            ExitCode::from(1)
        }
    }
}

/// Walk up from the current directory to find the repository root.
///
/// The repo root is identified by the presence of both `schemas/` and `modules/`
/// directories, matching the SEZ Stack repository layout.
fn resolve_repo_root() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let mut dir = cwd.as_path();
    loop {
        if dir.join("schemas").is_dir() && dir.join("modules").is_dir() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
}
