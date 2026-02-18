//! Licensepack — content-addressed snapshot of jurisdictional licensing state.
//!
//! The [`Licensepack`] struct is the top-level container for all license data
//! within a jurisdiction. It holds license types, individual licenses, and
//! license holders. The `compute_digest` method produces a deterministic
//! SHA-256 digest using [`CanonicalBytes`] for cross-language compatibility.

use std::collections::BTreeMap;

use mez_core::digest::Sha256Accumulator;
use mez_core::{CanonicalBytes, ContentDigest, JurisdictionId};
use serde::{Deserialize, Serialize};

use super::components::LicenseHolder;
use super::license::{License, LicenseTypeDefinition, LicensepackMetadata};
use super::types::LicenseComplianceState;
use crate::error::PackResult;

/// Digest prefix for licensepack v1 computation.
const LICENSEPACK_DIGEST_PREFIX: &[u8] = b"mez-licensepack-v1\0";

/// Content-addressed snapshot of jurisdictional licensing state.
///
/// Completes the pack trilogy:
/// - Lawpack: Static law (statutes, regulations)
/// - Regpack: Dynamic guidance (sanctions, calendars)
/// - Licensepack: Live registry (licenses, holders, conditions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Licensepack {
    /// The jurisdiction this licensepack applies to.
    pub jurisdiction: JurisdictionId,
    /// Human-readable name of the licensepack.
    pub name: String,
    /// Version string (semver).
    pub version: String,
    /// Content digest of the compiled licensepack.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<ContentDigest>,
    /// Metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<LicensepackMetadata>,
    /// License type definitions.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub license_types: BTreeMap<String, LicenseTypeDefinition>,
    /// License records.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub licenses: BTreeMap<String, License>,
    /// License holder records.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub holders: BTreeMap<String, LicenseHolder>,
}

impl Licensepack {
    /// Create a new empty licensepack.
    pub fn new(jurisdiction: JurisdictionId, name: String) -> Self {
        Self {
            jurisdiction,
            name,
            version: "1.0".to_string(),
            digest: None,
            metadata: None,
            license_types: BTreeMap::new(),
            licenses: BTreeMap::new(),
            holders: BTreeMap::new(),
        }
    }

    /// Add a license type definition.
    pub fn add_license_type(&mut self, lt: LicenseTypeDefinition) {
        self.digest = None; // Invalidate cached digest
        self.license_types.insert(lt.license_type_id.clone(), lt);
    }

    /// Add a license record.
    pub fn add_license(&mut self, license: License) {
        self.digest = None;
        self.licenses.insert(license.license_id.clone(), license);
    }

    /// Add a license holder.
    pub fn add_holder(&mut self, holder: LicenseHolder) {
        self.digest = None;
        self.holders.insert(holder.holder_id.clone(), holder);
    }

    /// Get a license by ID.
    pub fn get_license(&self, license_id: &str) -> Option<&License> {
        self.licenses.get(license_id)
    }

    /// Get all licenses for a holder DID.
    pub fn get_licenses_by_holder_did(&self, holder_did: &str) -> Vec<&License> {
        if holder_did.is_empty() {
            return Vec::new();
        }
        self.licenses
            .values()
            .filter(|lic| lic.holder_did.as_deref() == Some(holder_did))
            .collect()
    }

    /// Get all active licenses.
    pub fn get_active_licenses(&self) -> Vec<&License> {
        self.licenses
            .values()
            .filter(|lic| lic.is_active())
            .collect()
    }

    /// Verify if a holder has a valid license for an activity.
    ///
    /// Returns (is_valid, compliance_state, matching_license_id).
    pub fn verify_license(
        &self,
        holder_did: &str,
        activity: &str,
        today: &str,
    ) -> (bool, LicenseComplianceState, Option<String>) {
        let licenses = self.get_licenses_by_holder_did(holder_did);
        if licenses.is_empty() {
            return (false, LicenseComplianceState::NonCompliant, None);
        }

        for lic in &licenses {
            let state = lic.evaluate_compliance(activity, today);
            if state == LicenseComplianceState::Compliant {
                return (
                    true,
                    LicenseComplianceState::Compliant,
                    Some(lic.license_id.clone()),
                );
            }
        }

        // Return best state found
        let states: Vec<LicenseComplianceState> = licenses
            .iter()
            .map(|lic| lic.evaluate_compliance(activity, today))
            .collect();
        if states.contains(&LicenseComplianceState::Suspended) {
            return (false, LicenseComplianceState::Suspended, None);
        }
        if states.contains(&LicenseComplianceState::Pending) {
            return (false, LicenseComplianceState::Pending, None);
        }
        (false, LicenseComplianceState::NonCompliant, None)
    }

