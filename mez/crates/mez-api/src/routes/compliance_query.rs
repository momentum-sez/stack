//! # Compliance Query API
//!
//! Programmatic compliance visibility endpoints. These are the regulator-facing
//! and integrator-facing APIs that answer: "What is the compliance state of
//! this jurisdiction / entity / corridor?"
//!
//! ## Endpoints
//!
//! - `GET /v1/compliance/domains` — List all compliance domains with descriptions
//! - `GET /v1/compliance/:jurisdiction_id` — Tensor state across all 20 domains
//! - `GET /v1/compliance/entity/:entity_id` — Entity-level compliance with attestation provenance
//! - `GET /v1/compliance/corridor/:corridor_id` — Bilateral compliance for a corridor
//!
//! These endpoints fulfill Roadmap Priority 2 (API Surface Hardening) and Phase 2
//! requirement (Cross-zone compliance query).

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use mez_compliance::RegpackJurisdiction;
use mez_core::ComplianceDomain;
use mez_tensor::{ComplianceState, ComplianceTensor, JurisdictionConfig};

use crate::error::AppError;
use crate::state::AppState;

/// Compliance tensor evaluation result for a jurisdiction.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct JurisdictionComplianceResponse {
    /// The jurisdiction that was evaluated.
    pub jurisdiction_id: String,
    /// Aggregate compliance status across all 20 domains.
    pub overall_status: String,
    /// Per-domain compliance state.
    pub domains: Vec<DomainComplianceEntry>,
    /// Count of passing domains (compliant, exempt, not_applicable).
    pub passing_count: usize,
    /// Count of blocking domains (non_compliant, pending).
    pub blocking_count: usize,
    /// Total domain count (always 20 per spec).
    pub total_domains: usize,
    /// SHA-256 tensor commitment digest (hex).
    pub tensor_commitment: Option<String>,
    /// Evaluation timestamp.
    pub evaluated_at: String,
}

/// A single compliance domain entry in the evaluation result.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DomainComplianceEntry {
    /// Domain identifier (e.g., "aml", "kyc", "sanctions").
    pub domain: String,
    /// Current compliance state for this domain.
    pub status: String,
    /// Whether this domain is in a passing state.
    pub passing: bool,
}

/// Bilateral compliance result for a corridor's two jurisdictions.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CorridorComplianceResponse {
    /// The corridor that was evaluated.
    pub corridor_id: Uuid,
    /// Compliance evaluation for jurisdiction A.
    pub jurisdiction_a: JurisdictionComplianceSummary,
    /// Compliance evaluation for jurisdiction B.
    pub jurisdiction_b: JurisdictionComplianceSummary,
    /// Overall corridor compliance: true only if both jurisdictions are fully compliant.
    pub corridor_compliant: bool,
    /// Domains that are blocking in either jurisdiction.
    pub cross_blocking_domains: Vec<CrossBlockingDomain>,
    /// Evaluation timestamp.
    pub evaluated_at: String,
}

/// Summary of a single jurisdiction's compliance within a corridor.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct JurisdictionComplianceSummary {
    /// Jurisdiction identifier.
    pub jurisdiction_id: String,
    /// Aggregate compliance status.
    pub overall_status: String,
    /// Count of passing domains.
    pub passing_count: usize,
    /// Count of blocking domains.
    pub blocking_count: usize,
}

/// A domain that is blocking compliance in one or both corridor jurisdictions.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CrossBlockingDomain {
    /// The compliance domain name.
    pub domain: String,
    /// Status in jurisdiction A.
    pub status_a: String,
    /// Status in jurisdiction B.
    pub status_b: String,
}

/// Description of a compliance domain.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ComplianceDomainInfo {
    /// Domain identifier.
    pub domain: String,
    /// Human-readable description.
    pub description: String,
    /// Whether this is one of the original 8 or an extended domain.
    pub category: String,
}

