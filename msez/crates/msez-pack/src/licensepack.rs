//! # Licensepack — License Lifecycle Management
//!
//! Manages the full lifecycle of business licenses, professional certifications,
//! and regulatory authorizations (15+ categories for Pakistan deployment).
//!
//! ## Data Model
//!
//! - [`LicenseStatus`]: License state (Active, Suspended, Revoked, Expired, Pending, Surrendered).
//! - [`LicenseDomain`]: License domain category (Financial, Corporate, Professional, Trade, Insurance, Mixed).
//! - [`LicenseComplianceState`]: Compliance tensor state for LICENSING domain.
//! - [`LicenseCondition`]: Condition attached to a license (capital, operational, etc.).
//! - [`LicensePermission`]: Permission granted under a license (activity + scope + limits).
//! - [`LicenseRestriction`]: Restriction on a license (geographic, activity, product, client_type).
//! - [`LicenseHolder`]: License holder profile with identity and ownership data.
//! - [`License`]: Individual license record with conditions, permissions, restrictions.
//! - [`Licensepack`]: Content-addressed snapshot of jurisdictional licensing state.
//!
//! ## Digest Computation
//!
//! Licensepack digests follow the same content-addressed pattern as lawpack/regpack:
//!
//! ```text
//! SHA256( b"msez-licensepack-v1\0"
//!       + canonical(metadata) + b"\0"
//!       + for each license_type in sorted(license_types.keys()):
//!           "license-types/{type_id}\0" + canonical(type_data) + b"\0"
//!       + for each license in sorted(licenses.keys()):
//!           "licenses/{license_id}\0" + canonical(license_data) + b"\0"
//!           + conditions... + permissions... + restrictions...
//!       + for each holder in sorted(holders.keys()):
//!           "holders/{holder_id}\0" + canonical(holder_data) + b"\0" )
//! ```
//!
//! All canonicalization goes through [`CanonicalBytes`](msez_core::CanonicalBytes)
//! for cross-language digest equality with the Python implementation.
//!
//! ## Spec Reference
//!
//! Ports Python `tools/licensepack.py` with cross-language digest compatibility.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use msez_core::{CanonicalBytes, ContentDigest, JurisdictionId};

use crate::error::PackResult;
use crate::parser;

/// Digest prefix for licensepack v1 computation.
const LICENSEPACK_DIGEST_PREFIX: &[u8] = b"msez-licensepack-v1\0";

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// License status enumeration.
///
/// Mirrors Python `tools/licensepack.py:LicenseStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseStatus {
    /// License is currently active and valid.
    Active,
    /// License is temporarily suspended.
    Suspended,
    /// License has been revoked for cause (terminal).
    Revoked,
    /// License has expired (terminal).
    Expired,
    /// License application is pending.
    Pending,
    /// License was voluntarily surrendered (terminal).
    Surrendered,
}

impl LicenseStatus {
    /// Whether this status represents a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Revoked | Self::Expired | Self::Surrendered)
    }

    /// String representation matching Python enum values.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Revoked => "revoked",
            Self::Expired => "expired",
            Self::Pending => "pending",
            Self::Surrendered => "surrendered",
        }
    }
}

impl std::fmt::Display for LicenseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// License domain categories.
///
/// Mirrors Python `tools/licensepack.py:LicenseDomain`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseDomain {
    /// Financial services licenses.
    Financial,
    /// Corporate services licenses.
    Corporate,
    /// Professional certifications.
    Professional,
    /// Trade licenses.
    Trade,
    /// Insurance licenses.
    Insurance,
    /// Mixed/multi-domain licenses.
    Mixed,
}

impl LicenseDomain {
    /// String representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Financial => "financial",
            Self::Corporate => "corporate",
            Self::Professional => "professional",
            Self::Trade => "trade",
            Self::Insurance => "insurance",
            Self::Mixed => "mixed",
        }
    }
}

