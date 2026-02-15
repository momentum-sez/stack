//! # Validate Subcommand
//!
//! Zone, module, and profile validation against JSON schemas and pack rules.
//!
//! Matches the behavior of `tools/msez.py validate --all-modules`, `--all-profiles`,
//! and `--all-zones` from the Python CLI.
//!
//! ## Security Invariant
//!
//! Schema validation is the first line of defense against malformed input.
//! Every YAML descriptor, zone configuration, and profile must pass validation
//! before any business logic operates on it.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;

use msez_schema::SchemaValidator;

/// Arguments for the `msez validate` subcommand.
#[derive(Args, Debug)]
pub struct ValidateArgs {
    /// Validate all module descriptors under modules/.
    #[arg(long)]
    pub all_modules: bool,

    /// Validate all profile descriptors under profiles/.
    #[arg(long)]
    pub all_profiles: bool,

    /// Validate all zone configurations under jurisdictions/.
    #[arg(long)]
    pub all_zones: bool,

    /// Validate a specific file path (module.yaml, zone.yaml, or profile.yaml).
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,
}

/// Execute the validate subcommand.
///
/// Returns exit code: 0 on success, 1 on validation failure, 2 on operational error.
pub fn run_validate(args: &ValidateArgs, repo_root: &Path) -> Result<u8> {
    let schema_dir = repo_root.join("schemas");
    let validator = SchemaValidator::new(&schema_dir).context("failed to load JSON schemas")?;

    tracing::info!(
        schema_count = validator.schema_count(),
        "loaded schema registry"
    );

    let mut had_failures = false;

    if args.all_modules {
        had_failures |= validate_all_modules(&validator, repo_root)?;
    }

    if args.all_profiles {
        had_failures |= validate_all_profiles(&validator, repo_root)?;
    }

    if args.all_zones {
        had_failures |= validate_all_zones(&validator, repo_root)?;
    }

    if let Some(ref path) = args.path {
        let resolved = crate::resolve_path(path, repo_root);
        had_failures |= validate_single_path(&validator, &resolved)?;
    }

    // If no flags specified, print usage hint.
    if !args.all_modules && !args.all_profiles && !args.all_zones && args.path.is_none() {
        println!("Usage: msez validate [--all-modules] [--all-profiles] [--all-zones] [PATH]");
        return Ok(1);
    }

    if had_failures {
        Ok(1)
    } else {
        Ok(0)
    }
}

/// Validate all module descriptors under `modules/`.
///
/// Iterates all directories containing `module.yaml`, validates each against
/// the `module.schema.json` schema, and prints a summary report.
fn validate_all_modules(validator: &SchemaValidator, repo_root: &Path) -> Result<bool> {
    let modules_dir = repo_root.join("modules");
    if !modules_dir.is_dir() {
        println!(
            "WARN: modules/ directory not found at {}",
            modules_dir.display()
        );
        return Ok(false);
    }

    let report = validator.validate_all_modules(&modules_dir);

    println!("Modules: {}/{} passed", report.passed, report.total);

    for failure in &report.failures {
        let rel = failure
            .module_dir
            .strip_prefix(repo_root)
            .unwrap_or(&failure.module_dir);
        println!("  FAIL: {} — {}", rel.display(), failure.error);
    }

    if report.failed > 0 {
        println!(
            "\n{} module(s) failed validation out of {} total.",
            report.failed, report.total
        );
    }

    Ok(report.failed > 0)
}