/// Entity-level compliance evaluation response.
///
/// Returns the compliance tensor state for a specific entity, using stored
/// attestation records to populate domain states. Each attestation contributes
/// evidence to its corresponding compliance domain.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EntityComplianceResponse {
    /// The entity that was evaluated.
    pub entity_id: Uuid,
    /// Jurisdiction of the entity.
    pub jurisdiction_id: String,
    /// Aggregate compliance status.
    pub overall_status: String,
    /// Per-domain compliance state with attestation details.
    pub domains: Vec<EntityDomainEntry>,
    /// Count of passing domains.
    pub passing_count: usize,
    /// Count of blocking domains.
    pub blocking_count: usize,
    /// Total domain count.
    pub total_domains: usize,
    /// Number of attestation records found for this entity.
    pub attestation_count: usize,
    /// SHA-256 tensor commitment digest (hex).
    pub tensor_commitment: Option<String>,
    /// Evaluation timestamp.
    pub evaluated_at: String,
}

/// Entity-level domain entry with attestation provenance.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EntityDomainEntry {
    /// Domain identifier.
    pub domain: String,
    /// Current compliance state.
    pub status: String,
    /// Whether this domain is passing.
    pub passing: bool,
    /// Attestation ID that contributed to this domain state, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attestation_id: Option<Uuid>,
    /// When the attestation expires, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// Build the compliance query router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/compliance/domains", get(list_domains))
        .route("/v1/compliance/:jurisdiction_id", get(query_jurisdiction))
        .route(
            "/v1/compliance/entity/:entity_id",
            get(query_entity),
        )
        .route(
            "/v1/compliance/corridor/:corridor_id",
            get(query_corridor),
        )
}

/// GET /v1/compliance/domains — List all 20 compliance domains.
#[utoipa::path(
    get,
    path = "/v1/compliance/domains",
    responses(
        (status = 200, description = "All compliance domains", body = Vec<ComplianceDomainInfo>),
    ),
    tag = "compliance"
)]
async fn list_domains() -> Json<Vec<ComplianceDomainInfo>> {
    let domains: Vec<ComplianceDomainInfo> = ComplianceDomain::all()
        .iter()
        .map(|d| ComplianceDomainInfo {
            domain: d.as_str().to_string(),
            description: domain_description(d),
            category: domain_category(d),
        })
        .collect();
    Json(domains)
}

/// GET /v1/compliance/:jurisdiction_id — Evaluate compliance tensor for a jurisdiction.
///
/// Returns the full 20-domain compliance tensor state for the given jurisdiction.
/// In the current implementation, domains start in Pending state unless attestation
/// evidence has been submitted. This endpoint shows the current evaluation state.
#[utoipa::path(
    get,
    path = "/v1/compliance/{jurisdiction_id}",
    params(("jurisdiction_id" = String, Path, description = "Jurisdiction ID (e.g., pk, ae, ae-dubai-difc)")),
    responses(
        (status = 200, description = "Compliance tensor evaluation", body = JurisdictionComplianceResponse),
    ),
    tag = "compliance"
)]
async fn query_jurisdiction(
    State(state): State<AppState>,
    Path(jurisdiction_id): Path<String>,
) -> Result<Json<JurisdictionComplianceResponse>, AppError> {
    let tensor = build_jurisdiction_tensor(&jurisdiction_id);
    let response = evaluate_tensor(&jurisdiction_id, &tensor, &state);
    Ok(Json(response))
}

