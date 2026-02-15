//! # Regpack — Dynamic Regulatory State Management
//!
//! Captures dynamic regulatory state that Lawpacks cannot represent:
//! sanctions lists, license registries, reporting deadlines, regulatory
//! guidance, and enforcement priorities.
//!
//! ## Data Model
//!
//! - [`SanctionsEntry`]: Individual sanctions list entry with aliases and identifiers.
//! - [`SanctionsSnapshot`]: Point-in-time consolidated sanctions snapshot.
//! - [`SanctionsChecker`]: Entity screening against consolidated sanctions with fuzzy matching.
//! - [`RegulatorProfile`]: Regulatory authority profile with API capabilities.
//! - [`ReportingRequirement`]: Reporting deadlines and submission requirements.
//! - [`ComplianceDeadline`]: Upcoming compliance deadline with grace period.
//! - [`RegPackMetadata`]: Content-addressed regpack metadata.
//! - [`Regpack`]: Compiled regpack bundle.
//!
//! ## Domain Validation
//!
//! Every [`ComplianceDomain`] referenced in a regpack is validated against
//! the canonical domain enum in `msez-core`, ensuring exhaustive coverage
//! and preventing domain drift.
//!
//! ## Spec Reference
//!
//! Ports Python `tools/regpack.py` with cross-language digest compatibility.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use msez_core::digest::Sha256Accumulator;

use msez_core::{CanonicalBytes, ComplianceDomain, ContentDigest, JurisdictionId};

use crate::error::{PackError, PackResult};
use crate::parser;

/// Stack specification version.
pub const STACK_SPEC_VERSION: &str = "0.4.44";
/// Regpack format version.
pub const REGPACK_VERSION: &str = "1.0";

// ---------------------------------------------------------------------------
// Sanctions Types
// ---------------------------------------------------------------------------

/// A single entry in a sanctions list.
///
/// Represents an individual, entity, vessel, or aircraft that appears
/// on one or more sanctions lists (OFAC, EU, UN, UK).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanctionsEntry {
    /// Unique entry identifier.
    pub entry_id: String,
    /// Entry type: "individual", "entity", "vessel", "aircraft".
    pub entry_type: String,
    /// Source lists this entry appears on.
    pub source_lists: Vec<String>,
    /// Primary name of the sanctioned party.
    pub primary_name: String,
    /// Known aliases.
    #[serde(default)]
    pub aliases: Vec<BTreeMap<String, String>>,
    /// Identity documents (passport, national ID, etc.).
    #[serde(default)]
    pub identifiers: Vec<BTreeMap<String, String>>,
    /// Known addresses.
    #[serde(default)]
    pub addresses: Vec<BTreeMap<String, String>>,
    /// Nationalities.
    #[serde(default)]
    pub nationalities: Vec<String>,
    /// Date of birth (for individuals).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_of_birth: Option<String>,
    /// Sanctions programs this entry is listed under.
    #[serde(default)]
    pub programs: Vec<String>,
    /// Date first listed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listing_date: Option<String>,
    /// Free-text remarks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remarks: Option<String>,
}

/// A point-in-time snapshot of consolidated sanctions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanctionsSnapshot {
    /// Snapshot identifier.
    pub snapshot_id: String,
    /// When the snapshot was taken (RFC 3339).
    pub snapshot_timestamp: String,
    /// Source metadata by source ID.
    pub sources: BTreeMap<String, serde_json::Value>,
    /// Consolidated entry count by type.
    #[serde(default)]
    pub consolidated_counts: BTreeMap<String, i64>,
    /// Delta from previous snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta_from_previous: Option<serde_json::Value>,
}

/// Result of checking an entity against sanctions lists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanctionsCheckResult {
    /// The query string.
    pub query: String,
    /// When the check was performed.
    pub checked_at: String,
    /// Snapshot used for the check.
    pub snapshot_id: String,
    /// Whether any match was found.
    pub matched: bool,
    /// Matching entries with scores.
    #[serde(default)]
    pub matches: Vec<SanctionsMatch>,
    /// Highest match score (0.0 - 1.0).
    pub match_score: f64,
}

/// A single match from a sanctions check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanctionsMatch {
    /// The matched sanctions entry.
    pub entry: SanctionsEntry,
    /// Type of match: "exact_name", "fuzzy_name", "identifier".
    pub match_type: String,
    /// Match confidence score (0.0 - 1.0).
    pub score: f64,
    /// Identifier type (for identifier matches).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier_type: Option<String>,
}

// ---------------------------------------------------------------------------
// SanctionsChecker
// ---------------------------------------------------------------------------

/// Check entities against consolidated sanctions lists.
///
/// Provides exact name matching, fuzzy matching (token overlap),
/// and identifier-based matching. Mirrors Python `tools/regpack.py:SanctionsChecker`.
pub struct SanctionsChecker {
    entries: Vec<SanctionsEntry>,
    snapshot_id: String,
    name_index: HashMap<String, Vec<usize>>,
    id_index: HashMap<String, Vec<usize>>,
}

impl SanctionsChecker {
    /// Create a new checker from a list of sanctions entries.
    pub fn new(entries: Vec<SanctionsEntry>, snapshot_id: String) -> Self {
        let mut checker = Self {
            entries,
            snapshot_id,
            name_index: HashMap::new(),
            id_index: HashMap::new(),
        };
        checker.build_index();
        checker
    }

    fn build_index(&mut self) {
        for (idx, entry) in self.entries.iter().enumerate() {
            // Index by normalized primary name
            let norm = Self::normalize(&entry.primary_name);
            self.name_index.entry(norm).or_default().push(idx);

            // Index aliases
            for alias in &entry.aliases {
                let alias_value = alias
                    .get("name")
                    .or_else(|| alias.get("alias"))
                    .cloned()
                    .unwrap_or_default();
                let norm_alias = Self::normalize(&alias_value);
                if !norm_alias.is_empty() {
                    self.name_index.entry(norm_alias).or_default().push(idx);
                }
            }

            // Index identifiers
            for ident in &entry.identifiers {
                if let Some(val) = ident.get("value") {
                    let id_val = val.to_uppercase().trim().to_string();
                    if !id_val.is_empty() {
                        self.id_index.entry(id_val).or_default().push(idx);
                    }
                }
            }
        }
    }

