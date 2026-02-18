//! # Data Sovereignty Enforcement (M-012)
//!
//! Programmatic enforcement of data sovereignty constraints. For sovereign
//! deployment (e.g., Pakistan GovOS), all data, compute, AI models, and
//! inference must remain within the jurisdictional boundary.
//!
//! This module provides:
//! - [`SovereigntyPolicy`] — per-jurisdiction rules defining which data
//!   categories may cross jurisdictional boundaries
//! - [`SovereigntyEnforcer`] — validates data access against policy before
//!   any cross-boundary operation
//! - [`DataCategory`] — classification of data types for sovereignty rules
//!
//! ## Design Rationale
//!
//! Sovereignty enforcement is in `mez-core` (not `mez-api`) because the
//! invariant must be checked at the type level, not just at the HTTP boundary.
//! Any crate that handles data routing must be able to call
//! `SovereigntyEnforcer::check()` without depending on the API layer.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::JurisdictionId;

/// Categories of data subject to sovereignty constraints.
///
/// Each category may have different residency and replication rules
/// depending on the jurisdiction's data protection regime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataCategory {
    /// Personally Identifiable Information (CNIC, NTN, biometric data).
    Pii,
    /// Financial transaction records (payments, withholdings, tax events).
    Financial,
    /// Tax assessment and filing data (FBR IRIS reports).
    Tax,
    /// Corporate formation and governance records.
    Corporate,
    /// Compliance evaluation results (tensor snapshots, VC audit logs).
    Compliance,
    /// Cryptographic key material (signing keys, attestation keys).
    KeyMaterial,
    /// Aggregate statistics and anonymized analytics.
    Analytics,
    /// Publicly available regulatory data (laws, sanctions lists).
    PublicRegulatory,
}

impl DataCategory {
    /// All data categories.
    pub fn all() -> &'static [DataCategory] {
        &[
            Self::Pii,
            Self::Financial,
            Self::Tax,
            Self::Corporate,
            Self::Compliance,
            Self::KeyMaterial,
            Self::Analytics,
            Self::PublicRegulatory,
        ]
    }

    /// The canonical string name.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pii => "pii",
            Self::Financial => "financial",
            Self::Tax => "tax",
            Self::Corporate => "corporate",
            Self::Compliance => "compliance",
            Self::KeyMaterial => "key_material",
            Self::Analytics => "analytics",
            Self::PublicRegulatory => "public_regulatory",
        }
    }
}

impl std::fmt::Display for DataCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Outcome of a sovereignty check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SovereigntyVerdict {
    /// The data transfer is permitted under the sovereignty policy.
    Allowed,
    /// The data transfer is denied. The contained string describes why.
    Denied(String),
}

impl SovereigntyVerdict {
    /// Whether the verdict permits the operation.
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }
}

/// Per-jurisdiction sovereignty policy defining which data categories
/// may be replicated to which target jurisdictions.
///
/// The default policy for any jurisdiction not explicitly configured is
/// **deny all cross-boundary transfers** — the safe default for sovereign
/// deployments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SovereigntyPolicy {
    /// The jurisdiction this policy governs.
    pub jurisdiction_id: JurisdictionId,
    /// Per-category rules. Each entry maps a data category to the set of
    /// jurisdictions that data may be replicated to. An empty set means
    /// the category is confined to the home jurisdiction.
    pub allowed_targets: BTreeMap<DataCategory, BTreeSet<String>>,
    /// Data categories that must never leave the jurisdiction under any
    /// circumstances. Overrides `allowed_targets`.
    pub confined_categories: BTreeSet<DataCategory>,
}

impl SovereigntyPolicy {
    /// Create a policy that denies all cross-boundary transfers.
    ///
    /// This is the default for sovereign deployments. Categories must be
    /// explicitly allowed via [`allow`](Self::allow).
    pub fn deny_all(jurisdiction_id: JurisdictionId) -> Self {
        Self {
            jurisdiction_id,
            allowed_targets: BTreeMap::new(),
            confined_categories: BTreeSet::new(),
        }
    }

