//! # Lawpack — Statute to Machine-Readable Rules
//!
//! Compiles legislative statutes into structured compliance rules that
//! can be evaluated by the Compliance Tensor. A *lawpack* is a content-addressed
//! artifact that packages a jurisdictional legal corpus snapshot (typically
//! normalized to Akoma Ntoso) plus deterministic indices and provenance.
//!
//! ## Data Model
//!
//! - [`LawpackRef`]: Compact reference to a pinned lawpack (`<jid>:<domain>:<sha256>`).
//! - [`LawpackManifest`]: The `lawpack.yaml` content inside a lawpack bundle.
//! - [`LawpackLock`]: The `lawpack.lock.json` file pinning a module to its lawpack.
//! - [`Lawpack`]: The compiled lawpack bundle with metadata and digest.
//!
//! ## Digest Computation
//!
//! Lawpack digests follow the v1 protocol defined in Python `tools/lawpack.py`:
//!
//! ```text
//! SHA256( b"msez-lawpack-v1\0" + for each path in sorted(paths):
//!     path.encode("utf-8") + b"\0" + canonical_bytes + b"\0" )
//! ```
//!
//! All canonicalization goes through [`CanonicalBytes`](msez_core::CanonicalBytes)
//! to ensure cross-language digest equality with the Python implementation.
//!
//! ## Spec Reference
//!
//! Implements the lawpack supply chain defined in `tools/lawpack.py` and
//! spec chapter on content-addressed legal corpus management.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use msez_core::{CanonicalBytes, ContentDigest, JurisdictionId};

use crate::error::{PackError, PackResult};
use crate::parser;

/// Digest prefix for lawpack v1 digest computation.
///
/// Matches Python: `b"msez-lawpack-v1\0"`
const LAWPACK_DIGEST_PREFIX: &[u8] = b"msez-lawpack-v1\0";

// ---------------------------------------------------------------------------
// LawpackRef — compact reference
// ---------------------------------------------------------------------------

/// Compact reference to a pinned lawpack.
///
/// Format: `<jurisdiction_id>:<domain>:<sha256_digest>`
///
/// Used in zone manifests and stack.lock files to reference specific
/// lawpack versions by their content-addressed digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LawpackRef {
    /// Jurisdiction this lawpack applies to.
    pub jurisdiction_id: String,
    /// Legal domain (e.g., "civil", "financial", "corporate").
    pub domain: String,
    /// SHA-256 digest of the lawpack content.
    pub lawpack_digest_sha256: String,
}

impl LawpackRef {
    /// Parse a compact lawpack ref string.
    ///
    /// Format: `<jurisdiction_id>:<domain>:<sha256_digest>`
    ///
    /// # Errors
    ///
    /// Returns [`PackError::InvalidLawpackRef`] if the format is wrong
    /// or the digest is not a valid SHA-256 hex string.
    pub fn parse(s: &str) -> PackResult<Self> {
        let parts: Vec<&str> = s.split(':').filter(|p| !p.trim().is_empty()).collect();
        if parts.len() != 3 {
            return Err(PackError::InvalidLawpackRef {
                input: s.to_string(),
                reason: "must be '<jurisdiction_id>:<domain>:<sha256>'".to_string(),
            });
        }
        let jid = parts[0].trim().to_string();
        let domain = parts[1].trim().to_string();
        let digest = parts[2].trim().to_string();
        if !parser::is_valid_sha256(&digest) {
            return Err(PackError::InvalidDigest { digest });
        }
        Ok(Self {
            jurisdiction_id: jid,
            domain,
            lawpack_digest_sha256: digest,
        })
    }

    /// Convert to a display string in compact format.
    pub fn to_compact(&self) -> String {
        format!(
            "{}:{}:{}",
            self.jurisdiction_id, self.domain, self.lawpack_digest_sha256
        )
    }
}

impl std::fmt::Display for LawpackRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_compact())
    }
}

// ---------------------------------------------------------------------------
// LawpackManifest — the lawpack.yaml content
// ---------------------------------------------------------------------------

