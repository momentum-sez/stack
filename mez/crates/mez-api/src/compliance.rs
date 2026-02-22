//! # Shared Compliance Evaluation Logic
//!
//! Shared functions for building a compliance tensor, applying attestation
//! evidence, and building evaluation results. Used by both the compliance
//! evaluation endpoint (smart_assets) and the credential issuance endpoint
//! (credentials).

use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use mez_core::{ComplianceDomain, JurisdictionId};
use mez_compliance::RegpackJurisdiction;
use mez_pack::regpack;
use mez_tensor::{ComplianceState, ComplianceTensor, DefaultJurisdiction};

use crate::state::SmartAssetRecord;
#[cfg(test)]
use crate::state::{AssetComplianceStatus, AssetStatus, SmartAssetType};

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
    pub domain_results: BTreeMap<String, String>,
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
    let jid = JurisdictionId::new(jurisdiction_id)
        .unwrap_or_else(|_| JurisdictionId::from_static("UNKNOWN"));
    let jurisdiction = DefaultJurisdiction::new(jid);
    ComplianceTensor::new(jurisdiction)
}

/// Build a jurisdiction-scoped compliance tensor using regpack domain declarations.
///
/// Unlike [`build_tensor`] which evaluates all 20 domains equally, this function
/// consults the regpack registry to determine which domains are applicable for
/// the jurisdiction. Non-applicable domains are initialized as `NotApplicable`,
/// providing accurate compliance status rather than false-pending on irrelevant domains.
///
/// Falls back to all 20 domains if the jurisdiction has no regpack content (fail-closed).
pub fn build_jurisdiction_tensor(jurisdiction_id: &str) -> ComplianceTensor<RegpackJurisdiction> {
    let jid = JurisdictionId::new(jurisdiction_id)
        .unwrap_or_else(|_| JurisdictionId::from_static("UNKNOWN"));

    let domain_names = jurisdiction_applicable_domains(jurisdiction_id);
    let jurisdiction = RegpackJurisdiction::from_domain_names(jid, &domain_names);
    ComplianceTensor::new(jurisdiction)
}

/// Resolve the applicable compliance domains for a jurisdiction from regpack content.
///
/// The regpack registry uses broad categories ("financial", "sanctions");
/// these are expanded to specific `ComplianceDomain` variants. Unknown
/// jurisdictions evaluate all 20 domains (fail-closed).
pub fn jurisdiction_applicable_domains(jurisdiction_id: &str) -> Vec<String> {
    let regpack_domains = regpack::domains_for_jurisdiction(jurisdiction_id);

    match regpack_domains {
        Some(domains) => {
            let mut applicable = Vec::new();
            for domain_category in domains {
                match domain_category {
                    "financial" => {
                        applicable.extend_from_slice(&[
                            "aml".to_string(),
                            "kyc".to_string(),
                            "tax".to_string(),
                            "corporate".to_string(),
                            "banking".to_string(),
                            "payments".to_string(),
                            "licensing".to_string(),
                            "securities".to_string(),
                        ]);
                    }
                    "sanctions" => {
                        applicable.push("sanctions".to_string());
                    }
                    other => {
                        applicable.push(other.to_string());
                    }
                }
            }
            applicable.sort();
            applicable.dedup();
            applicable
        }
        None => {
            ComplianceDomain::all()
                .iter()
                .map(|d| d.as_str().to_string())
                .collect()
        }
    }
}

/// Apply attestations to a jurisdiction-scoped tensor.
pub fn apply_jurisdiction_attestations(
    tensor: &mut ComplianceTensor<RegpackJurisdiction>,
    attestations: &HashMap<String, AttestationInput>,
) {
    for (domain_str, input) in attestations {
        let domain: ComplianceDomain = match domain_str.parse() {
            Ok(d) => d,
            Err(_) => {
                tracing::warn!(domain = %domain_str, "Unknown compliance domain in attestation — skipping");
                continue;
            }
        };
        let state = match parse_status(&input.status) {
            Some(s) => s,
            None => {
                tracing::warn!(domain = %domain_str, status = %input.status, "Unknown attestation status — skipping");
                continue;
            }
        };
        tensor.set(domain, state, Vec::new(), None);
    }
}

