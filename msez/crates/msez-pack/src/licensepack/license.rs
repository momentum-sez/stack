//! License record and license type definitions.
//!
//! The [`License`] struct represents an individual license issued to a holder,
//! including its conditions, permissions, and restrictions. [`LicenseTypeDefinition`]
//! describes the category of license available in a jurisdiction.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::components::{LicenseCondition, LicensePermission, LicenseRestriction};
use super::types::{LicenseComplianceState, LicenseStatus};

/// Individual license record.
///
/// Represents a specific license issued by a regulator to a holder, with
/// associated conditions, permissions, and restrictions that govern the
/// holder's authorized activities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct License {
    /// Unique license identifier.
    pub license_id: String,
    /// License type identifier.
    pub license_type_id: String,
    /// License number (regulator-assigned).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license_number: Option<String>,
    /// Current status.
    pub status: LicenseStatus,
    /// Date the license was issued.
    pub issued_date: String,
    /// Holder identifier.
    pub holder_id: String,
    /// Holder legal name (denormalized for display).
    pub holder_legal_name: String,
    /// Issuing regulator identifier.
    pub regulator_id: String,
    /// Status effective date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_effective_date: Option<String>,
    /// Status change reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_reason: Option<String>,
    /// License effective date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_date: Option<String>,
    /// License expiry date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expiry_date: Option<String>,
    /// Holder registration number (denormalized).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holder_registration_number: Option<String>,
    /// Holder DID (denormalized).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holder_did: Option<String>,
    /// Issuing authority name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issuing_authority: Option<String>,
    /// Permitted activities.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permitted_activities: Vec<String>,
    /// Authorized asset classes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub asset_classes_authorized: Vec<String>,
    /// Permitted client types.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub client_types_permitted: Vec<String>,
    /// Geographic scope.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub geographic_scope: Vec<String>,
    /// Prudential category.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prudential_category: Option<String>,
    /// Capital requirements.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub capital_requirement: BTreeMap<String, String>,
    /// Conditions attached to the license.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<LicenseCondition>,
    /// Permissions granted under the license.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<LicensePermission>,
    /// Restrictions on the license.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub restrictions: Vec<LicenseRestriction>,
}

impl License {
    /// Whether the license is currently active.
    pub fn is_active(&self) -> bool {
        self.status == LicenseStatus::Active
    }

    /// Whether the license has expired based on the expiry date.
    pub fn is_expired(&self, today: &str) -> bool {
        match &self.expiry_date {
            Some(expiry) => super::components::date_before(expiry, today),
            None => false,
        }
    }

    /// Validate that required fields are non-empty.
    ///
    /// Checks that `license_id`, `holder_id`, and `regulator_id` are
    /// non-empty strings. Returns `true` if all required fields are present.
    pub fn validate(&self) -> bool {
        !self.license_id.trim().is_empty()
            && !self.holder_id.trim().is_empty()
            && !self.regulator_id.trim().is_empty()
    }

    /// Whether any active restriction blocks the given activity.
    pub fn has_blocking_restriction(&self, activity: &str) -> bool {
        self.restrictions
            .iter()
            .any(|r| r.blocks_activity(activity))
    }

    /// Whether the license permits the specified activity.
    pub fn permits_activity(&self, activity: &str) -> bool {
        // Check explicit permitted activities list — acts as an allowlist gate.
        if !self.permitted_activities.is_empty() {
            if !self.permitted_activities.contains(&activity.to_string()) {
                return false;
            }
            // Activity is in the explicit allowed list — permit it regardless
            // of whether fine-grained permission objects also cover it.
            return true;
        }
        // No explicit activities list — fall through to permission checks.
        for perm in &self.permissions {
            if perm.permits_activity(activity) {
                return true;
            }
        }
        // No permissions defined and no explicit activities — deny.
        false
    }

    /// Evaluate compliance state for the LICENSING domain.
    ///
    /// Maps the license status, expiry, activity permissions, and restrictions
    /// to a [`LicenseComplianceState`] for the compliance tensor.
    pub fn evaluate_compliance(&self, activity: &str, today: &str) -> LicenseComplianceState {
        match self.status {
            LicenseStatus::Suspended => return LicenseComplianceState::Suspended,
            LicenseStatus::Pending => return LicenseComplianceState::Pending,
            LicenseStatus::Revoked | LicenseStatus::Expired | LicenseStatus::Surrendered => {
                return LicenseComplianceState::NonCompliant;
            }
            LicenseStatus::Active => {}
        }
        if self.is_expired(today) {
            return LicenseComplianceState::NonCompliant;
        }
        if !self.permits_activity(activity) {
            return LicenseComplianceState::NonCompliant;
        }
        if self.has_blocking_restriction(activity) {
            return LicenseComplianceState::NonCompliant;
        }
        LicenseComplianceState::Compliant
    }
}

