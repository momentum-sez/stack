//! # Lock Subcommand
//!
//! Lockfile generation and deterministic byte-level verification.
//!
//! Generates a deterministic `stack.lock` file from a zone configuration by
//! resolving all module references, computing content digests, and serializing
//! the result using JCS-compatible canonicalization via `CanonicalBytes`.
//!
//! ## Spec Reference
//!
//! Matches the behavior of `tools/msez.py lock` which produces canonical JSON
//! lockfiles with SHA-256 content digests for every module and lawpack.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::Args;

use msez_core::{CanonicalBytes, Sha256Hasher};

/// Arguments for the `msez lock` subcommand.
#[derive(Args, Debug)]
pub struct LockArgs {
    /// Path to zone.yaml file.
    #[arg(value_name = "ZONE_YAML")]
    pub zone: PathBuf,

    /// Verify existing lockfile matches instead of generating.
    #[arg(long)]
    pub check: bool,

    /// Output path for the generated lockfile.
    #[arg(long, short)]
    pub out: Option<PathBuf>,

    /// Override the generated_at timestamp for deterministic output.
    #[arg(long)]
    pub generated_at: Option<String>,

    /// Enforce strict determinism (requires pinned generated_at).
    #[arg(long)]
    pub strict: bool,

    /// Output format (canonical-json or yaml).
    #[arg(long, default_value = "canonical-json")]
    pub format: String,
}

