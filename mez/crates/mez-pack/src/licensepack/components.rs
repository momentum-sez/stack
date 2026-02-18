//! License sub-record types: conditions, permissions, restrictions, and holders.
//!
//! These types model the constraints and grants attached to individual licenses.
//! They are referenced by [`License`](super::License) and participate in
//! content-addressed digest computation.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A condition attached to a license.
///
/// Conditions represent ongoing requirements (capital adequacy, operational
/// standards, reporting obligations) that a license holder must satisfy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseCondition {
    /// Unique condition identifier.
    pub condition_id: String,
    /// Condition type: "capital", "operational", "activity_restriction", "reporting".
    pub condition_type: String,
    /// Human-readable description.
    pub description: String,
    /// Metric to monitor (e.g., "minimum_capital").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metric: Option<String>,
    /// Threshold value (string decimal for precision).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold: Option<String>,
    /// Currency for monetary thresholds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// Comparison operator: ">=", "<=", "==", "<", ">".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator: Option<String>,
    /// Monitoring frequency: "continuous", "daily", "quarterly", "annual".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency: Option<String>,
    /// Reporting frequency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reporting_frequency: Option<String>,
    /// Effective date (YYYY-MM-DD).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_date: Option<String>,
    /// Expiry date (YYYY-MM-DD).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expiry_date: Option<String>,
    /// Condition status: "active", "waived", "expired".
    #[serde(default = "default_active_status")]
    pub status: String,
}

fn default_active_status() -> String {
    "active".to_string()
}

impl LicenseCondition {
    /// Whether the condition has valid required fields.
    ///
    /// Returns `false` if `condition_id` is empty, which indicates
    /// the condition was not properly initialized.
    pub fn is_valid(&self) -> bool {
        !self.condition_id.trim().is_empty()
    }

    /// Whether this condition is currently active.
    pub fn is_active(&self, today: &str) -> bool {
        if self.status != "active" {
            return false;
        }
        if let Some(ref expiry) = self.expiry_date {
            if date_before(expiry, today) {
                return false;
            }
        }
        true
    }
}

/// Compare two date strings in YYYY-MM-DD format.
///
/// Returns `true` if date `a` is strictly before date `b`.
/// If either date fails to parse, logs a warning and returns `true`
/// (fail-safe: in a compliance context this function gates license/condition
/// expiry checks, so malformed dates must be treated as "already expired"
/// rather than silently keeping an invalid license active).
pub(crate) fn date_before(a: &str, b: &str) -> bool {
    let da = match chrono::NaiveDate::parse_from_str(a, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => {
            tracing::warn!(date = %a, "invalid date format in date_before — fail-safe: treating as expired");
            return true;
        }
    };
    let db = match chrono::NaiveDate::parse_from_str(b, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => {
            tracing::warn!(date = %b, "invalid date format in date_before — fail-safe: treating as expired");
            return true;
        }
    };
    da < db
}

/// A permission granted under a license.
///
/// Permissions specify which activities a license holder is authorized to
/// perform, optionally with scope and limit constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensePermission {
    /// Unique permission identifier.
    pub permission_id: String,
    /// Activity this permission covers.
    pub activity: String,
    /// Scope details.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub scope: BTreeMap<String, serde_json::Value>,
    /// Limits on the permission.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub limits: BTreeMap<String, serde_json::Value>,
    /// Effective date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_date: Option<String>,
    /// Permission status.
    #[serde(default = "default_active_status")]
    pub status: String,
}

impl LicensePermission {
    /// Whether this permission allows the specified activity.
    pub fn permits_activity(&self, activity: &str) -> bool {
        self.activity == activity && self.status == "active"
    }
}

