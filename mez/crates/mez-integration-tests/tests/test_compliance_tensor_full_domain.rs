//! # Compliance Tensor Full Domain Test
//!
//! Constructs a ComplianceTensor with a test jurisdiction, evaluates all 20
//! domains, and verifies that the tensor commitment (computed via
//! CanonicalBytes → ContentDigest) is deterministic.

use mez_core::{ComplianceDomain, JurisdictionId};
use mez_tensor::{
    commitment::{commitment_digest, merkle_root, TensorCommitment},
    evaluation::ComplianceState,
    tensor::{ComplianceTensor, DefaultJurisdiction},
};

fn test_jurisdiction() -> DefaultJurisdiction {
    DefaultJurisdiction::new(JurisdictionId::new("PK-RSEZ").unwrap())
}

// ---------------------------------------------------------------------------
// 1. Tensor with all 20 domains
// ---------------------------------------------------------------------------

#[test]
fn tensor_has_20_cells() {
    let tensor = ComplianceTensor::new(test_jurisdiction());
    assert_eq!(tensor.cell_count(), 20);
}

#[test]
fn evaluate_all_20_domains() {
    let tensor = ComplianceTensor::new(test_jurisdiction());
    let results = tensor.evaluate_all("entity-test-001");
    assert_eq!(
        results.len(),
        20,
        "evaluate_all must return exactly 20 entries"
    );

    for &domain in ComplianceDomain::all() {
        assert!(
            results.contains_key(&domain),
            "evaluate_all missing domain {domain}"
        );
    }
}

// ---------------------------------------------------------------------------
// 2. Tensor commitment is deterministic (same input → same digest)
// ---------------------------------------------------------------------------