/// Execute the lock subcommand.
///
/// Returns exit code: 0 on success, 1 if --check fails, 2 on operational error.
pub fn run_lock(args: &LockArgs, repo_root: &Path) -> Result<u8> {
    let zone_path = crate::resolve_path(&args.zone, repo_root);

    if !zone_path.exists() {
        bail!("zone file not found: {}", zone_path.display());
    }

    // Parse zone YAML.
    let zone_content = std::fs::read_to_string(&zone_path)
        .with_context(|| format!("failed to read zone file: {}", zone_path.display()))?;
    let zone: serde_json::Value = serde_yaml::from_str(&zone_content)
        .with_context(|| format!("failed to parse zone YAML: {}", zone_path.display()))?;

    // Extract zone metadata.
    let zone_id = zone
        .get("zone_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let profile_id = zone
        .pointer("/profile/profile_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let profile_version = zone
        .pointer("/profile/version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0")
        .to_string();

    // Determine output path.
    let lockfile_path_from_zone = zone
        .get("lockfile_path")
        .and_then(|v| v.as_str())
        .unwrap_or("stack.lock");

    let out_path = if let Some(ref out) = args.out {
        crate::resolve_path(out, repo_root)
    } else {
        zone_path
            .parent()
            .unwrap_or(Path::new("."))
            .join(lockfile_path_from_zone)
    };

    // Determine generated_at timestamp.
    let generated_at = resolve_generated_at(args, &out_path)?;

    // Find profile.
    let profiles_dir = repo_root.join("profiles");
    let profile = find_profile(&profiles_dir, &profile_id)?;

    // Resolve modules from profile.
    let modules_dir = repo_root.join("modules");
    let mut module_entries = Vec::new();

    if let Some(modules) = profile.get("modules").and_then(|v| v.as_array()) {
        for m in modules {
            let mid = m
                .get("module_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            let variant = m
                .get("variant")
                .and_then(|v| v.as_str())
                .unwrap_or("default");

            let params = m.get("params").cloned().unwrap_or(serde_json::json!({}));

            // Find module directory.
            if let Some((mdir, mdata)) = find_module_by_id(&modules_dir, mid) {
                let manifest_sha256 = sha256_file(&mdir.join("module.yaml"))?;
                let content_sha256 = digest_dir(&mdir)?;
                let provides = mdata
                    .get("provides")
                    .cloned()
                    .unwrap_or(serde_json::json!([]));

                let entry = serde_json::json!({
                    "module_id": mid,
                    "version": mdata.get("version").and_then(|v| v.as_str()).unwrap_or("0.0.0"),
                    "variant": variant,
                    "params": params,
                    "manifest_sha256": manifest_sha256,
                    "content_sha256": content_sha256,
                    "provides": provides,
                });
                module_entries.push(entry);
            } else {
                tracing::warn!(module_id = mid, "module not found, skipping");
            }
        }
    }

    // Build lockfile object.
    let lock = serde_json::json!({
        "stack_spec_version": crate::STACK_SPEC_VERSION,
        "generated_at": generated_at,
        "zone_id": zone_id,
        "profile": {
            "profile_id": profile_id,
            "version": profile_version,
        },
        "modules": module_entries,
        "lawpacks": [],
        "overlays": [],
        "corridors": [],
    });

    // Serialize with canonical JSON.
    let canonical = CanonicalBytes::new(&lock).context("failed to canonicalize lockfile")?;
    let canonical_bytes = canonical.as_bytes();

    if args.check {
        // Check mode: compare against existing lockfile.
        if !out_path.exists() {
            println!("FAIL: lockfile does not exist: {}", out_path.display());
            return Ok(1);
        }

        let existing = std::fs::read(&out_path)
            .with_context(|| format!("failed to read lockfile: {}", out_path.display()))?;

        // Allow trailing newline.
        let matches =
            existing == canonical_bytes || existing == [canonical_bytes, b"\n".as_slice()].concat();

        if matches {
            println!("OK: lockfile is up to date");
            Ok(0)
        } else {
            println!("FAIL: lockfile is outdated or differs from computed lockfile");
            println!("  Expected digest: {}", sha256_of_bytes(canonical_bytes));
            println!("  Existing digest: {}", sha256_of_bytes(&existing));
            Ok(1)
        }
    } else {
        // Write mode: generate the lockfile.
        let output = [canonical_bytes, b"\n"].concat();
        std::fs::write(&out_path, &output)
            .with_context(|| format!("failed to write lockfile: {}", out_path.display()))?;
        println!("OK: wrote lockfile to {}", out_path.display());
        Ok(0)
    }
}

/// Resolve the generated_at timestamp.
///
/// Priority:
/// 1. Explicit --generated-at flag
/// 2. Existing lockfile's generated_at (for --check stability)
/// 3. SOURCE_DATE_EPOCH environment variable
/// 4. Current UTC time
fn resolve_generated_at(args: &LockArgs, out_path: &Path) -> Result<String> {
    if let Some(ref ts) = args.generated_at {
        return Ok(ts.clone());
    }

    // Try to reuse existing lockfile's timestamp for stability.
    if out_path.exists() {
        if let Ok(content) = std::fs::read_to_string(out_path) {
            if let Ok(existing) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(ts) = existing.get("generated_at").and_then(|v| v.as_str()) {
                    if !ts.is_empty() {
                        return Ok(ts.to_string());
                    }
                }
            }
        }
    }

    // Try SOURCE_DATE_EPOCH.
    if let Ok(epoch_str) = std::env::var("SOURCE_DATE_EPOCH") {
        if let Ok(epoch) = epoch_str.parse::<i64>() {
            if let Some(dt) = chrono::DateTime::from_timestamp(epoch, 0) {
                return Ok(dt.format("%Y-%m-%dT%H:%M:%SZ").to_string());
            }
        }
    }

    if args.strict || args.check {
        bail!(
            "--strict/--check requires a deterministic generated_at \
             (use --generated-at or SOURCE_DATE_EPOCH)"
        );
    }

    // Use current UTC time.
    let now = chrono::Utc::now();
    Ok(now.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

/// Find a profile by its profile_id in the profiles directory.
fn find_profile(profiles_dir: &Path, profile_id: &str) -> Result<serde_json::Value> {
    if !profiles_dir.is_dir() {
        bail!("profiles directory not found: {}", profiles_dir.display());
    }

    for entry in walkdir_recursive(profiles_dir) {
        if entry.file_name().and_then(|f| f.to_str()) == Some("profile.yaml") {
            let content = match std::fs::read_to_string(&entry) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(
                        path = %entry.display(),
                        error = %e,
                        "failed to read profile.yaml — file may be corrupted or inaccessible"
                    );
                    continue;
                }
            };
            let profile: serde_json::Value = match serde_yaml::from_str(&content) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!(
                        path = %entry.display(),
                        error = %e,
                        "failed to parse profile.yaml — file contains invalid YAML"
                    );
                    continue;
                }
            };
            if profile.get("profile_id").and_then(|v| v.as_str()) == Some(profile_id) {
                return Ok(profile);
            }
        }
    }

    bail!("profile not found for profile_id: {profile_id}")
}

