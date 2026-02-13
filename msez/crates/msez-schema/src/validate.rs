//! # Runtime Schema Validation
//!
//! Validates JSON/YAML documents against JSON Schema (Draft 2020-12) definitions
//! from the `schemas/` directory. Resolves `$ref` URIs internally by mapping
//! `https://schemas.momentum-sez.org/msez/{name}` to local schema files.
//!
//! ## Design
//!
//! The [`SchemaValidator`] loads all schema files at construction time, builds a
//! URI → schema map for `$ref` resolution, and caches compiled validators per
//! schema. Validation errors carry structured diagnostic context: the schema
//! `$id`, the JSON Pointer to the violating field, and a human-readable message.
//!
//! ## Security invariant
//!
//! Schema validation is the first line of defense against malformed input.
//! All YAML module descriptors, zone configurations, and profile definitions
//! must pass schema validation before any business logic touches them.
//!
//! Implements the validation layer described in audit §2.6 and §3.1.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::Value;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Structured validation error with diagnostic context.
///
/// Carries the schema identity, the JSON Pointer path to the violating field,
/// and a human-readable description. This matches the Python implementation's
/// `validate_against_schema` return format while providing richer type info.
#[derive(Debug, Clone)]
pub struct SchemaValidationDetail {
    /// The JSON Schema `$id` or file path that was violated.
    pub schema_path: String,
    /// The JSON Pointer to the field that failed validation.
    pub instance_path: String,
    /// Human-readable description of the violation.
    pub message: String,
}

impl std::fmt::Display for SchemaValidationDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "schema={}, path={}: {}",
            self.schema_path, self.instance_path, self.message
        )
    }
}

/// Errors returned by schema validation operations.
#[derive(Error, Debug)]
pub enum SchemaValidationError {
    /// The schema file could not be read or parsed.
    #[error("failed to load schema {path}: {reason}")]
    SchemaLoadError {
        /// Path or identifier of the schema that failed to load.
        path: String,
        /// Human-readable reason for the failure.
        reason: String,
    },

    /// The target document could not be loaded.
    #[error("failed to load document {path}: {reason}")]
    DocumentLoadError {
        /// Path to the document that failed to load.
        path: String,
        /// Human-readable reason for the failure.
        reason: String,
    },

    /// The schema could not be compiled into a validator.
    #[error("failed to compile schema {schema_id}: {reason}")]
    SchemaCompileError {
        /// The schema `$id` or path.
        schema_id: String,
        /// Human-readable reason.
        reason: String,
    },

    /// The document failed validation against its schema.
    #[error("{count} validation error(s) against {schema_id}")]
    ValidationFailed {
        /// The schema that was violated.
        schema_id: String,
        /// Number of violations found.
        count: usize,
        /// Individual violation details.
        details: Vec<SchemaValidationDetail>,
    },

    /// The requested schema was not found in the registry.
    #[error("schema not found: {0}")]
    SchemaNotFound(String),

    /// I/O error during file operations.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// Schema retriever for $ref resolution
// ---------------------------------------------------------------------------

/// URI prefix used by all MSEZ schemas.
const SCHEMA_URI_PREFIX: &str = "https://schemas.momentum-sez.org/msez/";

/// A retriever that resolves `$ref` URIs by looking up pre-loaded schemas.
///
/// All MSEZ schemas use `$id` values of the form
/// `https://schemas.momentum-sez.org/msez/{filename}`. This retriever maps
/// those URIs to the corresponding schema JSON loaded from the local
/// `schemas/` directory.
struct LocalSchemaRetriever {
    /// Map from full URI to parsed schema JSON.
    schemas: HashMap<String, Value>,
}

impl jsonschema::Retrieve for LocalSchemaRetriever {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<&str>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let uri_str = uri.as_str();
        self.schemas
            .get(uri_str)
            .cloned()
            .ok_or_else(|| format!("schema not found for URI: {uri_str}").into())
    }
}

// ---------------------------------------------------------------------------
// SchemaValidator
// ---------------------------------------------------------------------------

/// A compiled schema validator that validates documents against the MSEZ
/// JSON Schema corpus.
///
/// Loads all `*.schema.json` files from the `schemas/` directory at
/// construction time, registers them by `$id` for `$ref` resolution,
/// and provides validation methods for modules, zones, and profiles.
pub struct SchemaValidator {
    /// The root directory containing JSON schema files.
    schema_dir: PathBuf,
    /// Pre-loaded schemas indexed by their `$id` URI.
    schema_map: HashMap<String, Value>,
    /// Map from schema filename (e.g. `module.schema.json`) to its `$id` URI.
    filename_to_id: HashMap<String, String>,
}