impl std::fmt::Display for LicenseDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Compliance tensor states for the LICENSING domain.
///
/// Mirrors Python `tools/licensepack.py:ComplianceState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LicenseComplianceState {
    /// License is valid and permits the activity.
    Compliant,
    /// No valid license, expired, or activity not permitted.
    NonCompliant,
    /// License application is pending.
    Pending,
    /// License is temporarily suspended.
    Suspended,
    /// License state is unknown.
    Unknown,
}

impl LicenseComplianceState {
    /// String representation matching Python enum values.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Compliant => "COMPLIANT",
            Self::NonCompliant => "NON_COMPLIANT",
            Self::Pending => "PENDING",
            Self::Suspended => "SUSPENDED",
            Self::Unknown => "UNKNOWN",
        }
    }
}

impl std::fmt::Display for LicenseComplianceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Data Types
// ---------------------------------------------------------------------------

/// A condition attached to a license.
///
/// Mirrors Python `tools/licensepack.py:LicenseCondition`.
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
    /// Whether this condition is currently active.
    pub fn is_active(&self, today: &str) -> bool {
        if self.status != "active" {
            return false;
        }
        if let Some(ref expiry) = self.expiry_date {
            if expiry.as_str() < today {
                return false;
            }
        }
        true
    }
}

/// A permission granted under a license.
///
/// Mirrors Python `tools/licensepack.py:LicensePermission`.
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
/// Mirrors Python `tools/licensepack.py:LicenseRestriction`.
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
        if self.status != "active" {
            return false;
        }
        if self.blocked_jurisdictions.contains(&"*".to_string()) {
            return !self.allowed_jurisdictions.contains(&jurisdiction.to_string());
        }
        self.blocked_jurisdictions.contains(&jurisdiction.to_string())
    }
}

/// License holder profile.
///
/// Mirrors Python `tools/licensepack.py:LicenseHolder`.
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

/// Individual license record.
///
/// Mirrors Python `tools/licensepack.py:License`.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            Some(expiry) => expiry.as_str() < today,
            None => false,
        }
    }

    /// Whether any active restriction blocks the given activity.
    pub fn has_blocking_restriction(&self, activity: &str) -> bool {
        self.restrictions.iter().any(|r| r.blocks_activity(activity))
    }

    /// Whether the license permits the specified activity.
    pub fn permits_activity(&self, activity: &str) -> bool {
        // Check explicit permitted activities list
        if !self.permitted_activities.is_empty()
            && !self.permitted_activities.contains(&activity.to_string())
        {
            return false;
        }
        // Check permissions
        for perm in &self.permissions {
            if perm.permits_activity(activity) {
                return true;
            }
        }
        // If no permissions defined but activity in permitted_activities, allow
        self.permissions.is_empty() && self.permitted_activities.contains(&activity.to_string())
    }

    /// Evaluate compliance state for the LICENSING domain.
    ///
    /// Mirrors Python `License.evaluate_compliance()`.
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

// ---------------------------------------------------------------------------
// License Type Definition
// ---------------------------------------------------------------------------

/// License type definition.
///
/// Mirrors Python `tools/licensepack.py:LicenseType`.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
/// Mirrors Python `tools/licensepack.py:Regulator`.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
/// Mirrors Python `tools/licensepack.py:LicensepackMetadata`.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

// ---------------------------------------------------------------------------
// Licensepack
// ---------------------------------------------------------------------------