/// Find a module by its module_id in the modules directory.
///
/// Skips unreadable or malformed `module.yaml` files rather than aborting
/// the entire search — a single broken file must not hide other modules.
fn find_module_by_id(modules_dir: &Path, module_id: &str) -> Option<(PathBuf, serde_json::Value)> {
    for entry in walkdir_recursive(modules_dir) {
        if entry.file_name().and_then(|f| f.to_str()) == Some("module.yaml") {
            let content = match std::fs::read_to_string(&entry) {
                Ok(c) => c,
                Err(e) => {
                    tracing::debug!(path = %entry.display(), error = %e, "skipping unreadable module.yaml");
                    continue;
                }
            };
            let module: serde_json::Value = match serde_yaml::from_str(&content) {
                Ok(m) => m,
                Err(e) => {
                    tracing::debug!(path = %entry.display(), error = %e, "skipping malformed module.yaml");
                    continue;
                }
            };
            if module.get("module_id").and_then(|v| v.as_str()) == Some(module_id) {
                return Some((entry.parent()?.to_path_buf(), module));
            }
        }
    }
    None
}

/// Compute SHA-256 hex digest of a file's contents.
fn sha256_file(path: &Path) -> Result<String> {
    let bytes =
        std::fs::read(path).with_context(|| format!("failed to read file: {}", path.display()))?;
    Ok(sha256_of_bytes(&bytes))
}

/// Compute SHA-256 hex digest of raw bytes.
fn sha256_of_bytes(bytes: &[u8]) -> String {
    msez_core::sha256_hex(bytes)
}

/// Compute a deterministic digest over a directory's contents.
///
/// Walks all files in sorted order, hashing their relative paths and contents.
fn digest_dir(dir: &Path) -> Result<String> {
    let mut hasher = Sha256Hasher::new();
    let mut paths: Vec<PathBuf> = Vec::new();

    for entry in walkdir_recursive(dir) {
        if entry.is_file() {
            if let Ok(rel) = entry.strip_prefix(dir) {
                paths.push(rel.to_path_buf());
            }
        }
    }
    paths.sort();

    for rel_path in &paths {
        let full = dir.join(rel_path);
        let content = std::fs::read(&full)?;
        // Use forward slashes for cross-platform determinism.
        let path_str = rel_path.to_string_lossy().replace('\\', "/");
        hasher.update(path_str.as_bytes());
        hasher.update(b"\0");
        hasher.update(&content);
        hasher.update(b"\0");
    }

    Ok(hasher.finalize_hex())
}

/// Recursively walk a directory, returning all file paths sorted.
fn walkdir_recursive(dir: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    walk_recursive_inner(dir, &mut results);
    results.sort();
    results
}

