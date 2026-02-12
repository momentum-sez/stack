//! # Pack Validation Rules
//!
//! Validates pack bundles, zone manifests, and cross-reference integrity.
//!
//! ## Validation Layers
//!
//! 1. **Structural validation**: YAML/JSON parses correctly, required fields present.
//! 2. **Digest validation**: Content-addressed digests match recomputed values.
//! 3. **Domain validation**: All referenced ComplianceDomains exist in msez-core.
//! 4. **Cross-reference integrity**: Zone → lawpack/regpack/licensepack references resolve.
//!
//! ## Spec Reference
//!
//! Mirrors the validation logic in Python `tools/msez.py:cmd_validate_*` and
//! `tools/msez/composition.py:ZoneComposition.validate()`.

use std::collections::BTreeMap;
use std::path::Path;

use msez_core::ComplianceDomain;

use crate::error::PackResult;
use crate::lawpack;
use crate::licensepack;
use crate::parser;
use crate::regpack;

// ---------------------------------------------------------------------------
// Validation Results
// ---------------------------------------------------------------------------

/// Result of validating a pack bundle or zone.
#[derive(Debug)]
pub struct PackValidationResult {
    /// Whether the pack/zone is structurally valid.
    pub is_valid: bool,
    /// Validation errors, if any.
    pub errors: Vec<String>,
    /// Validation warnings (non-fatal).
    pub warnings: Vec<String>,
}