    /// Create the standard Pakistan GovOS sovereignty policy.
    ///
    /// Per the Pakistan GovOS architecture: "All data, compute, AI models
    /// & inference — Pakistani jurisdiction, Pakistani infrastructure,
    /// Pakistani engineers."
    ///
    /// - PII, Financial, Tax, KeyMaterial: confined to PK (no replication)
    /// - Corporate, Compliance: confined to PK
    /// - Analytics: may be shared with approved corridor partners
    /// - PublicRegulatory: unrestricted (public data)
    pub fn pakistan_govos() -> Self {
        // SAFETY: "PK" is a non-empty string literal; JurisdictionId::new only
        // rejects empty strings, so this construction is infallible.
        let jid = JurisdictionId::new("PK")
            .expect("BUG: static non-empty string rejected by JurisdictionId::new");

        let mut policy = Self::deny_all(jid);

        // Confined categories — never leave Pakistan under any circumstances.
        policy.confined_categories.insert(DataCategory::Pii);
        policy.confined_categories.insert(DataCategory::Financial);
        policy.confined_categories.insert(DataCategory::Tax);
        policy.confined_categories.insert(DataCategory::KeyMaterial);
        policy.confined_categories.insert(DataCategory::Corporate);
        policy.confined_categories.insert(DataCategory::Compliance);

        // Analytics may be shared with approved corridor partners.
        let mut analytics_targets = BTreeSet::new();
        analytics_targets.insert("ae".to_string()); // UAE corridor
        analytics_targets.insert("sa".to_string()); // KSA corridor
        analytics_targets.insert("cn".to_string()); // China corridor
        policy
            .allowed_targets
            .insert(DataCategory::Analytics, analytics_targets);

        // Public regulatory data is unrestricted.
        let mut public_targets = BTreeSet::new();
        public_targets.insert("*".to_string()); // wildcard: any jurisdiction
        policy
            .allowed_targets
            .insert(DataCategory::PublicRegulatory, public_targets);

        policy
    }

    /// Allow a data category to be replicated to a specific target jurisdiction.
    pub fn allow(&mut self, category: DataCategory, target_jurisdiction: &str) {
        self.allowed_targets
            .entry(category)
            .or_default()
            .insert(target_jurisdiction.to_string());
    }

    /// Mark a data category as confined — it may never leave the jurisdiction.
    pub fn confine(&mut self, category: DataCategory) {
        self.confined_categories.insert(category);
    }
}

/// Enforces sovereignty policy on data access operations.
///
/// Call [`check`](Self::check) before any cross-boundary data operation
/// (replication, API response to foreign jurisdiction, backup to external
/// storage). The enforcer logs all checks for audit trail purposes.
#[derive(Debug, Clone)]
pub struct SovereigntyEnforcer {
    policy: SovereigntyPolicy,
}

impl SovereigntyEnforcer {
    /// Create an enforcer with the given sovereignty policy.
    pub fn new(policy: SovereigntyPolicy) -> Self {
        Self { policy }
    }

    /// Check whether a data transfer is permitted.
    ///
    /// Returns [`SovereigntyVerdict::Allowed`] if the policy permits the
    /// transfer, or [`SovereigntyVerdict::Denied`] with a reason if not.
    pub fn check(&self, category: DataCategory, target_jurisdiction: &str) -> SovereigntyVerdict {
        // Same-jurisdiction transfers are always allowed.
        if target_jurisdiction == self.policy.jurisdiction_id.as_str() {
            return SovereigntyVerdict::Allowed;
        }

        // Confined categories are never allowed to leave.
        if self.policy.confined_categories.contains(&category) {
            return SovereigntyVerdict::Denied(format!(
                "{} data is confined to {} and may not be transferred to {}",
                category, self.policy.jurisdiction_id, target_jurisdiction,
            ));
        }

        // Check the allowed targets for this category.
        match self.policy.allowed_targets.get(&category) {
            Some(targets) => {
                if targets.contains("*") || targets.contains(target_jurisdiction) {
                    SovereigntyVerdict::Allowed
                } else {
                    SovereigntyVerdict::Denied(format!(
                        "{} data transfer from {} to {} is not in the allowed target list",
                        category, self.policy.jurisdiction_id, target_jurisdiction,
                    ))
                }
            }
            None => SovereigntyVerdict::Denied(format!(
                "no sovereignty policy allows {} data transfer from {} to {}",
                category, self.policy.jurisdiction_id, target_jurisdiction,
            )),
        }
    }