/// GET /v1/compliance/entity/:entity_id — Compliance tensor for a specific entity.
///
/// Evaluates the compliance tensor for an entity, using stored attestation
/// records to populate domain states. Each attestation's type is mapped to a
/// compliance domain. If the entity has no attestations, all applicable domains
/// start as `Pending` (fail-closed).
///
/// The entity's jurisdiction is inferred from the most recent attestation
/// record. If no attestations exist, the entity is evaluated against all 20
/// domains (worst-case).
#[utoipa::path(
    get,
    path = "/v1/compliance/entity/{entity_id}",
    params(("entity_id" = Uuid, Path, description = "Entity UUID")),
    responses(
        (status = 200, description = "Entity compliance evaluation", body = EntityComplianceResponse),
        (status = 404, description = "Entity not found", body = crate::error::ErrorBody),
    ),
    tag = "compliance"
)]
async fn query_entity(
    State(state): State<AppState>,
    Path(entity_id): Path<Uuid>,
) -> Result<Json<EntityComplianceResponse>, AppError> {
    // Find all attestation records for this entity.
    let attestations: Vec<_> = state
        .attestations
        .list()
        .into_iter()
        .filter(|a| a.entity_id == entity_id)
        .collect();

    if attestations.is_empty() {
        return Err(AppError::NotFound(format!(
            "no attestation records found for entity {entity_id}"
        )));
    }

    // Infer jurisdiction from the most recent attestation.
    let jurisdiction_id = attestations
        .iter()
        .max_by_key(|a| a.issued_at)
        .map(|a| a.jurisdiction_id.clone())
        .unwrap_or_else(|| "UNKNOWN".to_string());

    let tensor = build_jurisdiction_tensor(&jurisdiction_id);

    // Map attestation types to compliance domains and track provenance.
    let mut domain_attestations: std::collections::HashMap<String, &crate::state::AttestationRecord> =
        std::collections::HashMap::new();

    for att in &attestations {
        if let Some(domain_name) = attestation_type_to_domain(&att.attestation_type) {
            // Keep the most recent attestation per domain.
            let is_newer = domain_attestations
                .get(domain_name)
                .map_or(true, |existing| att.issued_at > existing.issued_at);
            if is_newer {
                domain_attestations.insert(domain_name.to_string(), att);
            }
        }
    }

    // Build the entity response with attestation provenance.
    let mut domains = Vec::new();
    let mut passing_count = 0;
    let mut blocking_count = 0;
    let now = Utc::now();

    for domain in ComplianceDomain::all() {
        let tensor_state = tensor.get(*domain);
        let domain_name = domain.as_str().to_string();

        // Check if we have an attestation for this domain.
        let (status, att_id, expires_at) =
            if let Some(att) = domain_attestations.get(&domain_name) {
                let expired = att.expires_at.is_some_and(|exp| exp < now);
                let att_status = if expired {
                    ComplianceState::NonCompliant
                } else {
                    match att.status {
                        crate::state::AttestationStatus::Active => ComplianceState::Compliant,
                        crate::state::AttestationStatus::Revoked => {
                            ComplianceState::NonCompliant
                        }
                        crate::state::AttestationStatus::Expired => {
                            ComplianceState::NonCompliant
                        }
                        crate::state::AttestationStatus::Pending => ComplianceState::Pending,
                    }
                };
                (
                    att_status,
                    Some(att.id),
                    att.expires_at.map(|e| e.to_rfc3339()),
                )
            } else {
                (tensor_state, None, None)
            };

        let passing = status.is_passing();
        if passing {
            passing_count += 1;
        } else {
            blocking_count += 1;
        }

        domains.push(EntityDomainEntry {
            domain: domain_name,
            status: format_state(status),
            passing,
            attestation_id: att_id,
            expires_at,
        });
    }

    let overall_status = if domains.iter().any(|d| d.status == "non_compliant") {
        "non_compliant".to_string()
    } else if domains.iter().all(|d| d.passing) {
        "compliant".to_string()
    } else {
        "pending".to_string()
    };

    let commitment = tensor.commit().map(|c| c.to_hex()).ok();

    Ok(Json(EntityComplianceResponse {
        entity_id,
        jurisdiction_id,
        overall_status,
        domains,
        passing_count,
        blocking_count,
        total_domains: ComplianceDomain::all().len(),
        attestation_count: attestations.len(),
        tensor_commitment: commitment,
        evaluated_at: Utc::now().to_rfc3339(),
    }))
}

