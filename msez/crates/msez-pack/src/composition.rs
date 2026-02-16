//! # Multi-Jurisdiction Zone Composition Engine
//!
//! Enables deployments like:
//! > "Deploy the civic code of NY with the corporate law of Delaware,
//! > but with the digital asset clearance, settlement and securities
//! > laws of ADGM with automated AI arbitration turned on"
//!
//! ## Architecture
//!
//! A [`ZoneComposition`] is built from multiple [`JurisdictionLayer`]s,
//! each contributing specific [`ComplianceDomain`]s:
//!
//! ```text
//! ZoneComposition
//! ├── layers[0]:    JurisdictionLayer (e.g., NY civic code)
//! ├── layers[1]:    JurisdictionLayer (e.g., Delaware corporate)
//! ├── layers[2]:    JurisdictionLayer (e.g., ADGM digital assets)
//! ├── arbitration:  ArbitrationConfig (e.g., AI-assisted DIFC-LCIA)
//! └── corridors:    Vec<CorridorConfig>
//! ```
//!
//! Each layer specifies:
//! - Jurisdiction ID (hierarchical: `country-region-zone`)
//! - Domains to import (from [`ComplianceDomain`])
//! - Lawpacks, regpacks, licensepacks to pin
//!
//! The composition engine:
//! 1. Validates layer compatibility (no domain conflicts)
//! 2. Generates a unified `zone.yaml`
//! 3. Computes a deterministic composition digest
//!
//! ## Ported from Python
//!
//! This module replaces `tools/msez/composition.py` (P1-006).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use msez_core::{CanonicalBytes, ComplianceDomain};

use crate::error::{PackError, PackResult};
use crate::lawpack::LawpackRef;
use crate::licensepack::LicensepackRef;
use crate::parser;
use crate::regpack::RegpackRef;

/// Jurisdiction ID format: two lowercase letters, optionally followed
/// by hyphen-separated segments of lowercase alphanumerics.
///
/// Examples: `us`, `us-ny`, `ae-abudhabi-adgm`, `pk-rsez`
fn is_valid_jurisdiction_id(s: &str) -> bool {
    lazy_static_regex(s)
}

/// Compiled check for `^[a-z]{2}(-[a-z0-9-]+)*$`.
fn lazy_static_regex(s: &str) -> bool {
    if s.len() < 2 {
        return false;
    }
    let bytes = s.as_bytes();
    // First two chars must be lowercase ascii letters
    if !bytes[0].is_ascii_lowercase() || !bytes[1].is_ascii_lowercase() {
        return false;
    }
    if bytes.len() == 2 {
        return true;
    }
    // Rest must be hyphen-separated segments of [a-z0-9-]
    if bytes[2] != b'-' {
        return false;
    }
    // After the first hyphen, allow lowercase, digits, and hyphens
    // but don't allow trailing hyphen or double hyphens
    let tail = &s[3..];
    if tail.is_empty() {
        return false;
    }
    let mut prev_was_hyphen = false;
    for ch in tail.bytes() {
        if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != b'-' {
            return false;
        }
        if ch == b'-' {
            if prev_was_hyphen {
                return false; // reject double hyphens
            }
            prev_was_hyphen = true;
        } else {
            prev_was_hyphen = false;
        }
    }
    // No trailing hyphen
    !s.ends_with('-')
}

/// Zone ID format: starts with lowercase letter, then lowercase, digits,
/// dots, and hyphens.
fn is_valid_zone_id(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let bytes = s.as_bytes();
    if !bytes[0].is_ascii_lowercase() {
        return false;
    }
    bytes
        .iter()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'.' || *b == b'-')
}

// -------------------------------------------------------------------------
// Arbitration
// -------------------------------------------------------------------------

/// Arbitration modes for dispute resolution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArbitrationMode {
    /// Traditional institutional arbitration.
    #[default]
    Traditional,
    /// AI-assisted with human oversight.
    AiAssisted,
    /// Fully autonomous AI arbitration.
    AiAutonomous,
    /// Hybrid traditional + AI.
    Hybrid,
}

/// Configuration for zone arbitration system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArbitrationConfig {
    /// Arbitration mode.
    pub mode: ArbitrationMode,
    /// Institution identifier (e.g., "DIFC-LCIA", "ICC").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub institution_id: Option<String>,
    /// Rules version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules_version: Option<String>,
    /// AI model identifier for AI-assisted/autonomous modes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai_model: Option<String>,
    /// Claims above this threshold require human review (USD).
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub human_review_threshold_usd: u64,
    /// Whether appeals are permitted.
    #[serde(default = "default_true")]
    pub appeal_allowed: bool,
    /// Maximum claim amount (USD). Zero means unlimited.
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub max_claim_usd: u64,
}

