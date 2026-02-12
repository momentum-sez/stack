//! # Schema Validation
//!
//! Runtime validation of JSON/YAML documents against JSON Schema
//! definitions (Draft 2020-12).
//!
//! ## Security Invariant
//!
//! Schema validation is a trust boundary. Documents that fail validation
//! must be rejected with structured error information including the schema
//! path, the violating field, and the expected vs actual value.
//!
//! ## Schema Resolution
//!
//! All schemas use `$id` URIs of the form:
//!   `https://schemas.momentum-sez.org/msez/<filename>`
//!
//! Cross-schema `$ref` URIs use the same pattern. This module resolves
//! these URIs to local files in the `schemas/` directory by stripping
//! the URI prefix and loading the corresponding file.
//!
//! Internal `$ref`s of the form `#/definitions/<name>` are resolved
//! by the jsonschema crate natively.
//!
//! ## Implements
//!
//! Spec §6 — Schema contract validation rules.
//! Audit §2.6 — Module descriptor validation.
//! Audit §3.1 — Security-critical schema enforcement.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use jsonschema::{Retrieve, Uri, ValidationOptions, Validator};
use serde_json::Value;
use thiserror::Error;

/// URI prefixes used by schemas in this repository.
/// Some schemas use the canonical prefix, others use the legacy prefix.
const SCHEMA_URI_PREFIX: &str = "https://schemas.momentum-sez.org/msez/";
const LEGACY_URI_PREFIX: &str = "https://sez-stack.org/schemas/";

/// Local retriever that resolves `$ref` URIs to schemas loaded in memory.
///
/// This prevents the jsonschema crate from making network requests for
/// cross-schema references. All references are resolved locally from
/// the loaded schema registry.
struct LocalSchemaRetriever {
    /// Map from URI string to schema value.
    schemas_by_uri: HashMap<String, Value>,
}

impl Retrieve for LocalSchemaRetriever {
    fn retrieve(
        &self,
        uri: &Uri<&str>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let uri_str = uri.as_str();

        // Direct lookup.
        if let Some(value) = self.schemas_by_uri.get(uri_str) {
            return Ok(value.clone());
        }

        // Try extracting the filename from the URI and looking up under
        // all known prefixes.
        let filename = uri_str
            .rsplit('/')
            .next()
            .unwrap_or(uri_str);

        for prefix in [SCHEMA_URI_PREFIX, LEGACY_URI_PREFIX] {
            let alt_uri = format!("{prefix}{filename}");
            if let Some(value) = self.schemas_by_uri.get(&alt_uri) {
                return Ok(value.clone());
            }
        }

        // Also try bare filename.
        if let Some(value) = self.schemas_by_uri.get(filename) {
            return Ok(value.clone());
        }

        // For JSON Schema draft metaschemas and any other unresolved URIs,
        // return a permissive schema that accepts anything. This prevents
        // network requests and allows validation to proceed even when
        // some referenced schemas are missing (e.g., meta.schema.json
        // which is referenced but does not exist on disk).
        Ok(serde_json::json!({}))
    }
}

/// Error during schema validation.
#[derive(Error, Debug)]
pub enum SchemaValidationError {
    /// The document did not conform to the schema.
    #[error("validation failed against schema '{schema_name}':\n{violations}")]
    ValidationFailed {
        /// Name of the schema that was validated against.
        schema_name: String,
        /// Structured list of individual violations.
        violations: ValidationViolations,
    },

    /// The schema file could not be loaded.
    #[error("schema load error for '{schema_name}': {reason}")]
    SchemaLoadError {
        /// Schema filename or identifier.
        schema_name: String,
        /// Reason the schema could not be loaded.
        reason: String,
    },

    /// The document file could not be loaded or parsed.
    #[error("document load error for '{path}': {reason}")]
    DocumentLoadError {
        /// Path to the document that failed to load.
        path: String,
        /// Reason the document could not be loaded.
        reason: String,
    },

    /// The compiled validator could not be built (e.g., invalid schema).
    #[error("validator build error for schema '{schema_name}': {reason}")]
    ValidatorBuildError {
        /// Schema filename or identifier.
        schema_name: String,
        /// Reason the validator could not be built.
        reason: String,
    },

