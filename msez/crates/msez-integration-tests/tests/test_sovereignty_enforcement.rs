//! # Data Sovereignty Enforcement Integration Tests (M-012)
//!
//! Cross-crate integration tests verifying that the sovereignty module in
//! `msez-core` correctly enforces data residency constraints for sovereign
//! deployments.

use msez_core::sovereignty::{
    DataCategory, SovereigntyEnforcer, SovereigntyPolicy, SovereigntyVerdict,
};
use msez_core::JurisdictionId;

/// Pakistan GovOS deployment: PII, financial, and tax data must never
/// leave the PK jurisdiction.
#[test]
fn pakistan_govos_enforces_data_confinement() {
    let enforcer = SovereigntyEnforcer::new(SovereigntyPolicy::pakistan_govos());

    // Sovereign data categories must be confined.
    let confined = [
        DataCategory::Pii,
        DataCategory::Financial,
        DataCategory::Tax,
        DataCategory::KeyMaterial,
        DataCategory::Corporate,
        DataCategory::Compliance,
    ];

    let foreign_jurisdictions = ["ae", "sa", "cn", "us", "gb", "sg"];

    for category in &confined {
        for foreign in &foreign_jurisdictions {
            let verdict = enforcer.check(*category, foreign);
            assert!(
                !verdict.is_allowed(),
                "{category} must be denied to {foreign} under PK GovOS policy"
            );
            if let SovereigntyVerdict::Denied(reason) = verdict {
                assert!(
                    reason.contains("confined"),
                    "denial reason should mention confinement"
                );
            }
        }
    }
}

/// Pakistan GovOS: analytics may flow to corridor partners (UAE, KSA, China).
#[test]
fn pakistan_analytics_to_corridor_partners() {
    let enforcer = SovereigntyEnforcer::new(SovereigntyPolicy::pakistan_govos());

    // Allowed corridor partner jurisdictions.
    let partners = ["ae", "sa", "cn"];
    for partner in &partners {
        assert!(
            enforcer.check(DataCategory::Analytics, partner).is_allowed(),
            "analytics should be shareable with corridor partner {partner}"
        );
    }

    // Non-partner jurisdictions denied.
    let non_partners = ["us", "gb", "de", "jp"];
    for non_partner in &non_partners {
        assert!(
            !enforcer
                .check(DataCategory::Analytics, non_partner)
                .is_allowed(),
            "analytics should NOT be shareable with non-partner {non_partner}"
        );
    }
}

/// Public regulatory data is unrestricted under Pakistan policy.
#[test]
fn public_regulatory_data_unrestricted() {
    let enforcer = SovereigntyEnforcer::new(SovereigntyPolicy::pakistan_govos());

    let any_jurisdictions = ["ae", "us", "gb", "cn", "sg", "jp", "de"];
    for jid in &any_jurisdictions {
        assert!(
            enforcer
                .check(DataCategory::PublicRegulatory, jid)
                .is_allowed(),
            "public regulatory data should be unrestricted to {jid}"
        );
    }
}

/// Intra-jurisdiction transfers are always allowed regardless of category.
#[test]
fn intra_jurisdiction_always_allowed() {
    let enforcer = SovereigntyEnforcer::new(SovereigntyPolicy::pakistan_govos());

    for category in DataCategory::all() {
        assert!(
            enforcer.check(*category, "PK").is_allowed(),
            "{category} should be allowed within PK"
        );
    }
}

/// Custom sovereignty policy for a multi-jurisdiction deployment.
#[test]
fn custom_policy_for_uae_deployment() {
    let jid = JurisdictionId::new("ae").expect("ae jurisdiction");
    let mut policy = SovereigntyPolicy::deny_all(jid);

    // UAE deployment allows financial data sharing with GCC states.
    policy.allow(DataCategory::Financial, "sa");
    policy.allow(DataCategory::Financial, "bh");
    policy.allow(DataCategory::Financial, "kw");
    policy.allow(DataCategory::Financial, "om");
    policy.allow(DataCategory::Financial, "qa");

    // PII is always confined.
    policy.confine(DataCategory::Pii);

    let enforcer = SovereigntyEnforcer::new(policy);

    // Financial data to GCC.
    assert!(enforcer.check(DataCategory::Financial, "sa").is_allowed());
    assert!(enforcer.check(DataCategory::Financial, "bh").is_allowed());

    // Financial data to non-GCC denied.
    assert!(!enforcer.check(DataCategory::Financial, "us").is_allowed());

    // PII to anyone denied.
    assert!(!enforcer.check(DataCategory::Pii, "sa").is_allowed());

    // Intra-jurisdiction always works.
    assert!(enforcer.check(DataCategory::Pii, "ae").is_allowed());
    assert!(enforcer.check(DataCategory::Financial, "ae").is_allowed());
}

/// Policy round-trips through JSON serialization.
#[test]
fn policy_json_roundtrip() {
    let policy = SovereigntyPolicy::pakistan_govos();
    let json = serde_json::to_string_pretty(&policy).expect("serialize");
    let recovered: SovereigntyPolicy = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(
        recovered.jurisdiction_id.as_str(),
        policy.jurisdiction_id.as_str()
    );
    assert_eq!(
        recovered.confined_categories.len(),
        policy.confined_categories.len()
    );
    assert_eq!(
        recovered.allowed_targets.len(),
        policy.allowed_targets.len()
    );

    // The enforcer built from recovered policy should behave identically.
    let enforcer = SovereigntyEnforcer::new(recovered);
    assert!(!enforcer.check(DataCategory::Pii, "ae").is_allowed());
    assert!(enforcer.check(DataCategory::PublicRegulatory, "us").is_allowed());
}