fn is_zero_u64(v: &u64) -> bool {
    *v == 0
}

fn default_true() -> bool {
    true
}

impl Default for ArbitrationConfig {
    fn default() -> Self {
        Self {
            mode: ArbitrationMode::Traditional,
            institution_id: None,
            rules_version: None,
            ai_model: None,
            human_review_threshold_usd: 0,
            appeal_allowed: true,
            max_claim_usd: 0,
        }
    }
}

// -------------------------------------------------------------------------
// Corridor Config
// -------------------------------------------------------------------------

/// Configuration for a settlement corridor within a composed zone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorridorConfig {
    /// Unique corridor identifier.
    pub corridor_id: String,
    /// Source jurisdiction.
    pub source_jurisdiction: String,
    /// Target jurisdiction.
    pub target_jurisdiction: String,
    /// Settlement currency (ISO 4217).
    #[serde(default = "default_usd")]
    pub settlement_currency: String,
    /// Settlement mechanism (e.g., "rtgs", "swift", "blockchain").
    #[serde(default = "default_rtgs")]
    pub settlement_mechanism: String,
    /// Maximum settlement amount (USD). Zero means unlimited.
    #[serde(default)]
    pub max_settlement_usd: u64,
    /// Expected finality time in seconds.
    #[serde(default = "default_finality")]
    pub finality_seconds: u64,
}

fn default_usd() -> String {
    "USD".to_string()
}

fn default_rtgs() -> String {
    "rtgs".to_string()
}

fn default_finality() -> u64 {
    3600
}

// -------------------------------------------------------------------------
// Jurisdiction Layer
// -------------------------------------------------------------------------

/// A layer contributing specific compliance domains from a jurisdiction.
///
/// # Examples
///
/// ```
/// use msez_core::ComplianceDomain;
/// use msez_pack::composition::JurisdictionLayer;
///
/// // Delaware corporate law
/// let layer = JurisdictionLayer {
///     jurisdiction_id: "us-de".to_string(),
///     domains: vec![ComplianceDomain::Corporate],
///     description: Some("Delaware General Corporation Law".to_string()),
///     lawpacks: vec![],
///     regpacks: vec![],
///     licensepacks: vec![],
///     module_overrides: Default::default(),
/// };
/// assert!(layer.validate().is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JurisdictionLayer {
    /// Jurisdiction identifier (e.g., `us-ny`, `ae-abudhabi-adgm`).
    pub jurisdiction_id: String,
    /// Compliance domains this layer provides.
    pub domains: Vec<ComplianceDomain>,
    /// Human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Pinned lawpack references.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lawpacks: Vec<LawpackRef>,
    /// Pinned regpack references.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regpacks: Vec<RegpackRef>,
    /// Pinned licensepack references.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub licensepacks: Vec<LicensepackRef>,
    /// Module-level overrides (module path → override value).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub module_overrides: BTreeMap<String, String>,
}

impl JurisdictionLayer {
    /// Validate layer configuration, returning a list of error messages.
    ///
    /// Checks:
    /// - Jurisdiction ID format
    /// - At least one domain specified
    /// - All pack reference digests are valid SHA-256
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if !is_valid_jurisdiction_id(&self.jurisdiction_id) {
            errors.push(format!(
                "Invalid jurisdiction_id format: {}",
                self.jurisdiction_id
            ));
        }

        if self.domains.is_empty() {
            errors.push(format!("Layer {} has no domains", self.jurisdiction_id));
        }

        for lp in &self.lawpacks {
            if !parser::is_valid_sha256(&lp.lawpack_digest_sha256) {
                errors.push(format!(
                    "Invalid lawpack digest: {}",
                    lp.lawpack_digest_sha256
                ));
            }
        }

        for rp in &self.regpacks {
            if !parser::is_valid_sha256(&rp.regpack_digest_sha256) {
                errors.push(format!(
                    "Invalid regpack digest: {}",
                    rp.regpack_digest_sha256
                ));
            }
        }

        for lcp in &self.licensepacks {
            if !parser::is_valid_sha256(&lcp.licensepack_digest_sha256) {
                errors.push(format!(
                    "Invalid licensepack digest: {}",
                    lcp.licensepack_digest_sha256
                ));
            }
        }