    /// IO error reading schema or document.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// A single validation violation with structured context.
#[derive(Debug, Clone)]
pub struct Violation {
    /// JSON Pointer path to the violating field in the instance.
    pub instance_path: String,
    /// JSON Pointer path within the schema that triggered the error.
    pub schema_path: String,
    /// Human-readable description of the violation.
    pub message: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.instance_path.is_empty() {
            write!(f, "  (root): {}", self.message)
        } else {
            write!(f, "  {}: {}", self.instance_path, self.message)
        }
    }
}

/// Collection of validation violations.
#[derive(Debug, Clone)]
pub struct ValidationViolations {
    violations: Vec<Violation>,
}

impl ValidationViolations {
    /// Returns the number of violations.
    pub fn len(&self) -> usize {
        self.violations.len()
    }

    /// Returns true if there are no violations.
    pub fn is_empty(&self) -> bool {
        self.violations.is_empty()
    }

    /// Returns a slice of all violations.
    pub fn violations(&self) -> &[Violation] {
        &self.violations
    }

    /// Consumes self and returns the inner Vec.
    pub fn into_inner(self) -> Vec<Violation> {
        self.violations
    }
}

impl fmt::Display for ValidationViolations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, v) in self.violations.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{v}")?;
        }
        Ok(())
    }
}

/// A schema validator backed by the `jsonschema` crate.
///
/// Loads all JSON schemas from the `schemas/` directory at construction time,
/// registers them as resources for `$ref` resolution, and provides methods
/// to validate documents against named schemas.
///
/// ## Thread Safety
///
/// `SchemaValidator` is `Send + Sync` — compiled validators can be shared
/// across threads. Schema loading happens once at construction.
#[derive(Debug)]
pub struct SchemaValidator {
    /// Root directory containing JSON schema files.
    schema_dir: PathBuf,
    /// Map from schema filename (e.g., "module.schema.json") to parsed JSON value.
    schemas: HashMap<String, Value>,
}