/// Map attestation type strings to compliance domain names.
///
/// Attestation types follow a naming convention established in
/// `orchestration.rs`: `FORMATION_COMPLIANCE`, `OWNERSHIP_COMPLIANCE`, etc.
/// This function maps those to the specific domains they attest to.
fn attestation_type_to_domain(attestation_type: &str) -> Option<&'static str> {
    match attestation_type {
        "FORMATION_COMPLIANCE" | "entity_compliance" => Some("corporate"),
        "OWNERSHIP_COMPLIANCE" | "ownership_compliance" => Some("securities"),
        "FISCAL_COMPLIANCE" | "fiscal_compliance" => Some("tax"),
        "PAYMENT_COMPLIANCE" | "payment_compliance" => Some("payments"),
        "IDENTITY_COMPLIANCE" | "identity_compliance" => Some("kyc"),
        "CONSENT_COMPLIANCE" | "consent_compliance" => Some("data_privacy"),
        "AML_COMPLIANCE" | "aml_compliance" => Some("aml"),
        "SANCTIONS_COMPLIANCE" | "sanctions_compliance" => Some("sanctions"),
        "LICENSING_COMPLIANCE" | "licensing_compliance" => Some("licensing"),
        "BANKING_COMPLIANCE" | "banking_compliance" => Some("banking"),
        _ => None,
    }
}

/// GET /v1/compliance/corridor/:corridor_id — Bilateral compliance for a corridor.
///
/// Evaluates the compliance tensor for both jurisdictions of the corridor and
/// computes bilateral compliance status. The corridor is fully compliant only
/// when both jurisdictions pass all mandatory domains.
#[utoipa::path(
    get,
    path = "/v1/compliance/corridor/{corridor_id}",
    params(("corridor_id" = Uuid, Path, description = "Corridor UUID")),
    responses(
        (status = 200, description = "Bilateral corridor compliance", body = CorridorComplianceResponse),
        (status = 404, description = "Corridor not found", body = crate::error::ErrorBody),
    ),
    tag = "compliance"
)]
async fn query_corridor(
    State(state): State<AppState>,
    Path(corridor_id): Path<Uuid>,
) -> Result<Json<CorridorComplianceResponse>, AppError> {
    let corridor = state
        .corridors
        .get(&corridor_id)
        .ok_or_else(|| AppError::NotFound(format!("corridor {corridor_id} not found")))?;

    let tensor_a = build_jurisdiction_tensor(&corridor.jurisdiction_a);
    let tensor_b = build_jurisdiction_tensor(&corridor.jurisdiction_b);

    let eval_a = evaluate_tensor(&corridor.jurisdiction_a, &tensor_a, &state);
    let eval_b = evaluate_tensor(&corridor.jurisdiction_b, &tensor_b, &state);

    // Find domains blocking in either jurisdiction.
    let mut cross_blocking = Vec::new();
    for domain in ComplianceDomain::all() {
        let state_a = tensor_a.get(*domain);
        let state_b = tensor_b.get(*domain);
        if !state_a.is_passing() || !state_b.is_passing() {
            cross_blocking.push(CrossBlockingDomain {
                domain: domain.as_str().to_string(),
                status_a: format_state(state_a),
                status_b: format_state(state_b),
            });
        }
    }

    let corridor_compliant = eval_a.blocking_count == 0 && eval_b.blocking_count == 0;

    let response = CorridorComplianceResponse {
        corridor_id,
        jurisdiction_a: JurisdictionComplianceSummary {
            jurisdiction_id: corridor.jurisdiction_a.clone(),
            overall_status: eval_a.overall_status,
            passing_count: eval_a.passing_count,
            blocking_count: eval_a.blocking_count,
        },
        jurisdiction_b: JurisdictionComplianceSummary {
            jurisdiction_id: corridor.jurisdiction_b.clone(),
            overall_status: eval_b.overall_status,
            passing_count: eval_b.passing_count,
            blocking_count: eval_b.blocking_count,
        },
        corridor_compliant,
        cross_blocking_domains: cross_blocking,
        evaluated_at: Utc::now().to_rfc3339(),
    };

    Ok(Json(response))
}

