//! # SHA-256 Bypass Detection (B-005)
//!
//! Compile-time/test-time enforcement that all SHA-256 computation flows through
//! `msez-core::digest`. This test greps the workspace source for `sha2::Sha256`
//! usage and verifies it appears only in the three allowed locations:
//!
//! 1. `msez-core/src/canonical.rs` — canonicalization pipeline
//! 2. `msez-core/src/digest.rs` — digest module
//! 3. `msez-crypto/src/mmr.rs` — MMR node hashing (documented exception)
//!
//! Any other file using `sha2::Sha256` directly is a violation of CLAUDE.md §V.5.

use std::path::PathBuf;
use std::process::Command;

/// The workspace root for the msez crates.
fn workspace_root() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(manifest_dir)
        .parent() // up from msez-integration-tests
        .expect("parent of integration-tests")
        .parent() // up from crates/
        .expect("parent of crates/")
        .to_path_buf()
}

/// Allowed files that may use `sha2::Sha256` directly.
const ALLOWED_SHA256_FILES: &[&str] = &[
    "msez-core/src/canonical.rs",
    "msez-core/src/digest.rs",
    "msez-crypto/src/mmr.rs",
];

/// Verify that `sha2::Sha256` does not appear in any Cargo.toml
/// other than `msez-core` and `msez-crypto`.
#[test]
fn sha2_dependency_only_in_core_and_crypto() {
    let root = workspace_root();
    let crates_dir = root.join("crates");

    if !crates_dir.exists() {
        // Skip test if directory structure doesn't match expected layout.
        return;
    }

    let entries = std::fs::read_dir(&crates_dir).expect("read crates directory");

    for entry in entries {
        let entry = entry.expect("dir entry");
        let crate_name = entry.file_name().to_string_lossy().to_string();
        let cargo_toml = entry.path().join("Cargo.toml");

        if !cargo_toml.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&cargo_toml).expect("read Cargo.toml");

        // Only msez-core and msez-crypto may have sha2 as a direct dependency.
        // msez-crypto needs raw sha2 for MMR node hashing (documented exception
        // per CLAUDE.md §V.5 — needs raw [u8; 32], not hex).
        if crate_name != "msez-core" && crate_name != "msez-crypto" {
            let has_sha2_dep = content
                .lines()
                .any(|line| {
                    let trimmed = line.trim();
                    trimmed.starts_with("sha2")
                        && !trimmed.starts_with('#')
                        && !trimmed.starts_with("//")
                });

            assert!(
                !has_sha2_dep,
                "VIOLATION: {crate_name}/Cargo.toml has a direct sha2 dependency. \
                 Per CLAUDE.md §V.5, only msez-core and msez-crypto (MMR exception) \
                 may depend on sha2 directly. \
                 Use msez_core::sha256_digest() or msez_core::sha256_raw() instead."
            );
        }
    }
}

/// Verify that `sha2::Sha256` (or `use sha2::`) does not appear in source
/// files outside the three allowed locations.
#[test]
fn sha256_usage_only_in_allowed_files() {
    let root = workspace_root();
    let crates_dir = root.join("crates");

    if !crates_dir.exists() {
        return;
    }

    // Try using `grep -r` to find violations.
    let output = Command::new("grep")
        .args([
            "-rn",
            "--include=*.rs",
            "sha2::",
            crates_dir.to_str().unwrap_or("."),
        ])
        .output();

    let output = match output {
        Ok(o) => o,
        Err(_) => {
            // grep not available — fall back to manual scan.
            manual_scan_sha2(&crates_dir);
            return;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let is_allowed = ALLOWED_SHA256_FILES
            .iter()
            .any(|allowed| line.contains(allowed));

        // Skip test files.
        let is_test = line.contains("/tests/") || line.contains("#[cfg(test)]");

        if !is_allowed && !is_test {
            panic!(
                "VIOLATION (B-005): sha2:: usage outside allowed files:\n{line}\n\n\
                 Allowed files: {ALLOWED_SHA256_FILES:?}\n\
                 Per CLAUDE.md §V.5, use msez_core::sha256_digest() instead."
            );
        }
    }
}

/// Fallback scan when `grep` is not available.
fn manual_scan_sha2(crates_dir: &std::path::Path) {
    fn scan_dir(dir: &std::path::Path, violations: &mut Vec<String>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            if path.is_dir() {
                scan_dir(&path, violations);
            } else if path.extension().map_or(false, |ext| ext == "rs") {
                let content = match std::fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let path_str = path.to_string_lossy().to_string();

                let is_allowed = ALLOWED_SHA256_FILES
                    .iter()
                    .any(|allowed| path_str.contains(allowed));
                let is_test = path_str.contains("/tests/");

                if !is_allowed && !is_test {
                    for (i, line) in content.lines().enumerate() {
                        if line.contains("sha2::") && !line.trim_start().starts_with("//") {
                            violations
                                .push(format!("{}:{}: {}", path_str, i + 1, line.trim()));
                        }
                    }
                }
            }
        }
    }

    let mut violations = Vec::new();
    scan_dir(crates_dir, &mut violations);

    if !violations.is_empty() {
        panic!(
            "VIOLATION (B-005): sha2:: usage outside allowed files:\n{}\n\n\
             Allowed files: {:?}",
            violations.join("\n"),
            ALLOWED_SHA256_FILES
        );
    }
}