#[test]
fn tensor_commitment_deterministic() {
    let tensor = ComplianceTensor::new(test_jurisdiction());

    let c1 = tensor.commit().unwrap();
    let c2 = tensor.commit().unwrap();

    assert_eq!(c1.to_hex(), c2.to_hex(), "commitment must be deterministic");
    assert_eq!(c1.to_hex().len(), 64);
    assert!(c1.to_hex().chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn tensor_commitment_changes_with_state() {
    let t1 = ComplianceTensor::new(test_jurisdiction());
    let mut t2 = ComplianceTensor::new(test_jurisdiction());
    t2.set(
        ComplianceDomain::Aml,
        ComplianceState::Compliant,
        vec![],
        None,
    );

    let c1 = t1.commit().unwrap();
    let c2 = t2.commit().unwrap();
    assert_ne!(
        c1.to_hex(),
        c2.to_hex(),
        "different states must produce different commitments"
    );
}

// ---------------------------------------------------------------------------
// 3. Set and evaluate every domain
// ---------------------------------------------------------------------------

#[test]
fn set_all_domains_and_commit() {
    let mut tensor = ComplianceTensor::new(test_jurisdiction());

    // Set every domain to a specific state
    let states = [
        (ComplianceDomain::Aml, ComplianceState::Compliant),
        (ComplianceDomain::Kyc, ComplianceState::Compliant),
        (ComplianceDomain::Sanctions, ComplianceState::Compliant),
        (ComplianceDomain::Tax, ComplianceState::Pending),
        (ComplianceDomain::Securities, ComplianceState::NotApplicable),
        (ComplianceDomain::Corporate, ComplianceState::Compliant),
        (ComplianceDomain::Custody, ComplianceState::Exempt),
        (ComplianceDomain::DataPrivacy, ComplianceState::Compliant),
        (ComplianceDomain::Licensing, ComplianceState::NotApplicable),
        (ComplianceDomain::Banking, ComplianceState::NotApplicable),
        (ComplianceDomain::Payments, ComplianceState::NotApplicable),
        (ComplianceDomain::Clearing, ComplianceState::NotApplicable),
        (ComplianceDomain::Settlement, ComplianceState::NotApplicable),
        (ComplianceDomain::DigitalAssets, ComplianceState::Pending),
        (ComplianceDomain::Employment, ComplianceState::NotApplicable),
        (
            ComplianceDomain::Immigration,
            ComplianceState::NotApplicable,
        ),
        (ComplianceDomain::Ip, ComplianceState::NotApplicable),
        (
            ComplianceDomain::ConsumerProtection,
            ComplianceState::NotApplicable,
        ),
        (
            ComplianceDomain::Arbitration,
            ComplianceState::NotApplicable,
        ),
        (ComplianceDomain::Trade, ComplianceState::Compliant),
    ];

    for (domain, state) in &states {
        tensor.set(*domain, *state, vec![], None);
    }

    // Verify each domain's state
    for (domain, state) in &states {
        assert_eq!(
            tensor.get(*domain),
            *state,
            "domain {domain} state mismatch"
        );
    }

    // Commitment should be deterministic
    let c1 = tensor.commit().unwrap();
    let c2 = tensor.commit().unwrap();
    assert_eq!(c1.to_hex(), c2.to_hex());
    assert_eq!(c1.cell_count(), 20);
    assert_eq!(c1.jurisdiction_id(), "PK-RSEZ");
}

// ---------------------------------------------------------------------------
// 4. Tensor slice operations
// ---------------------------------------------------------------------------

#[test]
fn full_slice_contains_all_20_domains() {
    let tensor = ComplianceTensor::new(test_jurisdiction());
    let slice = tensor.full_slice();
    assert_eq!(slice.len(), 20);
}

#[test]
fn partial_slice_and_aggregation() {
    let mut tensor = ComplianceTensor::new(test_jurisdiction());
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

    let slice = tensor.slice(&[
        ComplianceDomain::Aml,
        ComplianceDomain::Kyc,
        ComplianceDomain::Sanctions,
    ]);
    assert_eq!(slice.len(), 3);

    // Aggregate should be NonCompliant (most restrictive)
    assert_eq!(slice.aggregate_state(), ComplianceState::NonCompliant);
    assert!(!slice.all_passing());

    let nc_domains = slice.non_compliant_domains();
    assert_eq!(nc_domains.len(), 1);
    assert!(nc_domains.contains(&ComplianceDomain::Sanctions));

    let pending_domains = slice.pending_domains();
    assert_eq!(pending_domains.len(), 1);
    assert!(pending_domains.contains(&ComplianceDomain::Kyc));
}

// ---------------------------------------------------------------------------
// 5. Tensor merge (lattice meet)
// ---------------------------------------------------------------------------

#[test]
fn tensor_merge_takes_more_restrictive() {
    let mut a = ComplianceTensor::new(test_jurisdiction());
    a.set(
        ComplianceDomain::Aml,
        ComplianceState::Compliant,
        vec![],
        None,
    );
    a.set(
        ComplianceDomain::Kyc,
        ComplianceState::Compliant,
        vec![],
        None,
    );

    let mut b = ComplianceTensor::new(test_jurisdiction());
    b.set(
        ComplianceDomain::Aml,
        ComplianceState::NonCompliant,
        vec![],
        None,
    );
    b.set(
        ComplianceDomain::Kyc,
        ComplianceState::Compliant,
        vec![],
        None,
    );

    a.merge(&b);
    assert_eq!(a.get(ComplianceDomain::Aml), ComplianceState::NonCompliant);
    assert_eq!(a.get(ComplianceDomain::Kyc), ComplianceState::Compliant);
}

// ---------------------------------------------------------------------------
// 6. commitment_digest standalone function
// ---------------------------------------------------------------------------

#[test]
fn commitment_digest_standalone() {
    let states: Vec<_> = ComplianceDomain::all()
        .iter()
        .map(|&d| (d, ComplianceState::Pending))
        .collect();
    let digest = commitment_digest("PK-RSEZ", &states).unwrap();
    assert_eq!(digest.to_hex().len(), 64);

    // Running again should produce the same digest
    let digest2 = commitment_digest("PK-RSEZ", &states).unwrap();
    assert_eq!(digest, digest2);
}

#[test]
fn commitment_digest_different_jurisdictions_differ() {
    let states: Vec<_> = ComplianceDomain::all()
        .iter()
        .map(|&d| (d, ComplianceState::Pending))
        .collect();
    let d1 = commitment_digest("PK-RSEZ", &states).unwrap();
    let d2 = commitment_digest("AE-DIFC", &states).unwrap();
    assert_ne!(d1, d2);
}

// ---------------------------------------------------------------------------
// 7. Merkle root over tensor commitments
// ---------------------------------------------------------------------------

#[test]
fn merkle_root_deterministic() {
    let t1 = ComplianceTensor::new(test_jurisdiction());
    let mut t2 = ComplianceTensor::new(test_jurisdiction());
    t2.set(
        ComplianceDomain::Aml,
        ComplianceState::Compliant,
        vec![],
        None,
    );

    let c1 = t1.commit().unwrap();
    let c2 = t2.commit().unwrap();

    let root1 = merkle_root(&[c1.clone(), c2.clone()]);
    let root2 = merkle_root(&[c1, c2]);
    assert_eq!(root1, root2, "Merkle root must be deterministic");
    assert!(root1.is_some());
}

#[test]
fn merkle_root_empty() {
    assert_eq!(merkle_root(&[]), None);
}

#[test]
fn merkle_root_single() {
    let tensor = ComplianceTensor::new(test_jurisdiction());
    let commitment = tensor.commit().unwrap();
    let root = merkle_root(std::slice::from_ref(&commitment));
    assert_eq!(root, Some(commitment.to_hex()));
}

// ---------------------------------------------------------------------------
// 8. Empty commitment
// ---------------------------------------------------------------------------

#[test]
fn empty_commitment_is_deterministic() {
    let c1 = TensorCommitment::empty("PK-RSEZ").unwrap();
    let c2 = TensorCommitment::empty("PK-RSEZ").unwrap();
    assert_eq!(c1.to_hex(), c2.to_hex());
    assert_eq!(c1.cell_count(), 0);
}

// ---------------------------------------------------------------------------
// 9. Commitment determinism implies sorted cells internally
// ---------------------------------------------------------------------------

#[test]
fn commitment_proves_cells_are_sorted_internally() {
    // The commitment is deterministic, which can only happen if cells are
    // sorted before hashing. We verify this indirectly by checking that
    // two tensors with the same state produce the same commitment.
    let mut t1 = ComplianceTensor::new(test_jurisdiction());
    let mut t2 = ComplianceTensor::new(test_jurisdiction());

    // Set domains in different order
    t1.set(
        ComplianceDomain::Trade,
        ComplianceState::Compliant,
        vec![],
        None,
    );
    t1.set(
        ComplianceDomain::Aml,
        ComplianceState::Compliant,
        vec![],
        None,
    );

    t2.set(
        ComplianceDomain::Aml,
        ComplianceState::Compliant,
        vec![],
        None,
    );
    t2.set(
        ComplianceDomain::Trade,
        ComplianceState::Compliant,
        vec![],
        None,
    );

    let c1 = t1.commit().unwrap();
    let c2 = t2.commit().unwrap();
    assert_eq!(
        c1.to_hex(),
        c2.to_hex(),
        "order of set() calls must not affect commitment"
    );
}

// ---------------------------------------------------------------------------
// 10. Clone preserves state
// ---------------------------------------------------------------------------

#[test]
fn clone_preserves_tensor_state() {
    let mut tensor = ComplianceTensor::new(test_jurisdiction());
    tensor.set(
        ComplianceDomain::Trade,
        ComplianceState::Compliant,
        vec![],
        None,
    );

    let cloned = tensor.clone();
    assert_eq!(
        cloned.get(ComplianceDomain::Trade),
        ComplianceState::Compliant
    );
    assert_eq!(cloned.cell_count(), 20);

    // Commitments should match since state is preserved
    let c_orig = tensor.commit().unwrap();
    let c_clone = cloned.commit().unwrap();
    assert_eq!(c_orig.to_hex(), c_clone.to_hex());
}