/// Build a compliance tensor scoped to the applicable domains for a jurisdiction.
///
/// Delegates to `crate::compliance::build_jurisdiction_tensor` — the shared
/// implementation that consults regpack domain declarations.
fn build_jurisdiction_tensor(jurisdiction_id: &str) -> ComplianceTensor<RegpackJurisdiction> {
    crate::compliance::build_jurisdiction_tensor(jurisdiction_id)
}

/// Evaluate a tensor and produce the response structure.
fn evaluate_tensor<J: JurisdictionConfig>(
    jurisdiction_id: &str,
    tensor: &ComplianceTensor<J>,
    _state: &AppState,
) -> JurisdictionComplianceResponse {
    let mut domains = Vec::new();
    let mut passing_count = 0;
    let mut blocking_count = 0;

    for domain in ComplianceDomain::all() {
        let state = tensor.get(*domain);
        let passing = state.is_passing();
        if passing {
            passing_count += 1;
        } else {
            blocking_count += 1;
        }
        domains.push(DomainComplianceEntry {
            domain: domain.as_str().to_string(),
            status: format_state(state),
            passing,
        });
    }

    let slice = tensor.full_slice();
    let aggregate = slice.aggregate_state();
    let overall_status = format_state(aggregate);

    let commitment = tensor
        .commit()
        .map(|c| c.to_hex())
        .ok();

    JurisdictionComplianceResponse {
        jurisdiction_id: jurisdiction_id.to_string(),
        overall_status,
        domains,
        passing_count,
        blocking_count,
        total_domains: ComplianceDomain::all().len(),
        tensor_commitment: commitment,
        evaluated_at: Utc::now().to_rfc3339(),
    }
}

fn format_state(state: ComplianceState) -> String {
    match state {
        ComplianceState::Compliant => "compliant".to_string(),
        ComplianceState::NonCompliant => "non_compliant".to_string(),
        ComplianceState::Pending => "pending".to_string(),
        ComplianceState::Exempt => "exempt".to_string(),
        ComplianceState::NotApplicable => "not_applicable".to_string(),
    }
}

fn domain_description(domain: &ComplianceDomain) -> String {
    match domain {
        ComplianceDomain::Aml => "Anti-Money Laundering compliance".to_string(),
        ComplianceDomain::Kyc => "Know Your Customer verification".to_string(),
        ComplianceDomain::Sanctions => "Sanctions screening and enforcement".to_string(),
        ComplianceDomain::Tax => "Tax compliance and reporting".to_string(),
        ComplianceDomain::Securities => "Securities regulation compliance".to_string(),
        ComplianceDomain::Corporate => "Corporate governance and formation".to_string(),
        ComplianceDomain::Custody => "Asset custody and safekeeping".to_string(),
        ComplianceDomain::DataPrivacy => "Data protection and privacy".to_string(),
        ComplianceDomain::Licensing => "Business and professional licensing".to_string(),
        ComplianceDomain::Banking => "Banking regulation and capital adequacy".to_string(),
        ComplianceDomain::Payments => "Payment services regulation".to_string(),
        ComplianceDomain::Clearing => "Clearing and settlement rules".to_string(),
        ComplianceDomain::Settlement => "Settlement finality and DvP".to_string(),
        ComplianceDomain::DigitalAssets => "Digital asset and token regulation".to_string(),
        ComplianceDomain::Employment => "Employment and labor law".to_string(),
        ComplianceDomain::Immigration => "Immigration and visa regulation".to_string(),
        ComplianceDomain::Ip => "Intellectual property protection".to_string(),
        ComplianceDomain::ConsumerProtection => "Consumer protection regulation".to_string(),
        ComplianceDomain::Arbitration => "Arbitration and dispute resolution".to_string(),
        ComplianceDomain::Trade => "Trade regulation, import/export controls".to_string(),
    }
}