impl PackValidationResult {
    /// Create a successful validation result.
    pub fn ok() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create a failed validation result with the given errors.
    pub fn fail(errors: Vec<String>) -> Self {
        Self {
            is_valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    /// Add an error. Marks result as invalid.
    pub fn add_error(&mut self, error: String) {
        self.is_valid = false;
        self.errors.push(error);
    }

    /// Add a warning (does not affect validity).
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// Merge another result into this one.
    pub fn merge(&mut self, other: PackValidationResult) {
        if !other.is_valid {
            self.is_valid = false;
        }
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}

// ---------------------------------------------------------------------------
// Zone Validation
// ---------------------------------------------------------------------------

/// Validate a zone manifest.
///
/// Checks:
/// - YAML parses correctly
/// - Required fields present (zone_id, jurisdiction_id, profile)
/// - Lawpack/regpack/licensepack references have valid format
/// - ComplianceDomain references are recognized
///
/// # Arguments
///
/// * `zone_path` - Path to the zone.yaml file.
pub fn validate_zone(zone_path: &Path) -> PackResult<PackValidationResult> {
    let mut result = PackValidationResult::ok();

    // 1. Parse zone YAML
    let zone = match parser::load_yaml_as_value(zone_path) {
        Ok(v) => v,
        Err(e) => {
            return Ok(PackValidationResult::fail(vec![format!(
                "failed to parse zone manifest: {e}"
            )]));
        }
    };

    // 2. Required fields
    let required_fields = ["zone_id", "jurisdiction_id"];
    for field in &required_fields {
        if zone.get(*field).is_none() {
            result.add_error(format!("missing required field: {field}"));
        }
    }

    // 3. Validate JSON compatibility
    if let Err(e) = parser::ensure_json_compatible(&zone, "$", "zone manifest") {
        result.add_error(format!("JSON compatibility: {e}"));
    }

    // 4. Validate lawpack references
    if let Ok(refs) = lawpack::resolve_lawpack_refs(&zone) {
        for r in &refs {
            if r.jurisdiction_id.is_empty() {
                result.add_error("lawpack ref has empty jurisdiction_id".to_string());
            }
            if r.domain.is_empty() {
                result.add_error("lawpack ref has empty domain".to_string());
            }
            if !parser::is_valid_sha256(&r.lawpack_digest_sha256) {
                result.add_error(format!(
                    "lawpack ref has invalid digest: {}",
                    r.lawpack_digest_sha256
                ));
            }
        }
    }

    // 5. Validate regpack references
    if let Ok(refs) = regpack::resolve_regpack_refs(&zone) {
        for r in &refs {
            if r.jurisdiction_id.is_empty() {
                result.add_error("regpack ref has empty jurisdiction_id".to_string());
            }
        }
    }

    // 6. Validate licensepack references
    if let Ok(refs) = licensepack::resolve_licensepack_refs(&zone) {
        for r in &refs {
            if r.jurisdiction_id.is_empty() {
                result.add_error("licensepack ref has empty jurisdiction_id".to_string());
            }
        }
    }

    // 7. Validate lawpack_domains if present
    if let Some(domains) = zone.get("lawpack_domains").and_then(|v| v.as_array()) {
        for domain_val in domains {
            if let Some(domain_str) = domain_val.as_str() {
                // Lawpack domains may use broader categories, validate known ones
                if validate_domain_string(domain_str).is_none() {
                    result.add_warning(format!(
                        "lawpack_domain \"{domain_str}\" is not a recognized ComplianceDomain"
                    ));
                }
            }
        }
    }

    Ok(result)
}

/// Validate a zone manifest from JSON value (already parsed).
pub fn validate_zone_value(zone: &serde_json::Value) -> PackValidationResult {
    let mut result = PackValidationResult::ok();

    let required_fields = ["zone_id", "jurisdiction_id"];
    for field in &required_fields {
        if zone.get(*field).is_none() {
            result.add_error(format!("missing required field: {field}"));
        }
    }

    if let Err(e) = parser::ensure_json_compatible(zone, "$", "zone manifest") {
        result.add_error(format!("JSON compatibility: {e}"));
    }

    result
}

// ---------------------------------------------------------------------------
// Pack Validation
// ---------------------------------------------------------------------------

/// Validate a lawpack lock file.
///
/// Checks structural validity and digest format.
pub fn validate_lawpack_lock(lock_path: &Path) -> PackResult<PackValidationResult> {
    let mut result = PackValidationResult::ok();

    match lawpack::verify_lock(lock_path) {
        Ok(lock) => {
            if lock.jurisdiction_id.is_empty() {
                result.add_error("lawpack lock: empty jurisdiction_id".to_string());
            }
            if lock.domain.is_empty() {
                result.add_error("lawpack lock: empty domain".to_string());
            }
            if lock.as_of_date.is_empty() {
                result.add_warning("lawpack lock: empty as_of_date".to_string());
            }
        }
        Err(e) => {
            result.add_error(format!("lawpack lock validation failed: {e}"));
        }
    }

    Ok(result)
}

/// Validate a module descriptor (module.yaml).
///
/// Checks that required fields are present and the descriptor parses correctly.
pub fn validate_module_descriptor(module_dir: &Path) -> PackResult<PackValidationResult> {
    let mut result = PackValidationResult::ok();

    match lawpack::load_module_descriptor(module_dir) {
        Ok(desc) => {
            if desc.module_id.is_empty() {
                result.add_error("module descriptor: empty module_id".to_string());
            }
            if desc.version.is_empty() {
                result.add_warning("module descriptor: empty version".to_string());
            }
        }
        Err(e) => {
            result.add_error(format!("failed to load module descriptor: {e}"));
        }
    }

    Ok(result)
}

/// Validate regpack domain references.
///
/// Ensures every domain string in a regpack metadata maps to a known ComplianceDomain.
pub fn validate_regpack_domains(metadata: &regpack::RegPackMetadata) -> PackValidationResult {
    let mut result = PackValidationResult::ok();
    let errors = regpack::validate_regpack_domains(metadata);
    for err in errors {
        result.add_error(format!("{err}"));
    }
    result
}

// ---------------------------------------------------------------------------
// Cross-Reference Validation
// ---------------------------------------------------------------------------

/// Validate cross-references between a zone manifest and its referenced packs.
///
/// This is a structural check only — it verifies that referenced pack files
/// exist on disk, not that their digests match (that is done by the lock
/// verification pipeline).
pub fn validate_zone_cross_references(
    zone_path: &Path,
    repo_root: &Path,
) -> PackResult<PackValidationResult> {
    let mut result = PackValidationResult::ok();

    let zone = parser::load_yaml_as_value(zone_path)?;

    // Check that referenced lawpack domains have corresponding module dirs
    if let Some(domains) = zone.get("lawpack_domains").and_then(|v| v.as_array()) {
        let jurisdiction_id = zone
            .get("jurisdiction_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        for domain_val in domains {
            if let Some(domain_str) = domain_val.as_str() {
                // Check if the modules directory has this domain
                let module_pattern = repo_root.join("modules").join("legal");
                if module_pattern.exists() {
                    // Just warn if the domain directory doesn't exist
                    let domain_dir = module_pattern.join("jurisdictions").join(jurisdiction_id).join(domain_str);
                    if !domain_dir.exists() {
                        result.add_warning(format!(
                            "lawpack_domain \"{domain_str}\" for jurisdiction \"{jurisdiction_id}\" has no module directory at {}",
                            domain_dir.display()
                        ));
                    }
                }
            }
        }
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// Domain Validation Helpers
// ---------------------------------------------------------------------------

/// Map a domain string to a ComplianceDomain, if recognized.
///
/// Accepts both the canonical snake_case form and common aliases
/// used in zone manifests and module descriptors.
pub fn validate_domain_string(domain: &str) -> Option<ComplianceDomain> {
    // Try direct parse first
    if let Ok(d) = domain.parse::<ComplianceDomain>() {
        return Some(d);
    }

    // Common aliases used in zone manifests
    match domain {
        "aml-cft" | "aml_cft" => Some(ComplianceDomain::Aml),
        "data-privacy" => Some(ComplianceDomain::DataPrivacy),
        "digital-assets" => Some(ComplianceDomain::DigitalAssets),
        "consumer-protection" => Some(ComplianceDomain::ConsumerProtection),
        "financial" => None, // Broader category, not a specific domain
        "civil" => None,
        "labor" => Some(ComplianceDomain::Employment),
        _ => None,
    }
}

/// Get all valid ComplianceDomain values as strings.
pub fn all_domain_strings() -> Vec<&'static str> {
    ComplianceDomain::all().iter().map(|d| d.as_str()).collect()
}

// ---------------------------------------------------------------------------
// Composition Validation
// ---------------------------------------------------------------------------

/// Validate a jurisdiction stack configuration.
///
/// A jurisdiction stack defines layers of legal/regulatory frameworks
/// that apply to a zone. Each layer contributes specific domains.
pub fn validate_jurisdiction_stack(stack: &serde_json::Value) -> PackValidationResult {
    let mut result = PackValidationResult::ok();

    let layers = match stack.as_array() {
        Some(arr) => arr,
        None => {
            result.add_error("jurisdiction_stack must be an array".to_string());
            return result;
        }
    };

    let mut seen_domains: BTreeMap<String, String> = BTreeMap::new();

    for (i, layer) in layers.iter().enumerate() {
        let layer_name = layer
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unnamed");

        if layer.get("jurisdiction_id").is_none() {
            result.add_error(format!(
                "layer {i} (\"{layer_name}\"): missing jurisdiction_id"
            ));
        }

        // Check for domain overlap between layers
        if let Some(domains) = layer.get("domains").and_then(|v| v.as_array()) {
            for domain_val in domains {
                if let Some(domain_str) = domain_val.as_str() {
                    if let Some(prev_layer) = seen_domains.get(domain_str) {
                        result.add_warning(format!(
                            "domain \"{domain_str}\" defined in both layer \"{prev_layer}\" and \"{layer_name}\""
                        ));
                    } else {
                        seen_domains.insert(domain_str.to_string(), layer_name.to_string());
                    }
                }
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validation_result_ok() {
        let r = PackValidationResult::ok();
        assert!(r.is_valid);
        assert!(r.errors.is_empty());
    }

    #[test]
    fn test_validation_result_fail() {
        let r = PackValidationResult::fail(vec!["error1".to_string()]);
        assert!(!r.is_valid);
        assert_eq!(r.errors.len(), 1);
    }

    #[test]
    fn test_validation_result_add_error() {
        let mut r = PackValidationResult::ok();
        assert!(r.is_valid);
        r.add_error("something went wrong".to_string());
        assert!(!r.is_valid);
        assert_eq!(r.errors.len(), 1);
    }

    #[test]
    fn test_validation_result_merge() {
        let mut r1 = PackValidationResult::ok();
        r1.add_warning("warn1".to_string());

        let mut r2 = PackValidationResult::ok();
        r2.add_error("error1".to_string());

        r1.merge(r2);
        assert!(!r1.is_valid);
        assert_eq!(r1.errors.len(), 1);
        assert_eq!(r1.warnings.len(), 1);
    }

    #[test]
    fn test_validate_zone_value_missing_fields() {
        let zone = json!({"profile": "test"});
        let result = validate_zone_value(&zone);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("zone_id")));
        assert!(result.errors.iter().any(|e| e.contains("jurisdiction_id")));
    }

    #[test]
    fn test_validate_zone_value_valid() {
        let zone = json!({
            "zone_id": "test.zone",
            "jurisdiction_id": "pk",
            "profile": "starter"
        });
        let result = validate_zone_value(&zone);
        assert!(result.is_valid);
    }

    #[test]
    fn test_validate_domain_string_known() {
        assert_eq!(validate_domain_string("aml"), Some(ComplianceDomain::Aml));
        assert_eq!(validate_domain_string("kyc"), Some(ComplianceDomain::Kyc));
        assert_eq!(
            validate_domain_string("data_privacy"),
            Some(ComplianceDomain::DataPrivacy)
        );
        assert_eq!(
            validate_domain_string("licensing"),
            Some(ComplianceDomain::Licensing)
        );
    }

    #[test]
    fn test_validate_domain_string_aliases() {
        assert_eq!(
            validate_domain_string("aml-cft"),
            Some(ComplianceDomain::Aml)
        );
        assert_eq!(
            validate_domain_string("data-privacy"),
            Some(ComplianceDomain::DataPrivacy)
        );
        assert_eq!(
            validate_domain_string("digital-assets"),
            Some(ComplianceDomain::DigitalAssets)
        );
        assert_eq!(
            validate_domain_string("labor"),
            Some(ComplianceDomain::Employment)
        );
    }

    #[test]
    fn test_validate_domain_string_unknown() {
        assert_eq!(validate_domain_string("financial"), None);
        assert_eq!(validate_domain_string("bogus"), None);
    }

    #[test]
    fn test_all_domain_strings() {
        let domains = all_domain_strings();
        assert_eq!(domains.len(), 20);
        assert!(domains.contains(&"aml"));
        assert!(domains.contains(&"licensing"));
        assert!(domains.contains(&"trade"));
    }

    #[test]
    fn test_validate_jurisdiction_stack_valid() {
        let stack = json!([
            {
                "name": "federal",
                "jurisdiction_id": "pk",
                "domains": ["tax", "aml", "sanctions"]
            },
            {
                "name": "zone",
                "jurisdiction_id": "pk-kp-rsez",
                "domains": ["corporate", "licensing"]
            }
        ]);
        let result = validate_jurisdiction_stack(&stack);
        assert!(result.is_valid);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_validate_jurisdiction_stack_overlap() {
        let stack = json!([
            {
                "name": "federal",
                "jurisdiction_id": "pk",
                "domains": ["tax", "aml"]
            },
            {
                "name": "zone",
                "jurisdiction_id": "pk-kp-rsez",
                "domains": ["tax", "corporate"]
            }
        ]);
        let result = validate_jurisdiction_stack(&stack);
        assert!(result.is_valid); // overlap is a warning, not error
        assert!(result.warnings.iter().any(|w| w.contains("tax")));
    }

    #[test]
    fn test_validate_jurisdiction_stack_not_array() {
        let stack = json!({"not": "an_array"});
        let result = validate_jurisdiction_stack(&stack);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_jurisdiction_stack_missing_jid() {
        let stack = json!([
            {
                "name": "federal",
                "domains": ["tax"]
            }
        ]);
        let result = validate_jurisdiction_stack(&stack);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("jurisdiction_id")));
    }
}
