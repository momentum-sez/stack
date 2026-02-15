//! Licensepack references, lock files, and utility functions.
//!
//! Types for referencing licensepacks in zone compositions and lock files,
//! plus helper functions for zone manifest resolution and compliance evaluation.

use serde::{Deserialize, Serialize};

use msez_core::CanonicalBytes;

use super::pack::Licensepack;
use super::types::LicenseComplianceState;
use crate::error::PackResult;
use crate::parser;

/// Licensepack reference in a zone composition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicensepackRef {
    /// Jurisdiction identifier.
    pub jurisdiction_id: String,
    /// License domain.
    pub domain: String,
    /// SHA-256 digest.
    pub licensepack_digest_sha256: String,
    /// Snapshot date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub as_of_date: Option<String>,
}

/// Licensepack lock file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensepackLock {
    /// Lock version.
    pub lock_version: String,
    /// Generation timestamp.
    pub generated_at: String,
    /// Generator tool.
    pub generator: String,
    /// Generator version.
    pub generator_version: String,
    /// Licensepack info.
    pub licensepack: LicensepackLockInfo,
    /// Artifact info.
    pub artifact: LicensepackArtifactInfo,
}

/// Licensepack identification in a lock file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensepackLockInfo {
    /// Licensepack identifier.
    pub licensepack_id: String,
    /// Jurisdiction identifier.
    pub jurisdiction_id: String,
    /// License domain.
    pub domain: String,
    /// Snapshot date.
    pub as_of_date: String,
    /// Content digest.
    pub digest_sha256: String,
}

/// Artifact metadata in a licensepack lock file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensepackArtifactInfo {
    /// Artifact type.
    pub artifact_type: String,
    /// Content digest.
    pub digest_sha256: String,
    /// Artifact URI.
    pub uri: String,
    /// Media type.
    pub media_type: String,
    /// Byte length.
    pub byte_length: i64,
}

/// Resolve licensepack references from a zone manifest.
pub fn resolve_licensepack_refs(zone: &serde_json::Value) -> PackResult<Vec<LicensepackRef>> {
    let mut refs = Vec::new();
    if let Some(licensepacks) = zone.get("licensepacks").and_then(|v| v.as_array()) {
        for lp in licensepacks {
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
                .get("licensepack_digest_sha256")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if jid.is_empty() || domain.is_empty() {
                continue;
            }
            if !digest.is_empty() && parser::is_valid_sha256(&digest) {
                refs.push(LicensepackRef {
                    jurisdiction_id: jid,
                    domain,
                    licensepack_digest_sha256: digest,
                    as_of_date: lp
                        .get("as_of_date")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                });
            }
        }
    }
    Ok(refs)
}

/// Compute canonical JSON bytes for a value using the JCS-compatible pipeline.
pub fn canonical_json_bytes(value: &serde_json::Value) -> PackResult<Vec<u8>> {
    let canonical = CanonicalBytes::from_value(value.clone())?;
    Ok(canonical.into_bytes())
}