/// A restriction on a license.
///
/// Restrictions block specific activities, jurisdictions, products, or client
/// types. A wildcard `"*"` in `blocked_jurisdictions` blocks all except
/// those listed in `allowed_jurisdictions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseRestriction {
    /// Unique restriction identifier.
    pub restriction_id: String,
    /// Restriction type: "geographic", "activity", "product", "client_type".
    pub restriction_type: String,
    /// Human-readable description.
    pub description: String,
    /// Blocked jurisdictions (use "*" for all-except-allowed).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_jurisdictions: Vec<String>,
    /// Allowed jurisdictions (exceptions to "*" block).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_jurisdictions: Vec<String>,
    /// Blocked activities.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_activities: Vec<String>,
    /// Blocked products.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_products: Vec<String>,
    /// Blocked client types.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_client_types: Vec<String>,
    /// Maximum leverage ratio.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_leverage: Option<String>,
    /// Effective date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_date: Option<String>,
    /// Restriction status.
    #[serde(default = "default_active_status")]
    pub status: String,
}

impl LicenseRestriction {
    /// Whether this restriction blocks the specified activity.
    pub fn blocks_activity(&self, activity: &str) -> bool {
        self.blocked_activities.contains(&activity.to_string()) && self.status == "active"
    }

    /// Whether this restriction blocks the specified jurisdiction.
    pub fn blocks_jurisdiction(&self, jurisdiction: &str) -> bool {
        if jurisdiction.is_empty() {
            return false;
        }
        if self.status != "active" {
            return false;
        }
        if self.blocked_jurisdictions.contains(&"*".to_string()) {
            return !self
                .allowed_jurisdictions
                .contains(&jurisdiction.to_string());
        }
        self.blocked_jurisdictions
            .contains(&jurisdiction.to_string())
    }
}