/// Build an evaluation result from a jurisdiction-scoped tensor and asset.
pub fn build_jurisdiction_evaluation_result(
    tensor: &ComplianceTensor<RegpackJurisdiction>,
    asset: &SmartAssetRecord,
    asset_id: Uuid,
) -> ComplianceEvalResult {
    let slice = tensor.full_slice();
    let aggregate = slice.aggregate_state();
    let commitment = tensor
        .commit()
        .map_err(|e| {
            tracing::warn!(error = %e, "tensor commitment failed — response will omit commitment");
            e
        })
        .ok();

    let mut domain_results = BTreeMap::new();
    let mut passing_domains = Vec::new();
    let mut blocking_domains = Vec::new();

    for &domain in ComplianceDomain::all() {
        let state = tensor.get(domain);
        // Use Display (snake_case: "non_compliant") not Debug (CamelCase: "noncompliant")
        let state_str = format!("{state}");
        domain_results.insert(domain.as_str().to_string(), state_str);

        if state.is_passing() {
            passing_domains.push(domain.as_str().to_string());
        } else {
            blocking_domains.push(domain.as_str().to_string());
        }
    }

    passing_domains.sort();
    blocking_domains.sort();

    ComplianceEvalResult {
        asset_id,
        jurisdiction_id: asset.jurisdiction_id.clone(),
        // Use Display (snake_case) not Debug (CamelCase) for API consistency
        overall_status: format!("{aggregate}"),
        domain_results,
        domain_count: ComplianceDomain::all().len(),
        passing_domains,
        blocking_domains,
        tensor_commitment: commitment.map(|c| c.to_hex()),
        evaluated_at: Utc::now(),
    }
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
            Err(_) => {
                tracing::warn!(domain = %domain_str, "Unknown compliance domain in attestation — skipping");
                continue;
            }
        };
        let state = match parse_status(&input.status) {
            Some(s) => s,
            None => {
                tracing::warn!(domain = %domain_str, status = %input.status, "Unknown attestation status — skipping");
                continue;
            }
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
    let commitment = tensor
        .commit()
        .map_err(|e| {
            tracing::warn!(error = %e, "tensor commitment failed — response will omit commitment");
            e
        })
        .ok();

    let mut domain_results = BTreeMap::new();
    let mut passing_domains = Vec::new();
    let mut blocking_domains = Vec::new();

    for &domain in ComplianceDomain::all() {
        let state = tensor.get(domain);
        // Use Display (snake_case: "non_compliant") not Debug (CamelCase: "noncompliant")
        let state_str = format!("{state}");
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
        // Use Display (snake_case) not Debug (CamelCase) for API consistency
        overall_status: format!("{aggregate}"),
        domain_results,
        domain_count: ComplianceDomain::all().len(),
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
        let tensor = build_tensor("PK-PEZ");
        assert_eq!(tensor.cell_count(), 20);
    }

    #[test]
    fn apply_attestations_sets_domain_state() {
        let mut tensor = build_tensor("PK-PEZ");
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
        assert_eq!(
            tensor.get(ComplianceDomain::Aml),
            ComplianceState::Compliant
        );
    }

    #[test]
    fn apply_attestations_ignores_unknown_domain() {
        let mut tensor = build_tensor("PK-PEZ");
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
        let mut tensor = build_tensor("PK-PEZ");
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
        let mut tensor = build_tensor("PK-PEZ");
        for &domain in ComplianceDomain::all() {
            tensor.set(domain, ComplianceState::Compliant, Vec::new(), None);
        }
        let asset = SmartAssetRecord {
            id: Uuid::new_v4(),
            asset_type: SmartAssetType::new("bond").expect("valid"),
            jurisdiction_id: "PK-PEZ".to_string(),
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