        errors
    }

    /// Return the set of domains provided by this layer.
    pub fn domain_set(&self) -> HashSet<ComplianceDomain> {
        self.domains.iter().copied().collect()
    }
}

// -------------------------------------------------------------------------
// Zone Composition
// -------------------------------------------------------------------------

/// A composed zone built from multiple jurisdiction layers.
///
/// This is the central abstraction for multi-jurisdiction deployments.
/// Each layer contributes specific compliance domains from a particular
/// jurisdiction, and the composition engine validates that no domain
/// is claimed by more than one layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoneComposition {
    /// Unique zone identifier.
    pub zone_id: String,
    /// Human-readable name.
    pub name: String,
    /// Zone description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Jurisdiction layers composing this zone.
    #[serde(default)]
    pub layers: Vec<JurisdictionLayer>,
    /// Arbitration configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arbitration: Option<ArbitrationConfig>,
    /// Settlement corridors.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub corridors: Vec<CorridorConfig>,
    /// Zone profile (e.g., "digital-financial-center").
    #[serde(default = "default_profile")]
    pub profile: String,
}

fn default_profile() -> String {
    "digital-financial-center".to_string()
}

impl ZoneComposition {
    /// Validate the composition for conflicts and completeness.
    ///
    /// Returns a list of error messages. Empty list means valid.
    ///
    /// Checks:
    /// - Zone ID format
    /// - Individual layer validity
    /// - No domain conflicts (same domain from multiple layers)
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if !is_valid_zone_id(&self.zone_id) {
            errors.push(format!("Invalid zone_id format: {}", self.zone_id));
        }

        for layer in &self.layers {
            errors.extend(layer.validate());
        }

        // Check for domain conflicts
        let mut domain_sources: HashMap<ComplianceDomain, Vec<&str>> = HashMap::new();
        for layer in &self.layers {
            for domain in &layer.domains {
                domain_sources
                    .entry(*domain)
                    .or_default()
                    .push(&layer.jurisdiction_id);
            }
        }

        // Sort by domain name for deterministic error ordering
        let mut conflicts: Vec<_> = domain_sources
            .iter()
            .filter(|(_, sources)| sources.len() > 1)
            .collect();
        conflicts.sort_by_key(|(domain, _)| domain.as_str());

        for (domain, sources) in conflicts {
            errors.push(format!(
                "Domain conflict: {} provided by multiple layers: {}",
                domain,
                sources.join(", ")
            ));
        }

