//! # Compliance Query API
//!
//! Programmatic compliance visibility endpoints. These are the regulator-facing
//! and integrator-facing APIs that answer: "What is the compliance state of
//! this jurisdiction / entity / corridor?"
//!
//! ## Endpoints
//!
//! - `GET /v1/compliance/:jurisdiction_id` — Tensor state across all 20 domains
//! - `GET /v1/compliance/corridor/:corridor_id` — Bilateral compliance for a corridor
//! - `GET /v1/compliance/domains` — List all compliance domains with descriptions
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

use mez_core::ComplianceDomain;
use mez_tensor::{ComplianceState, ComplianceTensor, DefaultJurisdiction};

use crate::compliance::build_tensor;
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

/// Build the compliance query router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/compliance/domains", get(list_domains))
        .route("/v1/compliance/:jurisdiction_id", get(query_jurisdiction))
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
    let tensor = build_tensor(&jurisdiction_id);
    let response = evaluate_tensor(&jurisdiction_id, &tensor, &state);
    Ok(Json(response))
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

    let tensor_a = build_tensor(&corridor.jurisdiction_a);
    let tensor_b = build_tensor(&corridor.jurisdiction_b);

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

/// Evaluate a tensor and produce the response structure.
fn evaluate_tensor(
    jurisdiction_id: &str,
    tensor: &ComplianceTensor<DefaultJurisdiction>,
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
        total_domains: 20,
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
        // New tensor starts with all domains Pending.
        assert_eq!(result.blocking_count, 20);
        assert_eq!(result.passing_count, 0);
        assert_eq!(result.overall_status, "pending");
        assert!(result.tensor_commitment.is_some());
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
        assert!(!result.corridor_compliant, "new corridor should not be compliant (all domains pending)");
        assert_eq!(result.cross_blocking_domains.len(), 20);
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

    #[test]
    fn router_builds_successfully() {
        let _router = router();
    }
}
