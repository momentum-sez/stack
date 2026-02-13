//! # Version Consistency Test
//!
//! Verifies that the canonical version 0.4.44 is consistently referenced
//! across the entire codebase — Rust workspace, Python tools, YAML configs,
//! documentation, and deployment artifacts.
//!
//! ## Audit Reference
//!
//! The Feb 2026 audit found version inconsistencies (0.1.0 in Cargo.toml,
//! 0.5.0 in Dockerfile) that have been corrected. This test prevents
//! regression.

use std::path::PathBuf;

/// The single canonical version for the entire stack.
const CANONICAL_VERSION: &str = "0.4.44";

/// Locate the repository root (parent of the msez/ workspace directory).
fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // msez/crates/msez-integration-tests -> msez/ -> stack/
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("Could not find repo root from CARGO_MANIFEST_DIR")
        .to_path_buf()
}

#[test]
fn rust_workspace_version_matches_canonical() {
    let cargo_toml = repo_root().join("msez/Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", cargo_toml.display()));
    assert!(
        content.contains(&format!("version = \"{CANONICAL_VERSION}\"")),
        "Rust workspace Cargo.toml must contain version = \"{CANONICAL_VERSION}\""
    );
}

#[test]
fn version_file_matches_canonical() {
    let version_file = repo_root().join("VERSION");
    let content = std::fs::read_to_string(&version_file)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", version_file.display()));
    let trimmed = content.trim();
    assert!(
        trimmed.starts_with(CANONICAL_VERSION),
        "VERSION file must start with {CANONICAL_VERSION}, got: {trimmed}"
    );
}

#[test]
fn python_msez_version_matches_canonical() {
    let init = repo_root().join("tools/msez/__init__.py");
    let content = std::fs::read_to_string(&init)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", init.display()));
    assert!(
        content.contains(&format!("__version__ = \"{CANONICAL_VERSION}\"")),
        "tools/msez/__init__.py must contain __version__ = \"{CANONICAL_VERSION}\""
    );
}

#[test]
fn python_phoenix_version_matches_canonical() {
    let init = repo_root().join("tools/phoenix/__init__.py");
    let content = std::fs::read_to_string(&init)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", init.display()));
    assert!(
        content.contains(&format!("__version__ = \"{CANONICAL_VERSION}\"")),
        "tools/phoenix/__init__.py must contain __version__ = \"{CANONICAL_VERSION}\""
    );
}

#[test]
fn dockerfile_version_matches_canonical() {
    let dockerfile = repo_root().join("deploy/docker/Dockerfile");
    let content = std::fs::read_to_string(&dockerfile)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", dockerfile.display()));
    assert!(
        content.contains(&format!("image.version=\"{CANONICAL_VERSION}\"")),
        "Dockerfile must contain image.version=\"{CANONICAL_VERSION}\""
    );
}

#[test]
fn openapi_version_matches_canonical() {
    let openapi = repo_root().join("msez/crates/msez-api/src/openapi.rs");
    let content = std::fs::read_to_string(&openapi)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", openapi.display()));
    assert!(
        content.contains(&format!("version = \"{CANONICAL_VERSION}\"")),
        "OpenAPI spec in msez-api must contain version = \"{CANONICAL_VERSION}\""
    );
}

#[test]
fn cli_version_matches_canonical() {
    let main_rs = repo_root().join("msez/crates/msez-cli/src/main.rs");
    let content = std::fs::read_to_string(&main_rs)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", main_rs.display()));
    assert!(
        content.contains(&format!("version = \"{CANONICAL_VERSION}\"")),
        "CLI main.rs must contain version = \"{CANONICAL_VERSION}\""
    );
}

#[test]
fn readme_references_canonical_version() {
    let readme = repo_root().join("README.md");
    let content = std::fs::read_to_string(&readme)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", readme.display()));
    assert!(
        content.contains(CANONICAL_VERSION),
        "Root README.md must reference {CANONICAL_VERSION}"
    );
}

#[test]
fn starter_zone_version_matches_canonical() {
    let zone = repo_root().join("jurisdictions/_starter/zone.yaml");
    let content = std::fs::read_to_string(&zone)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", zone.display()));
    assert!(
        content.contains(&format!("version: {CANONICAL_VERSION}"))
            || content.contains(&format!("version: \"{CANONICAL_VERSION}\"")),
        "Starter zone.yaml must contain version: {CANONICAL_VERSION}"
    );
}

#[test]
fn all_child_crates_inherit_workspace_version() {
    let crates_dir = repo_root().join("msez/crates");
    let entries = std::fs::read_dir(&crates_dir)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", crates_dir.display()));
    for entry in entries.flatten() {
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            let cargo = entry.path().join("Cargo.toml");
            if cargo.exists() {
                let content = std::fs::read_to_string(&cargo).unwrap();
                assert!(
                    content.contains("version.workspace = true"),
                    "{}: must use version.workspace = true to inherit workspace version",
                    cargo.display()
                );
            }
        }
    }
}
