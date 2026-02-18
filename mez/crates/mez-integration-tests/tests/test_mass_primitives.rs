//! # Mass Primitives Integration Tests
//!
//! Python counterpart: `tests/test_mass_primitives.py`
//!
//! Tests the five mass primitives (entities, ownership, fiscal, identity, consent):
//! - Entity creation and lifecycle transitions
//! - Entity dissolution through all 10 stages
//! - License lifecycle management
//! - Identity type validation (DID, CNIC, NTN, PassportNumber)

use mez_core::{Cnic, Did, EntityId, Ntn, PassportNumber};
use mez_state::{DissolutionStage, Entity, EntityLifecycleState, License, LicenseState};

// ---------------------------------------------------------------------------
// 1. Entity creation and activation
// ---------------------------------------------------------------------------

#[test]
fn entity_creation_and_activation() {
    let mut entity = Entity::new(EntityId::new());
    assert_eq!(entity.state, EntityLifecycleState::Applied);
    assert!(!entity.state.is_terminal());

    entity.approve().unwrap();
    assert_eq!(entity.state, EntityLifecycleState::Active);
    assert!(!entity.state.is_terminal());
}

#[test]
fn entity_rejection_is_terminal() {
    let mut entity = Entity::new(EntityId::new());
    entity.reject().unwrap();
    assert_eq!(entity.state, EntityLifecycleState::Rejected);
    assert!(entity.state.is_terminal());
}

#[test]
fn entity_suspend_and_reinstate() {
    let mut entity = Entity::new(EntityId::new());
    entity.approve().unwrap();
    entity.suspend().unwrap();
    assert_eq!(entity.state, EntityLifecycleState::Suspended);

    entity.reinstate().unwrap();
    assert_eq!(entity.state, EntityLifecycleState::Active);
}

// ---------------------------------------------------------------------------
// 2. Entity dissolution through all 10 stages
// ---------------------------------------------------------------------------

#[test]
fn entity_dissolution_10_stages() {
    let mut entity = Entity::new(EntityId::new());
    entity.approve().unwrap();
    entity.initiate_dissolution().unwrap();

    assert_eq!(entity.state, EntityLifecycleState::Dissolving);
    assert_eq!(
        entity.dissolution_stage,
        Some(DissolutionStage::BoardResolution)
    );

    // All 10 stages
    let expected_stages = DissolutionStage::all_stages();
    assert_eq!(expected_stages.len(), 10);

    // Advance through stages 2-10
    for (i, expected_stage) in expected_stages.iter().enumerate().skip(1) {
        entity.advance_dissolution().unwrap();
        assert_eq!(
            entity.dissolution_stage,
            Some(*expected_stage),
            "mismatch at dissolution stage {}",
            i + 1
        );
    }

    // Advance past stage 10 -> Dissolved
    entity.advance_dissolution().unwrap();
    assert_eq!(entity.state, EntityLifecycleState::Dissolved);
    assert!(entity.state.is_terminal());
}

#[test]
fn entity_cannot_dissolve_from_applied() {
    let mut entity = Entity::new(EntityId::new());
    assert!(entity.initiate_dissolution().is_err());
}

// ---------------------------------------------------------------------------
// 3. License lifecycle
// ---------------------------------------------------------------------------

#[test]
fn license_lifecycle() {
    let mut lic = License::new("MANUFACTURING");
    assert_eq!(lic.state, LicenseState::Applied);

    lic.review().unwrap();
    assert_eq!(lic.state, LicenseState::UnderReview);

    lic.issue().unwrap();
    assert_eq!(lic.state, LicenseState::Active);
    assert!(!lic.state.is_terminal());
}

#[test]
fn license_active_suspend_revoke() {
    let mut lic = License::new("TRADING");
    lic.review().unwrap();
    lic.issue().unwrap();

    lic.suspend("Investigation pending").unwrap();
    assert_eq!(lic.state, LicenseState::Suspended);
    assert_eq!(lic.state_reason.as_deref(), Some("Investigation pending"));

    lic.revoke("Violation confirmed").unwrap();
    assert_eq!(lic.state, LicenseState::Revoked);
    assert!(lic.state.is_terminal());
}

#[test]
fn license_surrender_is_terminal() {
    let mut lic = License::new("PROFESSIONAL");
    lic.review().unwrap();
    lic.issue().unwrap();
    lic.surrender().unwrap();
    assert_eq!(lic.state, LicenseState::Surrendered);
    assert!(lic.state.is_terminal());
}

#[test]
fn license_expire_is_terminal() {
    let mut lic = License::new("EXPORT");
    lic.review().unwrap();
    lic.issue().unwrap();
    lic.expire().unwrap();
    assert_eq!(lic.state, LicenseState::Expired);
    assert!(lic.state.is_terminal());
}

// ---------------------------------------------------------------------------
// 4. Identity type validation — DID
// ---------------------------------------------------------------------------

#[test]
fn identity_type_validation_did() {
    assert!(Did::new("did:key:z6MkTest").is_ok());
    assert!(Did::new("did:web:example.com").is_ok());
    assert!(Did::new("did:ethr:0xabc123").is_ok());
}

#[test]
fn identity_type_validation_did_rejects_invalid() {
    assert!(Did::new("notadid").is_err());
    assert!(Did::new("did:").is_err());
    assert!(Did::new("did::something").is_err());
    assert!(Did::new("did:Web:id").is_err()); // uppercase method
    assert!(Did::new("did:method:").is_err()); // empty identifier
}

// ---------------------------------------------------------------------------
// 5. Identity type validation — CNIC
// ---------------------------------------------------------------------------

#[test]
fn identity_type_validation_cnic() {
    assert!(Cnic::new("1234567890123").is_ok());
    assert!(Cnic::new("12345-6789012-3").is_ok());
}

#[test]
fn identity_type_validation_cnic_rejects_invalid() {
    assert!(Cnic::new("123456789012").is_err()); // 12 digits
    assert!(Cnic::new("12345678901234").is_err()); // 14 digits
    assert!(Cnic::new("1234a67890123").is_err()); // non-digit
}

// ---------------------------------------------------------------------------
// 6. Identity type validation — NTN
// ---------------------------------------------------------------------------

#[test]
fn identity_type_validation_ntn() {
    assert!(Ntn::new("1234567").is_ok());
    assert!(Ntn::new("0012345").is_ok()); // leading zeros
}

#[test]
fn identity_type_validation_ntn_rejects_invalid() {
    assert!(Ntn::new("123456").is_err()); // 6 digits
    assert!(Ntn::new("12345678").is_err()); // 8 digits
    assert!(Ntn::new("123456a").is_err()); // non-digit
}

// ---------------------------------------------------------------------------
// 7. Identity type validation — Passport
// ---------------------------------------------------------------------------

#[test]
fn identity_type_validation_passport() {
    assert!(PassportNumber::new("AB1234567").is_ok());
    let pp = PassportNumber::new("ab123456").unwrap();
    assert_eq!(pp.as_str(), "AB123456"); // uppercased
}

#[test]
fn identity_type_validation_passport_rejects_invalid() {
    assert!(PassportNumber::new("ABCD").is_err()); // too short
    assert!(PassportNumber::new("AB12-3456").is_err()); // dash
}
