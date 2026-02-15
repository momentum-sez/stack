//! # Shared Compliance Evaluation Logic
//!
//! Shared functions for building a compliance tensor, applying attestation
//! evidence, and building evaluation results. Used by both the compliance
//! evaluation endpoint (smart_assets) and the credential issuance endpoint
//! (credentials).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use msez_core::{ComplianceDomain, JurisdictionId};
use msez_tensor::{ComplianceState, ComplianceTensor, DefaultJurisdiction};

use crate::state::SmartAssetRecord;
#[cfg(test)]
use crate::state::{AssetComplianceStatus, AssetStatus};

/// Attestation evidence for a single compliance domain.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AttestationInput {
    /// The compliance status to attest: "compliant", "exempt", or "not_applicable".
    pub status: String,
    /// DID of the attestation issuer.
    pub issuer_did: Option<String>,
    /// ISO 8601 expiration timestamp for this attestation.
    pub expires_at: Option<String>,
}

/// Compliance evaluation result returned by both the evaluation and
/// credential issuance endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ComplianceEvalResult {
    /// The asset that was evaluated.
    pub asset_id: Uuid,
    /// The jurisdiction used for evaluation.
    pub jurisdiction_id: String,
    /// Aggregate compliance status across all 20 domains.
    pub overall_status: String,
    /// Per-domain compliance status.
    pub domain_results: HashMap<String, String>,
    /// Total number of domains evaluated.
    pub domain_count: usize,
    /// Domains that are in a passing state (compliant, exempt, not_applicable).
    pub passing_domains: Vec<String>,
    /// Domains that are blocking (non_compliant or pending).
    pub blocking_domains: Vec<String>,
    /// SHA-256 tensor commitment digest (hex).
    pub tensor_commitment: Option<String>,
    /// When the evaluation was performed.
    pub evaluated_at: DateTime<Utc>,
}

/// Build a compliance tensor for a jurisdiction.
///
/// Falls back to `"UNKNOWN"` jurisdiction if the input fails validation,
/// since every evaluation must produce a result (even if degraded).
pub fn build_tensor(jurisdiction_id: &str) -> ComplianceTensor<DefaultJurisdiction> {
    // Both paths are infallible in practice — JurisdictionId::new only
    // rejects empty strings, and "UNKNOWN" is non-empty. However, we
    // avoid unwrap() in library code by providing a hardcoded fallback
    // that cannot fail structurally.
    let jid = JurisdictionId::new(jurisdiction_id).unwrap_or_else(|_| {
        // SAFETY: "UNKNOWN" is a non-empty string literal, so this
        // construction is infallible.
        JurisdictionId::new("UNKNOWN").expect("BUG: hardcoded 'UNKNOWN' rejected by JurisdictionId")
    });
    let jurisdiction = DefaultJurisdiction::new(jid);
    ComplianceTensor::new(jurisdiction)
}

/// Parse a status string into a `ComplianceState`.
fn parse_status(s: &str) -> Option<ComplianceState> {
    match s.to_lowercase().as_str() {
        "compliant" => Some(ComplianceState::Compliant),
        "non_compliant" | "noncompliant" => Some(ComplianceState::NonCompliant),
        "pending" => Some(ComplianceState::Pending),
        "exempt" => Some(ComplianceState::Exempt),
        "not_applicable" | "notapplicable" => Some(ComplianceState::NotApplicable),
        _ => None,
    }
}

/// Apply attestation inputs to a tensor, setting domain states based on
/// the provided evidence.
pub fn apply_attestations(
    tensor: &mut ComplianceTensor<DefaultJurisdiction>,
    attestations: &HashMap<String, AttestationInput>,
) {
    for (domain_str, input) in attestations {
        let domain: ComplianceDomain = match domain_str.parse() {
            Ok(d) => d,
            Err(_) => continue,
        };
        let state = match parse_status(&input.status) {
            Some(s) => s,
            None => continue,
        };
        tensor.set(domain, state, Vec::new(), None);
    }
}