    /// Access the underlying policy.
    pub fn policy(&self) -> &SovereigntyPolicy {
        &self.policy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pakistan_govos_denies_pii_export() {
        let enforcer = SovereigntyEnforcer::new(SovereigntyPolicy::pakistan_govos());
        let verdict = enforcer.check(DataCategory::Pii, "ae");
        assert!(!verdict.is_allowed());
        if let SovereigntyVerdict::Denied(reason) = &verdict {
            assert!(reason.contains("confined"));
        }
    }

    #[test]
    fn pakistan_govos_denies_financial_export() {
        let enforcer = SovereigntyEnforcer::new(SovereigntyPolicy::pakistan_govos());
        assert!(!enforcer.check(DataCategory::Financial, "ae").is_allowed());
    }

    #[test]
    fn pakistan_govos_denies_tax_export() {
        let enforcer = SovereigntyEnforcer::new(SovereigntyPolicy::pakistan_govos());
        assert!(!enforcer.check(DataCategory::Tax, "ae").is_allowed());
    }

    #[test]
    fn pakistan_govos_denies_key_material_export() {
        let enforcer = SovereigntyEnforcer::new(SovereigntyPolicy::pakistan_govos());
        assert!(!enforcer.check(DataCategory::KeyMaterial, "ae").is_allowed());
    }

    #[test]
    fn pakistan_govos_allows_analytics_to_corridor_partners() {
        let enforcer = SovereigntyEnforcer::new(SovereigntyPolicy::pakistan_govos());
        assert!(enforcer.check(DataCategory::Analytics, "ae").is_allowed());
        assert!(enforcer.check(DataCategory::Analytics, "sa").is_allowed());
        assert!(enforcer.check(DataCategory::Analytics, "cn").is_allowed());
    }

    #[test]
    fn pakistan_govos_denies_analytics_to_non_partner() {
        let enforcer = SovereigntyEnforcer::new(SovereigntyPolicy::pakistan_govos());
        assert!(!enforcer.check(DataCategory::Analytics, "us").is_allowed());
    }

    #[test]
    fn pakistan_govos_allows_public_regulatory_anywhere() {
        let enforcer = SovereigntyEnforcer::new(SovereigntyPolicy::pakistan_govos());
        assert!(enforcer
            .check(DataCategory::PublicRegulatory, "ae")
            .is_allowed());
        assert!(enforcer
            .check(DataCategory::PublicRegulatory, "us")
            .is_allowed());
        assert!(enforcer
            .check(DataCategory::PublicRegulatory, "cn")
            .is_allowed());
    }

    #[test]
    fn same_jurisdiction_always_allowed() {
        let enforcer = SovereigntyEnforcer::new(SovereigntyPolicy::pakistan_govos());
        // All categories allowed within PK.
        for category in DataCategory::all() {
            assert!(
                enforcer.check(*category, "PK").is_allowed(),
                "{category} should be allowed within home jurisdiction"
            );
        }
    }

    #[test]
    fn deny_all_policy_rejects_everything() {
        let jid = JurisdictionId::new("test").expect("test jurisdiction");
        let enforcer = SovereigntyEnforcer::new(SovereigntyPolicy::deny_all(jid));
        for category in DataCategory::all() {
            assert!(
                !enforcer.check(*category, "other").is_allowed(),
                "{category} should be denied under deny-all policy"
            );
        }
    }

    #[test]
    fn custom_allow_rule() {
        let jid = JurisdictionId::new("test").expect("test jurisdiction");
        let mut policy = SovereigntyPolicy::deny_all(jid);
        policy.allow(DataCategory::Analytics, "partner");

        let enforcer = SovereigntyEnforcer::new(policy);
        assert!(enforcer
            .check(DataCategory::Analytics, "partner")
            .is_allowed());
        assert!(!enforcer
            .check(DataCategory::Analytics, "other")
            .is_allowed());
        assert!(!enforcer.check(DataCategory::Pii, "partner").is_allowed());
    }

    #[test]
    fn confine_overrides_allow() {
        let jid = JurisdictionId::new("test").expect("test jurisdiction");
        let mut policy = SovereigntyPolicy::deny_all(jid);
        policy.allow(DataCategory::Pii, "partner");
        policy.confine(DataCategory::Pii);

        let enforcer = SovereigntyEnforcer::new(policy);
        // Confinement overrides the allow rule.
        assert!(!enforcer.check(DataCategory::Pii, "partner").is_allowed());
    }

    #[test]
    fn data_category_all_returns_eight() {
        assert_eq!(DataCategory::all().len(), 8);
    }

    #[test]
    fn data_category_display_roundtrip() {
        for cat in DataCategory::all() {
            let s = cat.to_string();
            assert!(!s.is_empty());
            assert_eq!(s, cat.as_str());
        }
    }

    #[test]
    fn sovereignty_policy_serde_roundtrip() {
        let policy = SovereigntyPolicy::pakistan_govos();
        let json = serde_json::to_string(&policy).expect("serialize");
        let recovered: SovereigntyPolicy = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(
            recovered.jurisdiction_id.as_str(),
            policy.jurisdiction_id.as_str()
        );
        assert_eq!(
            recovered.confined_categories.len(),
            policy.confined_categories.len()
        );
    }
}