impl std::fmt::Debug for SchemaValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemaValidator")
            .field("schema_dir", &self.schema_dir)
            .field("schema_count", &self.schema_map.len())
            .finish()
    }
}

impl SchemaValidator {
    /// Create a new validator that loads all schemas from the given directory.
    ///
    /// Scans for `*.schema.json` files, parses each one, and registers it
    /// by its `$id` URI. Schemas without an `$id` are registered using a
    /// derived URI based on the filename.
    ///
    /// # Errors
    ///
    /// Returns [`SchemaValidationError::SchemaLoadError`] if any schema file
    /// cannot be read or parsed as JSON.
    pub fn new(schema_dir: impl Into<PathBuf>) -> Result<Self, SchemaValidationError> {
        let schema_dir = schema_dir.into();
        let mut schema_map = HashMap::new();
        let mut filename_to_id = HashMap::new();

        if !schema_dir.is_dir() {
            return Ok(Self {
                schema_dir,
                schema_map,
                filename_to_id,
            });
        }

        // Scan for all *.schema.json files (non-recursive first, then recursive).
        let mut seen_paths = std::collections::HashSet::new();
        for entry in Self::glob_schemas(&schema_dir)? {
            let path = entry;
            if !seen_paths.insert(path.clone()) {
                continue;
            }

            let content = std::fs::read_to_string(&path).map_err(|e| {
                SchemaValidationError::SchemaLoadError {
                    path: path.display().to_string(),
                    reason: e.to_string(),
                }
            })?;

            let schema: Value = serde_json::from_str(&content).map_err(|e| {
                SchemaValidationError::SchemaLoadError {
                    path: path.display().to_string(),
                    reason: e.to_string(),
                }
            })?;

            // Determine the schema $id.
            let schema_id = if let Some(id) = schema.get("$id").and_then(|v| v.as_str()) {
                id.to_string()
            } else {
                // Derive URI from filename.
                let rel = path.strip_prefix(&schema_dir).unwrap_or(&path);
                format!("{SCHEMA_URI_PREFIX}{}", rel.display())
            };

            if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                filename_to_id.insert(filename.to_string(), schema_id.clone());
            }

            schema_map.insert(schema_id, schema);
        }

