//! # msez-compliance — Jurisdictional Compliance Bridge
//!
//! Connects the pack trilogy (jurisdictional configuration) to the
//! compliance tensor (evaluation engine). This crate provides:
//!
//! - [`RegpackJurisdiction`]: A `JurisdictionConfig` implementation driven
//!   by regpack metadata, scoping compliance evaluation to the domains
//!   that actually apply in each jurisdiction.
//!
//! - [`SanctionsEvaluator`]: A `DomainEvaluator` for the SANCTIONS domain
//!   that screens entities against the regpack's sanctions snapshot using
//!   fuzzy name matching.
//!
//! ## Architecture
//!
//! ```text
//! msez-pack (data)  -->  msez-compliance (bridge)  -->  msez-tensor (algebra)
//!   Regpack                RegpackJurisdiction             ComplianceTensor<J>
//!   SanctionsChecker       SanctionsEvaluator              DomainEvaluator trait
//! ```

pub mod evaluators;
pub mod jurisdiction;

pub use evaluators::SanctionsEvaluator;
pub use jurisdiction::RegpackJurisdiction;

use std::sync::Arc;

use msez_core::{ComplianceDomain, JurisdictionId};
use msez_pack::regpack::{SanctionsChecker, SanctionsEntry};
use msez_tensor::ComplianceTensor;