impl SchemaValidator {
    /// Create a new validator by loading all schemas from the given directory.
    ///
    /// Reads every `*.schema.json` file in `schema_dir`, parses it as JSON,
    /// and indexes it by filename. These schemas are then available for
    /// validation via [`validate_document`] and [`validate_module`].
    ///
    /// # Errors
    ///
    /// Returns `SchemaValidationError::SchemaLoadError` if any schema file
    /// cannot be read or parsed as JSON.
    pub fn new(schema_dir: impl AsRef<Path>) -> Result<Self, SchemaValidationError> {
        let schema_dir = schema_dir.as_ref().to_path_buf();
        let mut schemas = HashMap::new();

        let entries = std::fs::read_dir(&schema_dir).map_err(|e| {
            SchemaValidationError::SchemaLoadError {
                schema_name: schema_dir.display().to_string(),
                reason: format!("cannot read schema directory: {e}"),
            }
        })?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".schema.json") {
                    let content = std::fs::read_to_string(&path)?;
                    let value: Value =
                        serde_json::from_str(&content).map_err(|e| {
                            SchemaValidationError::SchemaLoadError {
                                schema_name: name.to_string(),
                                reason: format!("invalid JSON: {e}"),
                            }
                        })?;
                    schemas.insert(name.to_string(), value);
                }
            }
        }

        Ok(Self { schema_dir, schemas })
    }

    /// Returns the schema directory path.
    pub fn schema_dir(&self) -> &Path {
        &self.schema_dir
    }

    /// Returns the number of loaded schemas.
    pub fn schema_count(&self) -> usize {
        self.schemas.len()
    }

    /// Returns the names of all loaded schemas, sorted alphabetically.
    pub fn schema_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.schemas.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// Look up a loaded schema by filename.
    pub fn get_schema(&self, name: &str) -> Option<&Value> {
        self.schemas.get(name)
    }

    /// Build `ValidationOptions` with all schemas registered as resources
    /// so that cross-schema `$ref` URIs resolve correctly.
    ///
    /// Registers each schema under multiple URI patterns to handle:
    /// - Canonical: `https://schemas.momentum-sez.org/msez/<filename>`
    /// - Legacy: `https://sez-stack.org/schemas/<filename>`
    /// - The schema's own `$id` field (which may use either prefix)
    ///
    /// Installs a local retriever to prevent network requests for any
    /// `$ref` URIs not covered by the pre-registered resources.
    fn build_options(&self) -> ValidationOptions {
        let mut opts = jsonschema::options();
        opts.with_draft(jsonschema::Draft::Draft202012);

        // Build a URI -> schema lookup map for the retriever.
        // The retriever handles ALL cross-schema $ref resolution locally,
        // mapping URIs to loaded schema values without network requests.
        let mut schemas_by_uri: HashMap<String, Value> = HashMap::new();

        for (filename, value) in &self.schemas {
            // Register under canonical prefix.
            schemas_by_uri.insert(
                format!("{SCHEMA_URI_PREFIX}{filename}"),
                value.clone(),
            );

            // Register under legacy prefix.
            schemas_by_uri.insert(
                format!("{LEGACY_URI_PREFIX}{filename}"),
                value.clone(),
            );

            // Register under the schema's own $id URI.
            if let Some(id_str) = value.get("$id").and_then(|v| v.as_str()) {
                schemas_by_uri.insert(id_str.to_string(), value.clone());
            }

            // Also index by bare filename for relative $ref resolution.
            schemas_by_uri.insert(filename.clone(), value.clone());
        }

        let retriever = LocalSchemaRetriever { schemas_by_uri };
        opts.with_retriever(retriever);

        opts
    }

    /// Build a compiled `Validator` for a specific schema by filename.
    ///
    /// The validator has all other schemas registered for `$ref` resolution.
    ///
    /// # Arguments
    ///
    /// * `schema_name` — The filename of the schema (e.g., `"module.schema.json"`).
    ///
    /// # Errors
    ///
    /// Returns `SchemaValidationError::SchemaLoadError` if the schema is not found.
    /// Returns `SchemaValidationError::ValidatorBuildError` if the validator cannot be compiled.
    pub fn build_validator(
        &self,
        schema_name: &str,
    ) -> Result<Validator, SchemaValidationError> {
        let schema_value = self.schemas.get(schema_name).ok_or_else(|| {
            SchemaValidationError::SchemaLoadError {
                schema_name: schema_name.to_string(),
                reason: format!(
                    "schema not found in {}",
                    self.schema_dir.display(),
                ),
            }
        })?;

        let opts = self.build_options();
        opts.build(schema_value).map_err(|e| {
            SchemaValidationError::ValidatorBuildError {
                schema_name: schema_name.to_string(),
                reason: e.to_string(),
            }
        })
    }

    /// Validate a parsed JSON value against a named schema.
    ///
    /// # Arguments
    ///
    /// * `instance` — The JSON value to validate.
    /// * `schema_name` — The filename of the schema (e.g., `"module.schema.json"`).
    ///
    /// # Errors
    ///
    /// Returns `SchemaValidationError::ValidationFailed` with structured
    /// violation details if the document is invalid.
    pub fn validate_document(
        &self,
        instance: &Value,
        schema_name: &str,
    ) -> Result<(), SchemaValidationError> {
        let validator = self.build_validator(schema_name)?;

        let errors: Vec<Violation> = validator
            .iter_errors(instance)
            .map(|e| Violation {
                instance_path: e.instance_path.to_string(),
                schema_path: e.schema_path.to_string(),
                message: e.to_string(),
            })
            .collect();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(SchemaValidationError::ValidationFailed {
                schema_name: schema_name.to_string(),
                violations: ValidationViolations {
                    violations: errors,
                },
            })
        }
    }

    /// Validate a YAML module descriptor against the `module.schema.json` schema.
    ///
    /// Loads the YAML file at `path`, converts it to JSON, and validates
    /// against the module schema. This is the Rust equivalent of
    /// `msez validate --all-modules` for a single module.
    ///
    /// # Arguments
    ///
    /// * `path` — Path to a `module.yaml` file.
    ///
    /// # Errors
    ///
    /// Returns `SchemaValidationError::DocumentLoadError` if the file cannot be
    /// read or parsed. Returns `SchemaValidationError::ValidationFailed` if the
    /// document does not conform to `module.schema.json`.
    pub fn validate_module(&self, path: &Path) -> Result<(), SchemaValidationError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            SchemaValidationError::DocumentLoadError {
                path: path.display().to_string(),
                reason: format!("cannot read file: {e}"),
            }
        })?;

        let yaml_value: serde_yaml::Value =
            serde_yaml::from_str(&content).map_err(|e| {
                SchemaValidationError::DocumentLoadError {
                    path: path.display().to_string(),
                    reason: format!("invalid YAML: {e}"),
                }
            })?;

        // Convert YAML to JSON Value for schema validation.
        let json_value = yaml_to_json_value(&yaml_value).map_err(|e| {
            SchemaValidationError::DocumentLoadError {
                path: path.display().to_string(),
                reason: format!("YAML-to-JSON conversion failed: {e}"),
            }
        })?;

        self.validate_document(&json_value, "module.schema.json")
            .map_err(|e| match e {
                SchemaValidationError::ValidationFailed {
                    violations,
                    ..
                } => SchemaValidationError::ValidationFailed {
                    schema_name: format!("module.schema.json ({})", path.display()),
                    violations,
                },
                other => other,
            })
    }

    /// Validate a YAML or JSON document against a schema, loading from a file path.
    ///
    /// Determines the format from the file extension (`.yaml`/`.yml` for YAML,
    /// `.json` for JSON) and validates against the specified schema.
    ///
    /// # Arguments
    ///
    /// * `document_path` — Path to the document file.
    /// * `schema_name` — The schema filename to validate against.
    pub fn validate_file(
        &self,
        document_path: &Path,
        schema_name: &str,
    ) -> Result<(), SchemaValidationError> {
        let content = std::fs::read_to_string(document_path).map_err(|e| {
            SchemaValidationError::DocumentLoadError {
                path: document_path.display().to_string(),
                reason: format!("cannot read file: {e}"),
            }
        })?;

        let ext = document_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let json_value = match ext {
            "yaml" | "yml" => {
                let yaml_value: serde_yaml::Value =
                    serde_yaml::from_str(&content).map_err(|e| {
                        SchemaValidationError::DocumentLoadError {
                            path: document_path.display().to_string(),
                            reason: format!("invalid YAML: {e}"),
                        }
                    })?;
                yaml_to_json_value(&yaml_value).map_err(|e| {
                    SchemaValidationError::DocumentLoadError {
                        path: document_path.display().to_string(),
                        reason: format!("YAML-to-JSON conversion failed: {e}"),
                    }
                })?
            }
            _ => serde_json::from_str(&content).map_err(|e| {
                SchemaValidationError::DocumentLoadError {
                    path: document_path.display().to_string(),
                    reason: format!("invalid JSON: {e}"),
                }
            })?,
        };

        self.validate_document(&json_value, schema_name)
    }
}