/// License type definition.
///
/// Describes a category of license available in a jurisdiction, including
/// the issuing regulator, permitted activities, requirements, and fees.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LicenseTypeDefinition {
    /// Unique license type identifier.
    pub license_type_id: String,
    /// Human-readable name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Issuing regulator identifier.
    pub regulator_id: String,
    /// License category.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Permitted activities.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permitted_activities: Vec<String>,
    /// Requirements for obtaining the license.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub requirements: BTreeMap<String, serde_json::Value>,
    /// Application fee.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub application_fee: BTreeMap<String, String>,
    /// Annual fee.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub annual_fee: BTreeMap<String, String>,
    /// Validity period in years.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validity_period_years: Option<i32>,
}

/// Regulatory authority profile for licensepacks.
///
/// Identifies the regulator that issues and oversees licenses within
/// a jurisdiction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LicensepackRegulator {
    /// Unique regulator identifier.
    pub regulator_id: String,
    /// Human-readable name.
    pub name: String,
    /// Jurisdiction identifier.
    pub jurisdiction_id: String,
    /// Registry URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_url: Option<String>,
    /// Regulator DID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub did: Option<String>,
    /// API capabilities.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub api_capabilities: Vec<String>,
}

/// Licensepack metadata.
///
/// Describes the snapshot context: jurisdiction, domain, snapshot timing,
/// regulator, and optional delta from the previous snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LicensepackMetadata {
    /// Unique licensepack identifier.
    pub licensepack_id: String,
    /// Jurisdiction identifier.
    pub jurisdiction_id: String,
    /// License domain.
    pub domain: String,
    /// Snapshot date (YYYY-MM-DD).
    pub as_of_date: String,
    /// Snapshot timestamp (RFC 3339).
    pub snapshot_timestamp: String,
    /// Snapshot type: "quarterly", "monthly", "on_demand".
    pub snapshot_type: String,
    /// Regulator information.
    pub regulator: LicensepackRegulator,
    /// SPDX license identifier.
    #[serde(default)]
    pub license: String,
    /// Source feed metadata.
    #[serde(default)]
    pub sources: Vec<serde_json::Value>,
    /// Content summary.
    #[serde(default)]
    pub includes: BTreeMap<String, serde_json::Value>,
    /// Normalization metadata.
    #[serde(default)]
    pub normalization: BTreeMap<String, serde_json::Value>,
    /// Digest of the previous licensepack (for chaining).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_licensepack_digest: Option<String>,
    /// Delta from previous snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta: Option<BTreeMap<String, serde_json::Value>>,
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn test_license_is_active() {
        let active = make_test_license("lic-001", LicenseStatus::Active);
        assert!(active.is_active());
        let suspended = make_test_license("lic-002", LicenseStatus::Suspended);
        assert!(!suspended.is_active());
    }

    #[test]
    fn test_license_is_expired() {
        let lic = make_test_license("lic-001", LicenseStatus::Active);
        assert!(!lic.is_expired("2026-06-15"));
        assert!(lic.is_expired("2028-01-01"));
    }

    #[test]
    fn test_license_permits_activity() {
        let lic = make_test_license("lic-001", LicenseStatus::Active);
        assert!(lic.permits_activity("payment_services"));
        assert!(!lic.permits_activity("crypto_exchange"));
    }

    #[test]
    fn test_license_evaluate_compliance_active() {
        let lic = make_test_license("lic-001", LicenseStatus::Active);
        assert_eq!(
            lic.evaluate_compliance("payment_services", "2026-06-15"),
            LicenseComplianceState::Compliant
        );
    }

    #[test]
    fn test_license_evaluate_compliance_suspended() {
        let lic = make_test_license("lic-001", LicenseStatus::Suspended);
        assert_eq!(
            lic.evaluate_compliance("payment_services", "2026-06-15"),
            LicenseComplianceState::Suspended
        );
    }

    #[test]
    fn test_license_evaluate_compliance_expired() {
        let lic = make_test_license("lic-001", LicenseStatus::Active);
        assert_eq!(
            lic.evaluate_compliance("payment_services", "2028-01-01"),
            LicenseComplianceState::NonCompliant
        );
    }

    #[test]
    fn test_license_evaluate_compliance_unpermitted() {
        let lic = make_test_license("lic-001", LicenseStatus::Active);
        assert_eq!(
            lic.evaluate_compliance("crypto_exchange", "2026-06-15"),
            LicenseComplianceState::NonCompliant
        );
    }

    #[test]
    fn test_evaluate_compliance_revoked() {
        let lic = make_test_license("lic-001", LicenseStatus::Revoked);
        assert_eq!(
            lic.evaluate_compliance("payment_services", "2026-06-15"),
            LicenseComplianceState::NonCompliant
        );
    }

    #[test]
    fn test_evaluate_compliance_surrendered() {
        let lic = make_test_license("lic-001", LicenseStatus::Surrendered);
        assert_eq!(
            lic.evaluate_compliance("payment_services", "2026-06-15"),
            LicenseComplianceState::NonCompliant
        );
    }

    #[test]
    fn test_license_has_blocking_restriction() {
        let mut lic = make_test_license("lic-001", LicenseStatus::Active);
        lic.restrictions = vec![LicenseRestriction {
            restriction_id: "r-001".to_string(),
            restriction_type: "activity".to_string(),
            description: "No crypto".to_string(),
            blocked_activities: vec!["crypto_exchange".to_string()],
            blocked_jurisdictions: vec![],
            allowed_jurisdictions: vec![],
            blocked_products: vec![],
            blocked_client_types: vec![],
            max_leverage: None,
            effective_date: None,
            status: "active".to_string(),
        }];

        assert!(lic.has_blocking_restriction("crypto_exchange"));
        assert!(!lic.has_blocking_restriction("payment_services"));
    }

    #[test]
    fn test_evaluate_compliance_blocked_by_restriction() {
        let mut lic = make_test_license("lic-001", LicenseStatus::Active);
        lic.restrictions = vec![LicenseRestriction {
            restriction_id: "r-001".to_string(),
            restriction_type: "activity".to_string(),
            description: "No payment_services".to_string(),
            blocked_activities: vec!["payment_services".to_string()],
            blocked_jurisdictions: vec![],
            allowed_jurisdictions: vec![],
            blocked_products: vec![],
            blocked_client_types: vec![],
            max_leverage: None,
            effective_date: None,
            status: "active".to_string(),
        }];

        assert_eq!(
            lic.evaluate_compliance("payment_services", "2026-06-15"),
            LicenseComplianceState::NonCompliant
        );
    }

    #[test]
    fn test_license_permits_activity_via_permissions() {
        let mut lic = make_test_license("lic-001", LicenseStatus::Active);
        lic.permitted_activities = vec![];
        lic.permissions = vec![LicensePermission {
            permission_id: "p-001".to_string(),
            activity: "custody_services".to_string(),
            scope: BTreeMap::new(),
            limits: BTreeMap::new(),
            effective_date: None,
            status: "active".to_string(),
        }];

        assert!(lic.permits_activity("custody_services"));
        assert!(!lic.permits_activity("payment_services"));
    }

    #[test]
    fn test_license_permits_activity_inactive_permission() {
        let mut lic = make_test_license("lic-001", LicenseStatus::Active);
        lic.permitted_activities = vec![];
        lic.permissions = vec![LicensePermission {
            permission_id: "p-001".to_string(),
            activity: "custody_services".to_string(),
            scope: BTreeMap::new(),
            limits: BTreeMap::new(),
            effective_date: None,
            status: "revoked".to_string(),
        }];

        assert!(!lic.permits_activity("custody_services"));
    }

    #[test]
    fn test_license_no_expiry_date() {
        let mut lic = make_test_license("lic-001", LicenseStatus::Active);
        lic.expiry_date = None;
        assert!(!lic.is_expired("2099-12-31"));
    }

    #[test]
    fn test_license_expired_exactly_on_boundary() {
        let mut lic = make_test_license("lic-001", LicenseStatus::Active);
        lic.expiry_date = Some("2026-06-15".to_string());
        assert!(!lic.is_expired("2026-06-15"));
        assert!(lic.is_expired("2026-06-16"));
    }

    #[test]
    fn test_license_type_definition_serialization() {
        let lt = LicenseTypeDefinition {
            license_type_id: "fsra:emi".to_string(),
            name: "Electronic Money Institution".to_string(),
            description: "License to issue e-money".to_string(),
            regulator_id: "fsra".to_string(),
            category: Some("payments".to_string()),
            permitted_activities: vec!["issuing_e_money".to_string()],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        };
        let json = serde_json::to_value(&lt).unwrap();
        assert_eq!(json["license_type_id"], "fsra:emi");
        assert_eq!(json["validity_period_years"], 3);
    }

    #[test]
    fn test_licensepack_regulator_serialization() {
        let reg = LicensepackRegulator {
            regulator_id: "fsra".to_string(),
            name: "FSRA".to_string(),
            jurisdiction_id: "pk-kp-rsez".to_string(),
            registry_url: Some("https://registry.example.com".to_string()),
            did: Some("did:web:fsra.gov.pk".to_string()),
            api_capabilities: vec!["realtime_query".to_string()],
        };
        let json = serde_json::to_value(&reg).unwrap();
        assert_eq!(json["regulator_id"], "fsra");
        assert_eq!(json["registry_url"], "https://registry.example.com");
        assert_eq!(json["api_capabilities"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_licensepack_metadata_serialization() {
        let meta = make_test_metadata();
        let json = serde_json::to_value(&meta).unwrap();
        assert_eq!(
            json["licensepack_id"],
            "licensepack:pk:financial:2026-01-15"
        );
        assert_eq!(json["domain"], "financial");
        assert_eq!(json["snapshot_type"], "quarterly");
    }

    #[test]
    fn test_licensepack_metadata_with_delta() {
        let mut meta = make_test_metadata();
        let mut delta = BTreeMap::new();
        delta.insert("licenses_granted".to_string(), serde_json::json!(5));
        meta.delta = Some(delta);

        let json = serde_json::to_value(&meta).unwrap();
        assert_eq!(json["delta"]["licenses_granted"], 5);
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
}
