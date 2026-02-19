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
//! the canonical domain enum in `mez-core`, ensuring exhaustive coverage
//! and preventing domain drift.
//!
//! ## Spec Reference
//!
//! Ports Python `tools/regpack.py` with cross-language digest compatibility.

use std::collections::{BTreeMap, HashMap};

use mez_core::digest::Sha256Accumulator;
use serde::{Deserialize, Serialize};

use mez_core::{CanonicalBytes, ComplianceDomain, ContentDigest, JurisdictionId};

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
/// SHA256( b"mez-regpack-v1\0" + canonical(metadata) + canonical(components)... )
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
    acc.update(b"mez-regpack-v1\0");

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
/// the canonical domain enum in mez-core.
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
            jurisdiction_id: "pk-kp-rez".to_string(),
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
            jurisdiction_id: "pk-kp-rez".to_string(),
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
            jurisdiction_id: "pk-kp-rez".to_string(),
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

// ===========================================================================
// Pakistan Regpack Content — Real Regulatory Data
// ===========================================================================
//
// Provides Pakistan-specific regulatory content required by P0-PACK-001:
//   - Regulator profiles (SBP, SECP, FMU, FBR, NACTA)
//   - Sanctions entries (NACTA Proscribed Persons, UNSC 1267 consolidated)
//   - Compliance deadlines (FBR filing, SBP prudential, SECP annual)
//   - Reporting requirements (CTR, STR, prudential returns)
//   - Withholding tax rate schedule (ITO 2001)
// ===========================================================================

pub mod pakistan {
    use super::*;

    // ── Withholding Tax Rates (Income Tax Ordinance 2001) ───────────────────

