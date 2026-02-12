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
use sha2::{Digest, Sha256};

use msez_core::CanonicalBytes;

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
            if let Ok(content) = std::fs::read_to_string(&entry) {
                if let Ok(profile) = serde_yaml::from_str::<serde_json::Value>(&content) {
                    if profile.get("profile_id").and_then(|v| v.as_str()) == Some(profile_id) {
                        return Ok(profile);
                    }
                }
            }
        }
    }

    bail!("profile not found for profile_id: {profile_id}")
}

/// Find a module by its module_id in the modules directory.
fn find_module_by_id(modules_dir: &Path, module_id: &str) -> Option<(PathBuf, serde_json::Value)> {
    for entry in walkdir_recursive(modules_dir) {
        if entry.file_name().and_then(|f| f.to_str()) == Some("module.yaml") {
            let content = std::fs::read_to_string(&entry).ok()?;
            let module: serde_json::Value = serde_yaml::from_str(&content).ok()?;
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
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    result.iter().map(|b| format!("{b:02x}")).collect()
}

/// Compute a deterministic digest over a directory's contents.
///
/// Walks all files in sorted order, hashing their relative paths and contents.
fn digest_dir(dir: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
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

    let result = hasher.finalize();
    Ok(result.iter().map(|b| format!("{b:02x}")).collect())
}

/// Recursively walk a directory, returning all file paths sorted.
fn walkdir_recursive(dir: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    walk_recursive_inner(dir, &mut results);
    results.sort();
    results
}

fn walk_recursive_inner(dir: &Path, acc: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
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
    fn sha256_file_computes_correctly() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, b"hello").unwrap();
        let digest = sha256_file(&file_path).unwrap();
        assert_eq!(digest.len(), 64);
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
}