        errors
    }

    /// Return all domains covered by this composition.
    pub fn all_domains(&self) -> HashSet<ComplianceDomain> {
        let mut result = HashSet::new();
        for layer in &self.layers {
            result.extend(layer.domains.iter().copied());
        }
        result
    }

    /// Return mapping of domain → source jurisdiction.
    pub fn domain_coverage_report(&self) -> BTreeMap<String, String> {
        let mut report = BTreeMap::new();
        for layer in &self.layers {
            for domain in &layer.domains {
                report.insert(domain.as_str().to_string(), layer.jurisdiction_id.clone());
            }
        }
        report
    }

    /// Generate `zone.yaml` content from this composition.
    pub fn to_zone_yaml(&self) -> serde_json::Value {
        let mut zone = serde_json::json!({
            "zone_id": self.zone_id,
            "name": self.name,
            "spec_version": "0.4.44",
            "profile": self.profile,
            "composition": {
                "layers": self.layers.iter().map(|layer| {
                    serde_json::json!({
                        "jurisdiction_id": layer.jurisdiction_id,
                        "domains": layer.domains.iter().map(|d| d.as_str()).collect::<Vec<_>>(),
                        "description": layer.description.as_deref().unwrap_or(""),
                    })
                }).collect::<Vec<_>>(),
                "domain_mapping": self.domain_coverage_report(),
            },
        });

        if let Some(desc) = &self.description {
            zone["description"] = serde_json::Value::String(desc.clone());
        }

        // Aggregate lawpacks
        let lawpacks: Vec<serde_json::Value> = self
            .layers
            .iter()
            .flat_map(|l| &l.lawpacks)
            .map(|lp| serde_json::to_value(lp).expect("static struct — cannot fail"))
            .collect();
        if !lawpacks.is_empty() {
            zone["lawpacks"] = serde_json::Value::Array(lawpacks);
        }

        // Aggregate regpacks
        let regpacks: Vec<serde_json::Value> = self
            .layers
            .iter()
            .flat_map(|l| &l.regpacks)
            .map(|rp| serde_json::to_value(rp).expect("static struct — cannot fail"))
            .collect();
        if !regpacks.is_empty() {
            zone["regpacks"] = serde_json::Value::Array(regpacks);
        }

        // Aggregate licensepacks
        let licensepacks: Vec<serde_json::Value> = self
            .layers
            .iter()
            .flat_map(|l| &l.licensepacks)
            .map(|lcp| serde_json::to_value(lcp).expect("static struct — cannot fail"))
            .collect();
        if !licensepacks.is_empty() {
            zone["licensepacks"] = serde_json::Value::Array(licensepacks);
        }

        // Arbitration
        if let Some(arb) = &self.arbitration {
            zone["arbitration"] = serde_json::to_value(arb).expect("static struct — cannot fail");
        }

        // Corridors
        if !self.corridors.is_empty() {
            zone["corridors"] = serde_json::Value::Array(
                self.corridors
                    .iter()
                    .map(|c| serde_json::to_value(c).expect("static struct — cannot fail"))
                    .collect(),
            );
        }

        zone
    }

    /// Compute canonical digest of the composition.
    ///
    /// Deterministic: layers sorted by jurisdiction_id, domains sorted by name.
    pub fn composition_digest(&self) -> PackResult<String> {
        let mut sorted_layers: Vec<_> = self.layers.iter().collect();
        sorted_layers.sort_by_key(|l| &l.jurisdiction_id);

        let composition = serde_json::json!({
            "zone_id": self.zone_id,
            "layers": sorted_layers.iter().map(|l| {
                let mut domain_strs: Vec<&str> = l.domains.iter().map(|d| d.as_str()).collect();
                domain_strs.sort();
                serde_json::json!({
                    "jurisdiction_id": l.jurisdiction_id,
                    "domains": domain_strs,
                })
            }).collect::<Vec<_>>(),
        });

        let canonical = CanonicalBytes::from_value(composition)?;
        let digest = msez_core::sha256_digest(&canonical);
        Ok(digest.to_hex())
    }

    /// Generate `stack.lock` content from this composition.
    pub fn to_stack_lock(&self) -> PackResult<serde_json::Value> {
        let composition_digest = self.composition_digest()?;
        let generated_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

        let mut lock = serde_json::json!({
            "spec_version": "0.4.44",
            "zone_id": self.zone_id,
            "generated_at": generated_at,
            "composition_digest": composition_digest,
        });

        // Aggregate lawpacks
        let lawpacks: Vec<serde_json::Value> = self
            .layers
            .iter()
            .flat_map(|l| &l.lawpacks)
            .map(|lp| serde_json::to_value(lp).expect("static struct — cannot fail"))
            .collect();
        if !lawpacks.is_empty() {
            lock["lawpacks"] = serde_json::Value::Array(lawpacks);
        }

        // Aggregate regpacks
        let regpacks: Vec<serde_json::Value> = self
            .layers
            .iter()
            .flat_map(|l| &l.regpacks)
            .map(|rp| serde_json::to_value(rp).expect("static struct — cannot fail"))
            .collect();
        if !regpacks.is_empty() {
            lock["regpacks"] = serde_json::Value::Array(regpacks);
        }

        // Aggregate licensepacks
        let licensepacks: Vec<serde_json::Value> = self
            .layers
            .iter()
            .flat_map(|l| &l.licensepacks)
            .map(|lcp| serde_json::to_value(lcp).expect("static struct — cannot fail"))
            .collect();
        if !licensepacks.is_empty() {
            lock["licensepacks"] = serde_json::Value::Array(licensepacks);
        }

        Ok(lock)
    }
}

// -------------------------------------------------------------------------
// Convenience constructors
// -------------------------------------------------------------------------

/// Builder for constructing zone compositions with a fluent API.
///
/// # Example
///
/// ```
/// use msez_pack::composition::ZoneCompositionBuilder;
/// use msez_core::ComplianceDomain;
///
/// let zone = ZoneCompositionBuilder::new("momentum.demo.hybrid", "Hybrid Demo Zone")
///     .layer("us-ny", &[ComplianceDomain::Trade], "New York civic code")
///     .layer("us-de", &[ComplianceDomain::Corporate], "Delaware corporate law")
///     .layer(
///         "ae-abudhabi-adgm",
///         &[ComplianceDomain::DigitalAssets, ComplianceDomain::Securities,
///           ComplianceDomain::Clearing, ComplianceDomain::Custody],
///         "ADGM financial services",
///     )
///     .ai_arbitration()
///     .build()
///     .expect("valid composition");
///
/// assert_eq!(zone.layers.len(), 3);
/// ```
pub struct ZoneCompositionBuilder {
    zone_id: String,
    name: String,
    description: Option<String>,
    layers: Vec<JurisdictionLayer>,
    arbitration: Option<ArbitrationConfig>,
    corridors: Vec<CorridorConfig>,
    profile: String,
}