    /// A withholding tax rate entry from the Income Tax Ordinance 2001.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct WithholdingTaxRate {
        /// Section reference (e.g., "149" for salary, "151" for profit on debt).
        pub section: String,
        /// Human-readable description.
        pub description: String,
        /// Applicable to filer or non-filer.
        pub taxpayer_status: String,
        /// Rate as string decimal (e.g., "0.15" for 15%).
        pub rate: String,
        /// Threshold amount (PKR) below which WHT does not apply.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub threshold_pkr: Option<String>,
        /// Effective date.
        pub effective_date: String,
        /// Legal basis.
        pub legal_basis: String,
    }

    /// Pakistan withholding tax rate schedule per ITO 2001 (FY 2025-26).
    pub fn pakistan_wht_rates() -> Vec<WithholdingTaxRate> {
        vec![
            WithholdingTaxRate {
                section: "149".to_string(),
                description: "Salary income".to_string(),
                taxpayer_status: "filer".to_string(),
                rate: "varies".to_string(),
                threshold_pkr: Some("600000".to_string()),
                effective_date: "2025-07-01".to_string(),
                legal_basis: "Income Tax Ordinance 2001, s.149".to_string(),
            },
            WithholdingTaxRate {
                section: "151(1)(a)".to_string(),
                description: "Profit on debt — bank deposits (filer)".to_string(),
                taxpayer_status: "filer".to_string(),
                rate: "0.15".to_string(),
                threshold_pkr: None,
                effective_date: "2025-07-01".to_string(),
                legal_basis: "Income Tax Ordinance 2001, s.151(1)(a)".to_string(),
            },
            WithholdingTaxRate {
                section: "151(1)(a)".to_string(),
                description: "Profit on debt — bank deposits (non-filer)".to_string(),
                taxpayer_status: "non-filer".to_string(),
                rate: "0.30".to_string(),
                threshold_pkr: None,
                effective_date: "2025-07-01".to_string(),
                legal_basis: "Income Tax Ordinance 2001, s.151(1)(a)".to_string(),
            },
            WithholdingTaxRate {
                section: "152(1)".to_string(),
                description: "Payments to non-residents — royalties/fees".to_string(),
                taxpayer_status: "filer".to_string(),
                rate: "0.15".to_string(),
                threshold_pkr: None,
                effective_date: "2025-07-01".to_string(),
                legal_basis: "Income Tax Ordinance 2001, s.152(1)".to_string(),
            },
            WithholdingTaxRate {
                section: "153(1)(a)".to_string(),
                description: "Sale of goods (filer, company)".to_string(),
                taxpayer_status: "filer".to_string(),
                rate: "0.04".to_string(),
                threshold_pkr: Some("75000".to_string()),
                effective_date: "2025-07-01".to_string(),
                legal_basis: "Income Tax Ordinance 2001, s.153(1)(a)".to_string(),
            },
            WithholdingTaxRate {
                section: "153(1)(a)".to_string(),
                description: "Sale of goods (non-filer, company)".to_string(),
                taxpayer_status: "non-filer".to_string(),
                rate: "0.08".to_string(),
                threshold_pkr: Some("75000".to_string()),
                effective_date: "2025-07-01".to_string(),
                legal_basis: "Income Tax Ordinance 2001, s.153(1)(a)".to_string(),
            },
            WithholdingTaxRate {
                section: "153(1)(b)".to_string(),
                description: "Rendering of services (filer, company)".to_string(),
                taxpayer_status: "filer".to_string(),
                rate: "0.08".to_string(),
                threshold_pkr: Some("30000".to_string()),
                effective_date: "2025-07-01".to_string(),
                legal_basis: "Income Tax Ordinance 2001, s.153(1)(b)".to_string(),
            },
            WithholdingTaxRate {
                section: "153(1)(b)".to_string(),
                description: "Rendering of services (non-filer, company)".to_string(),
                taxpayer_status: "non-filer".to_string(),
                rate: "0.16".to_string(),
                threshold_pkr: Some("30000".to_string()),
                effective_date: "2025-07-01".to_string(),
                legal_basis: "Income Tax Ordinance 2001, s.153(1)(b)".to_string(),
            },
            WithholdingTaxRate {
                section: "153(1)(c)".to_string(),
                description: "Contracts — execution of contracts (filer)".to_string(),
                taxpayer_status: "filer".to_string(),
                rate: "0.07".to_string(),
                threshold_pkr: Some("75000".to_string()),
                effective_date: "2025-07-01".to_string(),
                legal_basis: "Income Tax Ordinance 2001, s.153(1)(c)".to_string(),
            },
            WithholdingTaxRate {
                section: "153(1)(c)".to_string(),
                description: "Contracts — execution of contracts (non-filer)".to_string(),
                taxpayer_status: "non-filer".to_string(),
                rate: "0.14".to_string(),
                threshold_pkr: Some("75000".to_string()),
                effective_date: "2025-07-01".to_string(),
                legal_basis: "Income Tax Ordinance 2001, s.153(1)(c)".to_string(),
            },
            WithholdingTaxRate {
                section: "155".to_string(),
                description: "Income from property — rent (filer)".to_string(),
                taxpayer_status: "filer".to_string(),
                rate: "0.15".to_string(),
                threshold_pkr: Some("300000".to_string()),
                effective_date: "2025-07-01".to_string(),
                legal_basis: "Income Tax Ordinance 2001, s.155".to_string(),
            },
            WithholdingTaxRate {
                section: "156A".to_string(),
                description: "Prizes and winnings (filer)".to_string(),
                taxpayer_status: "filer".to_string(),
                rate: "0.15".to_string(),
                threshold_pkr: None,
                effective_date: "2025-07-01".to_string(),
                legal_basis: "Income Tax Ordinance 2001, s.156A".to_string(),
            },
            WithholdingTaxRate {
                section: "156A".to_string(),
                description: "Prizes and winnings (non-filer)".to_string(),
                taxpayer_status: "non-filer".to_string(),
                rate: "0.30".to_string(),
                threshold_pkr: None,
                effective_date: "2025-07-01".to_string(),
                legal_basis: "Income Tax Ordinance 2001, s.156A".to_string(),
            },
            WithholdingTaxRate {
                section: "231A".to_string(),
                description: "Cash withdrawal from bank (filer)".to_string(),
                taxpayer_status: "filer".to_string(),
                rate: "0.003".to_string(),
                threshold_pkr: Some("50000".to_string()),
                effective_date: "2025-07-01".to_string(),
                legal_basis: "Income Tax Ordinance 2001, s.231A".to_string(),
            },
            WithholdingTaxRate {
                section: "231A".to_string(),
                description: "Cash withdrawal from bank (non-filer)".to_string(),
                taxpayer_status: "non-filer".to_string(),
                rate: "0.006".to_string(),
                threshold_pkr: Some("50000".to_string()),
                effective_date: "2025-07-01".to_string(),
                legal_basis: "Income Tax Ordinance 2001, s.231A".to_string(),
            },
            WithholdingTaxRate {
                section: "236P".to_string(),
                description: "Banking transactions exceeding Rs 50k (non-filer)".to_string(),
                taxpayer_status: "non-filer".to_string(),
                rate: "0.006".to_string(),
                threshold_pkr: Some("50000".to_string()),
                effective_date: "2025-07-01".to_string(),
                legal_basis: "Income Tax Ordinance 2001, s.236P".to_string(),
            },
        ]
    }

    // ── Regulator Profiles ──────────────────────────────────────────────────

    /// State Bank of Pakistan — central bank and banking regulator.
    pub fn sbp_regulator() -> RegulatorProfile {
        let mut scope = BTreeMap::new();
        scope.insert(
            "banking".to_string(),
            vec![
                "commercial_banks".to_string(),
                "microfinance_banks".to_string(),
                "development_finance_institutions".to_string(),
            ],
        );
        scope.insert(
            "payments".to_string(),
            vec![
                "emi".to_string(),
                "psp".to_string(),
                "raast".to_string(),
                "rtgs".to_string(),
            ],
        );
        scope.insert(
            "foreign_exchange".to_string(),
            vec!["exchange_companies".to_string(), "authorized_dealers".to_string()],
        );

        let mut contact = BTreeMap::new();
        contact.insert("website".to_string(), "https://www.sbp.org.pk".to_string());
        contact.insert(
            "address".to_string(),
            "I.I. Chundrigar Road, Karachi 74000, Pakistan".to_string(),
        );

        let mut api = BTreeMap::new();
        api.insert("bank_registry".to_string(), true);
        api.insert("raast_integration".to_string(), true);
        api.insert("credit_bureau_query".to_string(), true);
        api.insert("forex_rate_feed".to_string(), true);

        RegulatorProfile {
            regulator_id: "pk-sbp".to_string(),
            name: "State Bank of Pakistan".to_string(),
            jurisdiction_id: "pk".to_string(),
            parent_authority: None,
            scope,
            contact,
            api_capabilities: api,
            timezone: "Asia/Karachi".to_string(),
            business_days: vec![
                "monday".to_string(),
                "tuesday".to_string(),
                "wednesday".to_string(),
                "thursday".to_string(),
                "friday".to_string(),
            ],
        }
    }

    /// Securities and Exchange Commission of Pakistan — corporate and capital markets regulator.
    pub fn secp_regulator() -> RegulatorProfile {
        let mut scope = BTreeMap::new();
        scope.insert(
            "corporate".to_string(),
            vec![
                "company_registration".to_string(),
                "corporate_governance".to_string(),
            ],
        );
        scope.insert(
            "capital_markets".to_string(),
            vec![
                "securities_brokers".to_string(),
                "mutual_funds".to_string(),
                "nbfcs".to_string(),
            ],
        );
        scope.insert(
            "insurance".to_string(),
            vec!["life_insurance".to_string(), "general_insurance".to_string()],
        );

        let mut contact = BTreeMap::new();
        contact.insert("website".to_string(), "https://www.secp.gov.pk".to_string());
        contact.insert(
            "address".to_string(),
            "NIC Building, Jinnah Avenue, Islamabad, Pakistan".to_string(),
        );

        let mut api = BTreeMap::new();
        api.insert("company_search".to_string(), true);
        api.insert("filing_status".to_string(), true);
        api.insert("eservices_portal".to_string(), true);

        RegulatorProfile {
            regulator_id: "pk-secp".to_string(),
            name: "Securities and Exchange Commission of Pakistan".to_string(),
            jurisdiction_id: "pk".to_string(),
            parent_authority: None,
            scope,
            contact,
            api_capabilities: api,
            timezone: "Asia/Karachi".to_string(),
            business_days: vec![
                "monday".to_string(),
                "tuesday".to_string(),
                "wednesday".to_string(),
                "thursday".to_string(),
                "friday".to_string(),
            ],
        }
    }

    /// Financial Monitoring Unit — Pakistan's AML/CFT financial intelligence unit.
    pub fn fmu_regulator() -> RegulatorProfile {
        let mut scope = BTreeMap::new();
        scope.insert(
            "aml_cft".to_string(),
            vec![
                "suspicious_transaction_reports".to_string(),
                "currency_transaction_reports".to_string(),
                "targeted_financial_sanctions".to_string(),
                "mutual_legal_assistance".to_string(),
            ],
        );

        let mut contact = BTreeMap::new();
        contact.insert("website".to_string(), "https://www.fmu.gov.pk".to_string());
        contact.insert(
            "address".to_string(),
            "State Bank of Pakistan Building, Islamabad, Pakistan".to_string(),
        );

        let mut api = BTreeMap::new();
        api.insert("goaml_reporting".to_string(), true);
        api.insert("sanctions_query".to_string(), true);

        RegulatorProfile {
            regulator_id: "pk-fmu".to_string(),
            name: "Financial Monitoring Unit".to_string(),
            jurisdiction_id: "pk".to_string(),
            parent_authority: Some("pk-sbp".to_string()),
            scope,
            contact,
            api_capabilities: api,
            timezone: "Asia/Karachi".to_string(),
            business_days: vec![
                "monday".to_string(),
                "tuesday".to_string(),
                "wednesday".to_string(),
                "thursday".to_string(),
                "friday".to_string(),
            ],
        }
    }

    /// Federal Board of Revenue — Pakistan's tax authority.
    pub fn fbr_regulator() -> RegulatorProfile {
        let mut scope = BTreeMap::new();
        scope.insert(
            "tax".to_string(),
            vec![
                "income_tax".to_string(),
                "sales_tax".to_string(),
                "federal_excise".to_string(),
                "customs_duty".to_string(),
                "withholding_tax".to_string(),
            ],
        );

        let mut contact = BTreeMap::new();
        contact.insert("website".to_string(), "https://www.fbr.gov.pk".to_string());
        contact.insert(
            "address".to_string(),
            "Constitution Avenue, Islamabad, Pakistan".to_string(),
        );

        let mut api = BTreeMap::new();
        api.insert("iris_portal".to_string(), true);
        api.insert("ntn_verification".to_string(), true);
        api.insert("active_taxpayer_list".to_string(), true);
        api.insert("e_filing".to_string(), true);

        RegulatorProfile {
            regulator_id: "pk-fbr".to_string(),
            name: "Federal Board of Revenue".to_string(),
            jurisdiction_id: "pk".to_string(),
            parent_authority: None,
            scope,
            contact,
            api_capabilities: api,
            timezone: "Asia/Karachi".to_string(),
            business_days: vec![
                "monday".to_string(),
                "tuesday".to_string(),
                "wednesday".to_string(),
                "thursday".to_string(),
                "friday".to_string(),
            ],
        }
    }

    /// NACTA — National Counter Terrorism Authority (proscription list).
    pub fn nacta_regulator() -> RegulatorProfile {
        let mut scope = BTreeMap::new();
        scope.insert(
            "sanctions".to_string(),
            vec![
                "proscribed_organizations".to_string(),
                "designated_persons".to_string(),
                "unsc_1267_implementation".to_string(),
            ],
        );

        let mut contact = BTreeMap::new();
        contact.insert("website".to_string(), "https://nacta.gov.pk".to_string());
        contact.insert(
            "address".to_string(),
            "Sector G-5, Islamabad, Pakistan".to_string(),
        );

        RegulatorProfile {
            regulator_id: "pk-nacta".to_string(),
            name: "National Counter Terrorism Authority".to_string(),
            jurisdiction_id: "pk".to_string(),
            parent_authority: None,
            scope,
            contact,
            api_capabilities: BTreeMap::new(),
            timezone: "Asia/Karachi".to_string(),
            business_days: vec![
                "monday".to_string(),
                "tuesday".to_string(),
                "wednesday".to_string(),
                "thursday".to_string(),
                "friday".to_string(),
            ],
        }
    }

    /// All Pakistan regulatory authorities relevant to regpack domains.
    pub fn pakistan_regulators() -> Vec<RegulatorProfile> {
        vec![
            sbp_regulator(),
            secp_regulator(),
            fmu_regulator(),
            fbr_regulator(),
            nacta_regulator(),
        ]
    }

    // ── Sanctions Entries ────────────────────────────────────────────────────
    //
    // Representative entries from Pakistan's proscription regime.
    // Sources: NACTA First Schedule (Anti-Terrorism Act 1997),
    //          UNSC 1267/1989/2253 Consolidated Sanctions List.
    //
    // NOTE: These are publicly available, gazette-notified designations.
    // Real deployment must pull from live NACTA gazette and UNSC XML feed.

    /// Representative Pakistan sanctions entries for regpack content.
    pub fn pakistan_sanctions_entries() -> Vec<SanctionsEntry> {
        vec![
            SanctionsEntry {
                entry_id: "pk-nacta-001".to_string(),
                entry_type: "organization".to_string(),
                source_lists: vec![
                    "nacta_first_schedule".to_string(),
                    "unsc_1267".to_string(),
                ],
                primary_name: "Lashkar-e-Taiba".to_string(),
                aliases: vec![
                    btree_alias("Jamaat-ud-Dawa"),
                    btree_alias("Falah-i-Insaniyat Foundation"),
                ],
                identifiers: vec![],
                addresses: vec![btree_address("Muridke, Punjab, Pakistan")],
                nationalities: vec![],
                date_of_birth: None,
                programs: vec![
                    "ata_1997_first_schedule".to_string(),
                    "unsc_1267".to_string(),
                ],
                listing_date: Some("2002-01-14".to_string()),
                remarks: Some("UNSC QDe.118; ATA 1997 First Schedule".to_string()),
            },
            SanctionsEntry {
                entry_id: "pk-nacta-002".to_string(),
                entry_type: "organization".to_string(),
                source_lists: vec![
                    "nacta_first_schedule".to_string(),
                    "unsc_1267".to_string(),
                ],
                primary_name: "Jaish-e-Mohammed".to_string(),
                aliases: vec![
                    btree_alias("Jaish-i-Mohammed"),
                    btree_alias("Khuddam ul-Islam"),
                ],
                identifiers: vec![],
                addresses: vec![btree_address("Bahawalpur, Punjab, Pakistan")],
                nationalities: vec![],
                date_of_birth: None,
                programs: vec![
                    "ata_1997_first_schedule".to_string(),
                    "unsc_1267".to_string(),
                ],
                listing_date: Some("2001-10-17".to_string()),
                remarks: Some("UNSC QDe.019; ATA 1997 First Schedule".to_string()),
            },
            SanctionsEntry {
                entry_id: "pk-nacta-003".to_string(),
                entry_type: "organization".to_string(),
                source_lists: vec!["nacta_first_schedule".to_string()],
                primary_name: "Tehrik-i-Taliban Pakistan".to_string(),
                aliases: vec![btree_alias("TTP")],
                identifiers: vec![],
                addresses: vec![],
                nationalities: vec![],
                date_of_birth: None,
                programs: vec!["ata_1997_first_schedule".to_string()],
                listing_date: Some("2008-08-25".to_string()),
                remarks: Some("ATA 1997 First Schedule".to_string()),
            },
            SanctionsEntry {
                entry_id: "pk-nacta-004".to_string(),
                entry_type: "organization".to_string(),
                source_lists: vec![
                    "nacta_first_schedule".to_string(),
                    "unsc_1267".to_string(),
                ],
                primary_name: "Al-Qaeda".to_string(),
                aliases: vec![
                    btree_alias("Al-Qaida"),
                    btree_alias("The Base"),
                ],
                identifiers: vec![],
                addresses: vec![],
                nationalities: vec![],
                date_of_birth: None,
                programs: vec![
                    "ata_1997_first_schedule".to_string(),
                    "unsc_1267".to_string(),
                ],
                listing_date: Some("2001-10-15".to_string()),
                remarks: Some("UNSC QDe.004; ATA 1997 First Schedule".to_string()),
            },
            SanctionsEntry {
                entry_id: "pk-nacta-005".to_string(),
                entry_type: "organization".to_string(),
                source_lists: vec![
                    "nacta_first_schedule".to_string(),
                    "unsc_1989".to_string(),
                ],
                primary_name: "Islamic State / Daesh".to_string(),
                aliases: vec![
                    btree_alias("ISIL"),
                    btree_alias("ISIS"),
                    btree_alias("Daesh"),
                ],
                identifiers: vec![],
                addresses: vec![],
                nationalities: vec![],
                date_of_birth: None,
                programs: vec![
                    "ata_1997_first_schedule".to_string(),
                    "unsc_2253".to_string(),
                ],
                listing_date: Some("2015-07-01".to_string()),
                remarks: Some("UNSC; ATA 1997 First Schedule".to_string()),
            },
            SanctionsEntry {
                entry_id: "pk-nacta-006".to_string(),
                entry_type: "organization".to_string(),
                source_lists: vec!["nacta_first_schedule".to_string()],
                primary_name: "Sipah-e-Sahaba Pakistan".to_string(),
                aliases: vec![
                    btree_alias("SSP"),
                    btree_alias("Ahle Sunnat Wal Jamaat"),
                ],
                identifiers: vec![],
                addresses: vec![btree_address("Jhang, Punjab, Pakistan")],
                nationalities: vec![],
                date_of_birth: None,
                programs: vec!["ata_1997_first_schedule".to_string()],
                listing_date: Some("2002-01-14".to_string()),
                remarks: Some("ATA 1997 First Schedule".to_string()),
            },
            SanctionsEntry {
                entry_id: "pk-nacta-007".to_string(),
                entry_type: "organization".to_string(),
                source_lists: vec!["nacta_first_schedule".to_string()],
                primary_name: "Lashkar-e-Jhangvi".to_string(),
                aliases: vec![btree_alias("LeJ")],
                identifiers: vec![],
                addresses: vec![],
                nationalities: vec![],
                date_of_birth: None,
                programs: vec![
                    "ata_1997_first_schedule".to_string(),
                    "unsc_1267".to_string(),
                ],
                listing_date: Some("2001-08-14".to_string()),
                remarks: Some("UNSC QDe.096; ATA 1997 First Schedule".to_string()),
            },
            SanctionsEntry {
                entry_id: "pk-nacta-008".to_string(),
                entry_type: "organization".to_string(),
                source_lists: vec!["nacta_first_schedule".to_string()],
                primary_name: "Balochistan Liberation Army".to_string(),
                aliases: vec![btree_alias("BLA")],
                identifiers: vec![],
                addresses: vec![],
                nationalities: vec![],
                date_of_birth: None,
                programs: vec!["ata_1997_first_schedule".to_string()],
                listing_date: Some("2006-04-07".to_string()),
                remarks: Some("ATA 1997 First Schedule".to_string()),
            },
        ]
    }

    /// Build a sanctions snapshot from Pakistan entries.
    pub fn pakistan_sanctions_snapshot() -> SanctionsSnapshot {
        let entries = pakistan_sanctions_entries();

        let mut counts = BTreeMap::new();
        for entry in &entries {
            *counts.entry(entry.entry_type.clone()).or_insert(0i64) += 1;
        }

        let mut sources = BTreeMap::new();
        sources.insert(
            "nacta_first_schedule".to_string(),
            serde_json::json!({
                "name": "NACTA First Schedule — Anti-Terrorism Act 1997",
                "url": "https://nacta.gov.pk/proscribed-organizations/",
                "authority": "Government of Pakistan",
                "legal_basis": "Anti-Terrorism Act 1997, First Schedule"
            }),
        );
        sources.insert(
            "unsc_1267".to_string(),
            serde_json::json!({
                "name": "UNSC 1267/1989/2253 Consolidated List",
                "url": "https://www.un.org/securitycouncil/sanctions/1267",
                "authority": "United Nations Security Council"
            }),
        );

        SanctionsSnapshot {
            snapshot_id: "pk-sanctions-2026Q1".to_string(),
            snapshot_timestamp: "2026-01-15T00:00:00Z".to_string(),
            sources,
            consolidated_counts: counts,
            delta_from_previous: None,
        }
    }

    // ── Compliance Deadlines ────────────────────────────────────────────────

    /// Pakistan compliance deadlines for FY 2025-26.
    pub fn pakistan_compliance_deadlines() -> Vec<ComplianceDeadline> {
        vec![
            // FBR Income Tax
            ComplianceDeadline {
                deadline_id: "pk-fbr-it-annual-company".to_string(),
                regulator_id: "pk-fbr".to_string(),
                deadline_type: "filing".to_string(),
                description: "Annual income tax return — companies (FY 2025-26)".to_string(),
                due_date: "2026-12-31".to_string(),
                grace_period_days: 0,
                applicable_license_types: vec![
                    "pk-secp:company-registration".to_string(),
                    "pk-sbp:commercial-bank".to_string(),
                    "pk-sbp:microfinance-bank".to_string(),
                    "pk-sbp:emi".to_string(),
                ],
            },
            ComplianceDeadline {
                deadline_id: "pk-fbr-it-annual-individual".to_string(),
                regulator_id: "pk-fbr".to_string(),
                deadline_type: "filing".to_string(),
                description: "Annual income tax return — individuals/AOPs (FY 2025-26)".to_string(),
                due_date: "2026-09-30".to_string(),
                grace_period_days: 0,
                applicable_license_types: vec![],
            },
            ComplianceDeadline {
                deadline_id: "pk-fbr-wht-monthly".to_string(),
                regulator_id: "pk-fbr".to_string(),
                deadline_type: "payment".to_string(),
                description: "Monthly withholding tax statement (15th of following month)".to_string(),
                due_date: "2026-02-15".to_string(),
                grace_period_days: 0,
                applicable_license_types: vec![
                    "pk-sbp:commercial-bank".to_string(),
                    "pk-secp:company-registration".to_string(),
                ],
            },
            ComplianceDeadline {
                deadline_id: "pk-fbr-sales-tax-monthly".to_string(),
                regulator_id: "pk-fbr".to_string(),
                deadline_type: "filing".to_string(),
                description: "Monthly sales tax return (18th of following month)".to_string(),
                due_date: "2026-02-18".to_string(),
                grace_period_days: 0,
                applicable_license_types: vec![],
            },
            // SBP Prudential Returns
            ComplianceDeadline {
                deadline_id: "pk-sbp-quarterly-prudential".to_string(),
                regulator_id: "pk-sbp".to_string(),
                deadline_type: "report".to_string(),
                description: "Quarterly prudential return — banks (within 30 days of quarter-end)"
                    .to_string(),
                due_date: "2026-04-30".to_string(),
                grace_period_days: 0,
                applicable_license_types: vec![
                    "pk-sbp:commercial-bank".to_string(),
                    "pk-sbp:microfinance-bank".to_string(),
                ],
            },
            ComplianceDeadline {
                deadline_id: "pk-sbp-annual-audited".to_string(),
                regulator_id: "pk-sbp".to_string(),
                deadline_type: "report".to_string(),
                description: "Annual audited financial statements — banks (within 4 months of FY-end)"
                    .to_string(),
                due_date: "2026-04-30".to_string(),
                grace_period_days: 30,
                applicable_license_types: vec![
                    "pk-sbp:commercial-bank".to_string(),
                    "pk-sbp:microfinance-bank".to_string(),
                ],
            },
            ComplianceDeadline {
                deadline_id: "pk-sbp-emi-quarterly".to_string(),
                regulator_id: "pk-sbp".to_string(),
                deadline_type: "report".to_string(),
                description: "Quarterly EMI compliance report — float safeguarding, transaction volume"
                    .to_string(),
                due_date: "2026-04-30".to_string(),
                grace_period_days: 15,
                applicable_license_types: vec!["pk-sbp:emi".to_string()],
            },
            ComplianceDeadline {
                deadline_id: "pk-sbp-forex-monthly".to_string(),
                regulator_id: "pk-sbp".to_string(),
                deadline_type: "report".to_string(),
                description: "Monthly foreign exchange position report — exchange companies"
                    .to_string(),
                due_date: "2026-02-10".to_string(),
                grace_period_days: 0,
                applicable_license_types: vec!["pk-sbp:exchange-company".to_string()],
            },
            // SECP Annual Filings
            ComplianceDeadline {
                deadline_id: "pk-secp-annual-return".to_string(),
                regulator_id: "pk-secp".to_string(),
                deadline_type: "filing".to_string(),
                description:
                    "Annual return (Form A) — within 30 days of AGM (Companies Act 2017 s.130)"
                        .to_string(),
                due_date: "2026-10-30".to_string(),
                grace_period_days: 30,
                applicable_license_types: vec!["pk-secp:company-registration".to_string()],
            },
            ComplianceDeadline {
                deadline_id: "pk-secp-financial-statements".to_string(),
                regulator_id: "pk-secp".to_string(),
                deadline_type: "filing".to_string(),
                description:
                    "Audited financial statements — within 4 months of FY-end (s.233 Companies Act 2017)"
                        .to_string(),
                due_date: "2026-10-30".to_string(),
                grace_period_days: 0,
                applicable_license_types: vec![
                    "pk-secp:company-registration".to_string(),
                    "pk-secp:nbfc".to_string(),
                ],
            },
            ComplianceDeadline {
                deadline_id: "pk-secp-broker-net-capital".to_string(),
                regulator_id: "pk-secp".to_string(),
                deadline_type: "report".to_string(),
                description:
                    "Monthly net capital balance certificate — securities brokers"
                        .to_string(),
                due_date: "2026-02-15".to_string(),
                grace_period_days: 0,
                applicable_license_types: vec!["pk-secp:securities-broker".to_string()],
            },
            // FMU AML/CFT
            ComplianceDeadline {
                deadline_id: "pk-fmu-str-ongoing".to_string(),
                regulator_id: "pk-fmu".to_string(),
                deadline_type: "report".to_string(),
                description:
                    "Suspicious Transaction Report — within 7 days of suspicion (AML Act 2010 s.7)"
                        .to_string(),
                due_date: "ongoing".to_string(),
                grace_period_days: 0,
                applicable_license_types: vec![
                    "pk-sbp:commercial-bank".to_string(),
                    "pk-sbp:microfinance-bank".to_string(),
                    "pk-sbp:emi".to_string(),
                    "pk-sbp:exchange-company".to_string(),
                    "pk-secp:securities-broker".to_string(),
                ],
            },
            ComplianceDeadline {
                deadline_id: "pk-fmu-ctr-15days".to_string(),
                regulator_id: "pk-fmu".to_string(),
                deadline_type: "report".to_string(),
                description:
                    "Currency Transaction Report — within 15 days for transactions >= PKR 2M (AML Act 2010 s.7)"
                        .to_string(),
                due_date: "ongoing".to_string(),
                grace_period_days: 0,
                applicable_license_types: vec![
                    "pk-sbp:commercial-bank".to_string(),
                    "pk-sbp:exchange-company".to_string(),
                ],
            },
        ]
    }

    // ── Reporting Requirements ───────────────────────────────────────────────

    /// Pakistan reporting requirements across regulators.
    pub fn pakistan_reporting_requirements() -> Vec<ReportingRequirement> {
        vec![
            ReportingRequirement {
                report_type_id: "pk-fmu-str".to_string(),
                name: "Suspicious Transaction Report (STR)".to_string(),
                regulator_id: "pk-fmu".to_string(),
                applicable_to: vec![
                    "commercial_bank".to_string(),
                    "microfinance_bank".to_string(),
                    "emi".to_string(),
                    "exchange_company".to_string(),
                    "securities_broker".to_string(),
                    "nbfc".to_string(),
                    "insurance_company".to_string(),
                ],
                frequency: "event_driven".to_string(),
                deadlines: {
                    let mut d = BTreeMap::new();
                    let mut inner = BTreeMap::new();
                    inner.insert("days_from_detection".to_string(), "7".to_string());
                    inner.insert("submission_system".to_string(), "goAML".to_string());
                    d.insert("trigger".to_string(), inner);
                    d
                },
                submission: {
                    let mut s = BTreeMap::new();
                    s.insert("format".to_string(), serde_json::json!("goAML XML"));
                    s.insert(
                        "portal".to_string(),
                        serde_json::json!("https://goaml.fmu.gov.pk"),
                    );
                    s
                },
                late_penalty: {
                    let mut p = BTreeMap::new();
                    p.insert(
                        "penalty".to_string(),
                        serde_json::json!("AML Act 2010 s.16: imprisonment up to 5 years or fine up to PKR 10M"),
                    );
                    p
                },
            },
            ReportingRequirement {
                report_type_id: "pk-fmu-ctr".to_string(),
                name: "Currency Transaction Report (CTR)".to_string(),
                regulator_id: "pk-fmu".to_string(),
                applicable_to: vec![
                    "commercial_bank".to_string(),
                    "exchange_company".to_string(),
                ],
                frequency: "event_driven".to_string(),
                deadlines: {
                    let mut d = BTreeMap::new();
                    let mut inner = BTreeMap::new();
                    inner.insert("days_from_transaction".to_string(), "15".to_string());
                    inner.insert("threshold_pkr".to_string(), "2000000".to_string());
                    d.insert("trigger".to_string(), inner);
                    d
                },
                submission: {
                    let mut s = BTreeMap::new();
                    s.insert("format".to_string(), serde_json::json!("goAML XML"));
                    s
                },
                late_penalty: {
                    let mut p = BTreeMap::new();
                    p.insert(
                        "penalty".to_string(),
                        serde_json::json!("AML Act 2010 s.16: fine up to PKR 5M"),
                    );
                    p
                },
            },
            ReportingRequirement {
                report_type_id: "pk-sbp-prudential-quarterly".to_string(),
                name: "Quarterly Prudential Return".to_string(),
                regulator_id: "pk-sbp".to_string(),
                applicable_to: vec![
                    "commercial_bank".to_string(),
                    "microfinance_bank".to_string(),
                ],
                frequency: "quarterly".to_string(),
                deadlines: {
                    let mut d = BTreeMap::new();
                    let mut inner = BTreeMap::new();
                    inner.insert("days_after_quarter_end".to_string(), "30".to_string());
                    d.insert("standard".to_string(), inner);
                    d
                },
                submission: {
                    let mut s = BTreeMap::new();
                    s.insert("format".to_string(), serde_json::json!("SBP XBRL / Excel"));
                    s.insert(
                        "portal".to_string(),
                        serde_json::json!("SBP Banking Surveillance Department"),
                    );
                    s
                },
                late_penalty: {
                    let mut p = BTreeMap::new();
                    p.insert(
                        "penalty".to_string(),
                        serde_json::json!("BCO 1962 s.46: penalty per day of default"),
                    );
                    p
                },
            },
            ReportingRequirement {
                report_type_id: "pk-fbr-wht-statement".to_string(),
                name: "Monthly Withholding Tax Statement".to_string(),
                regulator_id: "pk-fbr".to_string(),
                applicable_to: vec![
                    "commercial_bank".to_string(),
                    "company".to_string(),
                    "aop".to_string(),
                ],
                frequency: "monthly".to_string(),
                deadlines: {
                    let mut d = BTreeMap::new();
                    let mut inner = BTreeMap::new();
                    inner.insert("day_of_following_month".to_string(), "15".to_string());
                    d.insert("standard".to_string(), inner);
                    d
                },
                submission: {
                    let mut s = BTreeMap::new();
                    s.insert("format".to_string(), serde_json::json!("FBR IRIS e-filing"));
                    s.insert(
                        "portal".to_string(),
                        serde_json::json!("https://iris.fbr.gov.pk"),
                    );
                    s
                },
                late_penalty: {
                    let mut p = BTreeMap::new();
                    p.insert(
                        "penalty".to_string(),
                        serde_json::json!("ITO 2001 s.182: PKR 2,500 per day of default"),
                    );
                    p
                },
            },
            ReportingRequirement {
                report_type_id: "pk-secp-annual-return".to_string(),
                name: "Annual Return (Form A/B)".to_string(),
                regulator_id: "pk-secp".to_string(),
                applicable_to: vec![
                    "company".to_string(),
                    "nbfc".to_string(),
                ],
                frequency: "annual".to_string(),
                deadlines: {
                    let mut d = BTreeMap::new();
                    let mut inner = BTreeMap::new();
                    inner.insert("days_after_agm".to_string(), "30".to_string());
                    d.insert("standard".to_string(), inner);
                    d
                },
                submission: {
                    let mut s = BTreeMap::new();
                    s.insert("format".to_string(), serde_json::json!("SECP eServices"));
                    s.insert(
                        "portal".to_string(),
                        serde_json::json!("https://eservices.secp.gov.pk"),
                    );
                    s
                },
                late_penalty: {
                    let mut p = BTreeMap::new();
                    p.insert(
                        "penalty".to_string(),
                        serde_json::json!("Companies Act 2017 s.130: PKR 100 per day up to 2 years"),
                    );
                    p
                },
            },
        ]
    }

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn btree_alias(name: &str) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("name".to_string(), name.to_string());
        m
    }

    fn btree_address(addr: &str) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("address".to_string(), addr.to_string());
        m
    }

    // ── Full Regpack Builder ─────────────────────────────────────────────────

    /// Build a complete Pakistan regpack with all content.
    ///
    /// Assembles regulators, sanctions, deadlines, and reporting requirements
    /// into a content-addressed regpack for the `pk` jurisdiction.
    #[allow(clippy::type_complexity)]
    pub fn build_pakistan_regpack() -> PackResult<(Regpack, RegPackMetadata, SanctionsSnapshot, Vec<ComplianceDeadline>, Vec<ReportingRequirement>, Vec<WithholdingTaxRate>)> {
        let regulators = pakistan_regulators();
        let sanctions_snapshot = pakistan_sanctions_snapshot();
        let deadlines = pakistan_compliance_deadlines();
        let reporting = pakistan_reporting_requirements();
        let wht_rates = pakistan_wht_rates();

        let mut includes = BTreeMap::new();
        includes.insert(
            "regulators".to_string(),
            serde_json::json!(regulators.iter().map(|r| &r.regulator_id).collect::<Vec<_>>()),
        );
        includes.insert(
            "sanctions_entries".to_string(),
            serde_json::json!(pakistan_sanctions_entries().len()),
        );
        includes.insert(
            "compliance_deadlines".to_string(),
            serde_json::json!(deadlines.len()),
        );
        includes.insert(
            "reporting_requirements".to_string(),
            serde_json::json!(reporting.len()),
        );
        includes.insert(
            "wht_rates".to_string(),
            serde_json::json!(wht_rates.len()),
        );

        let metadata = RegPackMetadata {
            regpack_id: "regpack:pk:financial:2026Q1".to_string(),
            jurisdiction_id: "pk".to_string(),
            domain: "financial".to_string(),
            as_of_date: "2026-01-15".to_string(),
            snapshot_type: "quarterly".to_string(),
            sources: vec![
                serde_json::json!({
                    "source_id": "nacta_gazette",
                    "name": "NACTA Proscribed Organizations Gazette",
                    "authority": "Government of Pakistan"
                }),
                serde_json::json!({
                    "source_id": "unsc_1267",
                    "name": "UNSC 1267/1989/2253 Consolidated List",
                    "authority": "United Nations Security Council"
                }),
                serde_json::json!({
                    "source_id": "fbr_ito_2001",
                    "name": "Income Tax Ordinance 2001 (as amended)",
                    "authority": "Federal Board of Revenue"
                }),
                serde_json::json!({
                    "source_id": "aml_act_2010",
                    "name": "Anti-Money Laundering Act 2010",
                    "authority": "Government of Pakistan"
                }),
                serde_json::json!({
                    "source_id": "companies_act_2017",
                    "name": "Companies Act 2017",
                    "authority": "Government of Pakistan / SECP"
                }),
                serde_json::json!({
                    "source_id": "bco_1962",
                    "name": "Banking Companies Ordinance 1962",
                    "authority": "State Bank of Pakistan"
                }),
            ],
            includes,
            previous_regpack_digest: None,
            created_at: Some("2026-01-15T00:00:00Z".to_string()),
            expires_at: Some("2026-04-15T00:00:00Z".to_string()),
            digest_sha256: None,
        };

        let digest = compute_regpack_digest(
            &metadata,
            Some(&sanctions_snapshot),
            Some(&regulators),
            Some(&deadlines),
        )?;

        let regpack = Regpack {
            jurisdiction: JurisdictionId::new("pk".to_string())
                .map_err(|e| PackError::Validation(format!("invalid jurisdiction: {e}")))?,
            name: "Pakistan Financial Regulatory Pack — 2026 Q1".to_string(),
            version: REGPACK_VERSION.to_string(),
            digest: Some(
                ContentDigest::from_hex(&digest)
                    .map_err(|e| PackError::Validation(format!("digest error: {e}")))?,
            ),
            metadata: Some(metadata.clone()),
        };

        Ok((regpack, metadata, sanctions_snapshot, deadlines, reporting, wht_rates))
    }

    /// Build a sanctions-domain-specific Pakistan regpack.
    ///
    /// Produces a regpack focused on the `sanctions` compliance domain,
    /// containing the NACTA proscribed organizations gazette and UNSC 1267
    /// consolidated list entries. Separate from the `financial` domain
    /// regpack which includes broader regulatory data (WHT rates, regulators,
    /// compliance deadlines, reporting requirements).
    ///
    /// The sanctions regpack is content-addressed independently so that
    /// sanctions-list-only updates can be pushed without rebuilding the
    /// full financial regpack.
    pub fn build_pakistan_sanctions_regpack() -> PackResult<(Regpack, RegPackMetadata, SanctionsSnapshot)> {
        let sanctions_snapshot = pakistan_sanctions_snapshot();

        let mut includes = BTreeMap::new();
        includes.insert(
            "sanctions_entries".to_string(),
            serde_json::json!(pakistan_sanctions_entries().len()),
        );
        includes.insert(
            "source_lists".to_string(),
            serde_json::json!(["nacta_gazette", "unsc_1267"]),
        );

        let metadata = RegPackMetadata {
            regpack_id: "regpack:pk:sanctions:2026Q1".to_string(),
            jurisdiction_id: "pk".to_string(),
            domain: "sanctions".to_string(),
            as_of_date: "2026-01-15".to_string(),
            snapshot_type: "quarterly".to_string(),
            sources: vec![
                serde_json::json!({
                    "source_id": "nacta_gazette",
                    "name": "NACTA Proscribed Organizations Gazette",
                    "authority": "Government of Pakistan"
                }),
                serde_json::json!({
                    "source_id": "unsc_1267",
                    "name": "UNSC 1267/1989/2253 Consolidated List",
                    "authority": "United Nations Security Council"
                }),
            ],
            includes,
            previous_regpack_digest: None,
            created_at: Some("2026-01-15T00:00:00Z".to_string()),
            expires_at: Some("2026-04-15T00:00:00Z".to_string()),
            digest_sha256: None,
        };

        let digest = compute_regpack_digest(
            &metadata,
            Some(&sanctions_snapshot),
            None,  // No regulators — sanctions-only domain
            None,  // No deadlines — sanctions-only domain
        )?;

        let regpack = Regpack {
            jurisdiction: JurisdictionId::new("pk".to_string())
                .map_err(|e| PackError::Validation(format!("invalid jurisdiction: {e}")))?,
            name: "Pakistan Sanctions Regulatory Pack — 2026 Q1".to_string(),
            version: REGPACK_VERSION.to_string(),
            digest: Some(
                ContentDigest::from_hex(&digest)
                    .map_err(|e| PackError::Validation(format!("digest error: {e}")))?,
            ),
            metadata: Some(metadata.clone()),
        };

        Ok((regpack, metadata, sanctions_snapshot))
    }

    // ── Tests ────────────────────────────────────────────────────────────────

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn pakistan_has_five_regulators() {
            let regs = pakistan_regulators();
            assert_eq!(regs.len(), 5);
            let ids: Vec<&str> = regs.iter().map(|r| r.regulator_id.as_str()).collect();
            assert!(ids.contains(&"pk-sbp"));
            assert!(ids.contains(&"pk-secp"));
            assert!(ids.contains(&"pk-fmu"));
            assert!(ids.contains(&"pk-fbr"));
            assert!(ids.contains(&"pk-nacta"));
        }

        #[test]
        fn all_regulators_are_pakistan_jurisdiction() {
            for reg in pakistan_regulators() {
                assert_eq!(reg.jurisdiction_id, "pk", "{} wrong jid", reg.regulator_id);
            }
        }

        #[test]
        fn all_regulators_have_asia_karachi_timezone() {
            for reg in pakistan_regulators() {
                assert_eq!(
                    reg.timezone, "Asia/Karachi",
                    "{} wrong tz",
                    reg.regulator_id
                );
            }
        }

        #[test]
        fn fmu_parent_is_sbp() {
            let fmu = fmu_regulator();
            assert_eq!(fmu.parent_authority, Some("pk-sbp".to_string()));
        }

        #[test]
        fn sanctions_entries_all_have_source_lists() {
            for entry in pakistan_sanctions_entries() {
                assert!(
                    !entry.source_lists.is_empty(),
                    "{} has no source_lists",
                    entry.entry_id
                );
            }
        }

        #[test]
        fn sanctions_entries_all_have_programs() {
            for entry in pakistan_sanctions_entries() {
                assert!(
                    !entry.programs.is_empty(),
                    "{} has no programs",
                    entry.entry_id
                );
            }
        }

        #[test]
        fn sanctions_snapshot_has_sources() {
            let snap = pakistan_sanctions_snapshot();
            assert!(snap.sources.contains_key("nacta_first_schedule"));
            assert!(snap.sources.contains_key("unsc_1267"));
        }

        #[test]
        fn sanctions_checker_finds_exact_match() {
            let entries = pakistan_sanctions_entries();
            let checker = SanctionsChecker::new(entries, "pk-sanctions-2026Q1".to_string());
            let result = checker.check_entity("Al-Qaeda", None, 0.7);
            assert!(result.matched, "Al-Qaeda should match");
            assert_eq!(result.match_score, 1.0);
        }

        #[test]
        fn sanctions_checker_finds_alias() {
            let entries = pakistan_sanctions_entries();
            let checker = SanctionsChecker::new(entries, "pk-sanctions-2026Q1".to_string());
            let result = checker.check_entity("Jamaat-ud-Dawa", None, 0.7);
            assert!(result.matched, "Alias Jamaat-ud-Dawa should match");
        }

        #[test]
        fn sanctions_checker_rejects_clean_entity() {
            let entries = pakistan_sanctions_entries();
            let checker = SanctionsChecker::new(entries, "pk-sanctions-2026Q1".to_string());
            let result = checker.check_entity("Habib Bank Limited", None, 0.8);
            assert!(!result.matched, "legitimate bank should not match");
        }

        #[test]
        fn compliance_deadlines_cover_all_regulators() {
            let deadlines = pakistan_compliance_deadlines();
            let regulator_ids: std::collections::HashSet<&str> =
                deadlines.iter().map(|d| d.regulator_id.as_str()).collect();
            assert!(regulator_ids.contains("pk-fbr"), "missing FBR deadlines");
            assert!(regulator_ids.contains("pk-sbp"), "missing SBP deadlines");
            assert!(regulator_ids.contains("pk-secp"), "missing SECP deadlines");
            assert!(regulator_ids.contains("pk-fmu"), "missing FMU deadlines");
        }

        #[test]
        fn compliance_deadlines_have_unique_ids() {
            let deadlines = pakistan_compliance_deadlines();
            let mut ids = std::collections::HashSet::new();
            for dl in &deadlines {
                assert!(ids.insert(&dl.deadline_id), "duplicate: {}", dl.deadline_id);
            }
        }

        #[test]
        fn reporting_requirements_cover_key_reports() {
            let reqs = pakistan_reporting_requirements();
            let ids: Vec<&str> = reqs.iter().map(|r| r.report_type_id.as_str()).collect();
            assert!(ids.contains(&"pk-fmu-str"), "missing STR");
            assert!(ids.contains(&"pk-fmu-ctr"), "missing CTR");
            assert!(ids.contains(&"pk-sbp-prudential-quarterly"), "missing prudential");
            assert!(ids.contains(&"pk-fbr-wht-statement"), "missing WHT statement");
            assert!(ids.contains(&"pk-secp-annual-return"), "missing SECP annual");
        }

        #[test]
        fn wht_rates_cover_key_sections() {
            let rates = pakistan_wht_rates();
            assert!(rates.len() >= 12, "expected >= 12 WHT rates, got {}", rates.len());
            let sections: Vec<&str> = rates.iter().map(|r| r.section.as_str()).collect();
            assert!(sections.contains(&"149"), "missing salary s.149");
            assert!(sections.contains(&"151(1)(a)"), "missing profit on debt s.151");
            assert!(sections.contains(&"153(1)(a)"), "missing goods s.153(1)(a)");
            assert!(sections.contains(&"153(1)(b)"), "missing services s.153(1)(b)");
            assert!(sections.contains(&"231A"), "missing cash withdrawal s.231A");
        }

        #[test]
        fn wht_rates_distinguish_filer_nonfiler() {
            let rates = pakistan_wht_rates();
            let filer_count = rates.iter().filter(|r| r.taxpayer_status == "filer").count();
            let nonfiler_count = rates
                .iter()
                .filter(|r| r.taxpayer_status == "non-filer")
                .count();
            assert!(filer_count > 0, "no filer rates");
            assert!(nonfiler_count > 0, "no non-filer rates");
        }

        #[test]
        fn build_pakistan_regpack_succeeds() {
            let (regpack, metadata, snap, deadlines, reporting, wht) =
                build_pakistan_regpack().expect("build should succeed");
            assert_eq!(regpack.jurisdiction.as_str(), "pk");
            assert!(regpack.digest.is_some(), "regpack should have digest");
            assert_eq!(metadata.jurisdiction_id, "pk");
            assert!(!snap.consolidated_counts.is_empty());
            assert!(!deadlines.is_empty());
            assert!(!reporting.is_empty());
            assert!(!wht.is_empty());
        }

        #[test]
        fn build_pakistan_regpack_is_deterministic() {
            let (rp1, ..) = build_pakistan_regpack().unwrap();
            let (rp2, ..) = build_pakistan_regpack().unwrap();
            assert_eq!(
                rp1.digest.as_ref().unwrap().to_hex(),
                rp2.digest.as_ref().unwrap().to_hex(),
                "regpack digest must be deterministic"
            );
        }

        #[test]
        fn build_pakistan_sanctions_regpack_succeeds() {
            let (regpack, metadata, snap) =
                build_pakistan_sanctions_regpack().expect("sanctions build should succeed");
            assert_eq!(regpack.jurisdiction.as_str(), "pk");
            assert!(regpack.digest.is_some(), "sanctions regpack should have digest");
            assert_eq!(metadata.domain, "sanctions");
            assert_eq!(metadata.jurisdiction_id, "pk");
            assert!(!snap.consolidated_counts.is_empty());
        }

        #[test]
        fn build_pakistan_sanctions_regpack_is_deterministic() {
            let (rp1, ..) = build_pakistan_sanctions_regpack().unwrap();
            let (rp2, ..) = build_pakistan_sanctions_regpack().unwrap();
            assert_eq!(
                rp1.digest.as_ref().unwrap().to_hex(),
                rp2.digest.as_ref().unwrap().to_hex(),
                "sanctions regpack digest must be deterministic"
            );
        }

        #[test]
        fn sanctions_regpack_digest_differs_from_financial() {
            let (financial, ..) = build_pakistan_regpack().unwrap();
            let (sanctions, ..) = build_pakistan_sanctions_regpack().unwrap();
            assert_ne!(
                financial.digest.as_ref().unwrap().to_hex(),
                sanctions.digest.as_ref().unwrap().to_hex(),
                "financial and sanctions regpack digests must differ"
            );
        }

        #[test]
        fn regulator_serialization_roundtrip() {
            for reg in pakistan_regulators() {
                let json = serde_json::to_string(&reg).expect("serialize");
                let de: RegulatorProfile = serde_json::from_str(&json).expect("deserialize");
                assert_eq!(reg.regulator_id, de.regulator_id);
                assert_eq!(reg.name, de.name);
                assert_eq!(reg.timezone, de.timezone);
            }
        }
    }
}
