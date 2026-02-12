//! Integration test: validate all 264 module descriptors against module.schema.json.
//!
//! This test matches the behavior of `msez validate --all-modules` from the Python CLI.
//! It walks the `modules/` directory, finds every `module.yaml` file, and validates
//! each against the `module.schema.json` schema.
//!
//! If some modules fail validation due to schema strictness, the failures are
//! documented rather than hidden — per audit policy, we do not weaken schemas.

use msez_schema::SchemaValidator;
use std::path::PathBuf;

/// Find the repository root.
fn repo_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop(); // crates/
    dir.pop(); // repo root
    dir
}

/// Recursively find all `module.yaml` files under a directory.
fn find_module_files(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut modules = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                modules.extend(find_module_files(&path));
            } else if path.file_name().is_some_and(|n| n == "module.yaml") {
                modules.push(path);
            }
        }
    }
    modules.sort();
    modules
}

#[test]
fn test_discover_all_module_files() {
    let modules_dir = repo_root().join("modules");
    let module_files = find_module_files(&modules_dir);

    // Exclude _template directory from count — those are scaffolds, not real modules.
    let real_modules: Vec<_> = module_files
        .iter()
        .filter(|p| !p.to_string_lossy().contains("_template"))
        .collect();

    assert!(
        real_modules.len() >= 200,
        "Expected >= 200 module descriptors, found {}. \
         Check that the modules/ directory exists at {}",
        real_modules.len(),
        modules_dir.display()
    );
}

#[test]
fn test_validate_all_modules() {
    let root = repo_root();
    let schema_dir = root.join("schemas");
    let modules_dir = root.join("modules");

    let validator = SchemaValidator::new(&schema_dir)
        .expect("Failed to load schemas");

    let module_files = find_module_files(&modules_dir);
    let real_modules: Vec<_> = module_files
        .into_iter()
        .filter(|p| !p.to_string_lossy().contains("_template"))
        .collect();

    let mut passed = 0usize;
    let mut failed = Vec::new();

    for module_path in &real_modules {
        match validator.validate_module(module_path) {
            Ok(()) => passed += 1,
            Err(e) => {
                // Strip repo root prefix for readable output.
                let relative = module_path
                    .strip_prefix(&root)
                    .unwrap_or(module_path);
                failed.push(format!("{}: {e}", relative.display()));
            }
        }
    }

    // Report results.
    let total = real_modules.len();
    eprintln!(
        "\n=== Module Validation Results ===\n\
         Total:  {total}\n\
         Passed: {passed}\n\
         Failed: {}\n",
        failed.len()
    );

    if !failed.is_empty() {
        eprintln!("Failures:");
        for (i, f) in failed.iter().enumerate() {
            eprintln!("  {}. {f}", i + 1);
        }
        eprintln!();
    }

    // All modules should validate. If any fail, this test fails.
    // Per audit policy: do not weaken schemas. If modules are malformed,
    // fix the modules, not the schema.
    assert!(
        failed.is_empty(),
        "{} of {total} module descriptors failed validation. See output above.",
        failed.len()
    );
}

#[test]
fn test_validate_modules_per_family() {
    let root = repo_root();
    let schema_dir = root.join("schemas");
    let modules_dir = root.join("modules");

    let validator = SchemaValidator::new(&schema_dir)
        .expect("Failed to load schemas");

    // Validate that we can process each module family directory.
    let expected_families = [
        "corridors",
        "financial",
        "governance",
        "legal",
        "licensing",
        "operational",
        "regulatory",
        "smart-assets",
    ];

    for family in &expected_families {
        let family_dir = modules_dir.join(family);
        if !family_dir.exists() {
            continue;
        }

        let family_modules = find_module_files(&family_dir);
        if family_modules.is_empty() {
            // Some families (e.g., smart-assets) contain only _template
            // directories with non-module YAML files. Skip silently.
            eprintln!("  Skipping family '{family}': no module.yaml files");
            continue;
        }

        for module_path in &family_modules {
            if let Err(e) = validator.validate_module(module_path) {
                let relative = module_path.strip_prefix(&root).unwrap_or(module_path);
                panic!(
                    "Module {}/{} failed validation: {e}",
                    family,
                    relative.display()
                );
            }
        }
    }
}

#[test]
fn test_security_critical_schemas_loadable() {
    let schema_dir = repo_root().join("schemas");
    let validator = SchemaValidator::new(&schema_dir)
        .expect("Failed to load schemas");

    for spec in msez_schema::SECURITY_CRITICAL_SCHEMAS {
        let result = validator.build_validator(spec.schema_name);
        assert!(
            result.is_ok(),
            "Security-critical schema '{}' ({}) failed to compile: {:?}",
            spec.schema_name,
            spec.description,
            result.err()
        );
    }
}

#[test]
fn test_security_schemas_additional_properties_audit() {
    let schema_dir = repo_root().join("schemas");
    let validator = SchemaValidator::new(&schema_dir)
        .expect("Failed to load schemas");

    let mut total_findings = 0;

    for spec in msez_schema::SECURITY_CRITICAL_SCHEMAS {
        if let Some(schema) = validator.get_schema(spec.schema_name) {
            let findings = msez_schema::audit_additional_properties(schema);
            if !findings.is_empty() {
                eprintln!(
                    "\n{} ({}):",
                    spec.schema_name, spec.description
                );
                for finding in &findings {
                    eprintln!("  {finding}");
                }
                total_findings += findings.len();
            }
        }
    }

    // Document the current state rather than failing.
    // The audit identified these as findings to be remediated.
    eprintln!(
        "\n=== additionalProperties Audit ===\n\
         Total findings across security-critical schemas: {total_findings}\n\
         These represent schemas that should have additionalProperties: false\n\
         per audit finding §3.1.\n"
    );
}