    /// Compute the content-addressed digest of this licensepack.
    ///
    /// Produces a deterministic SHA-256 hex string by canonicalizing all
    /// components in a fixed order (BTreeMap guarantees sorted iteration).
    ///
    /// # SHA-256 exception: composite domain-prefixed digest
    ///
    /// Uses `Sha256Accumulator` instead of `sha256_digest(&CanonicalBytes)`
    /// because this computes a composite digest over a domain prefix + multiple
    /// individually canonicalized components. Each component goes through
    /// `CanonicalBytes` before being fed to the hasher.
    pub fn compute_digest(&self) -> PackResult<String> {
        let mut acc = Sha256Accumulator::new();
        acc.update(LICENSEPACK_DIGEST_PREFIX);

        // Add metadata
        if let Some(ref meta) = self.metadata {
            let meta_value = serde_json::to_value(meta)?;
            let meta_canonical = CanonicalBytes::from_value(meta_value)?;
            acc.update(meta_canonical.as_bytes());
            acc.update(b"\0");
        }

        // Add license types (sorted by key — BTreeMap guarantees this)
        for (type_id, lt) in &self.license_types {
            let lt_value = serde_json::to_value(lt)?;
            let lt_canonical = CanonicalBytes::from_value(lt_value)?;
            acc.update(format!("license-types/{type_id}\0").as_bytes());
            acc.update(lt_canonical.as_bytes());
            acc.update(b"\0");
        }

        // Add licenses (sorted by key)
        for (license_id, lic) in &self.licenses {
            // Serialize license core fields (excluding conditions/permissions/restrictions
            // which are added separately to match Python structure)
            let lic_value = serde_json::to_value(lic)?;
            let lic_canonical = CanonicalBytes::from_value(lic_value)?;
            acc.update(format!("licenses/{license_id}\0").as_bytes());
            acc.update(lic_canonical.as_bytes());
            acc.update(b"\0");

            // Add conditions (sorted by condition_id)
            let mut sorted_conditions = lic.conditions.clone();
            sorted_conditions.sort_by(|a, b| a.condition_id.cmp(&b.condition_id));
            for cond in &sorted_conditions {
                let cond_value = serde_json::to_value(cond)?;
                let cond_canonical = CanonicalBytes::from_value(cond_value)?;
                acc.update(
                    format!("licenses/{license_id}/conditions/{}\0", cond.condition_id).as_bytes(),
                );
                acc.update(cond_canonical.as_bytes());
                acc.update(b"\0");
            }

            // Add permissions (sorted by permission_id)
            let mut sorted_permissions = lic.permissions.clone();
            sorted_permissions.sort_by(|a, b| a.permission_id.cmp(&b.permission_id));
            for perm in &sorted_permissions {
                let perm_value = serde_json::to_value(perm)?;
                let perm_canonical = CanonicalBytes::from_value(perm_value)?;
                acc.update(
                    format!("licenses/{license_id}/permissions/{}\0", perm.permission_id)
                        .as_bytes(),
                );
                acc.update(perm_canonical.as_bytes());
                acc.update(b"\0");
            }

            // Add restrictions (sorted by restriction_id)
            let mut sorted_restrictions = lic.restrictions.clone();
            sorted_restrictions.sort_by(|a, b| a.restriction_id.cmp(&b.restriction_id));
            for rest in &sorted_restrictions {
                let rest_value = serde_json::to_value(rest)?;
                let rest_canonical = CanonicalBytes::from_value(rest_value)?;
                acc.update(
                    format!(
                        "licenses/{license_id}/restrictions/{}\0",
                        rest.restriction_id
                    )
                    .as_bytes(),
                );
                acc.update(rest_canonical.as_bytes());
                acc.update(b"\0");
            }
        }

        // Add holders (sorted by key)
        for (holder_id, holder) in &self.holders {
            let holder_value = serde_json::to_value(holder)?;
            let holder_canonical = CanonicalBytes::from_value(holder_value)?;
            acc.update(format!("holders/{holder_id}\0").as_bytes());
            acc.update(holder_canonical.as_bytes());
            acc.update(b"\0");
        }

        Ok(acc.finalize_hex())
    }