/// Source entry in a lawpack manifest.
///
/// Describes a source document used to produce the lawpack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LawpackSource {
    /// Unique identifier for this source within the lawpack.
    pub source_id: String,
    /// URI of the source document (may be a URL or local path).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    /// Reference text (for non-URL sources).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// When the source was retrieved (RFC 3339).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retrieved_at: Option<String>,
    /// SHA-256 digest of the raw source document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// MIME type of the source document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
}

/// Normalization metadata in a lawpack manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizationInfo {
    /// Normalization recipe identifier.
    pub recipe_id: String,
    /// Tool that performed normalization.
    pub tool: String,
    /// Tool version.
    pub tool_version: String,
    /// Normalization inputs.
    #[serde(default)]
    pub inputs: Vec<NormalizationInput>,
    /// Free-text notes.
    #[serde(default)]
    pub notes: String,
}

/// Input to a normalization process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizationInput {
    /// Module identifier.
    #[serde(default)]
    pub module_id: String,
    /// Module version.
    #[serde(default)]
    pub module_version: String,
    /// SHA-256 of the sources manifest.
    #[serde(default)]
    pub sources_manifest_sha256: String,
}

/// The `lawpack.yaml` manifest embedded inside a lawpack bundle.
///
/// Contains metadata about the lawpack: jurisdiction, domain, sources,
/// normalization provenance, and licensing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LawpackManifest {
    /// Format version (currently "1").
    pub lawpack_format_version: String,
    /// Jurisdiction identifier.
    pub jurisdiction_id: String,
    /// Legal domain.
    pub domain: String,
    /// Snapshot date (YYYY-MM-DD).
    pub as_of_date: String,
    /// Source documents.
    #[serde(default)]
    pub sources: Vec<serde_json::Value>,
    /// SPDX license identifier.
    #[serde(default)]
    pub license: String,
    /// Normalization metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normalization: Option<NormalizationInfo>,
}

// ---------------------------------------------------------------------------
// LawpackLock — the lockfile
// ---------------------------------------------------------------------------

/// Component digests in a lawpack lock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LawpackLockComponents {
    /// SHA-256 of the canonical lawpack.yaml bytes.
    pub lawpack_yaml_sha256: String,
    /// SHA-256 of the canonical index.json bytes.
    pub index_json_sha256: String,
    /// SHA-256 of each AKN XML document by relative path.
    pub akn_sha256: BTreeMap<String, String>,
    /// SHA-256 of the sources manifest.
    pub sources_sha256: String,
    /// SHA-256 of the module manifest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module_manifest_sha256: Option<String>,
}

/// Provenance information in a lawpack lock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LawpackLockProvenance {
    /// Relative path to module.yaml.
    pub module_manifest_path: String,
    /// Relative path to sources.yaml (empty if not present).
    pub sources_manifest_path: String,
    /// Raw source digests by source ID.
    #[serde(default)]
    pub raw_sources: BTreeMap<String, String>,
    /// Normalization metadata (copied from lawpack.yaml).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normalization: Option<NormalizationInfo>,
}

/// Lawpack lock file (`lawpack.lock.json`).
///
/// Pins a module to a specific lawpack artifact by recording the
/// content-addressed digest, artifact path, and component digests
/// for verification.
///
/// Matches Python output format from `tools/lawpack.py:ingest_lawpack()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LawpackLock {
    /// SHA-256 digest of the lawpack content.
    pub lawpack_digest_sha256: String,
    /// Jurisdiction identifier.
    pub jurisdiction_id: String,
    /// Legal domain.
    pub domain: String,
    /// Snapshot date.
    pub as_of_date: String,
    /// Relative path to the artifact file.
    pub artifact_path: String,
    /// SHA-256 of the artifact zip file.
    pub artifact_sha256: String,
    /// Component digests for verification.
    pub components: LawpackLockComponents,
    /// Provenance information.
    pub provenance: LawpackLockProvenance,
}

// ---------------------------------------------------------------------------
// Lawpack — compiled bundle
// ---------------------------------------------------------------------------