impl ZoneCompositionBuilder {
    /// Create a new builder with the given zone ID and name.
    pub fn new(zone_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            zone_id: zone_id.into(),
            name: name.into(),
            description: None,
            layers: Vec::new(),
            arbitration: None,
            corridors: Vec::new(),
            profile: "digital-financial-center".to_string(),
        }
    }

    /// Set the zone description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set the zone profile.
    pub fn profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = profile.into();
        self
    }

    /// Add a jurisdiction layer with the given domains.
    pub fn layer(
        mut self,
        jurisdiction_id: impl Into<String>,
        domains: &[ComplianceDomain],
        description: impl Into<String>,
    ) -> Self {
        self.layers.push(JurisdictionLayer {
            jurisdiction_id: jurisdiction_id.into(),
            domains: domains.to_vec(),
            description: Some(description.into()),
            lawpacks: Vec::new(),
            regpacks: Vec::new(),
            licensepacks: Vec::new(),
            module_overrides: BTreeMap::new(),
        });
        self
    }

    /// Add a full jurisdiction layer.
    pub fn add_layer(mut self, layer: JurisdictionLayer) -> Self {
        self.layers.push(layer);
        self
    }

    /// Enable AI-assisted arbitration.
    pub fn ai_arbitration(mut self) -> Self {
        self.arbitration = Some(ArbitrationConfig {
            mode: ArbitrationMode::AiAssisted,
            human_review_threshold_usd: 100_000,
            appeal_allowed: true,
            ..Default::default()
        });
        self
    }

    /// Set arbitration configuration.
    pub fn arbitration(mut self, config: ArbitrationConfig) -> Self {
        self.arbitration = Some(config);
        self
    }

    /// Add a settlement corridor.
    pub fn corridor(mut self, config: CorridorConfig) -> Self {
        self.corridors.push(config);
        self
    }

    /// Build and validate the zone composition.
    ///
    /// # Errors
    ///
    /// Returns [`PackError::CompositionInvalid`] if validation fails.
    pub fn build(self) -> PackResult<ZoneComposition> {
        let composition = ZoneComposition {
            zone_id: self.zone_id,
            name: self.name,
            description: self.description,
            layers: self.layers,
            arbitration: self.arbitration,
            corridors: self.corridors,
            profile: self.profile,
        };

        let errors = composition.validate();
        if !errors.is_empty() {
            return Err(PackError::CompositionInvalid {
                errors: errors.clone(),
            });
        }

        Ok(composition)
    }
}

// -------------------------------------------------------------------------
// YAML Loading
// -------------------------------------------------------------------------

/// Load a zone composition from a YAML file.
///
/// # Errors
///
/// Returns [`PackError`] if the file cannot be read or parsed.
pub fn load_composition_from_yaml(path: &Path) -> PackResult<ZoneComposition> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            PackError::FileNotFound {
                path: path.to_path_buf(),
            }
        } else {
            PackError::Io(e)
        }
    })?;

    let data: serde_json::Value =
        serde_yaml::from_str(&content).map_err(|e| PackError::YamlParseStr {
            path: path.to_path_buf(),
            detail: e.to_string(),
        })?;

    load_composition_from_value(&data)
}

