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
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Check this directory for the target file.
            let candidate = path.join(target_name);
            if candidate.exists() && !acc.contains(&candidate) {
                acc.push(candidate);
            }
            walk_for_files(&path, target_name, acc);
        } else if path.file_name().and_then(|f| f.to_str()) == Some(target_name)
            && !acc.contains(&path)
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
}