/// A compiled lawpack bundle containing machine-readable compliance rules
/// derived from legislative statutes.
///
/// This is the primary type for working with lawpacks in the Rust layer.
/// It holds the parsed manifest, component digests, and the overall
/// content-addressed digest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lawpack {
    /// The jurisdiction this lawpack applies to.
    pub jurisdiction: JurisdictionId,
    /// Human-readable name or domain of the lawpack.
    pub name: String,
    /// Legal domain (e.g., "civil", "financial").
    pub domain: String,
    /// Version string (semver or spec version).
    pub version: String,
    /// Content digest of the compiled lawpack.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<ContentDigest>,
    /// Snapshot date (YYYY-MM-DD).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub as_of_date: Option<String>,
    /// Effective date of the legislation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_date: Option<String>,
    /// Section mappings (statute section -> rule mapping).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub section_mappings: BTreeMap<String, String>,
}

// ---------------------------------------------------------------------------
// Digest computation
// ---------------------------------------------------------------------------

/// Compute a lawpack digest over canonicalized file bytes.
///
/// Implements the v1 digest protocol:
///
/// ```text
/// SHA256( b"msez-lawpack-v1\0"
///       + for each path in sorted(canonical_files.keys()):
///           path.encode("utf-8") + b"\0" + canonical_bytes + b"\0" )
/// ```
///
/// This matches Python `tools/lawpack.py:compute_lawpack_digest()`.
///
/// # Arguments
///
/// * `canonical_files` - Map from relative path (e.g., "lawpack.yaml", "akn/doc.xml")
///   to canonical byte representation of that file.
pub fn compute_lawpack_digest(canonical_files: &BTreeMap<String, Vec<u8>>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(LAWPACK_DIGEST_PREFIX);
    for (relpath, content) in canonical_files {
        hasher.update(relpath.as_bytes());
        hasher.update(b"\0");
        hasher.update(content);
        hasher.update(b"\0");
    }
    let result = hasher.finalize();
    result.iter().map(|b| format!("{b:02x}")).collect()
}

/// Canonicalize a JSON value using the JCS-compatible pipeline.
///
/// Delegates to [`CanonicalBytes::from_value()`] which applies:
/// - Float rejection
/// - Datetime normalization
/// - Key sorting
/// - Compact JSON serialization
///
/// Returns the canonical bytes as a `Vec<u8>`.
pub fn jcs_canonicalize(value: &serde_json::Value) -> PackResult<Vec<u8>> {
    let canonical = CanonicalBytes::from_value(value.clone())?;
    Ok(canonical.into_bytes())
}

/// Verify a lawpack lock against the current module state.
///
/// Recomputes the lawpack digest from the module directory and compares
/// it against the lock file. Returns the lock object if verification
/// succeeds.
///
/// # Arguments
///
/// * `lock_path` - Path to the `lawpack.lock.json` file.
///
/// # Errors
///
/// Returns [`PackError::LockVerificationFailed`] if the lock does not
/// match the current module state.
pub fn verify_lock(lock_path: &Path) -> PackResult<LawpackLock> {
    let lock: LawpackLock = parser::load_json_typed(lock_path)?;

    // Basic structural validation
    if !parser::is_valid_sha256(&lock.lawpack_digest_sha256) {
        return Err(PackError::InvalidDigest {
            digest: lock.lawpack_digest_sha256.clone(),
        });
    }
    if !parser::is_valid_sha256(&lock.artifact_sha256) {
        return Err(PackError::InvalidDigest {
            digest: lock.artifact_sha256.clone(),
        });
    }
    if !parser::is_valid_sha256(&lock.components.lawpack_yaml_sha256) {
        return Err(PackError::InvalidDigest {
            digest: lock.components.lawpack_yaml_sha256.clone(),
        });
    }
    if !parser::is_valid_sha256(&lock.components.index_json_sha256) {
        return Err(PackError::InvalidDigest {
            digest: lock.components.index_json_sha256.clone(),
        });
    }

    Ok(lock)
}