/// Load a zone composition from a `serde_json::Value`.
///
/// Handles both direct deserialization and manual field extraction
/// for compatibility with varying YAML structures.
pub fn load_composition_from_value(data: &serde_json::Value) -> PackResult<ZoneComposition> {
    let obj = data.as_object().ok_or_else(|| PackError::SchemaViolation {
        message: "composition must be a YAML/JSON object".to_string(),
    })?;

    let zone_id = obj
        .get("zone_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let description = obj
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let profile = obj
        .get("profile")
        .and_then(|v| v.as_str())
        .unwrap_or("digital-financial-center")
        .to_string();

    // Parse layers
    let layers = match obj.get("layers") {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .map(parse_layer)
            .collect::<PackResult<Vec<_>>>()?,
        _ => Vec::new(),
    };

    // Parse arbitration
    let arbitration = match obj.get("arbitration") {
        Some(v) if !v.is_null() => Some(
            serde_json::from_value::<ArbitrationConfig>(v.clone()).map_err(|e| {
                PackError::SchemaViolation {
                    message: format!("invalid arbitration config: {e}"),
                }
            })?,
        ),
        _ => None,
    };

    // Parse corridors
    let corridors = match obj.get("corridors") {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .map(|v| {
                serde_json::from_value::<CorridorConfig>(v.clone()).map_err(|e| {
                    PackError::SchemaViolation {
                        message: format!("invalid corridor config: {e}"),
                    }
                })
            })
            .collect::<PackResult<Vec<_>>>()?,
        _ => Vec::new(),
    };

    Ok(ZoneComposition {
        zone_id,
        name,
        description,
        layers,
        arbitration,
        corridors,
        profile,
    })
}

/// Parse a single jurisdiction layer from a JSON value.
fn parse_layer(value: &serde_json::Value) -> PackResult<JurisdictionLayer> {
    let obj = value
        .as_object()
        .ok_or_else(|| PackError::SchemaViolation {
            message: "layer must be a JSON object".to_string(),
        })?;

    let jurisdiction_id = obj
        .get("jurisdiction_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let domains: Vec<ComplianceDomain> = match obj.get("domains") {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| {
                s.parse::<ComplianceDomain>()
                    .map_err(|e| PackError::SchemaViolation {
                        message: format!("invalid domain '{s}': {e}"),
                    })
            })
            .collect::<PackResult<Vec<_>>>()?,
        _ => Vec::new(),
    };

    let description = obj
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let lawpacks = match obj.get("lawpacks") {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .map(|v| {
                serde_json::from_value::<LawpackRef>(v.clone()).map_err(|e| {
                    PackError::SchemaViolation {
                        message: format!("invalid lawpack ref: {e}"),
                    }
                })
            })
            .collect::<PackResult<Vec<_>>>()?,
        _ => Vec::new(),
    };

    let regpacks = match obj.get("regpacks") {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .map(|v| {
                serde_json::from_value::<RegpackRef>(v.clone()).map_err(|e| {
                    PackError::SchemaViolation {
                        message: format!("invalid regpack ref: {e}"),
                    }
                })
            })
            .collect::<PackResult<Vec<_>>>()?,
        _ => Vec::new(),
    };

    let licensepacks = match obj.get("licensepacks") {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .map(|v| {
                serde_json::from_value::<LicensepackRef>(v.clone()).map_err(|e| {
                    PackError::SchemaViolation {
                        message: format!("invalid licensepack ref: {e}"),
                    }
                })
            })
            .collect::<PackResult<Vec<_>>>()?,
        _ => Vec::new(),
    };

    let module_overrides: BTreeMap<String, String> = match obj.get("module_overrides") {
        Some(serde_json::Value::Object(map)) => map
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect(),
        _ => BTreeMap::new(),
    };

    Ok(JurisdictionLayer {
        jurisdiction_id,
        domains,
        description,
        lawpacks,
        regpacks,
        licensepacks,
        module_overrides,
    })
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_jurisdiction_ids() {
        assert!(is_valid_jurisdiction_id("us"));
        assert!(is_valid_jurisdiction_id("us-ny"));
        assert!(is_valid_jurisdiction_id("ae-abudhabi-adgm"));
        assert!(is_valid_jurisdiction_id("pk-rsez"));
        assert!(is_valid_jurisdiction_id("sg"));
    }

    #[test]
    fn invalid_jurisdiction_ids() {
        assert!(!is_valid_jurisdiction_id(""));
        assert!(!is_valid_jurisdiction_id("U")); // too short
        assert!(!is_valid_jurisdiction_id("US")); // uppercase
        assert!(!is_valid_jurisdiction_id("us-")); // trailing hyphen
        assert!(!is_valid_jurisdiction_id("1s")); // starts with digit
        assert!(!is_valid_jurisdiction_id("u")); // too short
    }

    #[test]
    fn valid_zone_ids() {
        assert!(is_valid_zone_id("momentum.demo.hybrid"));
        assert!(is_valid_zone_id("pk-rsez"));
        assert!(is_valid_zone_id("test"));
    }

    #[test]
    fn invalid_zone_ids() {
        assert!(!is_valid_zone_id(""));
        assert!(!is_valid_zone_id("1test")); // starts with digit
        assert!(!is_valid_zone_id("TEST")); // uppercase
    }

    #[test]
    fn single_layer_composition_validates() {
        let zone = ZoneCompositionBuilder::new("test.zone", "Test Zone")
            .layer(
                "us-de",
                &[ComplianceDomain::Corporate],
                "Delaware corporate",
            )
            .build()
            .unwrap();

        assert_eq!(zone.zone_id, "test.zone");
        assert_eq!(zone.layers.len(), 1);
        assert!(zone.validate().is_empty());
    }

    #[test]
    fn multi_layer_composition_validates() {
        let zone = ZoneCompositionBuilder::new("momentum.demo.hybrid", "Hybrid Demo")
            .layer(
                "us-de",
                &[ComplianceDomain::Corporate],
                "Delaware corporate",
            )
            .layer(
                "ae-abudhabi-adgm",
                &[
                    ComplianceDomain::DigitalAssets,
                    ComplianceDomain::Securities,
                    ComplianceDomain::Clearing,
                ],
                "ADGM financial services",
            )
            .build()
            .unwrap();

        assert_eq!(zone.layers.len(), 2);
        assert_eq!(zone.all_domains().len(), 4);
        assert!(zone.validate().is_empty());
    }

    #[test]
    fn domain_conflict_detected() {
        let result = ZoneCompositionBuilder::new("conflict.zone", "Conflict Zone")
            .layer(
                "us-de",
                &[ComplianceDomain::Corporate],
                "Delaware corporate",
            )
            .layer(
                "us-ny",
                &[ComplianceDomain::Corporate], // conflict!
                "NY corporate",
            )
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("corporate"),
            "Error should mention domain: {msg}"
        );
    }

    #[test]
    fn domain_coverage_report_correct() {
        let zone = ZoneCompositionBuilder::new("test.zone", "Test")
            .layer("us-de", &[ComplianceDomain::Corporate], "Delaware")
            .layer("ae-abudhabi-adgm", &[ComplianceDomain::Securities], "ADGM")
            .build()
            .unwrap();

        let report = zone.domain_coverage_report();
        assert_eq!(report.get("corporate"), Some(&"us-de".to_string()));
        assert_eq!(
            report.get("securities"),
            Some(&"ae-abudhabi-adgm".to_string())
        );
    }

    #[test]
    fn composition_digest_deterministic() {
        let zone = ZoneCompositionBuilder::new("test.zone", "Test")
            .layer("us-de", &[ComplianceDomain::Corporate], "Delaware")
            .layer("ae-abudhabi-adgm", &[ComplianceDomain::Securities], "ADGM")
            .build()
            .unwrap();

        let d1 = zone.composition_digest().unwrap();
        let d2 = zone.composition_digest().unwrap();
        assert_eq!(d1, d2);
        assert_eq!(d1.len(), 64); // SHA-256 hex
    }

    #[test]
    fn composition_digest_order_independent() {
        // Layers in different order should produce same digest
        let zone1 = ZoneCompositionBuilder::new("test.zone", "Test")
            .layer("us-de", &[ComplianceDomain::Corporate], "Delaware")
            .layer("ae-abudhabi-adgm", &[ComplianceDomain::Securities], "ADGM")
            .build()
            .unwrap();

        let zone2 = ZoneCompositionBuilder::new("test.zone", "Test")
            .layer("ae-abudhabi-adgm", &[ComplianceDomain::Securities], "ADGM")
            .layer("us-de", &[ComplianceDomain::Corporate], "Delaware")
            .build()
            .unwrap();

        assert_eq!(
            zone1.composition_digest().unwrap(),
            zone2.composition_digest().unwrap()
        );
    }

    #[test]
    fn to_zone_yaml_structure() {
        let zone = ZoneCompositionBuilder::new("test.zone", "Test Zone")
            .description("A test zone")
            .layer("us-de", &[ComplianceDomain::Corporate], "Delaware")
            .build()
            .unwrap();

        let yaml = zone.to_zone_yaml();
        assert_eq!(yaml["zone_id"], "test.zone");
        assert_eq!(yaml["name"], "Test Zone");
        assert_eq!(yaml["spec_version"], "0.4.44");
        assert!(yaml["composition"]["layers"].is_array());
        assert!(yaml["composition"]["domain_mapping"].is_object());
    }

    #[test]
    fn to_stack_lock_structure() {
        let zone = ZoneCompositionBuilder::new("test.zone", "Test Zone")
            .layer("us-de", &[ComplianceDomain::Corporate], "Delaware")
            .build()
            .unwrap();

        let lock = zone.to_stack_lock().unwrap();
        assert_eq!(lock["zone_id"], "test.zone");
        assert_eq!(lock["spec_version"], "0.4.44");
        assert!(lock["generated_at"].is_string());
        assert!(lock["composition_digest"].is_string());
    }

    #[test]
    fn ai_arbitration_config() {
        let zone = ZoneCompositionBuilder::new("test.zone", "Test")
            .layer("us-de", &[ComplianceDomain::Corporate], "Delaware")
            .ai_arbitration()
            .build()
            .unwrap();

        let arb = zone.arbitration.as_ref().unwrap();
        assert_eq!(arb.mode, ArbitrationMode::AiAssisted);
        assert_eq!(arb.human_review_threshold_usd, 100_000);
        assert!(arb.appeal_allowed);
    }

    #[test]
    fn corridor_config() {
        let zone = ZoneCompositionBuilder::new("test.zone", "Test")
            .layer("pk", &[ComplianceDomain::Banking], "Pakistan")
            .corridor(CorridorConfig {
                corridor_id: "pk-ae-001".to_string(),
                source_jurisdiction: "pk".to_string(),
                target_jurisdiction: "ae".to_string(),
                settlement_currency: "USD".to_string(),
                settlement_mechanism: "swift".to_string(),
                max_settlement_usd: 1_000_000,
                finality_seconds: 7200,
            })
            .build()
            .unwrap();

        assert_eq!(zone.corridors.len(), 1);
        assert_eq!(zone.corridors[0].corridor_id, "pk-ae-001");
    }

    #[test]
    fn layer_validates_pack_digests() {
        let layer = JurisdictionLayer {
            jurisdiction_id: "us-de".to_string(),
            domains: vec![ComplianceDomain::Corporate],
            description: None,
            lawpacks: vec![LawpackRef {
                jurisdiction_id: "us-de".to_string(),
                domain: "corporate".to_string(),
                lawpack_digest_sha256: "not-a-valid-digest".to_string(),
            }],
            regpacks: Vec::new(),
            licensepacks: Vec::new(),
            module_overrides: BTreeMap::new(),
        };

        let errors = layer.validate();
        assert!(!errors.is_empty());
        assert!(errors[0].contains("Invalid lawpack digest"));
    }

    #[test]
    fn load_composition_from_value_roundtrip() {
        let zone = ZoneCompositionBuilder::new("test.zone", "Test Zone")
            .layer("us-de", &[ComplianceDomain::Corporate], "Delaware")
            .layer("ae-abudhabi-adgm", &[ComplianceDomain::Securities], "ADGM")
            .build()
            .unwrap();

        let value = serde_json::to_value(&zone).unwrap();
        let loaded = load_composition_from_value(&value).unwrap();

        assert_eq!(loaded.zone_id, zone.zone_id);
        assert_eq!(loaded.layers.len(), zone.layers.len());
        assert_eq!(loaded.layers[0].jurisdiction_id, "us-de");
    }

    #[test]
    fn serde_roundtrip() {
        let zone = ZoneCompositionBuilder::new("test.zone", "Test Zone")
            .description("A test")
            .layer("us-de", &[ComplianceDomain::Corporate], "Delaware")
            .ai_arbitration()
            .corridor(CorridorConfig {
                corridor_id: "test-corridor".to_string(),
                source_jurisdiction: "us".to_string(),
                target_jurisdiction: "ae".to_string(),
                settlement_currency: "USD".to_string(),
                settlement_mechanism: "rtgs".to_string(),
                max_settlement_usd: 0,
                finality_seconds: 3600,
            })
            .build()
            .unwrap();

        let json = serde_json::to_string_pretty(&zone).unwrap();
        let deser: ZoneComposition = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.zone_id, zone.zone_id);
        assert_eq!(deser.arbitration.unwrap().mode, ArbitrationMode::AiAssisted);
        assert_eq!(deser.corridors.len(), 1);
    }

    #[test]
    fn empty_layer_domains_rejected() {
        let result = ZoneCompositionBuilder::new("test.zone", "Test")
            .add_layer(JurisdictionLayer {
                jurisdiction_id: "us-de".to_string(),
                domains: vec![],
                description: None,
                lawpacks: vec![],
                regpacks: vec![],
                licensepacks: vec![],
                module_overrides: BTreeMap::new(),
            })
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn arbitration_mode_serde() {
        let modes = [
            (ArbitrationMode::Traditional, "\"traditional\""),
            (ArbitrationMode::AiAssisted, "\"ai-assisted\""),
            (ArbitrationMode::AiAutonomous, "\"ai-autonomous\""),
            (ArbitrationMode::Hybrid, "\"hybrid\""),
        ];

        for (mode, expected) in &modes {
            let json = serde_json::to_string(mode).unwrap();
            assert_eq!(&json, expected);
            let deser: ArbitrationMode = serde_json::from_str(&json).unwrap();
            assert_eq!(&deser, mode);
        }
    }
}