fn domain_category(domain: &ComplianceDomain) -> String {
    match domain {
        ComplianceDomain::Aml
        | ComplianceDomain::Kyc
        | ComplianceDomain::Sanctions
        | ComplianceDomain::Tax
        | ComplianceDomain::Securities
        | ComplianceDomain::Corporate
        | ComplianceDomain::Licensing
        | ComplianceDomain::DataPrivacy => "core".to_string(),
        ComplianceDomain::Custody
        | ComplianceDomain::Banking
        | ComplianceDomain::Payments
        | ComplianceDomain::Clearing
        | ComplianceDomain::Settlement
        | ComplianceDomain::DigitalAssets
        | ComplianceDomain::Employment
        | ComplianceDomain::Immigration
        | ComplianceDomain::Ip
        | ComplianceDomain::ConsumerProtection
        | ComplianceDomain::Arbitration
        | ComplianceDomain::Trade => "extended".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_app() -> Router<()> {
        router().with_state(AppState::new())
    }

    async fn body_json<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn list_domains_returns_20_domains() {
        let app = test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/v1/compliance/domains")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let domains: Vec<ComplianceDomainInfo> = body_json(resp).await;
        assert_eq!(domains.len(), 20, "must return all 20 compliance domains");

        // Verify core domains exist.
        let names: Vec<&str> = domains.iter().map(|d| d.domain.as_str()).collect();
        assert!(names.contains(&"aml"));
        assert!(names.contains(&"kyc"));
        assert!(names.contains(&"sanctions"));
        assert!(names.contains(&"tax"));
    }

    #[tokio::test]
    async fn query_jurisdiction_returns_tensor() {
        let app = test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/v1/compliance/pk")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let result: JurisdictionComplianceResponse = body_json(resp).await;
        assert_eq!(result.jurisdiction_id, "pk");
        assert_eq!(result.total_domains, 20);
        assert_eq!(result.domains.len(), 20);
        // Pakistan has regpack content for "financial" + "sanctions" domains,
        // which expands to 9 applicable domains. The remaining 11 are NotApplicable.
        assert!(result.passing_count > 0, "non-applicable domains should be passing");
        assert!(result.blocking_count > 0, "applicable domains should be pending/blocking");
        assert_eq!(
            result.passing_count + result.blocking_count,
            20,
            "passing + blocking must equal 20"
        );
        // Overall status is Pending (not all domains pass, but none are NonCompliant).
        assert_eq!(result.overall_status, "pending");
        assert!(result.tensor_commitment.is_some());
    }

    #[tokio::test]
    async fn query_unknown_jurisdiction_evaluates_all_20_domains() {
        let app = test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/v1/compliance/zz-unknown")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let result: JurisdictionComplianceResponse = body_json(resp).await;
        assert_eq!(result.jurisdiction_id, "zz-unknown");
        // Unknown jurisdictions: all 20 domains evaluated (fail-closed).
        assert_eq!(result.blocking_count, 20);
        assert_eq!(result.passing_count, 0);
    }

    #[test]
    fn jurisdiction_applicable_domains_known_jurisdiction() {
        let domains = crate::compliance::jurisdiction_applicable_domains("pk");
        // Pakistan has "financial" + "sanctions" regpack content.
        // "financial" expands to: aml, banking, corporate, kyc, licensing, payments, securities, tax
        // "sanctions" adds: sanctions
        assert!(domains.contains(&"aml".to_string()));
        assert!(domains.contains(&"sanctions".to_string()));
        assert!(domains.contains(&"banking".to_string()));
        assert!(domains.len() == 9, "pk should have 9 applicable domains, got {}", domains.len());
    }

    #[test]
    fn jurisdiction_applicable_domains_unknown_jurisdiction() {
        let domains = crate::compliance::jurisdiction_applicable_domains("zz-nonexistent");
        assert_eq!(domains.len(), 20, "unknown jurisdiction should have all 20 domains");
    }

    #[test]
    fn jurisdiction_applicable_domains_uae_includes_digital_assets() {
        let domains = crate::compliance::jurisdiction_applicable_domains("ae");
        // UAE: financial (8) + sanctions + digital_assets + custody + data_privacy = 12
        assert!(domains.contains(&"digital_assets".to_string()));
        assert!(domains.contains(&"custody".to_string()));
        assert!(domains.contains(&"data_privacy".to_string()));
        assert!(domains.contains(&"aml".to_string()));
        assert_eq!(domains.len(), 12, "ae should have 12 applicable domains, got {}", domains.len());
    }

    #[test]
    fn jurisdiction_applicable_domains_singapore() {
        let domains = crate::compliance::jurisdiction_applicable_domains("sg");
        // Singapore: financial (8) + sanctions + digital_assets + data_privacy = 11
        assert!(domains.contains(&"digital_assets".to_string()));
        assert!(domains.contains(&"data_privacy".to_string()));
        assert_eq!(domains.len(), 11, "sg should have 11 applicable domains, got {}", domains.len());
    }

    #[test]
    fn jurisdiction_applicable_domains_hong_kong() {
        let domains = crate::compliance::jurisdiction_applicable_domains("hk");
        // Hong Kong: financial (8) + sanctions + digital_assets = 10
        assert!(domains.contains(&"digital_assets".to_string()));
        assert_eq!(domains.len(), 10, "hk should have 10 applicable domains, got {}", domains.len());
    }

    #[test]
    fn jurisdiction_applicable_domains_cayman() {
        let domains = crate::compliance::jurisdiction_applicable_domains("ky");
        // Cayman: financial (8) + sanctions + digital_assets + custody = 11
        assert!(domains.contains(&"digital_assets".to_string()));
        assert!(domains.contains(&"custody".to_string()));
        assert_eq!(domains.len(), 11, "ky should have 11 applicable domains, got {}", domains.len());
    }

    #[tokio::test]
    async fn query_jurisdiction_uae_shows_expanded_domains() {
        let app = test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/v1/compliance/ae")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let result: JurisdictionComplianceResponse = body_json(resp).await;
        assert_eq!(result.jurisdiction_id, "ae");
        assert_eq!(result.total_domains, 20);
        // UAE: 12 applicable (blocking/pending) + 8 not-applicable (passing)
        assert_eq!(result.passing_count, 8, "8 non-applicable domains should pass");
        assert_eq!(result.blocking_count, 12, "12 applicable domains should be blocking");
    }

    #[tokio::test]
    async fn query_corridor_compliance() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Create a corridor first via direct state manipulation.
        let corridor_app =
            crate::routes::corridors::router().with_state(state.clone());
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/corridors")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"jurisdiction_a":"pk","jurisdiction_b":"ae"}"#,
            ))
            .unwrap();
        let create_resp = corridor_app.oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);
        let record: crate::state::CorridorRecord = body_json(create_resp).await;

        // Query corridor compliance.
        let req = Request::builder()
            .method("GET")
            .uri(format!("/v1/compliance/corridor/{}", record.id))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let result: CorridorComplianceResponse = body_json(resp).await;
        assert_eq!(result.corridor_id, record.id);
        assert_eq!(result.jurisdiction_a.jurisdiction_id, "pk");
        assert_eq!(result.jurisdiction_b.jurisdiction_id, "ae");
        assert!(!result.corridor_compliant, "new corridor should not be compliant (applicable domains pending)");
        // Both pk and ae have regpack content, so only applicable domains are blocking.
        // The cross_blocking_domains list contains domains that are blocking in EITHER jurisdiction.
        assert!(
            !result.cross_blocking_domains.is_empty(),
            "corridor should have blocking domains"
        );
    }

    #[tokio::test]
    async fn query_corridor_not_found_returns_404() {
        let app = test_app();
        let fake_id = Uuid::new_v4();
        let req = Request::builder()
            .method("GET")
            .uri(format!("/v1/compliance/corridor/{fake_id}"))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn format_state_all_variants() {
        assert_eq!(format_state(ComplianceState::Compliant), "compliant");
        assert_eq!(format_state(ComplianceState::NonCompliant), "non_compliant");
        assert_eq!(format_state(ComplianceState::Pending), "pending");
        assert_eq!(format_state(ComplianceState::Exempt), "exempt");
        assert_eq!(format_state(ComplianceState::NotApplicable), "not_applicable");
    }

    #[test]
    fn domain_descriptions_non_empty() {
        for domain in ComplianceDomain::all() {
            let desc = domain_description(domain);
            assert!(!desc.is_empty(), "domain {domain:?} should have a description");
        }
    }

    #[test]
    fn domain_categories_valid() {
        for domain in ComplianceDomain::all() {
            let cat = domain_category(domain);
            assert!(
                cat == "core" || cat == "extended",
                "domain {domain:?} has invalid category: {cat}"
            );
        }
    }

    #[tokio::test]
    async fn query_entity_compliance_with_attestations() {
        let state = AppState::new();

        // Create an entity attestation.
        let entity_id = Uuid::new_v4();
        let att = crate::state::AttestationRecord {
            id: Uuid::new_v4(),
            entity_id,
            attestation_type: "FORMATION_COMPLIANCE".to_string(),
            issuer: "did:mass:zone:pk-sifc".to_string(),
            status: crate::state::AttestationStatus::Active,
            jurisdiction_id: "pk".to_string(),
            issued_at: Utc::now(),
            expires_at: Some(Utc::now() + chrono::Duration::days(365)),
            details: serde_json::json!({"domains": ["corporate"]}),
        };
        state.attestations.insert(att.id, att);

        let app = router().with_state(state);
        let req = Request::builder()
            .method("GET")
            .uri(format!("/v1/compliance/entity/{entity_id}"))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let result: EntityComplianceResponse = body_json(resp).await;
        assert_eq!(result.entity_id, entity_id);
        assert_eq!(result.jurisdiction_id, "pk");
        assert_eq!(result.attestation_count, 1);
        assert_eq!(result.total_domains, 20);

        // The "corporate" domain should be compliant from the attestation.
        let corporate = result.domains.iter().find(|d| d.domain == "corporate").unwrap();
        assert_eq!(corporate.status, "compliant");
        assert!(corporate.passing);
        assert!(corporate.attestation_id.is_some());
        assert!(corporate.expires_at.is_some());
    }

    #[tokio::test]
    async fn query_entity_not_found_returns_404() {
        let app = test_app();
        let fake_id = Uuid::new_v4();
        let req = Request::builder()
            .method("GET")
            .uri(format!("/v1/compliance/entity/{fake_id}"))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn query_entity_expired_attestation_marks_non_compliant() {
        let state = AppState::new();

        let entity_id = Uuid::new_v4();
        let att = crate::state::AttestationRecord {
            id: Uuid::new_v4(),
            entity_id,
            attestation_type: "AML_COMPLIANCE".to_string(),
            issuer: "did:mass:zone:pk-sifc".to_string(),
            status: crate::state::AttestationStatus::Active,
            jurisdiction_id: "pk".to_string(),
            issued_at: Utc::now() - chrono::Duration::days(400),
            expires_at: Some(Utc::now() - chrono::Duration::days(30)),
            details: serde_json::json!({}),
        };
        state.attestations.insert(att.id, att);

        let app = router().with_state(state);
        let req = Request::builder()
            .method("GET")
            .uri(format!("/v1/compliance/entity/{entity_id}"))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let result: EntityComplianceResponse = body_json(resp).await;
        let aml = result.domains.iter().find(|d| d.domain == "aml").unwrap();
        assert_eq!(aml.status, "non_compliant", "expired attestation should be non_compliant");
        assert!(!aml.passing);
        assert_eq!(result.overall_status, "non_compliant");
    }

    #[test]
    fn attestation_type_mapping() {
        assert_eq!(attestation_type_to_domain("FORMATION_COMPLIANCE"), Some("corporate"));
        assert_eq!(attestation_type_to_domain("SANCTIONS_COMPLIANCE"), Some("sanctions"));
        assert_eq!(attestation_type_to_domain("identity_compliance"), Some("kyc"));
        assert_eq!(attestation_type_to_domain("unknown_type"), None);
    }

    #[test]
    fn router_builds_successfully() {
        let _router = router();
    }
}
