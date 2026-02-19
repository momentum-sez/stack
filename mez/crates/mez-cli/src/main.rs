//! # mez CLI entry point
//!
//! Parses command-line arguments and dispatches to subcommand handlers.
//! Uses clap derive macros for argument parsing with backward-compatible
//! subcommand structure matching the Python `tools/mez.py` CLI.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use mez_cli::artifact::{run_artifact, ArtifactArgs};
use mez_cli::corridor::{run_corridor, CorridorArgs};
use mez_cli::lock::{run_lock, LockArgs};
use mez_cli::regpack::{run_regpack, RegpackArgs};
use mez_cli::signing::{run_signing, SigningArgs};
use mez_cli::validate::{run_validate, ValidateArgs};
use mez_cli::zone::{run_zone, ZoneArgs};

/// MEZ Stack CLI — v0.4.44 GENESIS
///
/// Reference implementation of the EZ Stack toolchain. Provides zone/module
/// validation, deterministic lockfile generation, corridor lifecycle management,
/// content-addressed artifact storage, and Ed25519/VC signing.
#[derive(Parser, Debug)]
#[command(name = "mez", version = "0.4.44", about, long_about = None)]
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

    /// Build and verify content-addressed regpack artifacts.
    Regpack(RegpackArgs),

    /// Zone bootstrap and management operations.
    Zone(ZoneArgs),

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

    tracing::debug!("mez CLI v0.4.44 starting");

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
        Commands::Regpack(args) => run_regpack(&args, &repo_root),
        Commands::Zone(args) => run_zone(&args, &repo_root),
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
/// directories, matching the EZ Stack repository layout.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parse_validate_all_modules() {
        let cli = Cli::try_parse_from(["mez", "validate", "--all-modules"]).unwrap();
        assert!(matches!(cli.command, Commands::Validate(_)));
        if let Commands::Validate(args) = cli.command {
            assert!(args.all_modules);
            assert!(!args.all_profiles);
            assert!(!args.all_zones);
            assert!(args.path.is_none());
        }
    }

    #[test]
    fn cli_parse_validate_all_profiles() {
        let cli = Cli::try_parse_from(["mez", "validate", "--all-profiles"]).unwrap();
        if let Commands::Validate(args) = cli.command {
            assert!(args.all_profiles);
        }
    }

    #[test]
    fn cli_parse_validate_all_zones() {
        let cli = Cli::try_parse_from(["mez", "validate", "--all-zones"]).unwrap();
        if let Commands::Validate(args) = cli.command {
            assert!(args.all_zones);
        }
    }

    #[test]
    fn cli_parse_validate_with_path() {
        let cli = Cli::try_parse_from(["mez", "validate", "modules/test/module.yaml"]).unwrap();
        if let Commands::Validate(args) = cli.command {
            assert!(!args.all_modules);
            assert_eq!(args.path, Some(PathBuf::from("modules/test/module.yaml")));
        }
    }

    #[test]
    fn cli_parse_validate_combined_flags() {
        let cli = Cli::try_parse_from([
            "mez",
            "validate",
            "--all-modules",
            "--all-profiles",
            "--all-zones",
        ])
        .unwrap();
        if let Commands::Validate(args) = cli.command {
            assert!(args.all_modules);
            assert!(args.all_profiles);
            assert!(args.all_zones);
        }
    }

    #[test]
    fn cli_parse_lock_basic() {
        let cli = Cli::try_parse_from(["mez", "lock", "zone.yaml"]).unwrap();
        if let Commands::Lock(args) = cli.command {
            assert_eq!(args.zone, PathBuf::from("zone.yaml"));
            assert!(!args.check);
            assert!(args.out.is_none());
            assert!(args.generated_at.is_none());
            assert!(!args.strict);
        }
    }

    #[test]
    fn cli_parse_lock_with_check() {
        let cli = Cli::try_parse_from(["mez", "lock", "zone.yaml", "--check"]).unwrap();
        if let Commands::Lock(args) = cli.command {
            assert!(args.check);
        }
    }

    #[test]
    fn cli_parse_lock_with_all_options() {
        let cli = Cli::try_parse_from([
            "mez",
            "lock",
            "zone.yaml",
            "--check",
            "--out",
            "output.lock",
            "--generated-at",
            "2026-01-01T00:00:00Z",
            "--strict",
        ])
        .unwrap();
        if let Commands::Lock(args) = cli.command {
            assert!(args.check);
            assert_eq!(args.out, Some(PathBuf::from("output.lock")));
            assert_eq!(args.generated_at, Some("2026-01-01T00:00:00Z".to_string()));
            assert!(args.strict);
        }
    }

    #[test]
    fn cli_parse_corridor_create() {
        let cli = Cli::try_parse_from([
            "mez",
            "corridor",
            "create",
            "--id",
            "pk-ae",
            "--jurisdiction-a",
            "PK",
            "--jurisdiction-b",
            "AE",
        ])
        .unwrap();
        assert!(matches!(cli.command, Commands::Corridor(_)));
    }

    #[test]
    fn cli_parse_corridor_list() {
        let cli = Cli::try_parse_from(["mez", "corridor", "list"]).unwrap();
        assert!(matches!(cli.command, Commands::Corridor(_)));
    }

    #[test]
    fn cli_parse_corridor_status() {
        let cli = Cli::try_parse_from(["mez", "corridor", "status", "--id", "test"]).unwrap();
        assert!(matches!(cli.command, Commands::Corridor(_)));
    }

    #[test]
    fn cli_parse_artifact_store() {
        let cli = Cli::try_parse_from([
            "mez",
            "artifact",
            "store",
            "--artifact-type",
            "receipt",
            "data.json",
        ])
        .unwrap();
        assert!(matches!(cli.command, Commands::Artifact(_)));
    }

    #[test]
    fn cli_parse_artifact_resolve() {
        let hex = "a".repeat(64);
        let cli = Cli::try_parse_from([
            "mez",
            "artifact",
            "resolve",
            "--artifact-type",
            "vc",
            "--digest",
            &hex,
        ])
        .unwrap();
        assert!(matches!(cli.command, Commands::Artifact(_)));
    }

    #[test]
    fn cli_parse_artifact_verify() {
        let hex = "b".repeat(64);
        let cli = Cli::try_parse_from([
            "mez",
            "artifact",
            "verify",
            "--artifact-type",
            "receipt",
            "--digest",
            &hex,
        ])
        .unwrap();
        assert!(matches!(cli.command, Commands::Artifact(_)));
    }

    #[test]
    fn cli_parse_vc_keygen() {
        let cli = Cli::try_parse_from([
            "mez", "vc", "keygen", "--output", "/tmp", "--prefix", "test",
        ])
        .unwrap();
        assert!(matches!(cli.command, Commands::Signing(_)));
    }

    #[test]
    fn cli_parse_vc_sign() {
        let cli = Cli::try_parse_from([
            "mez",
            "vc",
            "sign",
            "--key",
            "private.key",
            "document.json",
        ])
        .unwrap();
        assert!(matches!(cli.command, Commands::Signing(_)));
    }

    #[test]
    fn cli_parse_vc_verify() {
        let sig_hex = "c".repeat(128);
        let cli = Cli::try_parse_from([
            "mez",
            "vc",
            "verify",
            "--pubkey",
            "public.pub",
            "document.json",
            "--signature",
            &sig_hex,
        ])
        .unwrap();
        assert!(matches!(cli.command, Commands::Signing(_)));
    }

    #[test]
    fn cli_parse_verbose_levels() {
        let cli0 = Cli::try_parse_from(["mez", "corridor", "list"]).unwrap();
        assert_eq!(cli0.verbose, 0);

        let cli1 = Cli::try_parse_from(["mez", "-v", "corridor", "list"]).unwrap();
        assert_eq!(cli1.verbose, 1);

        let cli2 = Cli::try_parse_from(["mez", "-vv", "corridor", "list"]).unwrap();
        assert_eq!(cli2.verbose, 2);

        let cli3 = Cli::try_parse_from(["mez", "-vvv", "corridor", "list"]).unwrap();
        assert_eq!(cli3.verbose, 3);
    }

    #[test]
    fn cli_parse_config_option() {
        let cli =
            Cli::try_parse_from(["mez", "--config", "mez.yaml", "corridor", "list"]).unwrap();
        assert_eq!(cli.config, Some(PathBuf::from("mez.yaml")));
    }

    #[test]
    fn cli_parse_output_dir_option() {
        let cli = Cli::try_parse_from(["mez", "--output-dir", "/tmp/output", "corridor", "list"])
            .unwrap();
        assert_eq!(cli.output_dir, Some(PathBuf::from("/tmp/output")));
    }

    #[test]
    fn cli_parse_no_subcommand_errors() {
        let result = Cli::try_parse_from(["mez"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_parse_invalid_subcommand_errors() {
        let result = Cli::try_parse_from(["mez", "nonexistent"]);
        assert!(result.is_err());
    }

    #[test]
    fn resolve_repo_root_returns_some_when_in_repo() {
        // The test is running from within the repo, so this should find it.
        let result = resolve_repo_root();
        // This may or may not work depending on CWD during testing.
        // We verify it either finds a root with schemas/ and modules/ or returns None.
        if let Some(root) = result {
            assert!(root.join("schemas").is_dir());
            assert!(root.join("modules").is_dir());
        }
    }

    #[test]
    fn cli_debug_impl() {
        let cli = Cli::try_parse_from(["mez", "corridor", "list"]).unwrap();
        let debug = format!("{cli:?}");
        assert!(debug.contains("Cli"));
    }

    #[test]
    fn commands_debug_impl() {
        let cli = Cli::try_parse_from(["mez", "corridor", "list"]).unwrap();
        let debug = format!("{:?}", cli.command);
        assert!(debug.contains("Corridor"));
    }

    #[test]
    fn cli_parse_corridor_submit() {
        let cli = Cli::try_parse_from([
            "mez",
            "corridor",
            "submit",
            "--id",
            "test-cor",
            "--agreement",
            "agreement.json",
            "--pack-trilogy",
            "trilogy.json",
        ])
        .unwrap();
        assert!(matches!(cli.command, Commands::Corridor(_)));
    }

    #[test]
    fn cli_parse_corridor_activate() {
        let cli = Cli::try_parse_from([
            "mez",
            "corridor",
            "activate",
            "--id",
            "test-cor",
            "--approval-a",
            "digest_a",
            "--approval-b",
            "digest_b",
        ])
        .unwrap();
        assert!(matches!(cli.command, Commands::Corridor(_)));
    }

    #[test]
    fn cli_parse_corridor_halt() {
        let cli = Cli::try_parse_from([
            "mez",
            "corridor",
            "halt",
            "--id",
            "test-cor",
            "--reason",
            "emergency",
            "--authority",
            "PK",
        ])
        .unwrap();
        assert!(matches!(cli.command, Commands::Corridor(_)));
    }

    #[test]
    fn cli_parse_corridor_suspend() {
        let cli = Cli::try_parse_from([
            "mez",
            "corridor",
            "suspend",
            "--id",
            "test-cor",
            "--reason",
            "maintenance",
        ])
        .unwrap();
        assert!(matches!(cli.command, Commands::Corridor(_)));
    }

    #[test]
    fn cli_parse_corridor_resume() {
        let cli = Cli::try_parse_from([
            "mez",
            "corridor",
            "resume",
            "--id",
            "test-cor",
            "--resolution",
            "resolved",
        ])
        .unwrap();
        assert!(matches!(cli.command, Commands::Corridor(_)));
    }
}