/// Evaluate licensing compliance for an activity.
///
/// Used by the compliance tensor to populate the LICENSING domain.
pub fn evaluate_license_compliance(
    license_id: &str,
    activity: &str,
    licensepack: &Licensepack,
    today: &str,
) -> LicenseComplianceState {
    match licensepack.get_license(license_id) {
        Some(license) => license.evaluate_compliance(activity, today),
        None => LicenseComplianceState::NonCompliant,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::license::License;
    use super::super::types::LicenseStatus;
    use msez_core::JurisdictionId;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn make_test_license(id: &str, status: LicenseStatus) -> License {
        License {
            license_id: id.to_string(),
            license_type_id: "test:emi".to_string(),
            license_number: Some("LIC-001".to_string()),
            status,
            issued_date: "2025-01-01".to_string(),
            holder_id: "holder-001".to_string(),
            holder_legal_name: "Test Corp".to_string(),
            regulator_id: "fsra".to_string(),
            status_effective_date: None,
            status_reason: None,
            effective_date: Some("2025-01-01".to_string()),
            expiry_date: Some("2027-12-31".to_string()),
            holder_registration_number: None,
            holder_did: Some("did:web:test.example".to_string()),
            issuing_authority: None,
            permitted_activities: vec!["payment_services".to_string(), "e_money".to_string()],
            asset_classes_authorized: vec![],
            client_types_permitted: vec![],
            geographic_scope: vec![],
            prudential_category: None,
            capital_requirement: BTreeMap::new(),
            conditions: vec![],
            permissions: vec![],
            restrictions: vec![],
        }
    }

    #[test]
    fn test_licensepack_ref_serialization() {
        let r = LicensepackRef {
            jurisdiction_id: "pk".to_string(),
            domain: "financial".to_string(),
            licensepack_digest_sha256: "a".repeat(64),
            as_of_date: Some("2026-01-15".to_string()),
        };
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["jurisdiction_id"], "pk");
        assert_eq!(json["domain"], "financial");
    }

    #[test]
    fn test_licensepack_ref_equality() {
        let r1 = LicensepackRef {
            jurisdiction_id: "pk".to_string(),
            domain: "financial".to_string(),
            licensepack_digest_sha256: "a".repeat(64),
            as_of_date: None,
        };
        let r2 = r1.clone();
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_resolve_licensepack_refs() {
        let zone = json!({
            "zone_id": "test.zone",
            "licensepacks": [
                {
                    "jurisdiction_id": "pk",
                    "domain": "financial",
                    "licensepack_digest_sha256": "a".repeat(64),
                    "as_of_date": "2026-01-15"
                }
            ]
        });
        let refs = resolve_licensepack_refs(&zone).unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].jurisdiction_id, "pk");
        assert_eq!(refs[0].domain, "financial");
    }

    #[test]
    fn test_resolve_licensepack_refs_empty() {
        let zone = json!({"zone_id": "test"});
        let refs = resolve_licensepack_refs(&zone).unwrap();
        assert!(refs.is_empty());
    }

    #[test]
    fn test_resolve_licensepack_refs_skips_invalid_digest() {
        let zone = json!({
            "licensepacks": [
                {
                    "jurisdiction_id": "pk",
                    "domain": "financial",
                    "licensepack_digest_sha256": "bad"
                }
            ]
        });
        let refs = resolve_licensepack_refs(&zone).unwrap();
        assert!(refs.is_empty());
    }

    #[test]
    fn test_resolve_licensepack_refs_multiple() {
        let zone = json!({
            "licensepacks": [
                {
                    "jurisdiction_id": "pk",
                    "domain": "financial",
                    "licensepack_digest_sha256": "a".repeat(64)
                },
                {
                    "jurisdiction_id": "ae",
                    "domain": "trade",
                    "licensepack_digest_sha256": "b".repeat(64),
                    "as_of_date": "2026-02-01"
                }
            ]
        });
        let refs = resolve_licensepack_refs(&zone).unwrap();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[1].as_of_date, Some("2026-02-01".to_string()));
    }

    #[test]
    fn test_evaluate_license_compliance_fn() {
        let mut pack = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Test".to_string(),
        );
        pack.add_license(make_test_license("lic-001", LicenseStatus::Active));

        let state =
            evaluate_license_compliance("lic-001", "payment_services", &pack, "2026-06-15");
        assert_eq!(state, LicenseComplianceState::Compliant);

        let state =
            evaluate_license_compliance("lic-999", "payment_services", &pack, "2026-06-15");
        assert_eq!(state, LicenseComplianceState::NonCompliant);
    }

    #[test]
    fn test_canonical_json_bytes_sorts_keys() {
        let val = json!({"z": 1, "a": 2});
        let bytes = canonical_json_bytes(&val).unwrap();
        let s = std::str::from_utf8(&bytes).unwrap();
        assert_eq!(s, r#"{"a":2,"z":1}"#);
    }

    #[test]
    fn test_canonical_json_bytes_rejects_float() {
        let val = json!({"rate": 3.15});
        assert!(canonical_json_bytes(&val).is_err());
    }

    #[test]
    fn test_licensepack_lock_deserialization() {
        let lock_json = json!({
            "lock_version": "1",
            "generated_at": "2026-01-15T00:00:00Z",
            "generator": "msez",
            "generator_version": "0.4.44",
            "licensepack": {
                "licensepack_id": "licensepack:pk:financial:2026-01-15",
                "jurisdiction_id": "pk",
                "domain": "financial",
                "as_of_date": "2026-01-15",
                "digest_sha256": "a".repeat(64)
            },
            "artifact": {
                "artifact_type": "licensepack",
                "digest_sha256": "a".repeat(64),
                "uri": "dist/licensepacks/pk/financial/test.zip",
                "media_type": "application/zip",
                "byte_length": 4096
            }
        });
        let lock: LicensepackLock = serde_json::from_value(lock_json).unwrap();
        assert_eq!(lock.licensepack.jurisdiction_id, "pk");
        assert_eq!(lock.artifact.byte_length, 4096);
    }

    #[test]
    fn test_licensepack_artifact_info_roundtrip() {
        let info = LicensepackArtifactInfo {
            artifact_type: "licensepack".to_string(),
            digest_sha256: "a".repeat(64),
            uri: "dist/licensepacks/pk/financial/test.zip".to_string(),
            media_type: "application/zip".to_string(),
            byte_length: 8192,
        };
        let json_str = serde_json::to_string(&info).unwrap();
        let deserialized: LicensepackArtifactInfo = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.byte_length, 8192);
        assert_eq!(deserialized.media_type, "application/zip");
    }
}