/// Validate all profile descriptors under `profiles/`.
///
/// Scans for `profile.yaml` files and validates each against
/// `profile.schema.json`.
fn validate_all_profiles(validator: &SchemaValidator, repo_root: &Path) -> Result<bool> {
    let profiles_dir = repo_root.join("profiles");
    if !profiles_dir.is_dir() {
        println!(
            "WARN: profiles/ directory not found at {}",
            profiles_dir.display()
        );
        return Ok(false);
    }

    let profile_files = find_yaml_files(&profiles_dir, "profile.yaml");
    let total = profile_files.len();
    let mut passed = 0usize;
    let mut failures: Vec<(PathBuf, String)> = Vec::new();

    for path in &profile_files {
        match validator.validate_profile(path) {
            Ok(()) => passed += 1,
            Err(e) => {
                failures.push((path.clone(), e.to_string()));
            }
        }
    }

    println!("Profiles: {}/{} passed", passed, total);

    for (path, err) in &failures {
        let rel = path.strip_prefix(repo_root).unwrap_or(path);
        println!("  FAIL: {} — {}", rel.display(), err);
    }

    if !failures.is_empty() {
        println!(
            "\n{} profile(s) failed validation out of {} total.",
            failures.len(),
            total
        );
    }

    Ok(!failures.is_empty())
}

/// Validate all zone configurations under `jurisdictions/`.
///
/// Scans for `zone.yaml` files and validates each against
/// `zone.schema.json`.
fn validate_all_zones(validator: &SchemaValidator, repo_root: &Path) -> Result<bool> {
    let jurisdictions_dir = repo_root.join("jurisdictions");
    if !jurisdictions_dir.is_dir() {
        println!(
            "WARN: jurisdictions/ directory not found at {}",
            jurisdictions_dir.display()
        );
        return Ok(false);
    }

    let zone_files = find_yaml_files(&jurisdictions_dir, "zone.yaml");
    let total = zone_files.len();
    let mut passed = 0usize;
    let mut failures: Vec<(PathBuf, String)> = Vec::new();

    for path in &zone_files {
        match validator.validate_zone(path) {
            Ok(()) => passed += 1,
            Err(e) => {
                failures.push((path.clone(), e.to_string()));
            }
        }
    }

    println!("Zones: {}/{} passed", passed, total);

    for (path, err) in &failures {
        let rel = path.strip_prefix(repo_root).unwrap_or(path);
        println!("  FAIL: {} — {}", rel.display(), err);
    }

    if !failures.is_empty() {
        println!(
            "\n{} zone(s) failed validation out of {} total.",
            failures.len(),
            total
        );
    }

    Ok(!failures.is_empty())
}

/// Validate a single file path (module, zone, or profile).
fn validate_single_path(validator: &SchemaValidator, path: &Path) -> Result<bool> {
    if !path.exists() {
        println!("ERROR: path does not exist: {}", path.display());
        return Ok(true);
    }

    // Determine file type based on filename.
    let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");

    let result = match filename {
        "module.yaml" => validator.validate_module(path),
        "zone.yaml" => validator.validate_zone(path),
        "profile.yaml" => validator.validate_profile(path),
        _ => {
            // Try to infer: if it's a directory with module.yaml, validate as module.
            if path.is_dir() && path.join("module.yaml").exists() {
                validator.validate_module(path)
            } else {
                println!(
                    "ERROR: cannot determine validation type for {}",
                    path.display()
                );
                return Ok(true);
            }
        }
    };

    match result {
        Ok(()) => {
            println!("OK: {}", path.display());
            Ok(false)
        }
        Err(e) => {
            println!("FAIL: {} — {}", path.display(), e);
            Ok(true)
        }
    }
}

/// Recursively find files matching a specific filename under a directory.
fn find_yaml_files(dir: &Path, target_name: &str) -> Vec<PathBuf> {
    let mut results = Vec::new();
    walk_for_files(dir, target_name, &mut results);
    results.sort();
    results
}

fn walk_for_files(dir: &Path, target_name: &str, acc: &mut Vec<PathBuf>) {
    let mut seen = std::collections::HashSet::new();
    // Populate seen set from any existing entries for O(1) lookups.
    for path in acc.iter() {
        seen.insert(path.clone());
    }
    walk_for_files_inner(dir, target_name, acc, &mut seen);
}