/// Content-addressed snapshot of jurisdictional licensing state.
///
/// Completes the pack trilogy:
/// - Lawpack: Static law (statutes, regulations)
/// - Regpack: Dynamic guidance (sanctions, calendars)
/// - Licensepack: Live registry (licenses, holders, conditions)
///
/// Mirrors Python `tools/licensepack.py:LicensePack`.
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
        self.licenses
            .values()
            .filter(|lic| lic.holder_did.as_deref() == Some(holder_did))
            .collect()
    }

    /// Get all active licenses.
    pub fn get_active_licenses(&self) -> Vec<&License> {
        self.licenses.values().filter(|lic| lic.is_active()).collect()
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
                return (true, LicenseComplianceState::Compliant, Some(lic.license_id.clone()));
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
    /// Mirrors Python `LicensePack.compute_digest()`.
    pub fn compute_digest(&self) -> PackResult<String> {
        let mut hasher = Sha256::new();
        hasher.update(LICENSEPACK_DIGEST_PREFIX);

        // Add metadata
        if let Some(ref meta) = self.metadata {
            let meta_value = serde_json::to_value(meta)?;
            let meta_canonical = CanonicalBytes::from_value(meta_value)?;
            hasher.update(meta_canonical.as_bytes());
            hasher.update(b"\0");
        }

        // Add license types (sorted by key — BTreeMap guarantees this)
        for (type_id, lt) in &self.license_types {
            let lt_value = serde_json::to_value(lt)?;
            let lt_canonical = CanonicalBytes::from_value(lt_value)?;
            hasher.update(format!("license-types/{type_id}\0").as_bytes());
            hasher.update(lt_canonical.as_bytes());
            hasher.update(b"\0");
        }

        // Add licenses (sorted by key)
        for (license_id, lic) in &self.licenses {
            // Serialize license core fields (excluding conditions/permissions/restrictions
            // which are added separately to match Python structure)
            let lic_value = serde_json::to_value(lic)?;
            let lic_canonical = CanonicalBytes::from_value(lic_value)?;
            hasher.update(format!("licenses/{license_id}\0").as_bytes());
            hasher.update(lic_canonical.as_bytes());
            hasher.update(b"\0");

            // Add conditions (sorted by condition_id)
            let mut sorted_conditions = lic.conditions.clone();
            sorted_conditions.sort_by(|a, b| a.condition_id.cmp(&b.condition_id));
            for cond in &sorted_conditions {
                let cond_value = serde_json::to_value(cond)?;
                let cond_canonical = CanonicalBytes::from_value(cond_value)?;
                hasher.update(
                    format!("licenses/{license_id}/conditions/{}\0", cond.condition_id).as_bytes(),
                );
                hasher.update(cond_canonical.as_bytes());
                hasher.update(b"\0");
            }

            // Add permissions (sorted by permission_id)
            let mut sorted_permissions = lic.permissions.clone();
            sorted_permissions.sort_by(|a, b| a.permission_id.cmp(&b.permission_id));
            for perm in &sorted_permissions {
                let perm_value = serde_json::to_value(perm)?;
                let perm_canonical = CanonicalBytes::from_value(perm_value)?;
                hasher.update(
                    format!("licenses/{license_id}/permissions/{}\0", perm.permission_id)
                        .as_bytes(),
                );
                hasher.update(perm_canonical.as_bytes());
                hasher.update(b"\0");
            }

            // Add restrictions (sorted by restriction_id)
            let mut sorted_restrictions = lic.restrictions.clone();
            sorted_restrictions.sort_by(|a, b| a.restriction_id.cmp(&b.restriction_id));
            for rest in &sorted_restrictions {
                let rest_value = serde_json::to_value(rest)?;
                let rest_canonical = CanonicalBytes::from_value(rest_value)?;
                hasher.update(
                    format!(
                        "licenses/{license_id}/restrictions/{}\0",
                        rest.restriction_id
                    )
                    .as_bytes(),
                );
                hasher.update(rest_canonical.as_bytes());
                hasher.update(b"\0");
            }
        }

        // Add holders (sorted by key)
        for (holder_id, holder) in &self.holders {
            let holder_value = serde_json::to_value(holder)?;
            let holder_canonical = CanonicalBytes::from_value(holder_value)?;
            hasher.update(format!("holders/{holder_id}\0").as_bytes());
            hasher.update(holder_canonical.as_bytes());
            hasher.update(b"\0");
        }

        let result = hasher.finalize();
        Ok(result.iter().map(|b| format!("{b:02x}")).collect())
    }

    /// Compute delta from a previous licensepack.
    ///
    /// Mirrors Python `LicensePack.compute_delta()`.
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
                    LicenseStatus::Active => granted.push((**id).clone()),
                    LicenseStatus::Suspended => suspended.push((**id).clone()),
                    _ => {}
                }
            }
        }

        for id in &removed_ids {
            if let Some(prev_lic) = previous.licenses.get(**id) {
                if prev_lic.status == LicenseStatus::Active {
                    revoked.push((**id).clone());
                }
            }
        }

        // Status changes in existing licenses
        for id in curr_ids.intersection(&prev_ids) {
            if let (Some(curr), Some(prev)) =
                (self.licenses.get(*id), previous.licenses.get(*id))
            {
                if curr.status != prev.status {
                    match (prev.status, curr.status) {
                        (LicenseStatus::Suspended, LicenseStatus::Active) => {
                            reinstated.push((*id).clone());
                        }
                        (_, LicenseStatus::Suspended) => {
                            suspended.push((*id).clone());
                        }
                        (_, LicenseStatus::Revoked) => {
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

// ---------------------------------------------------------------------------
// Licensepack Reference and Lock
// ---------------------------------------------------------------------------

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
///
/// Mirrors Python `tools/licensepack.py:LicensepackLock`.
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
    use serde_json::json;

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
    fn test_license_status_terminal() {
        assert!(!LicenseStatus::Active.is_terminal());
        assert!(!LicenseStatus::Suspended.is_terminal());
        assert!(!LicenseStatus::Pending.is_terminal());
        assert!(LicenseStatus::Revoked.is_terminal());
        assert!(LicenseStatus::Expired.is_terminal());
        assert!(LicenseStatus::Surrendered.is_terminal());
    }

    #[test]
    fn test_license_status_serialization() {
        let json = serde_json::to_string(&LicenseStatus::Active).unwrap();
        assert_eq!(json, "\"active\"");
        let parsed: LicenseStatus = serde_json::from_str("\"suspended\"").unwrap();
        assert_eq!(parsed, LicenseStatus::Suspended);
    }

    #[test]
    fn test_license_domain_serialization() {
        let json = serde_json::to_string(&LicenseDomain::Financial).unwrap();
        assert_eq!(json, "\"financial\"");
    }

    #[test]
    fn test_compliance_state_values() {
        assert_eq!(LicenseComplianceState::Compliant.as_str(), "COMPLIANT");
        assert_eq!(LicenseComplianceState::NonCompliant.as_str(), "NON_COMPLIANT");
        assert_eq!(LicenseComplianceState::Pending.as_str(), "PENDING");
        assert_eq!(LicenseComplianceState::Suspended.as_str(), "SUSPENDED");
        assert_eq!(LicenseComplianceState::Unknown.as_str(), "UNKNOWN");
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
        assert_eq!(delta["licenses_granted"], 1); // lic-003 is new+active
        assert_eq!(delta["licenses_revoked"], 1); // lic-002 removed, was active
        assert_eq!(delta["licenses_suspended"], 1); // lic-001 changed to suspended
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
    fn test_evaluate_license_compliance_fn() {
        let mut pack = Licensepack::new(
            JurisdictionId::new("pk".to_string()).unwrap(),
            "Test".to_string(),
        );
        pack.add_license(make_test_license("lic-001", LicenseStatus::Active));

        let state = evaluate_license_compliance("lic-001", "payment_services", &pack, "2026-06-15");
        assert_eq!(state, LicenseComplianceState::Compliant);

        let state = evaluate_license_compliance("lic-999", "payment_services", &pack, "2026-06-15");
        assert_eq!(state, LicenseComplianceState::NonCompliant);
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
}