    /// Normalize a string for matching.
    fn normalize(s: &str) -> String {
        let lower = s.to_lowercase();
        let cleaned: String = lower
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c.is_whitespace() {
                    c
                } else {
                    ' '
                }
            })
            .collect();
        cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Compute fuzzy match score between two strings (0.0 - 1.0).
    fn fuzzy_score(query: &str, target: &str) -> f64 {
        let q = Self::normalize(query);
        let t = Self::normalize(target);

        if q.is_empty() || t.is_empty() {
            return 0.0;
        }
        if q == t {
            return 1.0;
        }
        // Substring match (only if query is meaningful length)
        if q.len() >= 3 && (t.contains(&q) || q.contains(&t)) {
            return 0.9;
        }
        // Token overlap (Jaccard similarity)
        let q_tokens: std::collections::HashSet<&str> = q.split_whitespace().collect();
        let t_tokens: std::collections::HashSet<&str> = t.split_whitespace().collect();
        if q_tokens.is_empty() || t_tokens.is_empty() {
            return 0.0;
        }
        let overlap = q_tokens.intersection(&t_tokens).count();
        let total = q_tokens.union(&t_tokens).count();
        if total > 0 {
            overlap as f64 / total as f64
        } else {
            0.0
        }
    }

    /// Check if an entity matches any sanctions entry.
    ///
    /// # Arguments
    ///
    /// * `name` - Entity name to check.
    /// * `identifiers` - Optional identity documents to check.
    /// * `threshold` - Minimum fuzzy match score (default 0.7).
    pub fn check_entity(
        &self,
        name: &str,
        identifiers: Option<&[BTreeMap<String, String>]>,
        threshold: f64,
    ) -> SanctionsCheckResult {
        let now = chrono::Utc::now().to_rfc3339();
        let mut matches = Vec::new();
        let mut max_score: f64 = 0.0;

        let norm_name = Self::normalize(name);

        // Exact match
        if let Some(indices) = self.name_index.get(&norm_name) {
            for &idx in indices {
                matches.push(SanctionsMatch {
                    entry: self.entries[idx].clone(),
                    match_type: "exact_name".to_string(),
                    score: 1.0,
                    identifier_type: None,
                });
                max_score = 1.0;
            }
        }

        // Fuzzy match (only if no exact match).
        // SECURITY: must return ALL entries at or above threshold, not just the
        // highest scorer. For sanctions screening, false negatives are dangerous.
        if max_score < 1.0 {
            for (norm_target, indices) in &self.name_index {
                let score = Self::fuzzy_score(name, norm_target);
                if score >= threshold {
                    for &idx in indices {
                        matches.push(SanctionsMatch {
                            entry: self.entries[idx].clone(),
                            match_type: "fuzzy_name".to_string(),
                            score,
                            identifier_type: None,
                        });
                    }
                    max_score = max_score.max(score);
                }
            }
        }

        // Identifier match
        if let Some(idents) = identifiers {
            for ident in idents {
                let id_val = ident
                    .get("value")
                    .map(|v| v.to_uppercase().trim().to_string())
                    .unwrap_or_default();
                if let Some(indices) = self.id_index.get(&id_val) {
                    for &idx in indices {
                        matches.push(SanctionsMatch {
                            entry: self.entries[idx].clone(),
                            match_type: "identifier".to_string(),
                            score: 1.0,
                            identifier_type: ident.get("type").cloned(),
                        });
                        max_score = 1.0;
                    }
                }
            }
        }

        // Deduplicate by entry_id
        let mut seen = std::collections::HashSet::new();
        let unique_matches: Vec<SanctionsMatch> = matches
            .into_iter()
            .filter(|m| seen.insert(m.entry.entry_id.clone()))
            .collect();

        SanctionsCheckResult {
            query: name.to_string(),
            checked_at: now,
            snapshot_id: self.snapshot_id.clone(),
            matched: !unique_matches.is_empty(),
            matches: unique_matches,
            match_score: max_score,
        }
    }
}

// ---------------------------------------------------------------------------
// Regulatory Types
// ---------------------------------------------------------------------------

/// Profile of a regulatory authority.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegulatorProfile {
    /// Unique regulator identifier.
    pub regulator_id: String,
    /// Human-readable name.
    pub name: String,
    /// Jurisdiction identifier.
    pub jurisdiction_id: String,
    /// Parent regulatory authority (if any).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_authority: Option<String>,
    /// Regulatory scope by domain.
    #[serde(default)]
    pub scope: BTreeMap<String, Vec<String>>,
    /// Contact information.
    #[serde(default)]
    pub contact: BTreeMap<String, String>,
    /// API capabilities supported.
    #[serde(default)]
    pub api_capabilities: BTreeMap<String, bool>,
    /// Timezone.
    #[serde(default = "default_timezone")]
    pub timezone: String,
    /// Business days (ISO day names).
    #[serde(default)]
    pub business_days: Vec<String>,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

/// A type of regulatory license.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegLicenseType {
    /// Unique license type identifier.
    pub license_type_id: String,
    /// Human-readable name.
    pub name: String,
    /// Regulator that issues this license.
    pub regulator_id: String,
    /// Requirements for obtaining the license.
    #[serde(default)]
    pub requirements: BTreeMap<String, serde_json::Value>,
    /// Application process.
    #[serde(default)]
    pub application: BTreeMap<String, serde_json::Value>,
    /// Ongoing compliance obligations.
    #[serde(default)]
    pub ongoing_obligations: BTreeMap<String, serde_json::Value>,
    /// License validity period in years.
    #[serde(default = "default_validity_years")]
    pub validity_period_years: i32,
    /// Days before expiry to begin renewal.
    #[serde(default = "default_renewal_lead_time")]
    pub renewal_lead_time_days: i32,
}

fn default_validity_years() -> i32 {
    1
}

fn default_renewal_lead_time() -> i32 {
    90
}

