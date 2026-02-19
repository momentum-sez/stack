//! # Smart Asset Identity Validation and Compliance Evaluation Test
//!
//! Tests the domain-primitive identity types (DID, CNIC, NTN) for format
//! validation, and exercises the compliance tensor evaluation across all
//! 20 compliance domains for an entity in a test jurisdiction.

use mez_core::{Cnic, ComplianceDomain, Did, EntityId, JurisdictionId, Ntn};
use mez_tensor::evaluation::ComplianceState;
use mez_tensor::tensor::{ComplianceTensor, DefaultJurisdiction};

fn test_jurisdiction() -> DefaultJurisdiction {
    DefaultJurisdiction::new(JurisdictionId::new("PK-REZ").unwrap())
}

// ---------------------------------------------------------------------------
// 1. DID validation
// ---------------------------------------------------------------------------

#[test]
fn identity_validation_did() {
    // Valid DIDs
    let did_key = Did::new("did:key:z6MkTestKey12345");
    assert!(did_key.is_ok());
    assert_eq!(did_key.unwrap().method(), "key");

    let did_web = Did::new("did:web:example.com");
    assert!(did_web.is_ok());
    assert_eq!(did_web.unwrap().method(), "web");

    // Invalid DIDs
    assert!(Did::new("").is_err());
    assert!(Did::new("not-a-did").is_err());
    assert!(Did::new("did:").is_err());
    assert!(Did::new("did::empty-method").is_err());
    assert!(Did::new("did:method:").is_err());
}

// ---------------------------------------------------------------------------
// 2. CNIC validation (Pakistan NADRA format)
// ---------------------------------------------------------------------------

#[test]
fn identity_validation_cnic() {
    // Valid: 13 digits
    let cnic = Cnic::new("1234567890123").unwrap();
    assert_eq!(cnic.as_str(), "1234567890123");

    // Valid: formatted with dashes (5-7-1 pattern)
    let cnic_fmt = Cnic::new("12345-6789012-3").unwrap();
    assert_eq!(cnic_fmt.as_str(), "1234567890123");
    assert_eq!(cnic_fmt.formatted(), "12345-6789012-3");

    // Invalid
    assert!(Cnic::new("123456789012").is_err()); // 12 digits
    assert!(Cnic::new("12345678901234").is_err()); // 14 digits
    assert!(Cnic::new("1234a67890123").is_err()); // non-digit
    assert!(Cnic::new("12345-678901-23").is_err()); // wrong dash pattern
}

// ---------------------------------------------------------------------------
// 3. NTN validation (Pakistan FBR format)
// ---------------------------------------------------------------------------

#[test]
fn identity_validation_ntn() {
    // Valid: exactly 7 digits
    let ntn = Ntn::new("1234567").unwrap();
    assert_eq!(ntn.as_str(), "1234567");

    // Leading zeros are significant
    let ntn_leading = Ntn::new("0012345").unwrap();
    assert_eq!(ntn_leading.as_str(), "0012345");

    // Invalid
    assert!(Ntn::new("123456").is_err()); // 6 digits
    assert!(Ntn::new("12345678").is_err()); // 8 digits
    assert!(Ntn::new("123456a").is_err()); // non-digit
    assert!(Ntn::new("").is_err());
}

// ---------------------------------------------------------------------------
// 4. Compliance tensor evaluation across all 20 domains
// ---------------------------------------------------------------------------

#[test]
fn compliance_tensor_evaluation() {
    let mut tensor = ComplianceTensor::new(test_jurisdiction());
    assert_eq!(tensor.cell_count(), 20);

    // Set specific domains
    tensor.set(
        ComplianceDomain::Aml,
        ComplianceState::Compliant,
        vec![],
        None,
    );
    tensor.set(
        ComplianceDomain::Kyc,
        ComplianceState::Pending,
        vec![],
        None,
    );
    tensor.set(
        ComplianceDomain::Sanctions,
        ComplianceState::NonCompliant,
        vec![],
        None,
    );

    assert_eq!(
        tensor.get(ComplianceDomain::Aml),
        ComplianceState::Compliant
    );
    assert_eq!(tensor.get(ComplianceDomain::Kyc), ComplianceState::Pending);
    assert_eq!(
        tensor.get(ComplianceDomain::Sanctions),
        ComplianceState::NonCompliant
    );

    // Slice aggregation: most restrictive wins
    let slice = tensor.slice(&[
        ComplianceDomain::Aml,
        ComplianceDomain::Kyc,
        ComplianceDomain::Sanctions,
    ]);
    assert_eq!(slice.aggregate_state(), ComplianceState::NonCompliant);
    assert!(!slice.all_passing());

    // Non-compliant domains
    let nc_domains = slice.non_compliant_domains();
    assert_eq!(nc_domains.len(), 1);
    assert!(nc_domains.contains(&ComplianceDomain::Sanctions));
}

// ---------------------------------------------------------------------------
// 5. Entity ID uniqueness
// ---------------------------------------------------------------------------

#[test]
fn entity_ids_are_unique() {
    let id1 = EntityId::new();
    let id2 = EntityId::new();
    assert_ne!(id1, id2);
}

// ---------------------------------------------------------------------------
// 6. Jurisdiction ID validation
// ---------------------------------------------------------------------------

#[test]
fn jurisdiction_id_creation() {
    let jid = JurisdictionId::new("PK-REZ").unwrap();
    assert_eq!(jid.as_str(), "PK-REZ");

    // Empty is rejected
    assert!(JurisdictionId::new("").is_err());
}
