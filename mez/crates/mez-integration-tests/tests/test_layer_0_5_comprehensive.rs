//! # Comprehensive Layer 0.5 Tests
//!
//! Tests the integration between entities, compliance tensors, and the
//! core type system. Verifies that entities can be associated with tensor
//! compliance evaluations, that tensor slices are valid subsets of the
//! full domain set, and that entity/compliance state enums match the spec.

use mez_core::{sha256_digest, CanonicalBytes, ComplianceDomain, EntityId, JurisdictionId};
use mez_state::{Entity, EntityLifecycleState};
use mez_tensor::{
    evaluation::ComplianceState,
    tensor::{ComplianceTensor, DefaultJurisdiction},
};
use serde_json::json;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_jurisdiction() -> DefaultJurisdiction {
    DefaultJurisdiction::new(JurisdictionId::new("PK-REZ").unwrap())
}

// ---------------------------------------------------------------------------
// 1. Entity with tensor compliance
// ---------------------------------------------------------------------------

#[test]
fn entity_with_tensor_compliance() {
    // Create entity
    let mut entity = Entity::new(EntityId::new());
    entity.approve().unwrap();
    assert_eq!(entity.state, EntityLifecycleState::Active);

    // Create compliance tensor for the entity's jurisdiction
    let mut tensor = ComplianceTensor::new(test_jurisdiction());
    tensor.set(
        ComplianceDomain::Aml,
        ComplianceState::Compliant,
        vec![],
        None,
    );
    tensor.set(
        ComplianceDomain::Kyc,
        ComplianceState::Compliant,
        vec![],
        None,
    );
    tensor.set(
        ComplianceDomain::Sanctions,
        ComplianceState::Compliant,
        vec![],
        None,
    );

    // Tensor should have 20 cells (all domains)
    assert_eq!(tensor.cell_count(), 20);

    // Commitment should be deterministic
    let commitment = tensor.commit().unwrap();
    assert_eq!(commitment.to_hex().len(), 64);

    // The subset we set should be compliant
    let slice = tensor.slice(&[
        ComplianceDomain::Aml,
        ComplianceDomain::Kyc,
        ComplianceDomain::Sanctions,
    ]);
    assert!(slice.all_passing());
}

#[test]
fn entity_and_tensor_digest_combined() {
    let entity_data = json!({
        "entity_id": EntityId::new().to_string(),
        "jurisdiction": "PK-REZ",
        "status": "ACTIVE"
    });

    let tensor = ComplianceTensor::new(test_jurisdiction());
    let commitment = tensor.commit().unwrap();

    // Combined digest should be deterministic
    let combined = json!({
        "entity": entity_data,
        "compliance_commitment": commitment.to_hex()
    });
    let d1 = sha256_digest(&CanonicalBytes::new(&combined).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&combined).unwrap());
    assert_eq!(d1, d2);
}

// ---------------------------------------------------------------------------
// 2. Tensor slice is a valid subset of domains
// ---------------------------------------------------------------------------

#[test]
fn tensor_slice_subset_of_domains() {
    let tensor = ComplianceTensor::new(test_jurisdiction());

    // Full slice has all 20 domains
    let full_slice = tensor.full_slice();
    assert_eq!(full_slice.len(), 20);

    // Partial slice has exactly the requested domains
    let subset = [ComplianceDomain::Aml, ComplianceDomain::Tax];
    let partial = tensor.slice(&subset);
    assert_eq!(partial.len(), 2);

    // Single-domain slice
    let single = tensor.slice(&[ComplianceDomain::Kyc]);
    assert_eq!(single.len(), 1);
}

#[test]
fn tensor_slice_empty_is_fail_closed() {
    let tensor = ComplianceTensor::new(test_jurisdiction());
    let empty_slice = tensor.slice(&[]);
    assert_eq!(empty_slice.len(), 0);
    // P0-TENSOR-001: empty slices must NOT be treated as passing — fail-closed.
    // An empty domain set has no affirmative compliance evidence.
    assert!(!empty_slice.all_passing(), "empty slice must not pass (fail-closed)");
}

// ---------------------------------------------------------------------------
// 3. Entity lifecycle state names match spec
// ---------------------------------------------------------------------------

#[test]
fn entity_lifecycle_state_names() {
    assert_eq!(EntityLifecycleState::Applied.as_str(), "APPLIED");
    assert_eq!(EntityLifecycleState::Active.as_str(), "ACTIVE");
    assert_eq!(EntityLifecycleState::Suspended.as_str(), "SUSPENDED");
    assert_eq!(EntityLifecycleState::Dissolving.as_str(), "DISSOLVING");
    assert_eq!(EntityLifecycleState::Dissolved.as_str(), "DISSOLVED");
    assert_eq!(EntityLifecycleState::Rejected.as_str(), "REJECTED");
}

#[test]
fn entity_lifecycle_terminal_states() {
    assert!(EntityLifecycleState::Dissolved.is_terminal());
    assert!(EntityLifecycleState::Rejected.is_terminal());

    assert!(!EntityLifecycleState::Applied.is_terminal());
    assert!(!EntityLifecycleState::Active.is_terminal());
    assert!(!EntityLifecycleState::Suspended.is_terminal());
    assert!(!EntityLifecycleState::Dissolving.is_terminal());
}

// ---------------------------------------------------------------------------
// 4. Compliance state variants
// ---------------------------------------------------------------------------

#[test]
fn compliance_state_variants() {
    // Verify we can use all compliance state variants
    let states = [
        ComplianceState::Compliant,
        ComplianceState::NonCompliant,
        ComplianceState::Pending,
        ComplianceState::NotApplicable,
        ComplianceState::Exempt,
    ];

    for state in &states {
        let data = json!({"state": format!("{state:?}")});
        let cb = CanonicalBytes::new(&data).unwrap();
        assert!(
            !cb.as_bytes().is_empty(),
            "compliance state should serialize"
        );
    }
}

#[test]
fn compliance_state_ordering() {
    // NonCompliant is the most restrictive; aggregation should return it
    // when any domain is non-compliant
    let tensor_jurisdiction = test_jurisdiction();
    let mut tensor = ComplianceTensor::new(tensor_jurisdiction);
    tensor.set(
        ComplianceDomain::Aml,
        ComplianceState::Compliant,
        vec![],
        None,
    );
    tensor.set(
        ComplianceDomain::Kyc,
        ComplianceState::NonCompliant,
        vec![],
        None,
    );

    let slice = tensor.slice(&[ComplianceDomain::Aml, ComplianceDomain::Kyc]);
    assert_eq!(slice.aggregate_state(), ComplianceState::NonCompliant);
    assert!(!slice.all_passing());
}

// ---------------------------------------------------------------------------
// 5. Entity dissolution integration with compliance
// ---------------------------------------------------------------------------

#[test]
fn entity_dissolution_preserves_compliance_context() {
    let mut entity = Entity::new(EntityId::new());
    entity.approve().unwrap();

    // Create compliance tensor while entity is active
    let mut tensor = ComplianceTensor::new(test_jurisdiction());
    tensor.set(
        ComplianceDomain::Corporate,
        ComplianceState::Compliant,
        vec![],
        None,
    );
    let active_commitment = tensor.commit().unwrap();

    // Initiate dissolution
    entity.initiate_dissolution().unwrap();
    assert_eq!(entity.state, EntityLifecycleState::Dissolving);

    // Compliance commitment should still be computable and deterministic
    let dissolving_commitment = tensor.commit().unwrap();
    assert_eq!(
        active_commitment.to_hex(),
        dissolving_commitment.to_hex(),
        "compliance commitment should not change based on entity lifecycle state"
    );
}