/// Build a fully configured compliance tensor for a jurisdiction.
///
/// This is the primary entry point for compliance evaluation. It:
/// 1. Creates a `RegpackJurisdiction` from the provided domain names.
/// 2. Instantiates a `ComplianceTensor` with that jurisdiction.
/// 3. If sanctions entries are provided, registers a `SanctionsEvaluator`.
///
/// The returned tensor is ready for evaluation via `evaluate_all()`.
///
/// # Errors
///
/// Returns `None` if `jurisdiction_id` is empty (invalid `JurisdictionId`).
pub fn build_tensor(
    jurisdiction_id: &str,
    applicable_domains: &[String],
    sanctions_entries: Option<(Vec<SanctionsEntry>, String)>,
) -> Option<ComplianceTensor<RegpackJurisdiction>> {
    let jid = match JurisdictionId::new(jurisdiction_id) {
        Ok(id) => id,
        Err(e) => {
            tracing::debug!(jurisdiction_id, error = %e, "invalid jurisdiction ID for tensor build");
            return None;
        }
    };
    let jurisdiction = RegpackJurisdiction::from_domain_names(jid, applicable_domains);

    let mut tensor = ComplianceTensor::new(jurisdiction);

    if let Some((entries, snapshot_id)) = sanctions_entries {
        let checker = Arc::new(SanctionsChecker::new(entries, snapshot_id));
        let evaluator = SanctionsEvaluator::with_default_threshold(checker);
        tensor.set_evaluator(ComplianceDomain::Sanctions, Box::new(evaluator));
    }

    Some(tensor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use msez_core::ComplianceDomain;
    use msez_pack::regpack::SanctionsEntry;
    use msez_tensor::{ComplianceState, DomainEvaluator, EvaluationContext};
    use std::collections::HashMap;

    fn domain_strings(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn regpack_jurisdiction_narrows_applicable_domains() {
        let jid = JurisdictionId::new("pk-sez-01").unwrap();
        let domains = domain_strings(&["aml", "kyc", "sanctions", "tax", "trade"]);
        let jurisdiction = RegpackJurisdiction::from_domain_names(jid, &domains);

        assert_eq!(jurisdiction.domain_count(), 5);

        let tensor = ComplianceTensor::new(jurisdiction);
        let slice = tensor.full_slice();

        // The 5 applicable domains should be Pending (awaiting attestation).
        assert_eq!(slice.cells[&ComplianceDomain::Aml], ComplianceState::Pending);
        assert_eq!(slice.cells[&ComplianceDomain::Tax], ComplianceState::Pending);

        // Non-applicable domains should be NotApplicable.
        assert_eq!(
            slice.cells[&ComplianceDomain::DigitalAssets],
            ComplianceState::NotApplicable
        );
        assert_eq!(
            slice.cells[&ComplianceDomain::Custody],
            ComplianceState::NotApplicable
        );

        // Aggregate: meet of Pending and NotApplicable = Pending.
        // (NotApplicable > Pending in the lattice.)
        assert_eq!(slice.aggregate_state(), ComplianceState::Pending);
    }

    #[test]
    fn different_jurisdictions_produce_different_tensors() {
        let pk = RegpackJurisdiction::from_domain_names(
            JurisdictionId::new("pk-sez-01").unwrap(),
            &domain_strings(&["aml", "kyc", "tax", "trade", "employment"]),
        );
        let ae = RegpackJurisdiction::from_domain_names(
            JurisdictionId::new("ae-difc").unwrap(),
            &domain_strings(&[
                "aml",
                "kyc",
                "sanctions",
                "securities",
                "digital_assets",
                "custody",
            ]),
        );

        assert_eq!(pk.domain_count(), 5);
        assert_eq!(ae.domain_count(), 6);

        let pk_tensor = ComplianceTensor::new(pk);
        let ae_tensor = ComplianceTensor::new(ae);

        // Employment is applicable in PK but not AE.
        assert_eq!(
            pk_tensor.get(ComplianceDomain::Employment),
            ComplianceState::Pending
        );
        assert_eq!(
            ae_tensor.get(ComplianceDomain::Employment),
            ComplianceState::NotApplicable
        );

        // Digital assets is applicable in AE but not PK.
        assert_eq!(
            pk_tensor.get(ComplianceDomain::DigitalAssets),
            ComplianceState::NotApplicable
        );
        assert_eq!(
            ae_tensor.get(ComplianceDomain::DigitalAssets),
            ComplianceState::Pending
        );

        // Tensor commitments should differ because the applicable domain
        // sets differ.
        let pk_commit = pk_tensor.commit().unwrap();
        let ae_commit = ae_tensor.commit().unwrap();
        assert_ne!(pk_commit.to_hex(), ae_commit.to_hex());
    }

    fn make_bout_entry() -> SanctionsEntry {
        SanctionsEntry {
            entry_id: "SDN-001".into(),
            entry_type: "individual".into(),
            source_lists: vec!["OFAC-SDN".into()],
            primary_name: "Viktor Bout".into(),
            aliases: vec![],
            identifiers: vec![],
            addresses: vec![],
            nationalities: vec!["RU".into()],
            date_of_birth: None,
            programs: vec!["SDGT".into()],
            listing_date: None,
            remarks: None,
        }
    }

    #[test]
    fn sanctions_evaluator_detects_match() {
        let checker = Arc::new(SanctionsChecker::new(
            vec![make_bout_entry()],
            "snap-01".into(),
        ));
        let evaluator = SanctionsEvaluator::with_default_threshold(checker);

        let ctx = EvaluationContext {
            entity_id: "entity:test".into(),
            current_state: None,
            attestations: vec![],
            metadata: {
                let mut m = HashMap::new();
                m.insert("entity_name".into(), serde_json::json!("Viktor Bout"));
                m
            },
        };

        let (state, reason) = evaluator.evaluate(&ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert!(reason.unwrap().contains("sanctions match"));
    }

    #[test]
    fn sanctions_evaluator_clears_non_match() {
        let checker = Arc::new(SanctionsChecker::new(
            vec![make_bout_entry()],
            "snap-01".into(),
        ));
        let evaluator = SanctionsEvaluator::with_default_threshold(checker);

        let ctx = EvaluationContext {
            entity_id: "entity:clean".into(),
            current_state: None,
            attestations: vec![],
            metadata: {
                let mut m = HashMap::new();
                m.insert("entity_name".into(), serde_json::json!("Alice Smith"));
                m
            },
        };

        let (state, reason) = evaluator.evaluate(&ctx);
        assert_eq!(state, ComplianceState::Compliant);
        assert_eq!(reason.unwrap(), "sanctions_clear");
    }

    #[test]
    fn sanctions_evaluator_pending_without_entity_name() {
        let checker = Arc::new(SanctionsChecker::new(vec![], "empty".into()));
        let evaluator = SanctionsEvaluator::with_default_threshold(checker);

        let ctx = EvaluationContext {
            entity_id: "entity:x".into(),
            current_state: None,
            attestations: vec![],
            metadata: HashMap::new(),
        };

        let (state, _) = evaluator.evaluate(&ctx);
        assert_eq!(state, ComplianceState::Pending);
    }

    #[test]
    fn build_tensor_convenience_function() {
        let tensor = build_tensor(
            "pk-sez-01",
            &domain_strings(&["aml", "kyc", "sanctions", "tax"]),
            None,
        )
        .unwrap();

        assert_eq!(tensor.get(ComplianceDomain::Aml), ComplianceState::Pending);
        assert_eq!(
            tensor.get(ComplianceDomain::DigitalAssets),
            ComplianceState::NotApplicable
        );

        // Tensor should have 20 cells total.
        assert_eq!(tensor.cell_count(), 20);
    }

    #[test]
    fn unknown_domain_in_regpack_is_warned_and_skipped() {
        let jid = JurisdictionId::new("test-zone").unwrap();
        let domains = domain_strings(&[
            "aml",
            "space_law",             // Does not exist in ComplianceDomain.
            "kyc",
            "quantum_compliance",    // Also does not exist.
        ]);
        let jurisdiction = RegpackJurisdiction::from_domain_names(jid, &domains);

        // Only aml and kyc should be recognized.
        assert_eq!(jurisdiction.domain_count(), 2);
    }

    #[test]
    fn build_tensor_with_sanctions() {
        let entries = vec![make_bout_entry()];
        let tensor = build_tensor(
            "ae-difc",
            &domain_strings(&["aml", "kyc", "sanctions"]),
            Some((entries, "OFAC-2026-02".into())),
        )
        .unwrap();

        // Sanctions domain should be Pending (evaluator is registered but
        // tensor.get() returns the cell state, not the evaluator result).
        assert_eq!(
            tensor.get(ComplianceDomain::Sanctions),
            ComplianceState::Pending
        );

        // Evaluating with an entity triggers the evaluator.
        let result = tensor.evaluate("entity:bout", ComplianceDomain::Sanctions);
        // The default evaluate path builds context without metadata, so
        // the evaluator returns Pending (no entity_name). This confirms
        // the evaluator is registered and runs.
        assert_eq!(result, ComplianceState::Pending);
    }

    #[test]
    fn build_tensor_returns_none_for_invalid_jurisdiction() {
        assert!(build_tensor("", &domain_strings(&["aml"]), None).is_none());
        assert!(build_tensor("   ", &domain_strings(&["aml"]), None).is_none());
    }

    #[test]
    fn from_domains_constructor() {
        let jid = JurisdictionId::new("sg").unwrap();
        let jurisdiction = RegpackJurisdiction::from_domains(
            jid,
            vec![ComplianceDomain::Aml, ComplianceDomain::Kyc],
        );
        assert_eq!(jurisdiction.domain_count(), 2);

        let tensor = ComplianceTensor::new(jurisdiction);
        assert_eq!(tensor.get(ComplianceDomain::Aml), ComplianceState::Pending);
        assert_eq!(
            tensor.get(ComplianceDomain::Trade),
            ComplianceState::NotApplicable
        );
    }
}