/// License holder profile.
///
/// Represents the entity that holds one or more licenses, including
/// identity, ownership, and contact information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseHolder {
    /// Unique holder identifier.
    pub holder_id: String,
    /// Entity type.
    pub entity_type: String,
    /// Legal name.
    pub legal_name: String,
    /// Trading names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trading_names: Vec<String>,
    /// Registration number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_number: Option<String>,
    /// Incorporation date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incorporation_date: Option<String>,
    /// Jurisdiction of incorporation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jurisdiction_of_incorporation: Option<String>,
    /// Decentralized identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub did: Option<String>,
    /// Registered address.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub registered_address: BTreeMap<String, String>,
    /// Contact information.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub contact: BTreeMap<String, String>,
    /// Controllers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub controllers: Vec<serde_json::Value>,
    /// Beneficial owners.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beneficial_owners: Vec<serde_json::Value>,
    /// Group structure.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub group_structure: BTreeMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condition_is_active() {
        let cond = LicenseCondition {
            condition_id: "c-001".to_string(),
            condition_type: "capital".to_string(),
            description: "Min capital".to_string(),
            metric: Some("minimum_capital".to_string()),
            threshold: Some("1000000".to_string()),
            currency: Some("PKR".to_string()),
            operator: Some(">=".to_string()),
            frequency: Some("continuous".to_string()),
            reporting_frequency: None,
            effective_date: None,
            expiry_date: Some("2027-12-31".to_string()),
            status: "active".to_string(),
        };
        assert!(cond.is_active("2026-06-15"));
        assert!(!cond.is_active("2028-01-01"));
    }

    #[test]
    fn test_condition_is_active_waived() {
        let cond = LicenseCondition {
            condition_id: "c-001".to_string(),
            condition_type: "capital".to_string(),
            description: "Waived condition".to_string(),
            metric: None,
            threshold: None,
            currency: None,
            operator: None,
            frequency: None,
            reporting_frequency: None,
            effective_date: None,
            expiry_date: None,
            status: "waived".to_string(),
        };
        assert!(!cond.is_active("2026-06-15"));
    }

    #[test]
    fn test_condition_is_active_no_expiry() {
        let cond = LicenseCondition {
            condition_id: "c-002".to_string(),
            condition_type: "operational".to_string(),
            description: "No expiry".to_string(),
            metric: None,
            threshold: None,
            currency: None,
            operator: None,
            frequency: None,
            reporting_frequency: None,
            effective_date: None,
            expiry_date: None,
            status: "active".to_string(),
        };
        assert!(cond.is_active("2099-12-31"));
    }

    #[test]
    fn test_permission_permits_activity() {
        let perm = LicensePermission {
            permission_id: "p-001".to_string(),
            activity: "payment_services".to_string(),
            scope: BTreeMap::new(),
            limits: BTreeMap::new(),
            effective_date: None,
            status: "active".to_string(),
        };
        assert!(perm.permits_activity("payment_services"));
        assert!(!perm.permits_activity("crypto_exchange"));
    }

    #[test]
    fn test_license_restriction_blocks_activity() {
        let rest = LicenseRestriction {
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
        };
        assert!(rest.blocks_activity("crypto_exchange"));
        assert!(!rest.blocks_activity("payment_services"));
    }

    #[test]
    fn test_license_restriction_blocks_jurisdiction() {
        let rest = LicenseRestriction {
            restriction_id: "r-002".to_string(),
            restriction_type: "geographic".to_string(),
            description: "Global except US".to_string(),
            blocked_activities: vec![],
            blocked_jurisdictions: vec!["*".to_string()],
            allowed_jurisdictions: vec!["pk".to_string(), "ae".to_string()],
            blocked_products: vec![],
            blocked_client_types: vec![],
            max_leverage: None,
            effective_date: None,
            status: "active".to_string(),
        };
        assert!(rest.blocks_jurisdiction("us"));
        assert!(!rest.blocks_jurisdiction("pk"));
        assert!(!rest.blocks_jurisdiction("ae"));
    }

    #[test]
    fn test_restriction_specific_blocked_jurisdictions() {
        let rest = LicenseRestriction {
            restriction_id: "r-003".to_string(),
            restriction_type: "geographic".to_string(),
            description: "Block specific countries".to_string(),
            blocked_activities: vec![],
            blocked_jurisdictions: vec!["us".to_string(), "cn".to_string()],
            allowed_jurisdictions: vec![],
            blocked_products: vec![],
            blocked_client_types: vec![],
            max_leverage: None,
            effective_date: None,
            status: "active".to_string(),
        };
        assert!(rest.blocks_jurisdiction("us"));
        assert!(rest.blocks_jurisdiction("cn"));
        assert!(!rest.blocks_jurisdiction("pk"));
        assert!(!rest.blocks_jurisdiction("ae"));
    }

    #[test]
    fn test_restriction_inactive_does_not_block() {
        let rest = LicenseRestriction {
            restriction_id: "r-004".to_string(),
            restriction_type: "geographic".to_string(),
            description: "Waived restriction".to_string(),
            blocked_activities: vec!["crypto_exchange".to_string()],
            blocked_jurisdictions: vec!["us".to_string()],
            allowed_jurisdictions: vec![],
            blocked_products: vec![],
            blocked_client_types: vec![],
            max_leverage: None,
            effective_date: None,
            status: "waived".to_string(),
        };
        assert!(!rest.blocks_activity("crypto_exchange"));
        assert!(!rest.blocks_jurisdiction("us"));
    }

    #[test]
    fn test_license_holder_serialization() {
        let holder = LicenseHolder {
            holder_id: "h-001".to_string(),
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
        };
        let json_val = serde_json::to_value(&holder).unwrap();
        assert_eq!(json_val["holder_id"], "h-001");
        assert_eq!(json_val["entity_type"], "company");
        assert_eq!(json_val["legal_name"], "Test Corporation Ltd");
        assert_eq!(json_val["trading_names"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_license_holder_roundtrip() {
        let holder = LicenseHolder {
            holder_id: "h-001".to_string(),
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
        };
        let json_str = serde_json::to_string(&holder).unwrap();
        let deserialized: LicenseHolder = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.holder_id, "h-001");
        assert_eq!(deserialized.did, Some("did:web:test.example".to_string()));
    }

    #[test]
    fn test_license_holder_minimal() {
        let json_str = r#"{"holder_id":"h","entity_type":"company","legal_name":"Test"}"#;
        let holder: LicenseHolder = serde_json::from_str(json_str).unwrap();
        assert_eq!(holder.holder_id, "h");
        assert!(holder.trading_names.is_empty());
        assert!(holder.did.is_none());
        assert!(holder.controllers.is_empty());
    }
}
