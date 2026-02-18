//! # Regulatory Compliance Scenarios Integration Tests
//!
//! Python counterpart: `tests/test_regulatory_scenarios.py`
//!
//! Tests regulatory compliance scenarios combining the compliance tensor
//! with the agentic policy engine:
//! - Sanctions screening triggers halt
//! - Tax compliance evaluation
//! - AML/KYC compliance
//! - Cross-domain compliance checking

use mez_agentic::evaluation::PolicyEngine;
use mez_agentic::policy::{PolicyAction, Trigger, TriggerType};
use mez_core::{ComplianceDomain, JurisdictionId};
use mez_tensor::{
    evaluation::ComplianceState,
    tensor::{ComplianceTensor, DefaultJurisdiction},
};
use serde_json::json;

fn test_jurisdiction() -> DefaultJurisdiction {
    DefaultJurisdiction::new(JurisdictionId::new("PK-RSEZ").unwrap())
}

// ---------------------------------------------------------------------------
// 1. Sanctions screening scenario
// ---------------------------------------------------------------------------

#[test]
fn sanctions_screening_scenario() {
    // Step 1: Create a compliance tensor and mark sanctions as non-compliant
    let mut tensor = ComplianceTensor::new(test_jurisdiction());
    tensor.set(
        ComplianceDomain::Sanctions,
        ComplianceState::NonCompliant,
        vec![],
        None,
    );

    assert_eq!(
        tensor.get(ComplianceDomain::Sanctions),
        ComplianceState::NonCompliant
    );

    // Step 2: Fire a sanctions trigger through the policy engine
    let mut engine = PolicyEngine::with_standard_policies();
    let trigger = Trigger::new(
        TriggerType::SanctionsListUpdate,
        json!({"affected_parties": ["self"]}),
    );

    let actions = engine.process_trigger(&trigger, "asset:sanctioned-entity", None);
    assert!(
        actions.iter().any(|a| a.action == PolicyAction::Halt),
        "sanctions violation must trigger Halt action"
    );
}

// ---------------------------------------------------------------------------
// 2. Tax compliance scenario
// ---------------------------------------------------------------------------

#[test]
fn tax_compliance_scenario() {
    let mut tensor = ComplianceTensor::new(test_jurisdiction());
    tensor.set(
        ComplianceDomain::Tax,
        ComplianceState::Pending,
        vec![],
        None,
    );

    assert_eq!(
        tensor.get(ComplianceDomain::Tax),
        ComplianceState::Pending,
        "tax domain should be pending"
    );

    // After setting to compliant
    tensor.set(
        ComplianceDomain::Tax,
        ComplianceState::Compliant,
        vec![],
        None,
    );
    assert_eq!(
        tensor.get(ComplianceDomain::Tax),
        ComplianceState::Compliant
    );

    // Commitment should change when state changes
    let c_compliant = tensor.commit().unwrap();
    tensor.set(
        ComplianceDomain::Tax,
        ComplianceState::NonCompliant,
        vec![],
        None,
    );
    let c_noncompliant = tensor.commit().unwrap();
    assert_ne!(c_compliant.to_hex(), c_noncompliant.to_hex());
}

// ---------------------------------------------------------------------------
// 3. AML/KYC compliance scenario
// ---------------------------------------------------------------------------

#[test]
fn aml_kyc_compliance_scenario() {
    let mut tensor = ComplianceTensor::new(test_jurisdiction());

    // Set AML and KYC to compliant
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

    // Verify via slice
    let slice = tensor.slice(&[ComplianceDomain::Aml, ComplianceDomain::Kyc]);
    assert_eq!(slice.len(), 2);

    // Both compliant means passing
    assert!(
        slice.all_passing(),
        "AML and KYC both compliant should be all-passing"
    );

    // Set AML to non-compliant
    tensor.set(
        ComplianceDomain::Aml,
        ComplianceState::NonCompliant,
        vec![],
        None,
    );

    let slice_nc = tensor.slice(&[ComplianceDomain::Aml, ComplianceDomain::Kyc]);
    assert!(
        !slice_nc.all_passing(),
        "AML non-compliant should fail all_passing"
    );
    assert_eq!(slice_nc.non_compliant_domains().len(), 1);
    assert!(slice_nc
        .non_compliant_domains()
        .contains(&ComplianceDomain::Aml));
}

// ---------------------------------------------------------------------------
// 4. Cross-domain compliance check
// ---------------------------------------------------------------------------

#[test]
fn cross_domain_compliance_check() {
    let mut tensor = ComplianceTensor::new(test_jurisdiction());

    // Set various domains to different states
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
    tensor.set(
        ComplianceDomain::Tax,
        ComplianceState::Pending,
        vec![],
        None,
    );
    tensor.set(
        ComplianceDomain::Securities,
        ComplianceState::NotApplicable,
        vec![],
        None,
    );
    tensor.set(
        ComplianceDomain::Licensing,
        ComplianceState::Compliant,
        vec![],
        None,
    );

    // Full slice
    let full = tensor.full_slice();
    assert_eq!(full.len(), 20, "full slice must contain all 20 domains");

    // Check that pending domains are identified
    let pending = full.pending_domains();
    assert!(pending.contains(&ComplianceDomain::Tax));

    // Commitment is deterministic
    let c1 = tensor.commit().unwrap();
    let c2 = tensor.commit().unwrap();
    assert_eq!(c1.to_hex(), c2.to_hex());
}

// ---------------------------------------------------------------------------
// 5. Compliance domain count verification
// ---------------------------------------------------------------------------

#[test]
fn compliance_domain_count_is_20() {
    assert_eq!(ComplianceDomain::all().len(), 20);
    assert_eq!(ComplianceDomain::COUNT, 20);
}

// ---------------------------------------------------------------------------
// 6. Every domain in tensor evaluates without error
// ---------------------------------------------------------------------------

#[test]
fn every_domain_evaluates_without_error() {
    let tensor = ComplianceTensor::new(test_jurisdiction());
    let results = tensor.evaluate_all("entity-test");
    assert_eq!(results.len(), 20);

    for &domain in ComplianceDomain::all() {
        assert!(
            results.contains_key(&domain),
            "evaluate_all missing domain {domain}"
        );
    }
}