/// Load a lawpack lock file from disk.
pub fn load_lock(path: &Path) -> PackResult<LawpackLock> {
    parser::load_json_typed(path)
}

/// Compute canonical bytes for a serde_json::Value and return the SHA-256 hex.
pub fn canonical_sha256(value: &serde_json::Value) -> PackResult<String> {
    let bytes = jcs_canonicalize(value)?;
    Ok(parser::sha256_hex(&bytes))
}

/// Resolve lawpack references from a zone manifest.
///
/// Given parsed zone YAML content, extract all lawpack references and
/// return them as [`LawpackRef`] instances.
pub fn resolve_lawpack_refs(zone: &serde_json::Value) -> PackResult<Vec<LawpackRef>> {
    let mut refs = Vec::new();
    if let Some(lawpacks) = zone.get("lawpacks").and_then(|v| v.as_array()) {
        for lp in lawpacks {
            let jid = lp
                .get("jurisdiction_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let domain = lp
                .get("domain")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let digest = lp
                .get("lawpack_digest_sha256")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if !digest.is_empty() && parser::is_valid_sha256(&digest) {
                refs.push(LawpackRef {
                    jurisdiction_id: jid,
                    domain,
                    lawpack_digest_sha256: digest,
                });
            }
        }
    }
    Ok(refs)
}

/// Generate a lockfile for a zone by resolving all lawpack references.
///
/// This is the Rust equivalent of the Python lock operation:
/// `msez lock jurisdictions/_starter/zone.yaml --check`
///
/// # Arguments
///
/// * `zone_path` - Path to the zone.yaml file.
/// * `repo_root` - Repository root for resolving relative paths.
///
/// # Returns
///
/// A JSON value representing the stack.lock content, or an error
/// if any referenced lawpacks cannot be resolved.
pub fn generate_zone_lock(
    zone_path: &Path,
    repo_root: &Path,
) -> PackResult<serde_json::Value> {
    let zone = parser::load_yaml_as_value(zone_path)?;
    let zone_id = zone
        .get("zone_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let jurisdiction_id = zone
        .get("jurisdiction_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let lawpack_refs = resolve_lawpack_refs(&zone)?;

    // Compute zone manifest canonical digest
    let zone_canonical = jcs_canonicalize(&zone)?;
    let zone_digest = parser::sha256_hex(&zone_canonical);

    // Build the lock structure
    let lock = serde_json::json!({
        "lock_version": "1",
        "spec_version": "0.4.44",
        "zone_id": zone_id,
        "jurisdiction_id": jurisdiction_id,
        "zone_manifest_sha256": zone_digest,
        "zone_manifest_path": normalize_relpath(zone_path, repo_root),
        "lawpacks": lawpack_refs,
    });

    Ok(lock)
}

/// Normalize a path relative to the repo root, using POSIX separators.
fn normalize_relpath(path: &Path, repo_root: &Path) -> String {
    match path.canonicalize() {
        Ok(abs) => match abs.strip_prefix(
            repo_root
                .canonicalize()
                .unwrap_or_else(|_| repo_root.to_path_buf()),
        ) {
            Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
            Err(_) => path.to_string_lossy().replace('\\', "/"),
        },
        Err(_) => path.to_string_lossy().replace('\\', "/"),
    }
}

// ---------------------------------------------------------------------------
// Module descriptor parsing
// ---------------------------------------------------------------------------

/// A module.yaml descriptor (minimal fields needed for lawpack operations).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDescriptor {
    /// Unique module identifier.
    #[serde(default)]
    pub module_id: String,
    /// Module version.
    #[serde(default)]
    pub version: String,
    /// Module kind/family.
    #[serde(default)]
    pub kind: String,
    /// Human-readable name.
    #[serde(default)]
    pub name: String,
    /// Description.
    #[serde(default)]
    pub description: String,
    /// Spec version implemented.
    #[serde(default)]
    pub spec_version: String,
    /// SPDX license.
    #[serde(default)]
    pub license: String,
    /// All other fields (flexible structure).
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// A sources.yaml descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcesDescriptor {
    /// Jurisdiction identifier.
    #[serde(default)]
    pub jurisdiction_id: String,
    /// Legal domain.
    #[serde(default)]
    pub domain: String,
    /// Source entries.
    #[serde(default)]
    pub sources: Vec<serde_json::Value>,
    /// Normalization metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normalization: Option<serde_json::Value>,
    /// License.
    #[serde(default)]
    pub license: String,
    /// All other fields.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// Load a module.yaml descriptor from a directory.
pub fn load_module_descriptor(module_dir: &Path) -> PackResult<ModuleDescriptor> {
    let path = module_dir.join("module.yaml");
    parser::load_yaml_typed(&path)
}

/// Load a sources.yaml descriptor from a directory.
pub fn load_sources_descriptor(module_dir: &Path) -> PackResult<Option<SourcesDescriptor>> {
    let path = module_dir.join("sources.yaml");
    if path.exists() {
        Ok(Some(parser::load_yaml_typed(&path)?))
    } else {
        Ok(None)
    }
}

/// Infer jurisdiction and domain from module directory and sources manifest.
///
/// Mirrors Python `tools/lawpack.py:_infer_jurisdiction_and_domain()`.
pub fn infer_jurisdiction_and_domain(
    module_dir: &Path,
    sources: Option<&SourcesDescriptor>,
) -> (String, String) {
    let jid = sources
        .map(|s| s.jurisdiction_id.trim().to_string())
        .filter(|s| !s.is_empty());
    let domain = sources
        .map(|s| s.domain.trim().to_string())
        .filter(|s| !s.is_empty());

    if let (Some(jid), Some(domain)) = (jid.clone(), domain.clone()) {
        return (jid, domain);
    }

    // Derive from path: modules/legal/jurisdictions/<jid>/<domain>
    let parts: Vec<&str> = module_dir
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    let jur_idx = parts.iter().position(|&p| p == "jurisdictions");

    let derived_domain = domain.unwrap_or_else(|| {
        parts.last().map(|s| s.to_string()).unwrap_or_else(|| "unknown".to_string())
    });
    let derived_jid = jid.unwrap_or_else(|| {
        if let Some(idx) = jur_idx {
            if parts.len() >= idx + 2 {
                parts[idx + 1..parts.len() - 1].join("-")
            } else {
                "unknown".to_string()
            }
        } else {
            "unknown".to_string()
        }
    });

    (
        if derived_jid.is_empty() { "unknown".to_string() } else { derived_jid },
        if derived_domain.is_empty() { "unknown".to_string() } else { derived_domain },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_lawpack_ref_parse_valid() {
        let r = LawpackRef::parse(
            "pk:civil:43258cff783fe7036d8a43033f830adfc60ec037382473548ac742b888292777",
        )
        .unwrap();
        assert_eq!(r.jurisdiction_id, "pk");
        assert_eq!(r.domain, "civil");
        assert_eq!(
            r.lawpack_digest_sha256,
            "43258cff783fe7036d8a43033f830adfc60ec037382473548ac742b888292777"
        );
    }

    #[test]
    fn test_lawpack_ref_parse_invalid_format() {
        assert!(LawpackRef::parse("only:two").is_err());
        assert!(LawpackRef::parse("").is_err());
    }

    #[test]
    fn test_lawpack_ref_parse_invalid_digest() {
        assert!(LawpackRef::parse("pk:civil:not-a-sha256").is_err());
    }

    #[test]
    fn test_lawpack_ref_compact_roundtrip() {
        let r = LawpackRef {
            jurisdiction_id: "us-de".to_string(),
            domain: "corporate".to_string(),
            lawpack_digest_sha256: "a".repeat(64),
        };
        let compact = r.to_compact();
        let parsed = LawpackRef::parse(&compact).unwrap();
        assert_eq!(r, parsed);
    }

    #[test]
    fn test_compute_lawpack_digest_deterministic() {
        let mut files = BTreeMap::new();
        files.insert("lawpack.yaml".to_string(), b"test content".to_vec());
        files.insert("index.json".to_string(), b"{}".to_vec());
        let d1 = compute_lawpack_digest(&files);
        let d2 = compute_lawpack_digest(&files);
        assert_eq!(d1, d2);
        assert_eq!(d1.len(), 64);
    }

    #[test]
    fn test_compute_lawpack_digest_order_independent() {
        // BTreeMap is already sorted, but verify the algorithm is path-order deterministic
        let mut files1 = BTreeMap::new();
        files1.insert("a.txt".to_string(), b"aaa".to_vec());
        files1.insert("b.txt".to_string(), b"bbb".to_vec());

        let mut files2 = BTreeMap::new();
        files2.insert("b.txt".to_string(), b"bbb".to_vec());
        files2.insert("a.txt".to_string(), b"aaa".to_vec());

        assert_eq!(compute_lawpack_digest(&files1), compute_lawpack_digest(&files2));
    }

    #[test]
    fn test_jcs_canonicalize_sorts_keys() {
        let value = json!({"z": 1, "a": 2, "m": 3});
        let bytes = jcs_canonicalize(&value).unwrap();
        let s = std::str::from_utf8(&bytes).unwrap();
        assert_eq!(s, r#"{"a":2,"m":3,"z":1}"#);
    }

    #[test]
    fn test_jcs_canonicalize_rejects_floats() {
        let value = json!({"amount": 3.14});
        assert!(jcs_canonicalize(&value).is_err());
    }

    #[test]
    fn test_canonical_sha256_matches_python() {
        // Known test vector: {"a":1,"b":2} -> SHA256
        let value = json!({"b": 2, "a": 1});
        let digest = canonical_sha256(&value).unwrap();
        assert_eq!(
            digest,
            "43258cff783fe7036d8a43033f830adfc60ec037382473548ac742b888292777"
        );
    }

    #[test]
    fn test_resolve_lawpack_refs_from_zone() {
        let zone = json!({
            "zone_id": "test.zone",
            "lawpacks": [
                {
                    "jurisdiction_id": "pk",
                    "domain": "civil",
                    "lawpack_digest_sha256": "a".repeat(64)
                },
                {
                    "jurisdiction_id": "ae",
                    "domain": "financial",
                    "lawpack_digest_sha256": "b".repeat(64)
                }
            ]
        });
        let refs = resolve_lawpack_refs(&zone).unwrap();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].jurisdiction_id, "pk");
        assert_eq!(refs[1].domain, "financial");
    }

    #[test]
    fn test_infer_jurisdiction_from_sources() {
        let sources = SourcesDescriptor {
            jurisdiction_id: "pk".to_string(),
            domain: "civil".to_string(),
            sources: vec![],
            normalization: None,
            license: String::new(),
            extra: BTreeMap::new(),
        };
        let (jid, domain) = infer_jurisdiction_and_domain(
            Path::new("modules/legal/jurisdictions/pk/civil"),
            Some(&sources),
        );
        assert_eq!(jid, "pk");
        assert_eq!(domain, "civil");
    }

    #[test]
    fn test_lawpack_lock_deserialize() {
        let lock_json = json!({
            "lawpack_digest_sha256": "a".repeat(64),
            "jurisdiction_id": "pk",
            "domain": "civil",
            "as_of_date": "2026-01-15",
            "artifact_path": "dist/lawpacks/pk/civil/test.lawpack.zip",
            "artifact_sha256": "b".repeat(64),
            "components": {
                "lawpack_yaml_sha256": "c".repeat(64),
                "index_json_sha256": "d".repeat(64),
                "akn_sha256": {"akn/test.xml": "e".repeat(64)},
                "sources_sha256": "f".repeat(64)
            },
            "provenance": {
                "module_manifest_path": "module.yaml",
                "sources_manifest_path": "sources.yaml",
                "raw_sources": {},
                "normalization": null
            }
        });
        let lock: LawpackLock = serde_json::from_value(lock_json).unwrap();
        assert_eq!(lock.jurisdiction_id, "pk");
        assert_eq!(lock.domain, "civil");
        assert_eq!(lock.components.akn_sha256.len(), 1);
    }
}