    /// Compute delta from a previous licensepack.
    ///
    /// Compares license sets to identify granted, revoked, suspended, and
    /// reinstated licenses between snapshots.
    pub fn compute_delta(&self, previous: &Licensepack) -> serde_json::Value {
        let prev_ids: std::collections::HashSet<&String> = previous.licenses.keys().collect();
        let curr_ids: std::collections::HashSet<&String> = self.licenses.keys().collect();

        let new_ids: Vec<&&String> = curr_ids.difference(&prev_ids).collect();
        let removed_ids: Vec<&&String> = prev_ids.difference(&curr_ids).collect();

        let mut granted = Vec::new();
        let mut revoked = Vec::new();
        let mut suspended = Vec::new();
        let mut reinstated = Vec::new();

        for id in &new_ids {
            if let Some(lic) = self.licenses.get(**id) {
                match lic.status {
                    super::types::LicenseStatus::Active => granted.push((**id).clone()),
                    super::types::LicenseStatus::Suspended => suspended.push((**id).clone()),
                    _ => {}
                }
            }
        }

        for id in &removed_ids {
            if let Some(prev_lic) = previous.licenses.get(**id) {
                if prev_lic.status == super::types::LicenseStatus::Active {
                    revoked.push((**id).clone());
                }
            }
        }

        // Status changes in existing licenses
        for id in curr_ids.intersection(&prev_ids) {
            if let (Some(curr), Some(prev)) = (self.licenses.get(*id), previous.licenses.get(*id)) {
                if curr.status != prev.status {
                    match (prev.status, curr.status) {
                        (
                            super::types::LicenseStatus::Suspended,
                            super::types::LicenseStatus::Active,
                        ) => {
                            reinstated.push((*id).clone());
                        }
                        (_, super::types::LicenseStatus::Suspended) => {
                            suspended.push((*id).clone());
                        }
                        (_, super::types::LicenseStatus::Revoked) => {
                            revoked.push((*id).clone());
                        }
                        _ => {}
                    }
                }
            }
        }

        serde_json::json!({
            "licenses_granted": granted.len(),
            "licenses_revoked": revoked.len(),
            "licenses_suspended": suspended.len(),
            "licenses_reinstated": reinstated.len(),
            "details": {
                "granted": granted,
                "revoked": revoked,
                "suspended": suspended,
                "reinstated": reinstated,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::components::LicenseHolder;
    use super::super::components::{LicenseCondition, LicensePermission, LicenseRestriction};
    use super::super::license::{
        License, LicenseTypeDefinition, LicensepackMetadata, LicensepackRegulator,
    };
    use super::super::types::LicenseStatus;
    use super::*;

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

    fn make_test_holder(id: &str) -> LicenseHolder {
        LicenseHolder {
            holder_id: id.to_string(),
            entity_type: "company".to_string(),
            legal_name: "Test Corporation Ltd".to_string(),
            trading_names: vec!["TestCo".to_string()],
            registration_number: Some("REG-001".to_string()),
            incorporation_date: Some("2020-01-01".to_string()),
            jurisdiction_of_incorporation: Some("pk".to_string()),
            did: Some("did:web:test.example".to_string()),
            registered_address: BTreeMap::new(),
            contact: BTreeMap::new(),
            controllers: vec![],
            beneficial_owners: vec![],
            group_structure: BTreeMap::new(),
        }
    }

    fn make_test_license_type(id: &str) -> LicenseTypeDefinition {
        LicenseTypeDefinition {
            license_type_id: id.to_string(),
            name: "Test License Type".to_string(),
            description: "A test license type".to_string(),
            regulator_id: "fsra".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec!["trading".to_string()],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        }
    }

    fn make_test_metadata() -> LicensepackMetadata {
        LicensepackMetadata {
            licensepack_id: "licensepack:pk:financial:2026-01-15".to_string(),
            jurisdiction_id: "pk".to_string(),
            domain: "financial".to_string(),
            as_of_date: "2026-01-15".to_string(),
            snapshot_timestamp: "2026-01-15T00:00:00Z".to_string(),
            snapshot_type: "quarterly".to_string(),
            regulator: LicensepackRegulator {
                regulator_id: "fsra".to_string(),
                name: "FSRA".to_string(),
                jurisdiction_id: "pk-kp-rsez".to_string(),
                registry_url: None,
                did: None,
                api_capabilities: vec![],
            },
            license: "MIT".to_string(),
            sources: vec![],
            includes: BTreeMap::new(),
            normalization: BTreeMap::new(),
            previous_licensepack_digest: None,
            delta: None,
        }
    }

    #[test]
    fn test_licensepack_add_and_get() {
        let mut pack = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Test Pack".to_string(),
        );
        let lic = make_test_license("lic-001", LicenseStatus::Active);
        pack.add_license(lic);

        assert!(pack.get_license("lic-001").is_some());
        assert!(pack.get_license("lic-999").is_none());
    }

    #[test]
    fn test_licensepack_get_by_holder_did() {
        let mut pack = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Test Pack".to_string(),
        );
        pack.add_license(make_test_license("lic-001", LicenseStatus::Active));
        pack.add_license(make_test_license("lic-002", LicenseStatus::Suspended));

        let holder_licenses = pack.get_licenses_by_holder_did("did:web:test.example");
        assert_eq!(holder_licenses.len(), 2);

        let no_licenses = pack.get_licenses_by_holder_did("did:web:unknown");
        assert!(no_licenses.is_empty());
    }

    #[test]
    fn test_licensepack_active_licenses() {
        let mut pack = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Test Pack".to_string(),
        );
        pack.add_license(make_test_license("lic-001", LicenseStatus::Active));
        pack.add_license(make_test_license("lic-002", LicenseStatus::Suspended));
        pack.add_license(make_test_license("lic-003", LicenseStatus::Active));

        let active = pack.get_active_licenses();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn test_licensepack_verify_license() {
        let mut pack = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Test Pack".to_string(),
        );
        pack.add_license(make_test_license("lic-001", LicenseStatus::Active));

        let (valid, state, lic_id) =
            pack.verify_license("did:web:test.example", "payment_services", "2026-06-15");
        assert!(valid);
        assert_eq!(state, LicenseComplianceState::Compliant);
        assert_eq!(lic_id.unwrap(), "lic-001");

        let (valid, state, _) =
            pack.verify_license("did:web:unknown", "payment_services", "2026-06-15");
        assert!(!valid);
        assert_eq!(state, LicenseComplianceState::NonCompliant);
    }

    #[test]
    fn test_licensepack_digest_deterministic() {
        let mut pack = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Test Pack".to_string(),
        );
        pack.add_license(make_test_license("lic-001", LicenseStatus::Active));
        let d1 = pack.compute_digest().unwrap();
        let d2 = pack.compute_digest().unwrap();
        assert_eq!(d1, d2);
        assert_eq!(d1.len(), 64);
    }

