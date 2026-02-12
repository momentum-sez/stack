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
//! - `msez vc` — Ed25519 key generation and VC signing.
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

use std::path::{Path, PathBuf};

/// Stack specification version constant, matching the Python implementation.
pub const STACK_SPEC_VERSION: &str = "0.4.44";

/// Resolve a path that may be relative to the repository root.
///
/// If the path is absolute, returns it as-is. If relative and the file
/// exists relative to `repo_root`, uses that. Otherwise returns the path
/// relative to the current directory.
pub fn resolve_path(path: &Path, repo_root: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    let repo_relative = repo_root.join(path);
    if repo_relative.exists() {
        repo_relative
    } else {
        path.to_path_buf()
    }
}
