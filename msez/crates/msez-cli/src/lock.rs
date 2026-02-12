//! # Lock Subcommand
//!
//! Lockfile generation and deterministic byte-level verification.

use std::path::Path;

/// Generate or verify a lockfile for a zone configuration.
pub fn lock_zone(_zone_path: &Path, _check: bool) {
    todo!("implement lockfile generation/verification")
}