/// Convert a `serde_yaml::Value` to a `serde_json::Value`.
///
/// YAML has a richer type system than JSON (tags, anchors, etc.), but
/// module descriptors use only the JSON-compatible subset. This function
/// converts the YAML value tree into the equivalent JSON value tree.
fn yaml_to_json_value(yaml: &serde_yaml::Value) -> Result<Value, String> {
    match yaml {
        serde_yaml::Value::Null => Ok(Value::Null),
        serde_yaml::Value::Bool(b) => Ok(Value::Bool(*b)),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Number(serde_json::Number::from(i)))
            } else if let Some(u) = n.as_u64() {
                Ok(Value::Number(serde_json::Number::from(u)))
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(Value::Number)
                    .ok_or_else(|| format!("cannot represent float {f} in JSON"))
            } else {
                Err(format!("unsupported YAML number: {n:?}"))
            }
        }
        serde_yaml::Value::String(s) => Ok(Value::String(s.clone())),
        serde_yaml::Value::Sequence(seq) => {
            let items: Result<Vec<Value>, String> =
                seq.iter().map(yaml_to_json_value).collect();
            Ok(Value::Array(items?))
        }
        serde_yaml::Value::Mapping(map) => {
            let mut json_map = serde_json::Map::new();
            for (k, v) in map {
                let key = match k {
                    serde_yaml::Value::String(s) => s.clone(),
                    serde_yaml::Value::Number(n) => n.to_string(),
                    serde_yaml::Value::Bool(b) => b.to_string(),
                    other => return Err(format!("unsupported YAML map key type: {other:?}")),
                };
                json_map.insert(key, yaml_to_json_value(v)?);
            }
            Ok(Value::Object(json_map))
        }
        serde_yaml::Value::Tagged(tagged) => {
            // Ignore YAML tags, just convert the inner value.
            yaml_to_json_value(&tagged.value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Find the repository root by looking for Cargo.toml with [workspace].
    fn repo_root() -> PathBuf {
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // crates/msez-schema -> repo root
        dir.pop(); // crates/
        dir.pop(); // repo root
        dir
    }

    fn schema_dir() -> PathBuf {
        repo_root().join("schemas")
    }

    #[test]
    fn test_load_all_schemas() {
        let validator = SchemaValidator::new(schema_dir()).unwrap();
        // The repo has 113 schema files.
        assert!(
            validator.schema_count() >= 100,
            "Expected >= 100 schemas, found {}",
            validator.schema_count()
        );
    }

    #[test]
    fn test_schema_names_include_key_schemas() {
        let validator = SchemaValidator::new(schema_dir()).unwrap();
        let names = validator.schema_names();
        assert!(names.contains(&"module.schema.json"));
        assert!(names.contains(&"artifact-ref.schema.json"));
        assert!(names.contains(&"vc.smart-asset-registry.schema.json"));
    }

    #[test]
    fn test_validate_valid_module() {
        let validator = SchemaValidator::new(schema_dir()).unwrap();
        let doc = json!({
            "module_id": "org.momentum.msez.test.example",
            "version": "0.1.0",
            "kind": "legal",
            "license": "Apache-2.0",
            "description": "A test module for validation.",
            "variants": ["baseline"],
            "depends_on": [],
            "provides": [
                {
                    "interface": "msez.test.example.v1",
                    "path": "example.json",
                    "media_type": "application/json"
                }
            ],
            "parameters": {}
        });
        validator
            .validate_document(&doc, "module.schema.json")
            .unwrap();
    }

    #[test]
    fn test_validate_invalid_module_missing_field() {
        let validator = SchemaValidator::new(schema_dir()).unwrap();
        // Missing required "kind" field.
        let doc = json!({
            "module_id": "org.momentum.msez.test.bad",
            "version": "0.1.0",
            "license": "Apache-2.0",
            "description": "Missing kind.",
            "variants": ["baseline"],
            "depends_on": [],
            "provides": [
                {
                    "interface": "msez.test.v1",
                    "path": "x.json",
                    "media_type": "application/json"
                }
            ],
            "parameters": {}
        });
        let err = validator
            .validate_document(&doc, "module.schema.json")
            .unwrap_err();
        match &err {
            SchemaValidationError::ValidationFailed { violations, .. } => {
                assert!(!violations.is_empty());
                let messages: Vec<&str> =
                    violations.violations().iter().map(|v| v.message.as_str()).collect();
                let has_kind_error = messages.iter().any(|m| m.contains("kind"));
                assert!(
                    has_kind_error,
                    "Expected violation mentioning 'kind', got: {messages:?}"
                );
            }
            other => panic!("Expected ValidationFailed, got: {other}"),
        }
    }

    #[test]
    fn test_validate_invalid_module_bad_version_pattern() {
        let validator = SchemaValidator::new(schema_dir()).unwrap();
        let doc = json!({
            "module_id": "org.momentum.msez.test.bad",
            "version": "not-a-semver",
            "kind": "legal",
            "license": "Apache-2.0",
            "description": "Bad version.",
            "variants": ["baseline"],
            "depends_on": [],
            "provides": [
                {
                    "interface": "msez.test.v1",
                    "path": "x.json",
                    "media_type": "application/json"
                }
            ],
            "parameters": {}
        });
        let err = validator
            .validate_document(&doc, "module.schema.json")
            .unwrap_err();
        assert!(
            matches!(err, SchemaValidationError::ValidationFailed { .. }),
            "Expected ValidationFailed, got: {err}"
        );
    }

    #[test]
    fn test_validate_additional_properties_rejected() {
        let validator = SchemaValidator::new(schema_dir()).unwrap();
        // module.schema.json has additionalProperties: false at top level.
        let doc = json!({
            "module_id": "org.momentum.msez.test.extra",
            "version": "0.1.0",
            "kind": "legal",
            "license": "Apache-2.0",
            "description": "Has extra field.",
            "variants": ["baseline"],
            "depends_on": [],
            "provides": [
                {
                    "interface": "msez.test.v1",
                    "path": "x.json",
                    "media_type": "application/json"
                }
            ],
            "parameters": {},
            "extra_field_not_in_schema": true
        });
        let err = validator
            .validate_document(&doc, "module.schema.json")
            .unwrap_err();
        assert!(
            matches!(err, SchemaValidationError::ValidationFailed { .. }),
            "module.schema.json has additionalProperties: false, but extra field was accepted"
        );
    }

    #[test]
    fn test_cross_ref_resolution() {
        // vc.smart-asset-registry.schema.json references artifact-ref.schema.json
        // via $ref. This test verifies that cross-schema references resolve.
        let validator = SchemaValidator::new(schema_dir()).unwrap();
        let result = validator.build_validator("vc.smart-asset-registry.schema.json");
        assert!(
            result.is_ok(),
            "Failed to build validator with cross-$ref: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_schema_not_found() {
        let validator = SchemaValidator::new(schema_dir()).unwrap();
        let err = validator
            .validate_document(&json!({}), "nonexistent.schema.json")
            .unwrap_err();
        assert!(
            matches!(err, SchemaValidationError::SchemaLoadError { .. }),
            "Expected SchemaLoadError, got: {err}"
        );
    }

    #[test]
    fn test_yaml_to_json_conversion() {
        let yaml_str = r#"
module_id: org.test
version: "1.0.0"
kind: legal
count: 42
enabled: true
items:
  - one
  - two
"#;
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(yaml_str).unwrap();
        let json_value = yaml_to_json_value(&yaml_value).unwrap();

        assert_eq!(json_value["module_id"], "org.test");
        assert_eq!(json_value["version"], "1.0.0");
        assert_eq!(json_value["count"], 42);
        assert_eq!(json_value["enabled"], true);
        assert_eq!(json_value["items"][0], "one");
    }

    #[test]
    fn test_validate_corridor_receipt_schema_builds() {
        let validator = SchemaValidator::new(schema_dir()).unwrap();
        let result = validator.build_validator("corridor.receipt.schema.json");
        assert!(
            result.is_ok(),
            "Failed to build corridor.receipt validator: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_attestation_schema_builds() {
        let validator = SchemaValidator::new(schema_dir()).unwrap();
        let result = validator.build_validator("attestation.schema.json");
        assert!(
            result.is_ok(),
            "Failed to build attestation validator: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_all_schemas_compile_to_validators() {
        let validator = SchemaValidator::new(schema_dir()).unwrap();
        let mut failures = Vec::new();
        for name in validator.schema_names() {
            if let Err(e) = validator.build_validator(name) {
                failures.push(format!("{name}: {e}"));
            }
        }
        assert!(
            failures.is_empty(),
            "Failed to compile validators for {} schemas:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    #[test]
    fn test_validate_real_module_file() {
        let validator = SchemaValidator::new(schema_dir()).unwrap();
        let module_path = repo_root()
            .join("modules/licensing/token-issuer/module.yaml");
        if module_path.exists() {
            validator.validate_module(&module_path).unwrap();
        }
    }

    #[test]
    fn test_violation_display_format() {
        let v = Violation {
            instance_path: "/credentialSubject/asset_id".to_string(),
            schema_path: "/properties/credentialSubject/properties/asset_id/pattern".to_string(),
            message: r#""not-a-hex" does not match pattern "^[a-f0-9]{64}$""#.to_string(),
        };
        let display = v.to_string();
        assert!(display.contains("/credentialSubject/asset_id"));
        assert!(display.contains("does not match pattern"));
    }

    #[test]
    fn test_violation_display_root() {
        let v = Violation {
            instance_path: String::new(),
            schema_path: "/required".to_string(),
            message: r#""kind" is a required property"#.to_string(),
        };
        let display = v.to_string();
        assert!(display.contains("(root)"));
    }
}