    #[test]
    fn test_licensepack_digest_changes_on_mutation() {
        let mut pack = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Test Pack".to_string(),
        );
        pack.add_license(make_test_license("lic-001", LicenseStatus::Active));
        let d1 = pack.compute_digest().unwrap();

        pack.add_license(make_test_license("lic-002", LicenseStatus::Suspended));
        let d2 = pack.compute_digest().unwrap();

        assert_ne!(d1, d2);
    }

    #[test]
    fn test_licensepack_delta() {
        let mut prev = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Previous".to_string(),
        );
        prev.add_license(make_test_license("lic-001", LicenseStatus::Active));
        prev.add_license(make_test_license("lic-002", LicenseStatus::Active));

        let mut curr = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Current".to_string(),
        );
        curr.add_license(make_test_license("lic-001", LicenseStatus::Suspended));
        curr.add_license(make_test_license("lic-003", LicenseStatus::Active));

        let delta = curr.compute_delta(&prev);
        assert_eq!(delta["licenses_granted"], 1);
        assert_eq!(delta["licenses_revoked"], 1);
        assert_eq!(delta["licenses_suspended"], 1);
    }

    #[test]
    fn test_licensepack_digest_with_metadata() {
        let mut pack = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Test".to_string(),
        );
        pack.metadata = Some(make_test_metadata());
        pack.add_license(make_test_license("lic-001", LicenseStatus::Active));

        let d1 = pack.compute_digest().unwrap();
        assert_eq!(d1.len(), 64);

        let mut pack_no_meta = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Test".to_string(),
        );
        pack_no_meta.add_license(make_test_license("lic-001", LicenseStatus::Active));
        let d2 = pack_no_meta.compute_digest().unwrap();
        assert_ne!(d1, d2);
    }

    #[test]
    fn test_licensepack_digest_with_holders() {
        let mut pack = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Test".to_string(),
        );
        let d_empty = pack.compute_digest().unwrap();

        pack.add_holder(make_test_holder("h-001"));
        let d_with_holder = pack.compute_digest().unwrap();
        assert_ne!(d_empty, d_with_holder);

        pack.add_holder(make_test_holder("h-002"));
        let d_two_holders = pack.compute_digest().unwrap();
        assert_ne!(d_with_holder, d_two_holders);
    }

    #[test]
    fn test_licensepack_add_license_type() {
        let mut pack = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Test".to_string(),
        );
        assert!(pack.license_types.is_empty());

        pack.add_license_type(make_test_license_type("lt-001"));
        assert_eq!(pack.license_types.len(), 1);
        assert!(pack.license_types.contains_key("lt-001"));
        assert!(pack.digest.is_none());
    }

    #[test]
    fn test_licensepack_digest_with_license_types() {
        let mut pack = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Test".to_string(),
        );
        let d_empty = pack.compute_digest().unwrap();

        pack.add_license_type(make_test_license_type("lt-001"));
        let d_with_type = pack.compute_digest().unwrap();
        assert_ne!(d_empty, d_with_type);
    }

    #[test]
    fn test_licensepack_add_holder_invalidates_digest() {
        let mut pack = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Test".to_string(),
        );
        let _ = pack.compute_digest().unwrap();

        pack.add_holder(make_test_holder("h-001"));
        assert!(pack.digest.is_none());
    }

    #[test]
    fn test_verify_license_returns_suspended() {
        let mut pack = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Test".to_string(),
        );
        pack.add_license(make_test_license("lic-001", LicenseStatus::Suspended));

        let (valid, state, _) =
            pack.verify_license("did:web:test.example", "payment_services", "2026-06-15");
        assert!(!valid);
        assert_eq!(state, LicenseComplianceState::Suspended);
    }

    #[test]
    fn test_verify_license_returns_pending() {
        let mut pack = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Test".to_string(),
        );
        pack.add_license(make_test_license("lic-001", LicenseStatus::Pending));

        let (valid, state, _) =
            pack.verify_license("did:web:test.example", "payment_services", "2026-06-15");
        assert!(!valid);
        assert_eq!(state, LicenseComplianceState::Pending);
    }

    #[test]
    fn test_verify_license_prefers_compliant_over_suspended() {
        let mut pack = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Test".to_string(),
        );
        pack.add_license(make_test_license("lic-001", LicenseStatus::Suspended));
        pack.add_license(make_test_license("lic-002", LicenseStatus::Active));

        let (valid, state, lic_id) =
            pack.verify_license("did:web:test.example", "payment_services", "2026-06-15");
        assert!(valid);
        assert_eq!(state, LicenseComplianceState::Compliant);
        assert!(lic_id.is_some());
    }

    #[test]
    fn test_compute_delta_no_changes() {
        let mut pack = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Same".to_string(),
        );
        pack.add_license(make_test_license("lic-001", LicenseStatus::Active));

        let mut prev = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Same".to_string(),
        );
        prev.add_license(make_test_license("lic-001", LicenseStatus::Active));

        let delta = pack.compute_delta(&prev);
        assert_eq!(delta["licenses_granted"], 0);
        assert_eq!(delta["licenses_revoked"], 0);
        assert_eq!(delta["licenses_suspended"], 0);
        assert_eq!(delta["licenses_reinstated"], 0);
    }

    #[test]
    fn test_compute_delta_reinstated() {
        let mut prev = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Prev".to_string(),
        );
        prev.add_license(make_test_license("lic-001", LicenseStatus::Suspended));

        let mut curr = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Curr".to_string(),
        );
        curr.add_license(make_test_license("lic-001", LicenseStatus::Active));

        let delta = curr.compute_delta(&prev);
        assert_eq!(delta["licenses_reinstated"], 1);
    }

    #[test]
    fn test_compute_delta_empty_packs() {
        let prev = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Prev".to_string(),
        );
        let curr = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Curr".to_string(),
        );

        let delta = curr.compute_delta(&prev);
        assert_eq!(delta["licenses_granted"], 0);
        assert_eq!(delta["licenses_revoked"], 0);
    }

    #[test]
    fn test_compute_delta_all_new() {
        let prev = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Prev".to_string(),
        );
        let mut curr = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Curr".to_string(),
        );
        curr.add_license(make_test_license("lic-001", LicenseStatus::Active));
        curr.add_license(make_test_license("lic-002", LicenseStatus::Active));

        let delta = curr.compute_delta(&prev);
        assert_eq!(delta["licenses_granted"], 2);
    }

    #[test]
    fn test_compute_delta_status_revoked() {
        let mut prev = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Prev".to_string(),
        );
        prev.add_license(make_test_license("lic-001", LicenseStatus::Active));

        let mut curr = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Curr".to_string(),
        );
        curr.add_license(make_test_license("lic-001", LicenseStatus::Revoked));

        let delta = curr.compute_delta(&prev);
        assert_eq!(delta["licenses_revoked"], 1);
    }

    #[test]
    fn test_licensepack_digest_with_conditions() {
        let mut pack1 = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Test".to_string(),
        );
        let mut lic = make_test_license("lic-001", LicenseStatus::Active);
        lic.conditions = vec![LicenseCondition {
            condition_id: "c-001".to_string(),
            condition_type: "capital".to_string(),
            description: "Min capital".to_string(),
            metric: None,
            threshold: Some("1000000".to_string()),
            currency: Some("PKR".to_string()),
            operator: Some(">=".to_string()),
            frequency: None,
            reporting_frequency: None,
            effective_date: None,
            expiry_date: None,
            status: "active".to_string(),
        }];
        pack1.add_license(lic);

        let mut pack2 = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Test".to_string(),
        );
        pack2.add_license(make_test_license("lic-001", LicenseStatus::Active));

        assert_ne!(
            pack1.compute_digest().unwrap(),
            pack2.compute_digest().unwrap()
        );
    }

    #[test]
    fn test_licensepack_digest_with_permissions_and_restrictions() {
        let mut lic = make_test_license("lic-001", LicenseStatus::Active);
        lic.permissions = vec![LicensePermission {
            permission_id: "p-001".to_string(),
            activity: "payment".to_string(),
            scope: BTreeMap::new(),
            limits: BTreeMap::new(),
            effective_date: None,
            status: "active".to_string(),
        }];
        lic.restrictions = vec![LicenseRestriction {
            restriction_id: "r-001".to_string(),
            restriction_type: "geographic".to_string(),
            description: "Test".to_string(),
            blocked_activities: vec![],
            blocked_jurisdictions: vec!["us".to_string()],
            allowed_jurisdictions: vec![],
            blocked_products: vec![],
            blocked_client_types: vec![],
            max_leverage: None,
            effective_date: None,
            status: "active".to_string(),
        }];

        let mut pack = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Test".to_string(),
        );
        pack.add_license(lic);

        let digest = pack.compute_digest().unwrap();
        assert_eq!(digest.len(), 64);
        let digest2 = pack.compute_digest().unwrap();
        assert_eq!(digest, digest2);
    }
}