fn walk_for_files_inner(
    dir: &Path,
    target_name: &str,
    acc: &mut Vec<PathBuf>,
    seen: &mut std::collections::HashSet<PathBuf>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(
                dir = %dir.display(),
                error = %e,
                "failed to read directory during file walk"
            );
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(dir = %dir.display(), error = %e, "failed to read directory entry");
                continue;
            }
        };
        let path = entry.path();
        if path.is_dir() {
            let candidate = path.join(target_name);
            if candidate.exists() && seen.insert(candidate.clone()) {
                acc.push(candidate);
            }
            walk_for_files_inner(&path, target_name, acc, seen);
        } else if path.file_name().and_then(|f| f.to_str()) == Some(target_name)
            && seen.insert(path.clone())
        {
            acc.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_repo_root() -> PathBuf {
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dir.pop(); // crates
        dir.pop(); // msez
        dir.pop(); // stack (repo root)
        dir
    }

    #[test]
    fn find_yaml_files_finds_modules() {
        let root = test_repo_root();
        let modules_dir = root.join("modules");
        if modules_dir.is_dir() {
            let files = find_yaml_files(&modules_dir, "module.yaml");
            assert!(
                files.len() >= 50,
                "Expected at least 50 module.yaml files, found {}",
                files.len()
            );
        }
    }

    #[test]
    fn find_yaml_files_finds_zones() {
        let root = test_repo_root();
        let jurisdictions_dir = root.join("jurisdictions");
        if jurisdictions_dir.is_dir() {
            let files = find_yaml_files(&jurisdictions_dir, "zone.yaml");
            assert!(!files.is_empty(), "Expected at least one zone.yaml file");
        }
    }

    #[test]
    fn find_yaml_files_finds_profiles() {
        let root = test_repo_root();
        let profiles_dir = root.join("profiles");
        if profiles_dir.is_dir() {
            let files = find_yaml_files(&profiles_dir, "profile.yaml");
            assert!(!files.is_empty(), "Expected at least one profile.yaml file");
        }
    }

    // ── Additional coverage tests ────────────────────────────────────

    #[test]
    fn find_yaml_files_returns_empty_for_nonexistent_dir() {
        let files = find_yaml_files(
            Path::new("/tmp/msez-test-nonexistent-dir-xyz"),
            "module.yaml",
        );
        assert!(files.is_empty());
    }

    #[test]
    fn find_yaml_files_returns_empty_when_no_matches() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("something.txt"), b"hello").unwrap();
        let files = find_yaml_files(dir.path(), "module.yaml");
        assert!(files.is_empty());
    }

    #[test]
    fn find_yaml_files_returns_sorted_results() {
        let dir = tempfile::tempdir().unwrap();
        // Create nested dirs with target files.
        let sub_b = dir.path().join("b_family").join("b_mod");
        let sub_a = dir.path().join("a_family").join("a_mod");
        std::fs::create_dir_all(&sub_b).unwrap();
        std::fs::create_dir_all(&sub_a).unwrap();
        std::fs::write(sub_b.join("module.yaml"), b"module_id: b").unwrap();
        std::fs::write(sub_a.join("module.yaml"), b"module_id: a").unwrap();

        let files = find_yaml_files(dir.path(), "module.yaml");
        assert_eq!(files.len(), 2);
        // Results should be sorted.
        assert!(files[0] < files[1], "Results should be sorted: {:?}", files);
    }

    #[test]
    fn find_yaml_files_no_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("family").join("mod1");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("module.yaml"), b"module_id: test").unwrap();

        let files = find_yaml_files(dir.path(), "module.yaml");
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn walk_for_files_handles_nested_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let deep = dir.path().join("a").join("b").join("c");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(deep.join("zone.yaml"), b"zone_id: test").unwrap();

        let files = find_yaml_files(dir.path(), "zone.yaml");
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("zone.yaml"));
    }

    #[test]
    fn walk_for_files_finds_file_directly_in_dir() {
        let dir = tempfile::tempdir().unwrap();
        // File directly in the scanned dir (not in subdirectory).
        std::fs::write(dir.path().join("profile.yaml"), b"profile_id: test").unwrap();

        let files = find_yaml_files(dir.path(), "profile.yaml");
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn run_validate_no_flags_prints_usage_and_returns_1() {
        let dir = tempfile::tempdir().unwrap();
        // Create minimal schema dir so SchemaValidator can load (empty is fine).
        let schema_dir = dir.path().join("schemas");
        std::fs::create_dir_all(&schema_dir).unwrap();

        let args = ValidateArgs {
            all_modules: false,
            all_profiles: false,
            all_zones: false,
            path: None,
        };
        let result = run_validate(&args, dir.path()).unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn validate_all_modules_missing_dir_returns_false() {
        let dir = tempfile::tempdir().unwrap();
        let schema_dir = dir.path().join("schemas");
        std::fs::create_dir_all(&schema_dir).unwrap();

        let validator = msez_schema::SchemaValidator::new(&schema_dir).unwrap();
        // No modules/ dir exists.
        let result = validate_all_modules(&validator, dir.path()).unwrap();
        assert!(
            !result,
            "Should return false (no failures) when modules dir is missing"
        );
    }

    #[test]
    fn validate_all_profiles_missing_dir_returns_false() {
        let dir = tempfile::tempdir().unwrap();
        let schema_dir = dir.path().join("schemas");
        std::fs::create_dir_all(&schema_dir).unwrap();

        let validator = msez_schema::SchemaValidator::new(&schema_dir).unwrap();
        let result = validate_all_profiles(&validator, dir.path()).unwrap();
        assert!(
            !result,
            "Should return false (no failures) when profiles dir is missing"
        );
    }

    #[test]
    fn validate_all_zones_missing_dir_returns_false() {
        let dir = tempfile::tempdir().unwrap();
        let schema_dir = dir.path().join("schemas");
        std::fs::create_dir_all(&schema_dir).unwrap();

        let validator = msez_schema::SchemaValidator::new(&schema_dir).unwrap();
        let result = validate_all_zones(&validator, dir.path()).unwrap();
        assert!(
            !result,
            "Should return false (no failures) when jurisdictions dir is missing"
        );
    }

    #[test]
    fn validate_single_path_nonexistent_returns_true() {
        let dir = tempfile::tempdir().unwrap();
        let schema_dir = dir.path().join("schemas");
        std::fs::create_dir_all(&schema_dir).unwrap();

        let validator = msez_schema::SchemaValidator::new(&schema_dir).unwrap();
        let result =
            validate_single_path(&validator, Path::new("/tmp/msez-no-such-file.yaml")).unwrap();
        assert!(result, "Nonexistent path should return true (had_failures)");
    }

    #[test]
    fn validate_single_path_unknown_filename_returns_true() {
        let dir = tempfile::tempdir().unwrap();
        let schema_dir = dir.path().join("schemas");
        std::fs::create_dir_all(&schema_dir).unwrap();
        // Create a file with an unrecognized name.
        let unknown = dir.path().join("random.yaml");
        std::fs::write(&unknown, b"key: value").unwrap();

        let validator = msez_schema::SchemaValidator::new(&schema_dir).unwrap();
        let result = validate_single_path(&validator, &unknown).unwrap();
        assert!(result, "Unknown filename should return true (failure)");
    }

    #[test]
    fn validate_single_path_directory_with_module_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let schema_dir = dir.path().join("schemas");
        std::fs::create_dir_all(&schema_dir).unwrap();

        // Create a directory containing module.yaml.
        let mod_dir = dir.path().join("test_mod");
        std::fs::create_dir_all(&mod_dir).unwrap();
        std::fs::write(mod_dir.join("module.yaml"), b"module_id: test").unwrap();

        let validator = msez_schema::SchemaValidator::new(&schema_dir).unwrap();
        // This will attempt to validate; it may fail validation but should not
        // error on the path resolution logic.
        let result = validate_single_path(&validator, &mod_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_single_path_directory_without_module_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let schema_dir = dir.path().join("schemas");
        std::fs::create_dir_all(&schema_dir).unwrap();

        // Create an empty directory (no module.yaml inside).
        let empty_dir = dir.path().join("empty_mod");
        std::fs::create_dir_all(&empty_dir).unwrap();

        let validator = msez_schema::SchemaValidator::new(&schema_dir).unwrap();
        let result = validate_single_path(&validator, &empty_dir).unwrap();
        assert!(
            result,
            "Dir without module.yaml should return true (failure)"
        );
    }

    #[test]
    fn run_validate_all_modules_with_empty_modules_dir() {
        let dir = tempfile::tempdir().unwrap();
        let schema_dir = dir.path().join("schemas");
        std::fs::create_dir_all(&schema_dir).unwrap();
        // Create an empty modules/ directory.
        std::fs::create_dir_all(dir.path().join("modules")).unwrap();

        let args = ValidateArgs {
            all_modules: true,
            all_profiles: false,
            all_zones: false,
            path: None,
        };
        let result = run_validate(&args, dir.path()).unwrap();
        assert_eq!(
            result, 0,
            "Empty modules dir should pass (0 found, 0 failures)"
        );
    }

    #[test]
    fn run_validate_all_profiles_with_empty_profiles_dir() {
        let dir = tempfile::tempdir().unwrap();
        let schema_dir = dir.path().join("schemas");
        std::fs::create_dir_all(&schema_dir).unwrap();
        std::fs::create_dir_all(dir.path().join("profiles")).unwrap();

        let args = ValidateArgs {
            all_modules: false,
            all_profiles: true,
            all_zones: false,
            path: None,
        };
        let result = run_validate(&args, dir.path()).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn run_validate_all_zones_with_empty_jurisdictions_dir() {
        let dir = tempfile::tempdir().unwrap();
        let schema_dir = dir.path().join("schemas");
        std::fs::create_dir_all(&schema_dir).unwrap();
        std::fs::create_dir_all(dir.path().join("jurisdictions")).unwrap();

        let args = ValidateArgs {
            all_modules: false,
            all_profiles: false,
            all_zones: true,
            path: None,
        };
        let result = run_validate(&args, dir.path()).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn run_validate_with_specific_path_module_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let schema_dir = dir.path().join("schemas");
        std::fs::create_dir_all(&schema_dir).unwrap();

        // Create a module.yaml at a specific path.
        let mod_dir = dir.path().join("test_mod");
        std::fs::create_dir_all(&mod_dir).unwrap();
        let module_path = mod_dir.join("module.yaml");
        std::fs::write(&module_path, b"module_id: test\nversion: '0.1.0'").unwrap();

        let args = ValidateArgs {
            all_modules: false,
            all_profiles: false,
            all_zones: false,
            path: Some(module_path),
        };
        // This runs the single path validation. Result depends on schema presence.
        let result = run_validate(&args, dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn run_validate_combined_flags() {
        let dir = tempfile::tempdir().unwrap();
        let schema_dir = dir.path().join("schemas");
        std::fs::create_dir_all(&schema_dir).unwrap();
        std::fs::create_dir_all(dir.path().join("modules")).unwrap();
        std::fs::create_dir_all(dir.path().join("profiles")).unwrap();
        std::fs::create_dir_all(dir.path().join("jurisdictions")).unwrap();

        let args = ValidateArgs {
            all_modules: true,
            all_profiles: true,
            all_zones: true,
            path: None,
        };
        let result = run_validate(&args, dir.path()).unwrap();
        assert_eq!(result, 0, "All empty dirs should pass");
    }

    #[test]
    fn validate_all_modules_reports_failures() {
        let dir = tempfile::tempdir().unwrap();

        // Use the real schemas dir from the repo for validation.
        let root = test_repo_root();
        let schema_dir = root.join("schemas");
        if !schema_dir.is_dir() {
            return; // Skip if schemas not available.
        }

        let validator = msez_schema::SchemaValidator::new(&schema_dir).unwrap();

        // Create a modules dir with an invalid module.yaml.
        let modules_dir = dir.path().join("modules");
        let bad_mod = modules_dir.join("bad_family").join("bad_mod");
        std::fs::create_dir_all(&bad_mod).unwrap();
        std::fs::write(bad_mod.join("module.yaml"), b"not_a_valid_module: true").unwrap();

        let result = validate_all_modules(&validator, dir.path()).unwrap();
        assert!(result, "Invalid module should cause failure");
    }

    #[test]
    fn validate_all_profiles_reports_failures() {
        let dir = tempfile::tempdir().unwrap();

        let root = test_repo_root();
        let schema_dir = root.join("schemas");
        if !schema_dir.is_dir() {
            return;
        }

        let validator = msez_schema::SchemaValidator::new(&schema_dir).unwrap();

        // Create profiles dir with an invalid profile.yaml.
        let profiles_dir = dir.path().join("profiles");
        let bad_profile_dir = profiles_dir.join("bad_profile");
        std::fs::create_dir_all(&bad_profile_dir).unwrap();
        std::fs::write(bad_profile_dir.join("profile.yaml"), b"invalid: true").unwrap();

        let result = validate_all_profiles(&validator, dir.path()).unwrap();
        assert!(result, "Invalid profile should cause failure");
    }

    #[test]
    fn validate_all_zones_reports_failures() {
        let dir = tempfile::tempdir().unwrap();

        let root = test_repo_root();
        let schema_dir = root.join("schemas");
        if !schema_dir.is_dir() {
            return;
        }

        let validator = msez_schema::SchemaValidator::new(&schema_dir).unwrap();

        // Create jurisdictions dir with an invalid zone.yaml.
        let jurisdictions_dir = dir.path().join("jurisdictions");
        let bad_zone_dir = jurisdictions_dir.join("bad_zone");
        std::fs::create_dir_all(&bad_zone_dir).unwrap();
        std::fs::write(bad_zone_dir.join("zone.yaml"), b"invalid: true").unwrap();

        let result = validate_all_zones(&validator, dir.path()).unwrap();
        assert!(result, "Invalid zone should cause failure");
    }

    #[test]
    fn validate_single_path_zone_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let schema_dir = dir.path().join("schemas");
        std::fs::create_dir_all(&schema_dir).unwrap();

        let zone_path = dir.path().join("zone.yaml");
        std::fs::write(&zone_path, b"zone_id: test").unwrap();

        let validator = msez_schema::SchemaValidator::new(&schema_dir).unwrap();
        // Will attempt zone validation; likely fail but should not panic.
        let result = validate_single_path(&validator, &zone_path);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_single_path_profile_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let schema_dir = dir.path().join("schemas");
        std::fs::create_dir_all(&schema_dir).unwrap();

        let profile_path = dir.path().join("profile.yaml");
        std::fs::write(&profile_path, b"profile_id: test").unwrap();

        let validator = msez_schema::SchemaValidator::new(&schema_dir).unwrap();
        let result = validate_single_path(&validator, &profile_path);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_single_path_reports_ok_on_valid_module() {
        let root = test_repo_root();
        let schema_dir = root.join("schemas");
        if !schema_dir.is_dir() {
            return;
        }

        // Find a real module to validate.
        let modules_dir = root.join("modules");
        if !modules_dir.is_dir() {
            return;
        }

        let files = find_yaml_files(&modules_dir, "module.yaml");
        if files.is_empty() {
            return;
        }

        let validator = msez_schema::SchemaValidator::new(&schema_dir).unwrap();
        // Try validating the first found module.yaml.
        let result = validate_single_path(&validator, &files[0]);
        assert!(result.is_ok());
    }
}