fn walk_recursive_inner(dir: &Path, acc: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(
                dir = %dir.display(),
                error = %e,
                "failed to read directory during recursive walk"
            );
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(
                    dir = %dir.display(),
                    error = %e,
                    "failed to read directory entry"
                );
                continue;
            }
        };
        let path = entry.path();
        if path.is_dir() {
            walk_recursive_inner(&path, acc);
        } else {
            acc.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_of_bytes_known_vector() {
        let digest = sha256_of_bytes(b"");
        assert_eq!(
            digest,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_of_bytes_known_vector_hello() {
        // echo -n "hello" | sha256sum
        let digest = sha256_of_bytes(b"hello");
        assert_eq!(
            digest,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn sha256_file_computes_correctly() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, b"hello").unwrap();
        let digest = sha256_file(&file_path).unwrap();
        assert_eq!(digest.len(), 64);
        assert_eq!(
            digest,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn sha256_file_nonexistent_returns_error() {
        let result = sha256_file(Path::new("/tmp/msez-no-such-file-abc123.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn digest_dir_is_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), b"aaa").unwrap();
        std::fs::write(dir.path().join("b.txt"), b"bbb").unwrap();

        let d1 = digest_dir(dir.path()).unwrap();
        let d2 = digest_dir(dir.path()).unwrap();
        assert_eq!(d1, d2);
    }

    #[test]
    fn digest_dir_changes_with_content() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), b"aaa").unwrap();
        let d1 = digest_dir(dir.path()).unwrap();

        std::fs::write(dir.path().join("a.txt"), b"bbb").unwrap();
        let d2 = digest_dir(dir.path()).unwrap();
        assert_ne!(d1, d2, "Digest should change when file content changes");
    }

    #[test]
    fn digest_dir_changes_with_new_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), b"aaa").unwrap();
        let d1 = digest_dir(dir.path()).unwrap();

        std::fs::write(dir.path().join("b.txt"), b"bbb").unwrap();
        let d2 = digest_dir(dir.path()).unwrap();
        assert_ne!(d1, d2, "Digest should change when new file is added");
    }

    #[test]
    fn digest_dir_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let d = digest_dir(dir.path()).unwrap();
        assert_eq!(d.len(), 64);
        // SHA-256 of empty input.
        assert_eq!(
            d,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn digest_dir_includes_nested_files() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("subdir");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("nested.txt"), b"nested").unwrap();

        let d = digest_dir(dir.path()).unwrap();
        assert_eq!(d.len(), 64);
        // Should not be the empty digest.
        assert_ne!(
            d,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn walkdir_recursive_returns_sorted() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("z.txt"), b"z").unwrap();
        std::fs::write(dir.path().join("a.txt"), b"a").unwrap();
        std::fs::write(dir.path().join("m.txt"), b"m").unwrap();

        let results = walkdir_recursive(dir.path());
        // Should be sorted.
        let sorted: Vec<_> = {
            let mut v = results.clone();
            v.sort();
            v
        };
        assert_eq!(results, sorted);
    }

    #[test]
    fn walkdir_recursive_nonexistent_dir() {
        let results = walkdir_recursive(Path::new("/tmp/msez-no-such-dir-xyz123"));
        assert!(results.is_empty());
    }

    #[test]
    fn resolve_generated_at_explicit_flag() {
        let dir = tempfile::tempdir().unwrap();
        let out_path = dir.path().join("stack.lock");

        let args = LockArgs {
            zone: PathBuf::from("zone.yaml"),
            check: false,
            out: None,
            generated_at: Some("2026-01-15T00:00:00Z".to_string()),
            strict: false,
            format: "canonical-json".to_string(),
        };
        let result = resolve_generated_at(&args, &out_path).unwrap();
        assert_eq!(result, "2026-01-15T00:00:00Z");
    }

    #[test]
    fn resolve_generated_at_from_existing_lockfile() {
        let dir = tempfile::tempdir().unwrap();
        let out_path = dir.path().join("stack.lock");
        let lock_content = serde_json::json!({
            "generated_at": "2026-02-01T12:00:00Z",
            "zone_id": "test"
        });
        std::fs::write(&out_path, serde_json::to_string(&lock_content).unwrap()).unwrap();

        let args = LockArgs {
            zone: PathBuf::from("zone.yaml"),
            check: false,
            out: None,
            generated_at: None,
            strict: false,
            format: "canonical-json".to_string(),
        };
        let result = resolve_generated_at(&args, &out_path).unwrap();
        assert_eq!(result, "2026-02-01T12:00:00Z");
    }

    #[test]
    fn resolve_generated_at_existing_lockfile_empty_timestamp_fallthrough() {
        let dir = tempfile::tempdir().unwrap();
        let out_path = dir.path().join("stack.lock");
        // Lockfile with empty generated_at should fall through.
        let lock_content = serde_json::json!({
            "generated_at": "",
            "zone_id": "test"
        });
        std::fs::write(&out_path, serde_json::to_string(&lock_content).unwrap()).unwrap();

        let args = LockArgs {
            zone: PathBuf::from("zone.yaml"),
            check: false,
            out: None,
            generated_at: None,
            strict: false,
            format: "canonical-json".to_string(),
        };
        // Not strict, no SOURCE_DATE_EPOCH; should use current time.
        let result = resolve_generated_at(&args, &out_path).unwrap();
        assert!(result.contains("T"), "Should be ISO timestamp: {result}");
    }

    #[test]
    fn resolve_generated_at_strict_without_timestamp_errors() {
        let dir = tempfile::tempdir().unwrap();
        let out_path = dir.path().join("stack.lock");

        let args = LockArgs {
            zone: PathBuf::from("zone.yaml"),
            check: false,
            out: None,
            generated_at: None,
            strict: true,
            format: "canonical-json".to_string(),
        };
        let result = resolve_generated_at(&args, &out_path);
        assert!(result.is_err(), "Strict mode without timestamp should fail");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("deterministic"));
    }

    #[test]
    fn resolve_generated_at_check_without_timestamp_errors() {
        let dir = tempfile::tempdir().unwrap();
        let out_path = dir.path().join("stack.lock");

        let args = LockArgs {
            zone: PathBuf::from("zone.yaml"),
            check: true,
            out: None,
            generated_at: None,
            strict: false,
            format: "canonical-json".to_string(),
        };
        let result = resolve_generated_at(&args, &out_path);
        assert!(result.is_err(), "Check mode without timestamp should fail");
    }

    #[test]
    fn resolve_generated_at_fallback_to_current_time() {
        let dir = tempfile::tempdir().unwrap();
        let out_path = dir.path().join("nonexistent.lock");

        let args = LockArgs {
            zone: PathBuf::from("zone.yaml"),
            check: false,
            out: None,
            generated_at: None,
            strict: false,
            format: "canonical-json".to_string(),
        };
        let result = resolve_generated_at(&args, &out_path).unwrap();
        // Should be an ISO 8601 timestamp.
        assert!(result.contains("T"));
        assert!(result.ends_with("Z"));
    }

    #[test]
    fn find_profile_missing_dir_errors() {
        let result = find_profile(
            Path::new("/tmp/msez-no-profiles-dir-xyz123"),
            "test_profile",
        );
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("profiles directory not found"));
    }

    #[test]
    fn find_profile_not_found_errors() {
        let dir = tempfile::tempdir().unwrap();
        let profiles_dir = dir.path().join("profiles");
        std::fs::create_dir_all(&profiles_dir).unwrap();
        // No profile.yaml files inside.

        let result = find_profile(&profiles_dir, "nonexistent_profile");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("profile not found"));
    }

    #[test]
    fn find_profile_finds_matching_profile() {
        let dir = tempfile::tempdir().unwrap();
        let profiles_dir = dir.path();
        let sub = profiles_dir.join("my_profile");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(
            sub.join("profile.yaml"),
            "profile_id: test_profile\nversion: '1.0.0'\nmodules: []\n",
        )
        .unwrap();

        let result = find_profile(profiles_dir, "test_profile");
        assert!(result.is_ok());
        let profile = result.unwrap();
        assert_eq!(
            profile.get("profile_id").and_then(|v| v.as_str()),
            Some("test_profile")
        );
    }

    #[test]
    fn find_module_by_id_not_found() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("modules")).unwrap();
        let result = find_module_by_id(&dir.path().join("modules"), "nonexistent_module");
        assert!(result.is_none());
    }

    #[test]
    fn find_module_by_id_finds_matching_module() {
        let dir = tempfile::tempdir().unwrap();
        let mod_dir = dir.path().join("family").join("my_mod");
        std::fs::create_dir_all(&mod_dir).unwrap();
        std::fs::write(
            mod_dir.join("module.yaml"),
            "module_id: org.test.mymod\nversion: '0.1.0'\n",
        )
        .unwrap();

        let result = find_module_by_id(dir.path(), "org.test.mymod");
        assert!(result.is_some());
        let (path, data) = result.unwrap();
        assert!(path.ends_with("my_mod"));
        assert_eq!(
            data.get("module_id").and_then(|v| v.as_str()),
            Some("org.test.mymod")
        );
    }

    #[test]
    fn run_lock_zone_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let args = LockArgs {
            zone: PathBuf::from("nonexistent_zone.yaml"),
            check: false,
            out: None,
            generated_at: Some("2026-01-01T00:00:00Z".to_string()),
            strict: false,
            format: "canonical-json".to_string(),
        };
        let result = run_lock(&args, dir.path());
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("zone file not found"));
    }

    #[test]
    fn run_lock_invalid_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let zone_path = dir.path().join("zone.yaml");
        std::fs::write(&zone_path, b"[invalid yaml: {{{").unwrap();

        let args = LockArgs {
            zone: zone_path,
            check: false,
            out: None,
            generated_at: Some("2026-01-01T00:00:00Z".to_string()),
            strict: false,
            format: "canonical-json".to_string(),
        };
        let result = run_lock(&args, dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn run_lock_generates_lockfile() {
        let dir = tempfile::tempdir().unwrap();
        // Create a minimal zone.yaml.
        let zone_content =
            "zone_id: test-zone\nprofile:\n  profile_id: test-profile\n  version: '1.0.0'\n";
        let zone_path = dir.path().join("zone.yaml");
        std::fs::write(&zone_path, zone_content).unwrap();

        // Create profiles dir with matching profile.
        let profiles_dir = dir.path().join("profiles");
        let profile_sub = profiles_dir.join("test");
        std::fs::create_dir_all(&profile_sub).unwrap();
        std::fs::write(
            profile_sub.join("profile.yaml"),
            "profile_id: test-profile\nversion: '1.0.0'\nmodules: []\n",
        )
        .unwrap();

        // Create empty modules dir.
        std::fs::create_dir_all(dir.path().join("modules")).unwrap();

        let out_path = dir.path().join("stack.lock");
        let args = LockArgs {
            zone: zone_path,
            check: false,
            out: Some(out_path.clone()),
            generated_at: Some("2026-01-15T00:00:00Z".to_string()),
            strict: false,
            format: "canonical-json".to_string(),
        };

        let result = run_lock(&args, dir.path()).unwrap();
        assert_eq!(result, 0);
        assert!(out_path.exists(), "Lockfile should be created");

        let content = std::fs::read_to_string(&out_path).unwrap();
        assert!(content.contains("test-zone"));
        assert!(content.contains("test-profile"));
    }

    #[test]
    fn run_lock_check_mode_passes_when_matches() {
        let dir = tempfile::tempdir().unwrap();
        let zone_content =
            "zone_id: test-zone\nprofile:\n  profile_id: test-profile\n  version: '1.0.0'\n";
        let zone_path = dir.path().join("zone.yaml");
        std::fs::write(&zone_path, zone_content).unwrap();

        let profiles_dir = dir.path().join("profiles");
        let profile_sub = profiles_dir.join("test");
        std::fs::create_dir_all(&profile_sub).unwrap();
        std::fs::write(
            profile_sub.join("profile.yaml"),
            "profile_id: test-profile\nversion: '1.0.0'\nmodules: []\n",
        )
        .unwrap();

        std::fs::create_dir_all(dir.path().join("modules")).unwrap();

        let out_path = dir.path().join("stack.lock");

        // First generate.
        let gen_args = LockArgs {
            zone: zone_path.clone(),
            check: false,
            out: Some(out_path.clone()),
            generated_at: Some("2026-01-15T00:00:00Z".to_string()),
            strict: false,
            format: "canonical-json".to_string(),
        };
        run_lock(&gen_args, dir.path()).unwrap();

        // Then check.
        let check_args = LockArgs {
            zone: zone_path,
            check: true,
            out: Some(out_path),
            generated_at: Some("2026-01-15T00:00:00Z".to_string()),
            strict: false,
            format: "canonical-json".to_string(),
        };
        let result = run_lock(&check_args, dir.path()).unwrap();
        assert_eq!(result, 0, "Check should pass when lockfile matches");
    }

    #[test]
    fn run_lock_check_mode_fails_when_lockfile_missing() {
        let dir = tempfile::tempdir().unwrap();
        let zone_content =
            "zone_id: test-zone\nprofile:\n  profile_id: test-profile\n  version: '1.0.0'\n";
        let zone_path = dir.path().join("zone.yaml");
        std::fs::write(&zone_path, zone_content).unwrap();

        let profiles_dir = dir.path().join("profiles");
        let profile_sub = profiles_dir.join("test");
        std::fs::create_dir_all(&profile_sub).unwrap();
        std::fs::write(
            profile_sub.join("profile.yaml"),
            "profile_id: test-profile\nversion: '1.0.0'\nmodules: []\n",
        )
        .unwrap();

        std::fs::create_dir_all(dir.path().join("modules")).unwrap();

        let out_path = dir.path().join("stack.lock");
        // Do not generate the lockfile.

        let check_args = LockArgs {
            zone: zone_path,
            check: true,
            out: Some(out_path),
            generated_at: Some("2026-01-15T00:00:00Z".to_string()),
            strict: false,
            format: "canonical-json".to_string(),
        };
        let result = run_lock(&check_args, dir.path()).unwrap();
        assert_eq!(result, 1, "Check should fail when lockfile is missing");
    }

    #[test]
    fn run_lock_check_mode_fails_when_lockfile_differs() {
        let dir = tempfile::tempdir().unwrap();
        let zone_content =
            "zone_id: test-zone\nprofile:\n  profile_id: test-profile\n  version: '1.0.0'\n";
        let zone_path = dir.path().join("zone.yaml");
        std::fs::write(&zone_path, zone_content).unwrap();

        let profiles_dir = dir.path().join("profiles");
        let profile_sub = profiles_dir.join("test");
        std::fs::create_dir_all(&profile_sub).unwrap();
        std::fs::write(
            profile_sub.join("profile.yaml"),
            "profile_id: test-profile\nversion: '1.0.0'\nmodules: []\n",
        )
        .unwrap();

        std::fs::create_dir_all(dir.path().join("modules")).unwrap();

        let out_path = dir.path().join("stack.lock");
        // Write stale lockfile content.
        std::fs::write(&out_path, b"{\"stale\":\"content\"}\n").unwrap();

        let check_args = LockArgs {
            zone: zone_path,
            check: true,
            out: Some(out_path),
            generated_at: Some("2026-01-15T00:00:00Z".to_string()),
            strict: false,
            format: "canonical-json".to_string(),
        };
        let result = run_lock(&check_args, dir.path()).unwrap();
        assert_eq!(result, 1, "Check should fail when lockfile differs");
    }

    #[test]
    fn run_lock_with_modules_in_profile() {
        let dir = tempfile::tempdir().unwrap();
        let zone_content =
            "zone_id: test-zone\nprofile:\n  profile_id: test-profile\n  version: '1.0.0'\n";
        let zone_path = dir.path().join("zone.yaml");
        std::fs::write(&zone_path, zone_content).unwrap();

        // Profile referencing a module.
        let profiles_dir = dir.path().join("profiles");
        let profile_sub = profiles_dir.join("test");
        std::fs::create_dir_all(&profile_sub).unwrap();
        std::fs::write(
            profile_sub.join("profile.yaml"),
            "profile_id: test-profile\nversion: '1.0.0'\nmodules:\n  - module_id: org.test.mod\n    variant: default\n",
        )
        .unwrap();

        // Create the matching module.
        let mod_dir = dir.path().join("modules").join("test").join("mod");
        std::fs::create_dir_all(&mod_dir).unwrap();
        std::fs::write(
            mod_dir.join("module.yaml"),
            "module_id: org.test.mod\nversion: '0.1.0'\nprovides:\n  - interface: test.v1\n",
        )
        .unwrap();

        let out_path = dir.path().join("stack.lock");
        let args = LockArgs {
            zone: zone_path,
            check: false,
            out: Some(out_path.clone()),
            generated_at: Some("2026-01-15T00:00:00Z".to_string()),
            strict: false,
            format: "canonical-json".to_string(),
        };

        let result = run_lock(&args, dir.path()).unwrap();
        assert_eq!(result, 0);

        let content = std::fs::read_to_string(&out_path).unwrap();
        assert!(content.contains("org.test.mod"));
    }

    #[test]
    fn run_lock_uses_default_out_path() {
        let dir = tempfile::tempdir().unwrap();
        let zone_content =
            "zone_id: test-zone\nprofile:\n  profile_id: test-profile\n  version: '1.0.0'\n";
        let zone_path = dir.path().join("zone.yaml");
        std::fs::write(&zone_path, zone_content).unwrap();

        let profiles_dir = dir.path().join("profiles");
        let profile_sub = profiles_dir.join("test");
        std::fs::create_dir_all(&profile_sub).unwrap();
        std::fs::write(
            profile_sub.join("profile.yaml"),
            "profile_id: test-profile\nversion: '1.0.0'\nmodules: []\n",
        )
        .unwrap();

        std::fs::create_dir_all(dir.path().join("modules")).unwrap();

        // No --out flag; should write to zone.yaml's parent dir / stack.lock.
        let args = LockArgs {
            zone: zone_path.clone(),
            check: false,
            out: None,
            generated_at: Some("2026-01-15T00:00:00Z".to_string()),
            strict: false,
            format: "canonical-json".to_string(),
        };

        let result = run_lock(&args, dir.path()).unwrap();
        assert_eq!(result, 0);
        // Default lockfile path is same dir as zone.yaml / stack.lock.
        assert!(dir.path().join("stack.lock").exists());
    }

    #[test]
    fn run_lock_custom_lockfile_path_from_zone() {
        let dir = tempfile::tempdir().unwrap();
        let zone_content = "zone_id: test-zone\nlockfile_path: custom.lock\nprofile:\n  profile_id: test-profile\n  version: '1.0.0'\n";
        let zone_path = dir.path().join("zone.yaml");
        std::fs::write(&zone_path, zone_content).unwrap();

        let profiles_dir = dir.path().join("profiles");
        let profile_sub = profiles_dir.join("test");
        std::fs::create_dir_all(&profile_sub).unwrap();
        std::fs::write(
            profile_sub.join("profile.yaml"),
            "profile_id: test-profile\nversion: '1.0.0'\nmodules: []\n",
        )
        .unwrap();

        std::fs::create_dir_all(dir.path().join("modules")).unwrap();

        let args = LockArgs {
            zone: zone_path,
            check: false,
            out: None,
            generated_at: Some("2026-01-15T00:00:00Z".to_string()),
            strict: false,
            format: "canonical-json".to_string(),
        };

        let result = run_lock(&args, dir.path()).unwrap();
        assert_eq!(result, 0);
        assert!(dir.path().join("custom.lock").exists());
    }

    #[test]
    fn run_lock_missing_profile_errors() {
        let dir = tempfile::tempdir().unwrap();
        let zone_content =
            "zone_id: test-zone\nprofile:\n  profile_id: nonexistent-profile\n  version: '1.0.0'\n";
        let zone_path = dir.path().join("zone.yaml");
        std::fs::write(&zone_path, zone_content).unwrap();

        // Create profiles dir but no matching profile.
        let profiles_dir = dir.path().join("profiles");
        std::fs::create_dir_all(&profiles_dir).unwrap();
        std::fs::create_dir_all(dir.path().join("modules")).unwrap();

        let args = LockArgs {
            zone: zone_path,
            check: false,
            out: None,
            generated_at: Some("2026-01-15T00:00:00Z".to_string()),
            strict: false,
            format: "canonical-json".to_string(),
        };

        let result = run_lock(&args, dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn run_lock_module_not_found_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let zone_content =
            "zone_id: test-zone\nprofile:\n  profile_id: test-profile\n  version: '1.0.0'\n";
        let zone_path = dir.path().join("zone.yaml");
        std::fs::write(&zone_path, zone_content).unwrap();

        // Profile references a module that does not exist.
        let profiles_dir = dir.path().join("profiles");
        let profile_sub = profiles_dir.join("test");
        std::fs::create_dir_all(&profile_sub).unwrap();
        std::fs::write(
            profile_sub.join("profile.yaml"),
            "profile_id: test-profile\nversion: '1.0.0'\nmodules:\n  - module_id: org.test.missing\n",
        )
        .unwrap();

        std::fs::create_dir_all(dir.path().join("modules")).unwrap();

        let out_path = dir.path().join("stack.lock");
        let args = LockArgs {
            zone: zone_path,
            check: false,
            out: Some(out_path.clone()),
            generated_at: Some("2026-01-15T00:00:00Z".to_string()),
            strict: false,
            format: "canonical-json".to_string(),
        };

        // Missing module should be skipped (warning logged), not error.
        let result = run_lock(&args, dir.path()).unwrap();
        assert_eq!(result, 0);

        let content = std::fs::read_to_string(&out_path).unwrap();
        // modules array should be empty since the module was not found.
        let parsed: serde_json::Value = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(parsed["modules"].as_array().unwrap().len(), 0);
    }
}
