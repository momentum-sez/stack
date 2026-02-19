//! # mez-cli — CLI Tool for the EZ Stack
//!
//! Provides the `mez` command-line interface, replacing the 15,472-line
//! Python `tools/mez.py` monolith with a structured Rust implementation.
//!
//! ## Subcommands
//!
//! - `mez validate` — Zone, module, and profile validation.
//! - `mez lock` — Lockfile generation and deterministic verification.
//! - `mez corridor` — Corridor lifecycle management.
//! - `mez artifact` — Content-addressed storage operations.
//! - `mez vc` — Ed25519 key generation and VC signing.
//!
//! ## Backward Compatibility
//!
//! The CLI interface matches the Python implementation exactly. Every
//! subcommand, every flag, every output format is preserved to ensure
//! CI pipeline compatibility:
//!
//! ```bash
//! mez validate --all-modules
//! mez validate --all-profiles
//! mez validate --all-zones
//! mez lock jurisdictions/_starter/zone.yaml --check
//! ```

pub mod artifact;
pub mod corridor;
pub mod lock;
pub mod regpack;
pub mod signing;
pub mod validate;
pub mod zone;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_spec_version_is_correct() {
        assert_eq!(STACK_SPEC_VERSION, "0.4.44");
    }

    #[test]
    fn resolve_path_absolute_path_returned_as_is() {
        let repo_root = Path::new("/some/repo");
        let abs_path = Path::new("/absolute/path/to/file.yaml");
        let result = resolve_path(abs_path, repo_root);
        assert_eq!(result, PathBuf::from("/absolute/path/to/file.yaml"));
    }

    #[test]
    fn resolve_path_relative_path_exists_in_repo_root() {
        let dir = tempfile::tempdir().unwrap();
        let repo_root = dir.path();
        // Create a file in the repo root.
        std::fs::write(repo_root.join("test.yaml"), b"content").unwrap();

        let result = resolve_path(Path::new("test.yaml"), repo_root);
        assert_eq!(result, repo_root.join("test.yaml"));
        assert!(result.exists());
    }

    #[test]
    fn resolve_path_relative_path_does_not_exist_in_repo_root() {
        let dir = tempfile::tempdir().unwrap();
        let repo_root = dir.path();
        // Do not create the file.

        let result = resolve_path(Path::new("missing.yaml"), repo_root);
        // Should return the path as-is (relative to CWD).
        assert_eq!(result, PathBuf::from("missing.yaml"));
    }

    #[test]
    fn resolve_path_relative_nested_path() {
        let dir = tempfile::tempdir().unwrap();
        let repo_root = dir.path();
        let sub = repo_root.join("sub").join("dir");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("data.json"), b"{}").unwrap();

        let result = resolve_path(Path::new("sub/dir/data.json"), repo_root);
        assert_eq!(result, repo_root.join("sub/dir/data.json"));
    }

    #[test]
    fn resolve_path_empty_relative_path() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve_path(Path::new(""), dir.path());
        // Empty path should match the repo root itself which typically exists.
        // Depending on OS, this may or may not exist. The function should not panic.
        let _ = result;
    }

    #[test]
    fn public_modules_are_accessible() {
        // Verify that the public module re-exports compile.
        let _ = std::any::type_name::<artifact::ArtifactArgs>();
        let _ = std::any::type_name::<corridor::CorridorArgs>();
        let _ = std::any::type_name::<lock::LockArgs>();
        let _ = std::any::type_name::<regpack::RegpackArgs>();
        let _ = std::any::type_name::<signing::SigningArgs>();
        let _ = std::any::type_name::<validate::ValidateArgs>();
        let _ = std::any::type_name::<zone::ZoneArgs>();
    }
}