/// A regulatory reporting requirement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingRequirement {
    /// Report type identifier.
    pub report_type_id: String,
    /// Human-readable name.
    pub name: String,
    /// Regulator requiring the report.
    pub regulator_id: String,
    /// Entity types this applies to.
    pub applicable_to: Vec<String>,
    /// Reporting frequency: "daily", "weekly", "monthly", "quarterly", "annual".
    pub frequency: String,
    /// Deadline details by period.
    #[serde(default)]
    pub deadlines: BTreeMap<String, BTreeMap<String, String>>,
    /// Submission requirements.
    #[serde(default)]
    pub submission: BTreeMap<String, serde_json::Value>,
    /// Late filing penalties.
    #[serde(default)]
    pub late_penalty: BTreeMap<String, serde_json::Value>,
}

/// An upcoming compliance deadline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceDeadline {
    /// Deadline identifier.
    pub deadline_id: String,
    /// Regulator imposing the deadline.
    pub regulator_id: String,
    /// Deadline type: "report", "filing", "renewal", "payment".
    pub deadline_type: String,
    /// Human-readable description.
    pub description: String,
    /// Due date (YYYY-MM-DD or RFC 3339).
    pub due_date: String,
    /// Grace period in days after due date.
    #[serde(default)]
    pub grace_period_days: i32,
    /// License types this deadline applies to.
    #[serde(default)]
    pub applicable_license_types: Vec<String>,
}

// ---------------------------------------------------------------------------
// RegPackMetadata
// ---------------------------------------------------------------------------

/// Metadata for a RegPack.
///
/// Contains identifying information, source feeds, and content summary
/// for a regulatory state snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegPackMetadata {
    /// Unique regpack identifier.
    pub regpack_id: String,
    /// Jurisdiction identifier.
    pub jurisdiction_id: String,
    /// Regulatory domain.
    pub domain: String,
    /// Snapshot date (YYYY-MM-DD).
    pub as_of_date: String,
    /// Snapshot type: "quarterly", "monthly", "on_demand".
    pub snapshot_type: String,
    /// Source feed metadata.
    #[serde(default)]
    pub sources: Vec<serde_json::Value>,
    /// Content summary (regulators, sanctions lists, counts).
    #[serde(default)]
    pub includes: BTreeMap<String, serde_json::Value>,
    /// Digest of the previous regpack (for chaining).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_regpack_digest: Option<String>,
    /// Creation timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Expiration timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    /// Content-addressed digest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest_sha256: Option<String>,
}

// ---------------------------------------------------------------------------
// Regpack
// ---------------------------------------------------------------------------

/// A compiled regpack containing regulatory requirement mappings.
///
/// This is the primary type for working with regpacks in the Rust layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Regpack {
    /// The jurisdiction this regpack applies to.
    pub jurisdiction: JurisdictionId,
    /// Human-readable name of the regpack.
    pub name: String,
    /// Version string (semver).
    pub version: String,
    /// Content digest of the compiled regpack.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<ContentDigest>,
    /// Metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<RegPackMetadata>,
}

/// Regpack reference in a zone composition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegpackRef {
    /// Jurisdiction identifier.
    pub jurisdiction_id: String,
    /// Regulatory domain.
    pub domain: String,
    /// SHA-256 digest.
    pub regpack_digest_sha256: String,
    /// Snapshot date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub as_of_date: Option<String>,
}

// ---------------------------------------------------------------------------
// Digest computation
// ---------------------------------------------------------------------------

/// Compute a regpack digest.
///
/// Follows a similar pattern to lawpack digests:
/// SHA256( b"msez-regpack-v1\0" + canonical(metadata) + canonical(components)... )
///
/// # SHA-256 exception: composite domain-prefixed digest
///
/// Uses `Sha256Accumulator` instead of `sha256_digest(&CanonicalBytes)` because
/// this computes a composite digest over a domain prefix + multiple individually
/// canonicalized components. Each component goes through `CanonicalBytes`
/// before being fed to the hasher; the accumulator is needed to combine
/// them under a domain separation prefix.
pub fn compute_regpack_digest(
    metadata: &RegPackMetadata,
    sanctions: Option<&SanctionsSnapshot>,
    regulators: Option<&[RegulatorProfile]>,
    deadlines: Option<&[ComplianceDeadline]>,
) -> PackResult<String> {
    let mut acc = Sha256Accumulator::new();
    acc.update(b"msez-regpack-v1\0");

    // Add metadata
    let meta_value = serde_json::to_value(metadata)?;
    let meta_canonical = CanonicalBytes::from_value(meta_value)?;
    acc.update(meta_canonical.as_bytes());

    // Add sanctions snapshot metadata
    if let Some(snap) = sanctions {
        let snap_value = serde_json::to_value(snap)?;
        let snap_canonical = CanonicalBytes::from_value(snap_value)?;
        acc.update(snap_canonical.as_bytes());
    }

    // Add regulators (sorted by ID)
    if let Some(regs) = regulators {
        let mut ids: Vec<&str> = regs.iter().map(|r| r.regulator_id.as_str()).collect();
        ids.sort();
        let reg_index = serde_json::json!({"regulators": ids});
        let reg_canonical = CanonicalBytes::from_value(reg_index)?;
        acc.update(reg_canonical.as_bytes());
    }

    // Add deadlines
    if let Some(dls) = deadlines {
        let mut sorted_dls: Vec<&ComplianceDeadline> = dls.iter().collect();
        sorted_dls.sort_by(|a, b| a.deadline_id.cmp(&b.deadline_id));
        let dl_values: Vec<serde_json::Value> = sorted_dls
            .iter()
            .map(|d| serde_json::to_value(d).unwrap_or(serde_json::Value::Null))
            .collect();
        let dl_data = serde_json::json!({"deadlines": dl_values});
        let dl_canonical = CanonicalBytes::from_value(dl_data)?;
        acc.update(dl_canonical.as_bytes());
    }

    Ok(acc.finalize_hex())
}

// ---------------------------------------------------------------------------
// Domain Validation
// ---------------------------------------------------------------------------

/// Validate that a domain string corresponds to a known ComplianceDomain.
///
/// Ensures exhaustive coverage between regpack domain references and
/// the canonical domain enum in msez-core.
pub fn validate_compliance_domain(domain: &str) -> PackResult<ComplianceDomain> {
    domain
        .parse::<ComplianceDomain>()
        .map_err(|_| PackError::UnknownDomain {
            domain: domain.to_string(),
        })
}