/// Build an evaluation result from a tensor and asset.
pub fn build_evaluation_result(
    tensor: &ComplianceTensor<DefaultJurisdiction>,
    asset: &SmartAssetRecord,
    asset_id: Uuid,
) -> ComplianceEvalResult {
    let slice = tensor.full_slice();
    let aggregate = slice.aggregate_state();
    let commitment = tensor.commit().map_err(|e| {
        tracing::warn!(error = %e, "tensor commitment failed — response will omit commitment");
        e
    }).ok();

    let mut domain_results = HashMap::new();
    let mut passing_domains = Vec::new();
    let mut blocking_domains = Vec::new();

    for &domain in ComplianceDomain::all() {
        let state = tensor.get(domain);
        let state_str = format!("{state:?}").to_lowercase();
        domain_results.insert(domain.as_str().to_string(), state_str);

        if state.is_passing() {
            passing_domains.push(domain.as_str().to_string());
        } else {
            blocking_domains.push(domain.as_str().to_string());
        }
    }

    // Sort for deterministic output.
    passing_domains.sort();
    blocking_domains.sort();

    ComplianceEvalResult {
        asset_id,
        jurisdiction_id: asset.jurisdiction_id.clone(),
        overall_status: format!("{aggregate:?}").to_lowercase(),
        domain_results,
        domain_count: 20,
        passing_domains,
        blocking_domains,
        tensor_commitment: commitment.map(|c| c.to_hex()),
        evaluated_at: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_tensor_creates_20_cells() {
        let tensor = build_tensor("PK-PSEZ");
        assert_eq!(tensor.cell_count(), 20);
    }

    #[test]
    fn apply_attestations_sets_domain_state() {
        let mut tensor = build_tensor("PK-PSEZ");
        let mut attestations = HashMap::new();
        attestations.insert(
            "aml".to_string(),
            AttestationInput {
                status: "compliant".to_string(),
                issuer_did: None,
                expires_at: None,
            },
        );
        apply_attestations(&mut tensor, &attestations);
        assert_eq!(tensor.get(ComplianceDomain::Aml), ComplianceState::Compliant);
    }

    #[test]
    fn apply_attestations_ignores_unknown_domain() {
        let mut tensor = build_tensor("PK-PSEZ");
        let mut attestations = HashMap::new();
        attestations.insert(
            "nonexistent_domain".to_string(),
            AttestationInput {
                status: "compliant".to_string(),
                issuer_did: None,
                expires_at: None,
            },
        );
        apply_attestations(&mut tensor, &attestations);
        // All domains should still be Pending (no change).
        assert_eq!(tensor.get(ComplianceDomain::Aml), ComplianceState::Pending);
    }

    #[test]
    fn apply_attestations_ignores_unknown_status() {
        let mut tensor = build_tensor("PK-PSEZ");
        let mut attestations = HashMap::new();
        attestations.insert(
            "aml".to_string(),
            AttestationInput {
                status: "bogus_status".to_string(),
                issuer_did: None,
                expires_at: None,
            },
        );
        apply_attestations(&mut tensor, &attestations);
        assert_eq!(tensor.get(ComplianceDomain::Aml), ComplianceState::Pending);
    }

    #[test]
    fn parse_status_all_variants() {
        assert_eq!(parse_status("compliant"), Some(ComplianceState::Compliant));
        assert_eq!(
            parse_status("non_compliant"),
            Some(ComplianceState::NonCompliant)
        );
        assert_eq!(parse_status("pending"), Some(ComplianceState::Pending));
        assert_eq!(parse_status("exempt"), Some(ComplianceState::Exempt));
        assert_eq!(
            parse_status("not_applicable"),
            Some(ComplianceState::NotApplicable)
        );
        assert_eq!(parse_status("unknown"), None);
    }

    #[test]
    fn build_evaluation_result_with_all_passing() {
        let mut tensor = build_tensor("PK-PSEZ");
        for &domain in ComplianceDomain::all() {
            tensor.set(domain, ComplianceState::Compliant, Vec::new(), None);
        }
        let asset = SmartAssetRecord {
            id: Uuid::new_v4(),
            asset_type: "bond".to_string(),
            jurisdiction_id: "PK-PSEZ".to_string(),
            status: AssetStatus::Genesis,
            genesis_digest: None,
            compliance_status: AssetComplianceStatus::Unevaluated,
            metadata: serde_json::json!({}),
            owner_entity_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = build_evaluation_result(&tensor, &asset, asset.id);
        assert_eq!(result.overall_status, "compliant");
        assert_eq!(result.domain_count, 20);
        assert_eq!(result.passing_domains.len(), 20);
        assert!(result.blocking_domains.is_empty());
        assert!(result.tensor_commitment.is_some());
    }
}
