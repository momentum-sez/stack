//! Tests for licensepack operations.
//!
//! Validates licensepack creation, digest determinism, license type
//! management, and compliance evaluation.

use msez_core::JurisdictionId;
use msez_pack::licensepack::{
    License, LicensePermission, LicenseStatus, LicenseTypeDefinition, Licensepack,
};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Licensepack creation
// ---------------------------------------------------------------------------

#[test]
fn licensepack_creation() {
    let jid = JurisdictionId::new("PK-RSEZ").unwrap();
    let lp = Licensepack::new(jid.clone(), "PK-RSEZ Financial Licenses".to_string());

    assert_eq!(lp.jurisdiction.as_str(), "PK-RSEZ");
    assert_eq!(lp.name, "PK-RSEZ Financial Licenses");
    assert_eq!(lp.version, "1.0");
}

// ---------------------------------------------------------------------------
// Licensepack digest deterministic
// ---------------------------------------------------------------------------

#[test]
fn licensepack_digest_deterministic() {
    let jid = JurisdictionId::new("PK-RSEZ").unwrap();
    let lp1 = Licensepack::new(jid.clone(), "Test Pack".to_string());
    let lp2 = Licensepack::new(jid, "Test Pack".to_string());

    let d1 = lp1.compute_digest().unwrap();
    let d2 = lp2.compute_digest().unwrap();

    assert_eq!(d1, d2, "Same-parameter licensepacks must have same digest");
    assert_eq!(d1.len(), 64);
}

#[test]
fn licensepack_different_content_different_digest() {
    let jid = JurisdictionId::new("PK-RSEZ").unwrap();
    let lp1 = Licensepack::new(jid.clone(), "Pack A".to_string());

    let mut lp2 = Licensepack::new(jid, "Pack B".to_string());
    lp2.license_types.insert(
        "test_type".to_string(),
        LicenseTypeDefinition {
            license_type_id: "test_type".to_string(),
            name: "Test Type".to_string(),
            description: "A test license type".to_string(),
            regulator_id: "test_reg".to_string(),
            category: None,
            permitted_activities: vec![],
            requirements: Default::default(),
            application_fee: Default::default(),
            annual_fee: Default::default(),
            validity_period_years: None,
        },
    );

    assert_ne!(
        lp1.compute_digest().unwrap(),
        lp2.compute_digest().unwrap(),
        "Different license type content must produce different digests"
    );
}

// ---------------------------------------------------------------------------
// License type definitions
// ---------------------------------------------------------------------------

#[test]
fn licensepack_license_types() {
    let jid = JurisdictionId::new("PK-RSEZ").unwrap();
    let mut lp = Licensepack::new(jid, "Financial Licenses".to_string());

    assert!(lp.license_types.is_empty());

    lp.license_types.insert(
        "banking_license".to_string(),
        LicenseTypeDefinition {
            license_type_id: "banking_license".to_string(),
            name: "Banking License".to_string(),
            description: "License to conduct banking operations".to_string(),
            regulator_id: "sbp".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec!["deposit_taking".to_string(), "lending".to_string()],
            requirements: Default::default(),
            application_fee: Default::default(),
            annual_fee: Default::default(),
            validity_period_years: Some(5),
        },
    );

    assert_eq!(lp.license_types.len(), 1);
    assert!(lp.license_types.contains_key("banking_license"));
}

// ---------------------------------------------------------------------------
// Licensepack compliance evaluation
// ---------------------------------------------------------------------------

#[test]
fn licensepack_compliance_evaluation() {
    let jid = JurisdictionId::new("PK-RSEZ").unwrap();
    let mut lp = Licensepack::new(jid, "Financial Licenses".to_string());

    lp.add_license(License {
        license_id: "LIC-001".to_string(),
        license_type_id: "banking_license".to_string(),
        license_number: Some("SBP/2026/001".to_string()),
        status: LicenseStatus::Active,
        issued_date: "2026-01-01".to_string(),
        holder_id: "HOLDER-001".to_string(),
        holder_legal_name: "Test Financial Corp".to_string(),
        holder_did: Some("did:web:test.example".to_string()),
        regulator_id: "sbp".to_string(),
        status_effective_date: Some("2026-01-01".to_string()),
        status_reason: None,
        effective_date: None,
        expiry_date: Some("2031-01-01".to_string()),
        holder_registration_number: None,
        issuing_authority: Some("SBP".to_string()),
        conditions: vec![],
        permissions: vec![LicensePermission {
            permission_id: "PERM-001".to_string(),
            activity: "deposit_taking".to_string(),
            scope: BTreeMap::new(),
            limits: BTreeMap::new(),
            effective_date: None,
            status: "active".to_string(),
        }],
        restrictions: vec![],
        permitted_activities: vec!["deposit_taking".to_string()],
        asset_classes_authorized: vec![],
        client_types_permitted: vec![],
        geographic_scope: vec![],
        prudential_category: None,
        capital_requirement: BTreeMap::new(),
    });

    let license = lp.get_license("LIC-001").unwrap();
    assert_eq!(license.status, LicenseStatus::Active);
    assert!(!license.status.is_terminal());
}

#[test]
fn licensepack_terminal_license_status() {
    // Revoked, expired, and surrendered statuses are terminal.
    assert!(LicenseStatus::Revoked.is_terminal());
    assert!(LicenseStatus::Expired.is_terminal());
    assert!(LicenseStatus::Surrendered.is_terminal());

    // Active, suspended, pending are not terminal.
    assert!(!LicenseStatus::Active.is_terminal());
    assert!(!LicenseStatus::Suspended.is_terminal());
    assert!(!LicenseStatus::Pending.is_terminal());
}

#[test]
fn licensepack_digest_changes_with_license() {
    let jid = JurisdictionId::new("PK-RSEZ").unwrap();
    let lp_empty = Licensepack::new(jid.clone(), "Test Pack".to_string());
    let d1 = lp_empty.compute_digest().unwrap();

    let mut lp_with = Licensepack::new(jid, "Test Pack".to_string());
    lp_with.add_license(License {
        license_id: "LIC-001".to_string(),
        license_type_id: "test".to_string(),
        license_number: None,
        status: LicenseStatus::Active,
        issued_date: "2026-01-01".to_string(),
        holder_id: "H1".to_string(),
        holder_legal_name: "Test Corp".to_string(),
        holder_did: Some("did:web:test".to_string()),
        regulator_id: "sbp".to_string(),
        status_effective_date: None,
        status_reason: None,
        effective_date: None,
        expiry_date: None,
        holder_registration_number: None,
        issuing_authority: None,
        conditions: vec![],
        permissions: vec![],
        restrictions: vec![],
        permitted_activities: vec![],
        asset_classes_authorized: vec![],
        client_types_permitted: vec![],
        geographic_scope: vec![],
        prudential_category: None,
        capital_requirement: BTreeMap::new(),
    });
    let d2 = lp_with.compute_digest().unwrap();

    assert_ne!(d1, d2, "Adding a license must change the digest");
}