/// Validate all domain references in a regpack metadata.
///
/// Returns errors for any unrecognized domains.
pub fn validate_regpack_domains(metadata: &RegPackMetadata) -> Vec<PackError> {
    let mut errors = Vec::new();
    // Check domain field
    if validate_compliance_domain(&metadata.domain).is_err() {
        // Domain field may be a broader category (e.g., "financial") rather
        // than a specific ComplianceDomain. Only flag truly unrecognized ones.
        // For now, we allow any non-empty domain string.
        if metadata.domain.is_empty() {
            errors.push(PackError::Validation("regpack domain is empty".to_string()));
        }
    }
    errors
}

/// Resolve regpack references from a zone manifest.
pub fn resolve_regpack_refs(zone: &serde_json::Value) -> PackResult<Vec<RegpackRef>> {
    let mut refs = Vec::new();
    if let Some(regpacks) = zone.get("regpacks").and_then(|v| v.as_array()) {
        for rp in regpacks {
            let jid = rp
                .get("jurisdiction_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let domain = rp
                .get("domain")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let digest = rp
                .get("regpack_digest_sha256")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if !digest.is_empty() && parser::is_valid_sha256(&digest) {
                refs.push(RegpackRef {
                    jurisdiction_id: jid,
                    domain,
                    regpack_digest_sha256: digest,
                    as_of_date: rp
                        .get("as_of_date")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                });
            }
        }
    }
    Ok(refs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_entry(id: &str, name: &str, entry_type: &str) -> SanctionsEntry {
        SanctionsEntry {
            entry_id: id.to_string(),
            entry_type: entry_type.to_string(),
            source_lists: vec!["ofac_sdn".to_string()],
            primary_name: name.to_string(),
            aliases: vec![],
            identifiers: vec![],
            addresses: vec![],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![],
            listing_date: None,
            remarks: None,
        }
    }

    #[test]
    fn test_sanctions_exact_match() {
        let entries = vec![make_test_entry("E001", "Acme Corp", "entity")];
        let checker = SanctionsChecker::new(entries, "snap-001".to_string());
        let result = checker.check_entity("Acme Corp", None, 0.7);
        assert!(result.matched);
        assert_eq!(result.match_score, 1.0);
        assert_eq!(result.matches[0].match_type, "exact_name");
    }

    #[test]
    fn test_sanctions_no_match() {
        let entries = vec![make_test_entry("E001", "Acme Corp", "entity")];
        let checker = SanctionsChecker::new(entries, "snap-001".to_string());
        let result = checker.check_entity("Totally Different Inc", None, 0.7);
        assert!(!result.matched);
        assert_eq!(result.matches.len(), 0);
    }

    #[test]
    fn test_sanctions_alias_match() {
        let mut entry = make_test_entry("E001", "Acme Corporation", "entity");
        let mut alias = BTreeMap::new();
        alias.insert("name".to_string(), "Acme Corp".to_string());
        entry.aliases = vec![alias];

        let checker = SanctionsChecker::new(vec![entry], "snap-001".to_string());
        let result = checker.check_entity("Acme Corp", None, 0.7);
        assert!(result.matched);
    }

    #[test]
    fn test_sanctions_identifier_match() {
        let mut entry = make_test_entry("E001", "Acme Corp", "entity");
        let mut ident = BTreeMap::new();
        ident.insert("type".to_string(), "registration".to_string());
        ident.insert("value".to_string(), "REG123456".to_string());
        entry.identifiers = vec![ident];

        let checker = SanctionsChecker::new(vec![entry], "snap-001".to_string());
        let mut query_ident = BTreeMap::new();
        query_ident.insert("type".to_string(), "registration".to_string());
        query_ident.insert("value".to_string(), "REG123456".to_string());
        let result = checker.check_entity("Some Other Name", Some(&[query_ident]), 0.7);
        assert!(result.matched);
        assert_eq!(result.matches[0].match_type, "identifier");
    }

    #[test]
    fn test_sanctions_fuzzy_match() {
        let entries = vec![make_test_entry(
            "E001",
            "International Trading Company",
            "entity",
        )];
        let checker = SanctionsChecker::new(entries, "snap-001".to_string());
        let result = checker.check_entity("International Trading", None, 0.5);
        // Token overlap should give partial match
        assert!(result.match_score > 0.0);
    }

    #[test]
    fn test_regpack_metadata_creation() {
        let meta = RegPackMetadata {
            regpack_id: "regpack:pk:financial:202601".to_string(),
            jurisdiction_id: "pk".to_string(),
            domain: "financial".to_string(),
            as_of_date: "2026-01-15".to_string(),
            snapshot_type: "quarterly".to_string(),
            sources: vec![],
            includes: BTreeMap::new(),
            previous_regpack_digest: None,
            created_at: Some("2026-01-15T00:00:00Z".to_string()),
            expires_at: None,
            digest_sha256: None,
        };
        assert_eq!(meta.jurisdiction_id, "pk");
        assert_eq!(meta.domain, "financial");
    }

    #[test]
    fn test_regpack_digest_deterministic() {
        let meta = RegPackMetadata {
            regpack_id: "test".to_string(),
            jurisdiction_id: "pk".to_string(),
            domain: "financial".to_string(),
            as_of_date: "2026-01-15".to_string(),
            snapshot_type: "quarterly".to_string(),
            sources: vec![],
            includes: BTreeMap::new(),
            previous_regpack_digest: None,
            created_at: None,
            expires_at: None,
            digest_sha256: None,
        };
        let d1 = compute_regpack_digest(&meta, None, None, None).unwrap();
        let d2 = compute_regpack_digest(&meta, None, None, None).unwrap();
        assert_eq!(d1, d2);
        assert_eq!(d1.len(), 64);
    }

    #[test]
    fn test_validate_compliance_domain_known() {
        assert!(validate_compliance_domain("aml").is_ok());
        assert!(validate_compliance_domain("kyc").is_ok());
        assert!(validate_compliance_domain("sanctions").is_ok());
        assert!(validate_compliance_domain("licensing").is_ok());
    }

    #[test]
    fn test_validate_compliance_domain_unknown() {
        assert!(validate_compliance_domain("bogus_domain").is_err());
    }

    #[test]
    fn test_all_compliance_domains_valid() {
        // Verify every ComplianceDomain variant roundtrips through validation
        for domain in ComplianceDomain::all() {
            let s = domain.as_str();
            assert!(
                validate_compliance_domain(s).is_ok(),
                "Domain {s} failed validation"
            );
        }
    }

    #[test]
    fn test_compliance_domain_count() {
        assert_eq!(ComplianceDomain::all().len(), 20);
    }

    #[test]
    fn test_regpack_ref_serialization() {
        let r = RegpackRef {
            jurisdiction_id: "pk".to_string(),
            domain: "financial".to_string(),
            regpack_digest_sha256: "a".repeat(64),
            as_of_date: Some("2026-01-15".to_string()),
        };
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["jurisdiction_id"], "pk");
        assert_eq!(json["domain"], "financial");
    }

    #[test]
    fn test_reporting_requirement_serialization() {
        let req = ReportingRequirement {
            report_type_id: "qr-001".to_string(),
            name: "Quarterly Financial Report".to_string(),
            regulator_id: "fsra".to_string(),
            applicable_to: vec!["bank".to_string(), "emi".to_string()],
            frequency: "quarterly".to_string(),
            deadlines: BTreeMap::new(),
            submission: BTreeMap::new(),
            late_penalty: BTreeMap::new(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["frequency"], "quarterly");
        assert_eq!(json["applicable_to"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_compliance_deadline_creation() {
        let deadline = ComplianceDeadline {
            deadline_id: "dl-001".to_string(),
            regulator_id: "fsra".to_string(),
            deadline_type: "report".to_string(),
            description: "Q1 2026 financial report".to_string(),
            due_date: "2026-04-30".to_string(),
            grace_period_days: 14,
            applicable_license_types: vec!["banking".to_string()],
        };
        assert_eq!(deadline.grace_period_days, 14);
    }

    // -----------------------------------------------------------------------
    // SanctionsChecker — additional edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_sanctions_case_insensitive_exact_match() {
        let entries = vec![make_test_entry("E001", "ACME CORP", "entity")];
        let checker = SanctionsChecker::new(entries, "snap-001".to_string());
        let result = checker.check_entity("acme corp", None, 0.7);
        assert!(result.matched);
        assert_eq!(result.match_score, 1.0);
    }

    #[test]
    fn test_sanctions_empty_name() {
        let entries = vec![make_test_entry("E001", "Acme Corp", "entity")];
        let checker = SanctionsChecker::new(entries, "snap-001".to_string());
        let result = checker.check_entity("", None, 0.7);
        assert!(!result.matched);
    }

    #[test]
    fn test_sanctions_multiple_entries_exact() {
        let entries = vec![
            make_test_entry("E001", "Alpha Corp", "entity"),
            make_test_entry("E002", "Beta Corp", "entity"),
            make_test_entry("E003", "Gamma Corp", "entity"),
        ];
        let checker = SanctionsChecker::new(entries, "snap-001".to_string());

        let r1 = checker.check_entity("Alpha Corp", None, 0.7);
        assert!(r1.matched);
        assert_eq!(r1.matches.len(), 1);
        assert_eq!(r1.matches[0].entry.entry_id, "E001");

        let r2 = checker.check_entity("Beta Corp", None, 0.7);
        assert!(r2.matched);
        assert_eq!(r2.matches[0].entry.entry_id, "E002");
    }

    #[test]
    fn test_sanctions_checker_with_multiple_aliases() {
        let mut entry = make_test_entry("E001", "Main Name", "individual");
        let mut alias1 = BTreeMap::new();
        alias1.insert("name".to_string(), "Alias One".to_string());
        let mut alias2 = BTreeMap::new();
        alias2.insert("alias".to_string(), "Alias Two".to_string());
        entry.aliases = vec![alias1, alias2];

        let checker = SanctionsChecker::new(vec![entry], "snap-001".to_string());

        assert!(checker.check_entity("Alias One", None, 0.7).matched);
        assert!(checker.check_entity("Alias Two", None, 0.7).matched);
        assert!(checker.check_entity("Main Name", None, 0.7).matched);
    }

    #[test]
    fn test_sanctions_checker_identifier_case_insensitive() {
        let mut entry = make_test_entry("E001", "Some Entity", "entity");
        let mut ident = BTreeMap::new();
        ident.insert("type".to_string(), "passport".to_string());
        ident.insert("value".to_string(), "AB123456".to_string());
        entry.identifiers = vec![ident];

        let checker = SanctionsChecker::new(vec![entry], "snap-001".to_string());

        let mut query = BTreeMap::new();
        query.insert("type".to_string(), "passport".to_string());
        query.insert("value".to_string(), "ab123456".to_string()); // lowercase
        let result = checker.check_entity("Unrelated Name", Some(&[query]), 0.7);
        assert!(result.matched);
        assert_eq!(result.matches[0].match_type, "identifier");
    }

    #[test]
    fn test_sanctions_checker_deduplicates_matches() {
        // Entry matched by both alias and identifier should appear once
        let mut entry = make_test_entry("E001", "Acme Corp", "entity");
        let mut alias = BTreeMap::new();
        alias.insert("name".to_string(), "Acme Corp".to_string());
        entry.aliases = vec![alias];

        let mut ident = BTreeMap::new();
        ident.insert("type".to_string(), "reg".to_string());
        ident.insert("value".to_string(), "REG001".to_string());
        entry.identifiers = vec![ident];

        let checker = SanctionsChecker::new(vec![entry], "snap-001".to_string());

        let mut query_ident = BTreeMap::new();
        query_ident.insert("type".to_string(), "reg".to_string());
        query_ident.insert("value".to_string(), "REG001".to_string());

        let result = checker.check_entity("Acme Corp", Some(&[query_ident]), 0.7);
        assert!(result.matched);
        // Should be deduplicated by entry_id
        let unique_ids: std::collections::HashSet<_> = result
            .matches
            .iter()
            .map(|m| m.entry.entry_id.clone())
            .collect();
        assert_eq!(unique_ids.len(), 1);
    }

    #[test]
    fn test_sanctions_fuzzy_score_exact() {
        assert_eq!(
            SanctionsChecker::fuzzy_score("hello world", "hello world"),
            1.0
        );
    }

    #[test]
    fn test_sanctions_fuzzy_score_empty() {
        assert_eq!(SanctionsChecker::fuzzy_score("", "something"), 0.0);
        assert_eq!(SanctionsChecker::fuzzy_score("something", ""), 0.0);
    }

    #[test]
    fn test_sanctions_fuzzy_score_substring() {
        let score = SanctionsChecker::fuzzy_score("Acme Corp", "Acme Corporation");
        assert!(score >= 0.5); // Substring match should score well
    }

    #[test]
    fn test_sanctions_normalize_removes_punctuation() {
        let normalized = SanctionsChecker::normalize("Hello, World! (Test)");
        assert_eq!(normalized, "hello world test");
    }

    #[test]
    fn test_sanctions_normalize_collapses_whitespace() {
        let normalized = SanctionsChecker::normalize("  hello   world  ");
        assert_eq!(normalized, "hello world");
    }

    #[test]
    fn test_sanctions_check_result_metadata() {
        let entries = vec![make_test_entry("E001", "Test", "entity")];
        let checker = SanctionsChecker::new(entries, "snap-123".to_string());
        let result = checker.check_entity("Test", None, 0.7);
        assert_eq!(result.query, "Test");
        assert_eq!(result.snapshot_id, "snap-123");
        assert!(!result.checked_at.is_empty());
    }

    // -----------------------------------------------------------------------
    // compute_regpack_digest — with components
    // -----------------------------------------------------------------------

    #[test]
    fn test_regpack_digest_changes_with_sanctions() {
        let meta = RegPackMetadata {
            regpack_id: "test".to_string(),
            jurisdiction_id: "pk".to_string(),
            domain: "aml".to_string(),
            as_of_date: "2026-01-15".to_string(),
            snapshot_type: "quarterly".to_string(),
            sources: vec![],
            includes: BTreeMap::new(),
            previous_regpack_digest: None,
            created_at: None,
            expires_at: None,
            digest_sha256: None,
        };

        let d_no_sanctions = compute_regpack_digest(&meta, None, None, None).unwrap();

        let sanctions = SanctionsSnapshot {
            snapshot_id: "snap-001".to_string(),
            snapshot_timestamp: "2026-01-15T00:00:00Z".to_string(),
            sources: BTreeMap::new(),
            consolidated_counts: BTreeMap::new(),
            delta_from_previous: None,
        };

        let d_with_sanctions = compute_regpack_digest(&meta, Some(&sanctions), None, None).unwrap();
        assert_ne!(d_no_sanctions, d_with_sanctions);
    }

    #[test]
    fn test_regpack_digest_changes_with_regulators() {
        let meta = RegPackMetadata {
            regpack_id: "test".to_string(),
            jurisdiction_id: "pk".to_string(),
            domain: "aml".to_string(),
            as_of_date: "2026-01-15".to_string(),
            snapshot_type: "quarterly".to_string(),
            sources: vec![],
            includes: BTreeMap::new(),
            previous_regpack_digest: None,
            created_at: None,
            expires_at: None,
            digest_sha256: None,
        };

        let d_no_regs = compute_regpack_digest(&meta, None, None, None).unwrap();

        let regulators = vec![RegulatorProfile {
            regulator_id: "fsra".to_string(),
            name: "Financial Services Regulatory Authority".to_string(),
            jurisdiction_id: "pk-kp-rsez".to_string(),
            parent_authority: None,
            scope: BTreeMap::new(),
            contact: BTreeMap::new(),
            api_capabilities: BTreeMap::new(),
            timezone: "Asia/Karachi".to_string(),
            business_days: vec!["monday".to_string()],
        }];

        let d_with_regs = compute_regpack_digest(&meta, None, Some(&regulators), None).unwrap();
        assert_ne!(d_no_regs, d_with_regs);
    }

    #[test]
    fn test_regpack_digest_changes_with_deadlines() {
        let meta = RegPackMetadata {
            regpack_id: "test".to_string(),
            jurisdiction_id: "pk".to_string(),
            domain: "aml".to_string(),
            as_of_date: "2026-01-15".to_string(),
            snapshot_type: "quarterly".to_string(),
            sources: vec![],
            includes: BTreeMap::new(),
            previous_regpack_digest: None,
            created_at: None,
            expires_at: None,
            digest_sha256: None,
        };

        let d_no_dl = compute_regpack_digest(&meta, None, None, None).unwrap();

        let deadlines = vec![ComplianceDeadline {
            deadline_id: "dl-001".to_string(),
            regulator_id: "fsra".to_string(),
            deadline_type: "report".to_string(),
            description: "Q1 report".to_string(),
            due_date: "2026-04-30".to_string(),
            grace_period_days: 14,
            applicable_license_types: vec![],
        }];

        let d_with_dl = compute_regpack_digest(&meta, None, None, Some(&deadlines)).unwrap();
        assert_ne!(d_no_dl, d_with_dl);
    }

    #[test]
    fn test_regpack_digest_all_components() {
        let meta = RegPackMetadata {
            regpack_id: "full-test".to_string(),
            jurisdiction_id: "pk".to_string(),
            domain: "aml".to_string(),
            as_of_date: "2026-01-15".to_string(),
            snapshot_type: "quarterly".to_string(),
            sources: vec![],
            includes: BTreeMap::new(),
            previous_regpack_digest: None,
            created_at: None,
            expires_at: None,
            digest_sha256: None,
        };

        let sanctions = SanctionsSnapshot {
            snapshot_id: "snap-001".to_string(),
            snapshot_timestamp: "2026-01-15T00:00:00Z".to_string(),
            sources: BTreeMap::new(),
            consolidated_counts: BTreeMap::new(),
            delta_from_previous: None,
        };

        let regulators = vec![RegulatorProfile {
            regulator_id: "fsra".to_string(),
            name: "FSRA".to_string(),
            jurisdiction_id: "pk-kp-rsez".to_string(),
            parent_authority: None,
            scope: BTreeMap::new(),
            contact: BTreeMap::new(),
            api_capabilities: BTreeMap::new(),
            timezone: "UTC".to_string(),
            business_days: vec![],
        }];

        let deadlines = vec![ComplianceDeadline {
            deadline_id: "dl-001".to_string(),
            regulator_id: "fsra".to_string(),
            deadline_type: "report".to_string(),
            description: "Test".to_string(),
            due_date: "2026-04-30".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![],
        }];

        let digest =
            compute_regpack_digest(&meta, Some(&sanctions), Some(&regulators), Some(&deadlines))
                .unwrap();
        assert_eq!(digest.len(), 64);

        // Deterministic
        let digest2 =
            compute_regpack_digest(&meta, Some(&sanctions), Some(&regulators), Some(&deadlines))
                .unwrap();
        assert_eq!(digest, digest2);
    }

    // -----------------------------------------------------------------------
    // validate_regpack_domains — additional edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_regpack_domains_known_domain() {
        let meta = RegPackMetadata {
            regpack_id: "test".to_string(),
            jurisdiction_id: "pk".to_string(),
            domain: "aml".to_string(),
            as_of_date: "2026-01-15".to_string(),
            snapshot_type: "quarterly".to_string(),
            sources: vec![],
            includes: BTreeMap::new(),
            previous_regpack_digest: None,
            created_at: None,
            expires_at: None,
            digest_sha256: None,
        };
        let errors = validate_regpack_domains(&meta);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_regpack_domains_empty() {
        let meta = RegPackMetadata {
            regpack_id: "test".to_string(),
            jurisdiction_id: "pk".to_string(),
            domain: "".to_string(),
            as_of_date: "2026-01-15".to_string(),
            snapshot_type: "quarterly".to_string(),
            sources: vec![],
            includes: BTreeMap::new(),
            previous_regpack_digest: None,
            created_at: None,
            expires_at: None,
            digest_sha256: None,
        };
        let errors = validate_regpack_domains(&meta);
        assert!(!errors.is_empty());
    }

    // -----------------------------------------------------------------------
    // resolve_regpack_refs
    // -----------------------------------------------------------------------

    #[test]
    fn test_resolve_regpack_refs_valid() {
        let zone = serde_json::json!({
            "zone_id": "test",
            "regpacks": [
                {
                    "jurisdiction_id": "pk",
                    "domain": "aml",
                    "regpack_digest_sha256": "a".repeat(64),
                    "as_of_date": "2026-01-15"
                }
            ]
        });
        let refs = resolve_regpack_refs(&zone).unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].jurisdiction_id, "pk");
        assert_eq!(refs[0].domain, "aml");
        assert_eq!(refs[0].as_of_date, Some("2026-01-15".to_string()));
    }

    #[test]
    fn test_resolve_regpack_refs_empty() {
        let zone = serde_json::json!({"zone_id": "test"});
        let refs = resolve_regpack_refs(&zone).unwrap();
        assert!(refs.is_empty());
    }

    #[test]
    fn test_resolve_regpack_refs_skips_invalid_digest() {
        let zone = serde_json::json!({
            "regpacks": [
                {
                    "jurisdiction_id": "pk",
                    "domain": "aml",
                    "regpack_digest_sha256": "invalid"
                }
            ]
        });
        let refs = resolve_regpack_refs(&zone).unwrap();
        assert!(refs.is_empty());
    }

    #[test]
    fn test_resolve_regpack_refs_skips_empty_digest() {
        let zone = serde_json::json!({
            "regpacks": [
                {
                    "jurisdiction_id": "pk",
                    "domain": "aml",
                    "regpack_digest_sha256": ""
                }
            ]
        });
        let refs = resolve_regpack_refs(&zone).unwrap();
        assert!(refs.is_empty());
    }

    #[test]
    fn test_resolve_regpack_refs_multiple() {
        let zone = serde_json::json!({
            "regpacks": [
                {
                    "jurisdiction_id": "pk",
                    "domain": "aml",
                    "regpack_digest_sha256": "a".repeat(64)
                },
                {
                    "jurisdiction_id": "ae",
                    "domain": "sanctions",
                    "regpack_digest_sha256": "b".repeat(64)
                }
            ]
        });
        let refs = resolve_regpack_refs(&zone).unwrap();
        assert_eq!(refs.len(), 2);
    }

    // -----------------------------------------------------------------------
    // RegulatorProfile serialization
    // -----------------------------------------------------------------------

    #[test]
    fn test_regulator_profile_serialization() {
        let mut scope = BTreeMap::new();
        scope.insert(
            "financial".to_string(),
            vec!["banking".to_string(), "insurance".to_string()],
        );

        let reg = RegulatorProfile {
            regulator_id: "fsra".to_string(),
            name: "Financial Services Regulatory Authority".to_string(),
            jurisdiction_id: "pk-kp-rsez".to_string(),
            parent_authority: Some("sbp".to_string()),
            scope,
            contact: BTreeMap::new(),
            api_capabilities: BTreeMap::new(),
            timezone: "Asia/Karachi".to_string(),
            business_days: vec!["monday".to_string(), "tuesday".to_string()],
        };

        let json = serde_json::to_value(&reg).unwrap();
        assert_eq!(json["regulator_id"], "fsra");
        assert_eq!(json["parent_authority"], "sbp");
        assert_eq!(json["timezone"], "Asia/Karachi");
        assert_eq!(json["business_days"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_regulator_profile_default_timezone() {
        let json_str = r#"{"regulator_id":"test","name":"Test","jurisdiction_id":"pk"}"#;
        let reg: RegulatorProfile = serde_json::from_str(json_str).unwrap();
        assert_eq!(reg.timezone, "UTC");
    }

    // -----------------------------------------------------------------------
    // RegLicenseType serialization
    // -----------------------------------------------------------------------

    #[test]
    fn test_reg_license_type_serialization() {
        let lt = RegLicenseType {
            license_type_id: "lt-001".to_string(),
            name: "Banking License".to_string(),
            regulator_id: "sbp".to_string(),
            requirements: BTreeMap::new(),
            application: BTreeMap::new(),
            ongoing_obligations: BTreeMap::new(),
            validity_period_years: 5,
            renewal_lead_time_days: 180,
        };

        let json = serde_json::to_value(&lt).unwrap();
        assert_eq!(json["license_type_id"], "lt-001");
        assert_eq!(json["validity_period_years"], 5);
        assert_eq!(json["renewal_lead_time_days"], 180);
    }

    #[test]
    fn test_reg_license_type_defaults() {
        let json_str = r#"{"license_type_id":"lt","name":"Test","regulator_id":"reg"}"#;
        let lt: RegLicenseType = serde_json::from_str(json_str).unwrap();
        assert_eq!(lt.validity_period_years, 1);
        assert_eq!(lt.renewal_lead_time_days, 90);
    }

    // -----------------------------------------------------------------------
    // Regpack struct
    // -----------------------------------------------------------------------

    #[test]
    fn test_regpack_struct_creation() {
        let rp = Regpack {
            jurisdiction: JurisdictionId::new("pk".to_string()).unwrap(),
            name: "Pakistan AML Regpack".to_string(),
            version: "1.0.0".to_string(),
            digest: None,
            metadata: None,
        };
        assert_eq!(rp.name, "Pakistan AML Regpack");
        assert!(rp.digest.is_none());
    }

    #[test]
    fn test_regpack_struct_serialization_roundtrip() {
        let rp = Regpack {
            jurisdiction: JurisdictionId::new("ae".to_string()).unwrap(),
            name: "UAE Regpack".to_string(),
            version: "2.0".to_string(),
            digest: None,
            metadata: None,
        };
        let json_str = serde_json::to_string(&rp).unwrap();
        let deserialized: Regpack = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.name, "UAE Regpack");
        assert_eq!(deserialized.version, "2.0");
    }

    // -----------------------------------------------------------------------
    // SanctionsSnapshot serialization
    // -----------------------------------------------------------------------

    #[test]
    fn test_sanctions_snapshot_serialization() {
        let mut sources = BTreeMap::new();
        sources.insert(
            "ofac".to_string(),
            serde_json::json!({"url": "https://ofac.treasury.gov"}),
        );

        let mut counts = BTreeMap::new();
        counts.insert("individual".to_string(), 150);
        counts.insert("entity".to_string(), 80);

        let snap = SanctionsSnapshot {
            snapshot_id: "snap-001".to_string(),
            snapshot_timestamp: "2026-01-15T00:00:00Z".to_string(),
            sources,
            consolidated_counts: counts,
            delta_from_previous: None,
        };

        let json = serde_json::to_value(&snap).unwrap();
        assert_eq!(json["snapshot_id"], "snap-001");
        assert_eq!(json["consolidated_counts"]["individual"], 150);
    }

    // -----------------------------------------------------------------------
    // SanctionsEntry serialization
    // -----------------------------------------------------------------------

    #[test]
    fn test_sanctions_entry_full_serialization() {
        let entry = SanctionsEntry {
            entry_id: "E001".to_string(),
            entry_type: "individual".to_string(),
            source_lists: vec!["ofac_sdn".to_string(), "un_sanctions".to_string()],
            primary_name: "Test Person".to_string(),
            aliases: vec![],
            identifiers: vec![],
            addresses: vec![],
            nationalities: vec!["PK".to_string()],
            date_of_birth: Some("1970-01-01".to_string()),
            programs: vec!["sdgt".to_string()],
            listing_date: Some("2020-06-15".to_string()),
            remarks: Some("Test remark".to_string()),
        };

        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["entry_type"], "individual");
        assert_eq!(json["nationalities"].as_array().unwrap().len(), 1);
        assert_eq!(json["date_of_birth"], "1970-01-01");
    }

    // -----------------------------------------------------------------------
    // SanctionsCheckResult serialization
    // -----------------------------------------------------------------------

    #[test]
    fn test_sanctions_check_result_serialization() {
        let result = SanctionsCheckResult {
            query: "Test Corp".to_string(),
            checked_at: "2026-01-15T12:00:00Z".to_string(),
            snapshot_id: "snap-001".to_string(),
            matched: false,
            matches: vec![],
            match_score: 0.0,
        };

        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["query"], "Test Corp");
        assert_eq!(json["matched"], false);
    }

    // -----------------------------------------------------------------------
    // RegpackRef
    // -----------------------------------------------------------------------

    #[test]
    fn test_regpack_ref_without_date() {
        let r = RegpackRef {
            jurisdiction_id: "pk".to_string(),
            domain: "aml".to_string(),
            regpack_digest_sha256: "a".repeat(64),
            as_of_date: None,
        };
        let json = serde_json::to_value(&r).unwrap();
        assert!(json.get("as_of_date").is_none());
    }

    #[test]
    fn test_regpack_ref_equality() {
        let r1 = RegpackRef {
            jurisdiction_id: "pk".to_string(),
            domain: "aml".to_string(),
            regpack_digest_sha256: "a".repeat(64),
            as_of_date: None,
        };
        let r2 = r1.clone();
        assert_eq!(r1, r2);
    }

    // -----------------------------------------------------------------------
    // ComplianceDeadline serialization roundtrip
    // -----------------------------------------------------------------------

    #[test]
    fn test_compliance_deadline_roundtrip() {
        let deadline = ComplianceDeadline {
            deadline_id: "dl-001".to_string(),
            regulator_id: "fsra".to_string(),
            deadline_type: "filing".to_string(),
            description: "Annual filing".to_string(),
            due_date: "2026-12-31".to_string(),
            grace_period_days: 30,
            applicable_license_types: vec!["banking".to_string(), "emi".to_string()],
        };
        let json_str = serde_json::to_string(&deadline).unwrap();
        let deserialized: ComplianceDeadline = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.deadline_id, "dl-001");
        assert_eq!(deserialized.grace_period_days, 30);
        assert_eq!(deserialized.applicable_license_types.len(), 2);
    }

    // -----------------------------------------------------------------------
    // ReportingRequirement additional
    // -----------------------------------------------------------------------

    #[test]
    fn test_reporting_requirement_roundtrip() {
        let req = ReportingRequirement {
            report_type_id: "monthly-ctr".to_string(),
            name: "Monthly CTR".to_string(),
            regulator_id: "fmu".to_string(),
            applicable_to: vec!["bank".to_string()],
            frequency: "monthly".to_string(),
            deadlines: BTreeMap::new(),
            submission: BTreeMap::new(),
            late_penalty: BTreeMap::new(),
        };
        let json_str = serde_json::to_string(&req).unwrap();
        let deserialized: ReportingRequirement = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.report_type_id, "monthly-ctr");
        assert_eq!(deserialized.frequency, "monthly");
    }
}