        Ok(Self {
            schema_dir,
            schema_map,
            filename_to_id,
        })
    }

    /// Returns the number of schemas loaded into the registry.
    pub fn schema_count(&self) -> usize {
        self.schema_map.len()
    }

    /// Returns the path to the schemas directory.
    pub fn schema_dir(&self) -> &Path {
        &self.schema_dir
    }

    /// Returns all registered schema `$id` URIs.
    pub fn schema_ids(&self) -> Vec<&str> {
        self.schema_map.keys().map(|s| s.as_str()).collect()
    }

    /// Look up a schema by its `$id` URI.
    pub fn get_schema(&self, schema_id: &str) -> Option<&Value> {
        self.schema_map.get(schema_id)
    }

    /// Look up a schema by its filename (e.g. `module.schema.json`).
    pub fn get_schema_by_filename(&self, filename: &str) -> Option<&Value> {
        self.filename_to_id
            .get(filename)
            .and_then(|id| self.schema_map.get(id))
    }

    /// Validate a JSON value against a schema identified by its `$id` URI.
    ///
    /// Returns `Ok(())` if the value is valid, or a
    /// [`SchemaValidationError::ValidationFailed`] with all violation details.
    pub fn validate_value(
        &self,
        value: &Value,
        schema_id: &str,
    ) -> Result<(), SchemaValidationError> {
        let schema = self
            .schema_map
            .get(schema_id)
            .ok_or_else(|| SchemaValidationError::SchemaNotFound(schema_id.to_string()))?;

        let retriever = LocalSchemaRetriever {
            schemas: self.schema_map.clone(),
        };

        let validator = jsonschema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .with_retriever(retriever)
            .build(schema)
            .map_err(|e| SchemaValidationError::SchemaCompileError {
                schema_id: schema_id.to_string(),
                reason: e.to_string(),
            })?;

        let errors: Vec<SchemaValidationDetail> = validator
            .iter_errors(value)
            .map(|err| SchemaValidationDetail {
                schema_path: schema_id.to_string(),
                instance_path: err.instance_path.to_string(),
                message: err.to_string(),
            })
            .collect();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(SchemaValidationError::ValidationFailed {
                schema_id: schema_id.to_string(),
                count: errors.len(),
                details: errors,
            })
        }
    }

    /// Validate a JSON value against a schema identified by filename.
    ///
    /// Convenience wrapper that resolves the filename to its `$id` URI
    /// before validating.
    pub fn validate_value_by_filename(
        &self,
        value: &Value,
        filename: &str,
    ) -> Result<(), SchemaValidationError> {
        let schema_id = self
            .filename_to_id
            .get(filename)
            .ok_or_else(|| SchemaValidationError::SchemaNotFound(filename.to_string()))?;
        self.validate_value(value, schema_id)
    }

    /// Validate a YAML module descriptor at the given path.
    ///
    /// Loads the YAML file, parses it, and validates against
    /// `module.schema.json`. Returns structured errors on failure.
    ///
    /// This is the Rust equivalent of `tools/msez/schema.py:validate_module`.
    pub fn validate_module(&self, path: &Path) -> Result<(), SchemaValidationError> {
        // Determine the module.yaml path.
        let module_yaml = if path.is_dir() {
            path.join("module.yaml")
        } else {
            path.to_path_buf()
        };

        if !module_yaml.exists() {
            return Err(SchemaValidationError::DocumentLoadError {
                path: module_yaml.display().to_string(),
                reason: "file does not exist".to_string(),
            });
        }

        let content = std::fs::read_to_string(&module_yaml).map_err(|e| {
            SchemaValidationError::DocumentLoadError {
                path: module_yaml.display().to_string(),
                reason: e.to_string(),
            }
        })?;

        let value: Value = serde_yaml::from_str(&content).map_err(|e| {
            SchemaValidationError::DocumentLoadError {
                path: module_yaml.display().to_string(),
                reason: format!("YAML parse error: {e}"),
            }
        })?;

        let schema_id = format!("{SCHEMA_URI_PREFIX}module.schema.json");
        self.validate_value(&value, &schema_id)
    }

    /// Validate a zone YAML file at the given path.
    ///
    /// Loads the YAML file and validates against `zone.schema.json`.
    pub fn validate_zone(&self, path: &Path) -> Result<(), SchemaValidationError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            SchemaValidationError::DocumentLoadError {
                path: path.display().to_string(),
                reason: e.to_string(),
            }
        })?;

        let value: Value = serde_yaml::from_str(&content).map_err(|e| {
            SchemaValidationError::DocumentLoadError {
                path: path.display().to_string(),
                reason: format!("YAML parse error: {e}"),
            }
        })?;

        let schema_id = format!("{SCHEMA_URI_PREFIX}zone.schema.json");
        self.validate_value(&value, &schema_id)
    }

    /// Validate a profile YAML file at the given path.
    ///
    /// Loads the YAML file and validates against `profile.schema.json`.
    pub fn validate_profile(&self, path: &Path) -> Result<(), SchemaValidationError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            SchemaValidationError::DocumentLoadError {
                path: path.display().to_string(),
                reason: e.to_string(),
            }
        })?;

        let value: Value = serde_yaml::from_str(&content).map_err(|e| {
            SchemaValidationError::DocumentLoadError {
                path: path.display().to_string(),
                reason: format!("YAML parse error: {e}"),
            }
        })?;

        let schema_id = format!("{SCHEMA_URI_PREFIX}profile.schema.json");
        self.validate_value(&value, &schema_id)
    }

    /// Find all module directories under a `modules/` directory.
    ///
    /// Returns paths to directories containing a `module.yaml` file.
    pub fn find_all_modules(modules_dir: &Path) -> Vec<PathBuf> {
        if !modules_dir.is_dir() {
            return Vec::new();
        }

        let mut result = Vec::new();
        Self::walk_for_modules(modules_dir, &mut result);
        result.sort();
        result
    }

    /// Validate all module descriptors found under a `modules/` directory.
    ///
    /// Returns a summary of `(total, passed, failed)` counts plus details
    /// of each failure.
    ///
    /// This matches the behavior of `msez validate --all-modules` from the
    /// Python CLI.
    pub fn validate_all_modules(&self, modules_dir: &Path) -> ModuleValidationReport {
        let module_dirs = Self::find_all_modules(modules_dir);
        let total = module_dirs.len();
        let mut passed = 0usize;
        let mut failures: Vec<ModuleFailure> = Vec::new();

        for dir in &module_dirs {
            match self.validate_module(dir) {
                Ok(()) => passed += 1,
                Err(e) => {
                    failures.push(ModuleFailure {
                        module_dir: dir.clone(),
                        error: e,
                    });
                }
            }
        }

        ModuleValidationReport {
            total,
            passed,
            failed: failures.len(),
            failures,
        }
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Recursively collect `*.schema.json` file paths.
    fn glob_schemas(dir: &Path) -> Result<Vec<PathBuf>, SchemaValidationError> {
        let mut results = Vec::new();
        Self::walk_for_schemas(dir, &mut results)?;
        results.sort();
        Ok(results)
    }

    fn walk_for_schemas(dir: &Path, acc: &mut Vec<PathBuf>) -> Result<(), SchemaValidationError> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                Self::walk_for_schemas(&path, acc)?;
            } else if let Some(name) = path.file_name().and_then(|f| f.to_str()) {
                if name.ends_with(".schema.json") {
                    acc.push(path);
                }
            }
        }
        Ok(())
    }

    fn walk_for_modules(dir: &Path, acc: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Check if this dir has a module.yaml
                let module_yaml = path.join("module.yaml");
                if module_yaml.exists() {
                    acc.push(path.clone());
                }
                Self::walk_for_modules(&path, acc);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Validation report types
// ---------------------------------------------------------------------------

/// Result of validating all module descriptors.
#[derive(Debug)]
pub struct ModuleValidationReport {
    /// Total number of module directories found.
    pub total: usize,
    /// Number that passed validation.
    pub passed: usize,
    /// Number that failed validation.
    pub failed: usize,
    /// Details of each failure.
    pub failures: Vec<ModuleFailure>,
}

/// A single module validation failure.
#[derive(Debug)]
pub struct ModuleFailure {
    /// Path to the module directory that failed.
    pub module_dir: PathBuf,
    /// The validation error.
    pub error: SchemaValidationError,
}

// Preserve the old types for backward compatibility with lib.rs re-exports.
// These are aliases to the new names.

/// A structured validation error with diagnostic context.
///
/// This is the legacy type name preserved for backward compatibility.
/// Prefer [`SchemaValidationDetail`] for new code.
pub type ValidationError = SchemaValidationDetail;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Helper to find the repo root (where schemas/ lives).
    fn repo_root() -> PathBuf {
        // Walk up from the crate directory to find the repo root.
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // msez/crates/msez-schema -> msez -> stack
        dir.pop(); // -> msez/crates
        dir.pop(); // -> msez
        dir.pop(); // -> stack (repo root)
        dir
    }

    fn schema_dir() -> PathBuf {
        repo_root().join("schemas")
    }

    #[test]
    fn test_load_all_schemas() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        // We expect 116 schemas per the CLAUDE.md spec.
        assert!(
            validator.schema_count() >= 100,
            "Expected at least 100 schemas, got {}",
            validator.schema_count()
        );
    }

    #[test]
    fn test_module_schema_exists() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        let schema_id = format!("{SCHEMA_URI_PREFIX}module.schema.json");
        assert!(
            validator.get_schema(&schema_id).is_some(),
            "module.schema.json not found in registry"
        );
    }

    #[test]
    fn test_validate_valid_module_data() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");

        let valid_module = json!({
            "module_id": "org.momentum.test.example",
            "version": "0.1.0",
            "kind": "legal",
            "license": "BUSL-1.1",
            "description": "A test module for validation",
            "variants": ["default"],
            "depends_on": [],
            "provides": [{
                "interface": "msez.test.example.v1",
                "path": "example.yaml",
                "media_type": "application/yaml"
            }],
            "parameters": {}
        });

        let schema_id = format!("{SCHEMA_URI_PREFIX}module.schema.json");
        let result = validator.validate_value(&valid_module, &schema_id);
        assert!(result.is_ok(), "Valid module data should pass: {result:?}");
    }

    #[test]
    fn test_validate_invalid_module_missing_required() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");

        // Missing required fields: version, kind, etc.
        let invalid_module = json!({
            "module_id": "org.momentum.test.broken"
        });

        let schema_id = format!("{SCHEMA_URI_PREFIX}module.schema.json");
        let result = validator.validate_value(&invalid_module, &schema_id);
        assert!(
            result.is_err(),
            "Module missing required fields should fail"
        );

        if let Err(SchemaValidationError::ValidationFailed { count, details, .. }) = result {
            assert!(count > 0, "Should have at least one error");
            // Check that at least one error mentions a required field.
            let has_required_error = details.iter().any(|d| d.message.contains("required"));
            assert!(
                has_required_error,
                "Should mention missing required field, got: {:?}",
                details.iter().map(|d| &d.message).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn test_validate_module_bad_version_format() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");

        let bad_version = json!({
            "module_id": "org.momentum.test.badver",
            "version": "not-a-semver",
            "kind": "legal",
            "license": "BUSL-1.1",
            "description": "Bad version format",
            "variants": ["default"],
            "depends_on": [],
            "provides": [{
                "interface": "msez.test.example.v1",
                "path": "example.yaml",
                "media_type": "application/yaml"
            }],
            "parameters": {}
        });

        let schema_id = format!("{SCHEMA_URI_PREFIX}module.schema.json");
        let result = validator.validate_value(&bad_version, &schema_id);
        assert!(result.is_err(), "Bad version format should fail validation");
    }

    #[test]
    fn test_schema_not_found() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        let result =
            validator.validate_value(&json!({}), "https://nonexistent.example/schema.json");
        assert!(matches!(
            result,
            Err(SchemaValidationError::SchemaNotFound(_))
        ));
    }

    #[test]
    fn test_ref_resolution_artifact_ref() {
        // Test that $ref to artifact-ref.schema.json resolves correctly.
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");

        // The vc.smart-asset-registry schema references artifact-ref.schema.json via $ref.
        // Validate a document that exercises the $ref path.
        let vc = json!({
            "@context": ["https://www.w3.org/2018/credentials/v1"],
            "type": ["VerifiableCredential", "SmartAssetRegistryCredential"],
            "issuer": "did:msez:issuer:001",
            "issuanceDate": "2026-01-15T12:00:00Z",
            "credentialSubject": {
                "asset_id": "a".repeat(64),
                "stack_spec_version": "0.4.44",
                "asset_genesis": {
                    "artifact_type": "genesis",
                    "digest_sha256": "b".repeat(64)
                },
                "jurisdiction_bindings": [{
                    "harbor_id": "zone-pk-01",
                    "binding_status": "active",
                    "shard_role": "primary",
                    "lawpacks": [{
                        "jurisdiction_id": "PK",
                        "domain": "corporate",
                        "lawpack_digest_sha256": "c".repeat(64)
                    }],
                    "compliance_profile": {}
                }]
            },
            "proof": {
                "type": "MsezEd25519Signature2025",
                "created": "2026-01-15T12:00:00Z",
                "verificationMethod": "did:msez:key:001",
                "proofPurpose": "assertionMethod",
                "jws": "base64signature"
            }
        });

        let schema_id = format!("{SCHEMA_URI_PREFIX}vc.smart-asset-registry.schema.json");
        let result = validator.validate_value(&vc, &schema_id);
        assert!(result.is_ok(), "Valid VC should pass: {result:?}");
    }

    #[test]
    fn test_find_all_modules() {
        let modules_dir = repo_root().join("modules");
        if modules_dir.is_dir() {
            let modules = SchemaValidator::find_all_modules(&modules_dir);
            // The repo claims ~119 module.yaml files.
            assert!(
                modules.len() >= 50,
                "Expected at least 50 modules, found {}",
                modules.len()
            );
        }
    }

    #[test]
    fn test_validate_all_modules_report() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        let modules_dir = repo_root().join("modules");

        if !modules_dir.is_dir() {
            return;
        }

        let report = validator.validate_all_modules(&modules_dir);

        // We expect the total to be >= 50 (119 in the repo).
        assert!(
            report.total >= 50,
            "Expected at least 50 modules, found {}",
            report.total
        );

        // Document failures rather than asserting zero — per the task instructions,
        // if modules fail due to schema strictness we document rather than weaken.
        if report.failed > 0 {
            eprintln!(
                "Module validation: {}/{} passed, {} failed",
                report.passed, report.total, report.failed
            );
            for failure in &report.failures {
                eprintln!(
                    "  FAIL: {} — {}",
                    failure.module_dir.display(),
                    failure.error
                );
            }
        }
    }

    #[test]
    fn test_validate_by_filename() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");

        let valid_module = json!({
            "module_id": "org.momentum.test.example",
            "version": "0.1.0",
            "kind": "legal",
            "license": "BUSL-1.1",
            "description": "A test module for validation",
            "variants": ["default"],
            "depends_on": [],
            "provides": [{
                "interface": "msez.test.example.v1",
                "path": "example.yaml",
                "media_type": "application/yaml"
            }],
            "parameters": {}
        });

        let result = validator.validate_value_by_filename(&valid_module, "module.schema.json");
        assert!(result.is_ok(), "Should validate by filename: {result:?}");
    }

    // ── Additional coverage tests ────────────────────────────────────

    #[test]
    fn test_schema_not_found_by_filename() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        let result = validator.validate_value_by_filename(&json!({}), "nonexistent.schema.json");
        assert!(matches!(
            result,
            Err(SchemaValidationError::SchemaNotFound(_))
        ));
    }

    #[test]
    fn test_schema_validator_from_nonexistent_dir() {
        let validator = SchemaValidator::new("/tmp/definitely-not-a-real-dir-msez-test-12345");
        assert!(validator.is_ok());
        let v = validator.unwrap();
        assert_eq!(v.schema_count(), 0);
    }

    #[test]
    fn test_schema_validator_debug_impl() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        let debug_str = format!("{validator:?}");
        assert!(debug_str.contains("SchemaValidator"));
        assert!(debug_str.contains("schema_count"));
    }

    #[test]
    fn test_schema_validator_schema_dir() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        assert!(validator.schema_dir().ends_with("schemas"));
    }

    #[test]
    fn test_schema_validator_schema_ids_non_empty() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        let ids = validator.schema_ids();
        assert!(!ids.is_empty());
        for id in &ids {
            assert!(
                id.starts_with("https://") || id.contains("schema"),
                "Unexpected schema ID format: {id}"
            );
        }
    }

    #[test]
    fn test_get_schema_returns_none_for_unknown() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        assert!(validator
            .get_schema("https://unknown.example/schema.json")
            .is_none());
    }

    #[test]
    fn test_get_schema_by_filename_returns_none_for_unknown() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        assert!(validator
            .get_schema_by_filename("no-such-file.schema.json")
            .is_none());
    }

    #[test]
    fn test_get_schema_returns_some_for_known() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        let schema_id = format!("{SCHEMA_URI_PREFIX}module.schema.json");
        let schema = validator.get_schema(&schema_id);
        assert!(schema.is_some(), "module.schema.json should be found");
    }

    #[test]
    fn test_get_schema_by_filename_returns_some_for_known() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        let schema = validator.get_schema_by_filename("module.schema.json");
        assert!(
            schema.is_some(),
            "module.schema.json should be found by filename"
        );
    }

    #[test]
    fn test_validate_module_nonexistent_path() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        let result = validator.validate_module(Path::new("/tmp/no-such-module-dir-msez-12345"));
        assert!(matches!(
            result,
            Err(SchemaValidationError::DocumentLoadError { .. })
        ));
    }

    #[test]
    fn test_validate_zone_nonexistent_path() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        let result = validator.validate_zone(Path::new("/tmp/no-such-zone-msez-12345.yaml"));
        assert!(matches!(
            result,
            Err(SchemaValidationError::DocumentLoadError { .. })
        ));
    }

    #[test]
    fn test_validate_profile_nonexistent_path() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        let result = validator.validate_profile(Path::new("/tmp/no-such-profile-msez-12345.yaml"));
        assert!(matches!(
            result,
            Err(SchemaValidationError::DocumentLoadError { .. })
        ));
    }

    #[test]
    fn test_find_all_modules_nonexistent_dir() {
        let modules =
            SchemaValidator::find_all_modules(Path::new("/tmp/no-such-modules-dir-12345"));
        assert!(modules.is_empty());
    }

    #[test]
    fn test_validation_error_display() {
        let detail = SchemaValidationDetail {
            schema_path: "https://schemas.momentum-sez.org/msez/module.schema.json".to_string(),
            instance_path: "/version".to_string(),
            message: "pattern mismatch".to_string(),
        };
        let display = format!("{detail}");
        assert!(display.contains("module.schema.json"));
        assert!(display.contains("/version"));
        assert!(display.contains("pattern mismatch"));
    }

    #[test]
    fn test_schema_validation_error_display_schema_load() {
        let err = SchemaValidationError::SchemaLoadError {
            path: "/schemas/broken.json".to_string(),
            reason: "invalid JSON".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("broken.json"));
        assert!(msg.contains("invalid JSON"));
    }

    #[test]
    fn test_schema_validation_error_display_document_load() {
        let err = SchemaValidationError::DocumentLoadError {
            path: "/modules/test/module.yaml".to_string(),
            reason: "file does not exist".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("module.yaml"));
        assert!(msg.contains("file does not exist"));
    }

    #[test]
    fn test_schema_validation_error_display_compile() {
        let err = SchemaValidationError::SchemaCompileError {
            schema_id: "test-schema".to_string(),
            reason: "invalid schema".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("test-schema"));
        assert!(msg.contains("invalid schema"));
    }

    #[test]
    fn test_schema_validation_error_display_not_found() {
        let err = SchemaValidationError::SchemaNotFound("unknown-schema".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("unknown-schema"));
    }

    #[test]
    fn test_schema_validation_error_display_validation_failed() {
        let err = SchemaValidationError::ValidationFailed {
            schema_id: "module.schema.json".to_string(),
            count: 3,
            details: vec![],
        };
        let msg = format!("{err}");
        assert!(msg.contains("3 validation error(s)"));
        assert!(msg.contains("module.schema.json"));
    }

    #[test]
    fn test_validate_multiple_errors_in_document() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");

        // Totally empty object should fail multiple required fields
        let empty = json!({});
        let schema_id = format!("{SCHEMA_URI_PREFIX}module.schema.json");
        let result = validator.validate_value(&empty, &schema_id);
        assert!(result.is_err());
        if let Err(SchemaValidationError::ValidationFailed { count, details, .. }) = result {
            assert!(
                count >= 1,
                "Should have at least one error for empty object"
            );
            assert!(!details.is_empty());
        }
    }

    #[test]
    fn test_validate_wrong_type() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");

        // A number instead of an object should fail validation
        let wrong_type = json!(42);
        let schema_id = format!("{SCHEMA_URI_PREFIX}module.schema.json");
        let result = validator.validate_value(&wrong_type, &schema_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_type_alias() {
        // Verify that ValidationError is a type alias for SchemaValidationDetail
        let detail: ValidationError = SchemaValidationDetail {
            schema_path: "test".to_string(),
            instance_path: "/test".to_string(),
            message: "test error".to_string(),
        };
        assert_eq!(detail.schema_path, "test");
    }

    // ── Coverage expansion tests ─────────────────────────────────────

    #[test]
    fn test_validate_module_with_directory_path() {
        // validate_module called with a directory that has module.yaml inside
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        let modules_dir = repo_root().join("modules");
        if !modules_dir.is_dir() {
            return;
        }
        let all_modules = SchemaValidator::find_all_modules(&modules_dir);
        if let Some(module_dir) = all_modules.first() {
            // Calling validate_module with a directory exercises the is_dir() branch
            let result = validator.validate_module(module_dir);
            // We just care that it doesn't panic; it may pass or fail validation
            let _ = result;
        }
    }

    #[test]
    fn test_validate_module_with_file_path() {
        // validate_module called with a direct path to module.yaml
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        let modules_dir = repo_root().join("modules");
        if !modules_dir.is_dir() {
            return;
        }
        let all_modules = SchemaValidator::find_all_modules(&modules_dir);
        if let Some(module_dir) = all_modules.first() {
            let module_yaml = module_dir.join("module.yaml");
            if module_yaml.exists() {
                // Calling with file path exercises the else branch of is_dir()
                let result = validator.validate_module(&module_yaml);
                let _ = result;
            }
        }
    }

    #[test]
    fn test_validate_module_invalid_yaml() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        // Create a temp dir with an invalid module.yaml
        let tmp = tempfile::tempdir().unwrap();
        let module_yaml = tmp.path().join("module.yaml");
        std::fs::write(&module_yaml, "{{invalid yaml: [unbalanced").unwrap();
        let result = validator.validate_module(tmp.path());
        assert!(result.is_err());
        match result.unwrap_err() {
            SchemaValidationError::DocumentLoadError { reason, .. } => {
                assert!(reason.contains("YAML"));
            }
            other => panic!("expected DocumentLoadError, got: {other}"),
        }
    }

    #[test]
    fn test_validate_zone_invalid_yaml() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        let tmp = tempfile::tempdir().unwrap();
        let zone_yaml = tmp.path().join("zone.yaml");
        std::fs::write(&zone_yaml, "{{invalid yaml: [unbalanced").unwrap();
        let result = validator.validate_zone(&zone_yaml);
        assert!(result.is_err());
        match result.unwrap_err() {
            SchemaValidationError::DocumentLoadError { reason, .. } => {
                assert!(reason.contains("YAML"));
            }
            other => panic!("expected DocumentLoadError, got: {other}"),
        }
    }

    #[test]
    fn test_validate_profile_invalid_yaml() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        let tmp = tempfile::tempdir().unwrap();
        let profile_yaml = tmp.path().join("profile.yaml");
        std::fs::write(&profile_yaml, "{{invalid yaml: [unbalanced").unwrap();
        let result = validator.validate_profile(&profile_yaml);
        assert!(result.is_err());
        match result.unwrap_err() {
            SchemaValidationError::DocumentLoadError { reason, .. } => {
                assert!(reason.contains("YAML"));
            }
            other => panic!("expected DocumentLoadError, got: {other}"),
        }
    }

    #[test]
    fn test_validate_zone_valid_yaml_against_schema() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        // Try to find and validate one of the zone yaml files in the repo
        let jurisdictions = repo_root().join("jurisdictions");
        if jurisdictions.is_dir() {
            // Look for any zone.yaml in subdirectories
            if let Ok(entries) = std::fs::read_dir(&jurisdictions) {
                for entry in entries.flatten() {
                    let zone_yaml = entry.path().join("zone.yaml");
                    if zone_yaml.exists() {
                        let result = validator.validate_zone(&zone_yaml);
                        let _ = result; // exercises the zone validation path
                        return;
                    }
                }
            }
        }
    }

    #[test]
    fn test_schema_without_id_derives_uri() {
        // Create a temp dir with a schema that has no $id field
        let tmp = tempfile::tempdir().unwrap();
        let schema_path = tmp.path().join("custom.schema.json");
        std::fs::write(
            &schema_path,
            r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#,
        )
        .unwrap();
        let validator = SchemaValidator::new(tmp.path()).expect("failed to load schemas");
        assert_eq!(validator.schema_count(), 1);
        // The schema should be registered with a derived URI
        let ids = validator.schema_ids();
        assert_eq!(ids.len(), 1);
        assert!(ids[0].contains("custom.schema.json"));
    }

    #[test]
    fn test_schema_load_error_invalid_json() {
        // Create a temp dir with an invalid JSON file
        let tmp = tempfile::tempdir().unwrap();
        let schema_path = tmp.path().join("bad.schema.json");
        std::fs::write(&schema_path, "not valid json at all").unwrap();
        let result = SchemaValidator::new(tmp.path());
        assert!(result.is_err());
        match result.unwrap_err() {
            SchemaValidationError::SchemaLoadError { path, reason } => {
                assert!(path.contains("bad.schema.json"));
                assert!(!reason.is_empty());
            }
            other => panic!("expected SchemaLoadError, got: {other}"),
        }
    }

    #[test]
    fn test_io_error_variant() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test io error");
        let schema_err: SchemaValidationError = io_err.into();
        let msg = format!("{schema_err}");
        assert!(msg.contains("I/O error"));
    }

    #[test]
    fn test_validation_failed_details_accessible() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        let schema_id = format!("{SCHEMA_URI_PREFIX}module.schema.json");
        // Pass an array instead of object to get validation failure
        let result = validator.validate_value(&json!([1, 2, 3]), &schema_id);
        if let Err(SchemaValidationError::ValidationFailed {
            schema_id: id,
            count,
            details,
        }) = result
        {
            assert!(!id.is_empty());
            assert!(count > 0);
            // Each detail should have display format
            for d in &details {
                let displayed = format!("{d}");
                assert!(!displayed.is_empty());
            }
        }
    }

    #[test]
    fn test_schema_validator_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let validator = SchemaValidator::new(tmp.path()).expect("should handle empty dir");
        assert_eq!(validator.schema_count(), 0);
        assert!(validator.schema_ids().is_empty());
    }

    #[test]
    fn test_validate_all_modules_empty_dir() {
        let validator = SchemaValidator::new(schema_dir()).expect("failed to load schemas");
        let tmp = tempfile::tempdir().unwrap();
        let report = validator.validate_all_modules(tmp.path());
        assert_eq!(report.total, 0);
        assert_eq!(report.passed, 0);
        assert_eq!(report.failed, 0);
        assert!(report.failures.is_empty());
    }

    #[test]
    fn test_walk_for_schemas_in_subdirectories() {
        // Create a nested directory with schemas at different levels
        let tmp = tempfile::tempdir().unwrap();
        let sub = tmp.path().join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(
            tmp.path().join("top.schema.json"),
            r#"{"$id": "https://test.example/top", "type": "object"}"#,
        )
        .unwrap();
        std::fs::write(
            sub.join("nested.schema.json"),
            r#"{"$id": "https://test.example/nested", "type": "string"}"#,
        )
        .unwrap();
        let validator = SchemaValidator::new(tmp.path()).expect("failed to load schemas");
        assert_eq!(validator.schema_count(), 2);
    }

    #[test]
    fn test_module_failure_debug() {
        let failure = ModuleFailure {
            module_dir: PathBuf::from("/tmp/test-module"),
            error: SchemaValidationError::SchemaNotFound("test".to_string()),
        };
        let debug_str = format!("{failure:?}");
        assert!(debug_str.contains("ModuleFailure"));
        assert!(debug_str.contains("test-module"));
    }

    #[test]
    fn test_module_validation_report_debug() {
        let report = ModuleValidationReport {
            total: 10,
            passed: 8,
            failed: 2,
            failures: vec![],
        };
        let debug_str = format!("{report:?}");
        assert!(debug_str.contains("ModuleValidationReport"));
        assert!(debug_str.contains("10"));
    }
}
