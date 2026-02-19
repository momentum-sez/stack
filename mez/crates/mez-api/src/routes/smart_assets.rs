//! # Smart Asset API
//!
//! Handles smart asset CRUD, compliance evaluation triggering,
//! and anchor verification.
//! Route structure based on apis/smart-assets.openapi.yaml.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::CallerIdentity;
use crate::compliance::{
    apply_attestations, build_evaluation_result, build_tensor, AttestationInput,
};
use crate::error::AppError;
use crate::extractors::{extract_validated_json, Validate};
use crate::state::{
    AppState, AssetComplianceStatus, AssetStatus, SmartAssetRecord, SmartAssetType,
};
use axum::extract::rejection::JsonRejection;

/// Request to create a smart asset genesis.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAssetRequest {
    pub asset_type: String,
    pub jurisdiction_id: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl Validate for CreateAssetRequest {
    fn validate(&self) -> Result<(), String> {
        if self.asset_type.trim().is_empty() {
            return Err("asset_type must not be empty".to_string());
        }
        if self.asset_type.len() > 255 {
            return Err("asset_type must not exceed 255 characters".to_string());
        }
        if self.jurisdiction_id.trim().is_empty() {
            return Err("jurisdiction_id must not be empty".to_string());
        }
        if self.jurisdiction_id.len() > 255 {
            return Err("jurisdiction_id must not exceed 255 characters".to_string());
        }
        Ok(())
    }
}

/// Compliance evaluation request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ComplianceEvalRequest {
    /// Domains to evaluate (currently ignored — all 20 domains are always evaluated).
    #[serde(default)]
    pub domains: Vec<String>,
    /// Additional evaluation context.
    #[serde(default)]
    pub context: serde_json::Value,
    /// Attestation evidence per compliance domain.
    #[serde(default)]
    pub attestations: std::collections::HashMap<String, AttestationInput>,
}

impl Validate for ComplianceEvalRequest {
    fn validate(&self) -> Result<(), String> {
        const MAX_ATTESTATIONS: usize = 100;
        if self.attestations.len() > MAX_ATTESTATIONS {
            return Err(format!(
                "attestations must not exceed {MAX_ATTESTATIONS} entries"
            ));
        }
        for key in self.attestations.keys() {
            if key.len() > 100 {
                return Err("attestation domain name must not exceed 100 characters".to_string());
            }
        }
        Ok(())
    }
}

/// Compliance evaluation response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ComplianceEvalResponse {
    pub asset_id: Uuid,
    pub overall_status: String,
    pub domain_results: serde_json::Value,
    pub domain_count: usize,
    pub passing_domains: Vec<String>,
    pub blocking_domains: Vec<String>,
    pub evaluated_at: chrono::DateTime<chrono::Utc>,
}

/// Anchor verification request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AnchorVerifyRequest {
    pub anchor_digest: String,
    pub chain: String,
}

impl Validate for AnchorVerifyRequest {
    fn validate(&self) -> Result<(), String> {
        if self.anchor_digest.trim().is_empty() {
            return Err("anchor_digest must not be empty".to_string());
        }
        Ok(())
    }
}

/// Build the smart assets router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/assets/genesis", post(create_asset))
        .route("/v1/assets/registry", post(submit_registry))
        .route("/v1/assets/:id", get(get_asset))
        .route(
            "/v1/assets/:id/compliance/evaluate",
            post(evaluate_compliance),
        )
        .route(
            "/v1/assets/:id/anchors/corridor/verify",
            post(verify_anchor),
        )
}

/// POST /v1/assets/genesis — Create smart asset genesis.
#[utoipa::path(
    post,
    path = "/v1/assets/genesis",
    request_body = CreateAssetRequest,
    responses(
        (status = 201, description = "Asset created", body = SmartAssetRecord),
    ),
    tag = "smart_assets"
)]
async fn create_asset(
    State(state): State<AppState>,
    caller: CallerIdentity,
    body: Result<Json<CreateAssetRequest>, JsonRejection>,
) -> Result<(axum::http::StatusCode, Json<SmartAssetRecord>), AppError> {
    let req = extract_validated_json(body)?;
    let now = Utc::now();
    let id = Uuid::new_v4();

    let asset_type = SmartAssetType::new(req.asset_type).map_err(AppError::Validation)?;
    let record = SmartAssetRecord {
        id,
        asset_type,
        jurisdiction_id: req.jurisdiction_id,
        status: AssetStatus::Genesis,
        genesis_digest: None,
        compliance_status: AssetComplianceStatus::Unevaluated,
        metadata: req.metadata,
        owner_entity_id: caller.entity_id,
        created_at: now,
        updated_at: now,
    };

    state.smart_assets.insert(id, record.clone());

    // Persist to database (write-through). Failure is surfaced to the client
    // because the in-memory record would be lost on restart, causing silent data loss.
    if let Some(pool) = &state.db_pool {
        if let Err(e) = crate::db::smart_assets::insert(pool, &record).await {
            tracing::error!(asset_id = %id, error = %e, "failed to persist smart asset to database");
            return Err(AppError::Internal(
                "smart asset recorded in-memory but database persist failed".to_string(),
            ));
        }
    }

    Ok((axum::http::StatusCode::CREATED, Json(record)))
}

/// POST /v1/assets/registry — Submit/update smart asset registry VC.
#[utoipa::path(
    post,
    path = "/v1/assets/registry",
    responses(
        (status = 200, description = "Registry submitted"),
    ),
    tag = "smart_assets"
)]
async fn submit_registry(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    Err(AppError::NotImplemented(
        "Registry VC submission is a Phase 2 feature".to_string(),
    ))
}

/// GET /v1/assets/:id — Get a smart asset.
#[utoipa::path(
    get,
    path = "/v1/assets/{id}",
    params(("id" = Uuid, Path, description = "Asset ID")),
    responses(
        (status = 200, description = "Asset found", body = SmartAssetRecord),
        (status = 404, description = "Not found", body = crate::error::ErrorBody),
    ),
    tag = "smart_assets"
)]
async fn get_asset(
    State(state): State<AppState>,
    caller: CallerIdentity,
    Path(id): Path<Uuid>,
) -> Result<Json<SmartAssetRecord>, AppError> {
    let asset = state
        .smart_assets
        .get(&id)
        .ok_or_else(|| AppError::NotFound(format!("asset {id} not found")))?;

    if !caller.can_access_asset(&asset) {
        // Return 404 instead of 403 to prevent UUID enumeration.
        return Err(AppError::NotFound(format!("asset {id} not found")));
    }

    Ok(Json(asset))
}

/// POST /v1/assets/:id/compliance/evaluate — Evaluate compliance.
#[utoipa::path(
    post,
    path = "/v1/assets/{id}/compliance/evaluate",
    params(("id" = Uuid, Path, description = "Asset ID")),
    request_body = ComplianceEvalRequest,
    responses(
        (status = 200, description = "Compliance evaluated", body = ComplianceEvalResponse),
        (status = 404, description = "Not found", body = crate::error::ErrorBody),
    ),
    tag = "smart_assets"
)]
async fn evaluate_compliance(
    State(state): State<AppState>,
    caller: CallerIdentity,
    Path(id): Path<Uuid>,
    body: Result<Json<ComplianceEvalRequest>, JsonRejection>,
) -> Result<Json<ComplianceEvalResponse>, AppError> {
    let req = extract_validated_json(body)?;

    let asset = state
        .smart_assets
        .get(&id)
        .ok_or_else(|| AppError::NotFound(format!("asset {id} not found")))?;

    if !caller.can_access_asset(&asset) {
        return Err(AppError::NotFound(format!("asset {id} not found")));
    }

    // Build and evaluate the compliance tensor using the shared logic.
    let mut tensor = build_tensor(&asset.jurisdiction_id);
    apply_attestations(&mut tensor, &req.attestations);
    let eval = build_evaluation_result(&tensor, &asset, id);

    Ok(Json(ComplianceEvalResponse {
        asset_id: id,
        overall_status: eval.overall_status,
        domain_results: serde_json::to_value(&eval.domain_results)
            .unwrap_or_else(|_| serde_json::json!({})),
        domain_count: eval.domain_count,
        passing_domains: eval.passing_domains,
        blocking_domains: eval.blocking_domains,
        evaluated_at: eval.evaluated_at,
    }))
}

/// POST /v1/assets/:id/anchors/corridor/verify — Verify anchor.
#[utoipa::path(
    post,
    path = "/v1/assets/{id}/anchors/corridor/verify",
    params(("id" = Uuid, Path, description = "Asset ID")),
    request_body = AnchorVerifyRequest,
    responses(
        (status = 200, description = "Anchor verified"),
    ),
    tag = "smart_assets"
)]
async fn verify_anchor(
    State(state): State<AppState>,
    caller: CallerIdentity,
    Path(id): Path<Uuid>,
    body: Result<Json<AnchorVerifyRequest>, JsonRejection>,
) -> Result<Json<serde_json::Value>, AppError> {
    let req = extract_validated_json(body)?;

    let asset = state
        .smart_assets
        .get(&id)
        .ok_or_else(|| AppError::NotFound(format!("asset {id} not found")))?;

    if !caller.can_access_asset(&asset) {
        return Err(AppError::NotFound(format!("asset {id} not found")));
    }

    // Phase 2: anchor verification will cross-reference the on-chain
    // commitment with the corridor receipt chain. Until then, return 501.
    let _ = (id, req);
    Err(AppError::NotImplemented(
        "Anchor verification is a Phase 2 feature".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{CallerIdentity, Role};
    use crate::extractors::Validate;

    /// A zone admin identity for tests that need full access.
    fn zone_admin() -> CallerIdentity {
        CallerIdentity {
            role: Role::ZoneAdmin,
            entity_id: None,
            jurisdiction_id: None,
        }
    }

    // ── CreateAssetRequest validation ─────────────────────────────

    #[test]
    fn test_create_asset_request_valid() {
        let req = CreateAssetRequest {
            asset_type: "bond".to_string(),
            jurisdiction_id: "PK-PEZ".to_string(),
            metadata: serde_json::json!({}),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_asset_request_valid_with_metadata() {
        let req = CreateAssetRequest {
            asset_type: "equity".to_string(),
            jurisdiction_id: "AE-DIFC".to_string(),
            metadata: serde_json::json!({"issuer": "Acme Corp", "value": 1000}),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_asset_request_empty_asset_type() {
        let req = CreateAssetRequest {
            asset_type: "".to_string(),
            jurisdiction_id: "PK-PEZ".to_string(),
            metadata: serde_json::json!({}),
        };
        let err = req.validate().unwrap_err();
        assert!(
            err.contains("asset_type"),
            "error should mention asset_type: {err}"
        );
    }

    #[test]
    fn test_create_asset_request_whitespace_asset_type() {
        let req = CreateAssetRequest {
            asset_type: "   ".to_string(),
            jurisdiction_id: "PK-PEZ".to_string(),
            metadata: serde_json::json!({}),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_create_asset_request_empty_jurisdiction_id() {
        let req = CreateAssetRequest {
            asset_type: "bond".to_string(),
            jurisdiction_id: "".to_string(),
            metadata: serde_json::json!({}),
        };
        let err = req.validate().unwrap_err();
        assert!(
            err.contains("jurisdiction_id"),
            "error should mention jurisdiction_id: {err}"
        );
    }

    // ── ComplianceEvalRequest validation ──────────────────────────

    #[test]
    fn test_compliance_eval_request_valid() {
        let req = ComplianceEvalRequest {
            domains: vec!["aml".to_string(), "kyc".to_string()],
            context: serde_json::json!({"entity_id": "123"}),
            attestations: std::collections::HashMap::new(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_compliance_eval_request_empty_domains() {
        let req = ComplianceEvalRequest {
            domains: vec![],
            context: serde_json::json!({}),
            attestations: std::collections::HashMap::new(),
        };
        // The current validation always returns Ok.
        assert!(req.validate().is_ok());
    }

    // ── AnchorVerifyRequest validation ────────────────────────────

    #[test]
    fn test_anchor_verify_request_valid() {
        let req = AnchorVerifyRequest {
            anchor_digest: "sha256:abc123def456".to_string(),
            chain: "ethereum".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_anchor_verify_request_empty_digest() {
        let req = AnchorVerifyRequest {
            anchor_digest: "".to_string(),
            chain: "ethereum".to_string(),
        };
        let err = req.validate().unwrap_err();
        assert!(
            err.contains("anchor_digest"),
            "error should mention anchor_digest: {err}"
        );
    }

    #[test]
    fn test_anchor_verify_request_whitespace_digest() {
        let req = AnchorVerifyRequest {
            anchor_digest: "   ".to_string(),
            chain: "ethereum".to_string(),
        };
        assert!(req.validate().is_err());
    }

    // ── Router construction ───────────────────────────────────────

    #[test]
    fn test_router_builds_successfully() {
        let _router = router();
    }

    // ── Handler integration tests ──────────────────────────────────

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    /// Helper: build the smart assets router with a fresh AppState and
    /// ZoneAdmin identity injected for full access.
    fn test_app() -> Router<()> {
        router()
            .layer(axum::Extension(zone_admin()))
            .with_state(AppState::new())
    }

    /// Helper: build the router with shared state and ZoneAdmin identity.
    fn test_app_with_state(state: AppState) -> Router<()> {
        router()
            .layer(axum::Extension(zone_admin()))
            .with_state(state)
    }

    /// Helper: read the response body as bytes and deserialize from JSON.
    async fn body_json<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn handler_create_asset_returns_201() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/assets/genesis")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"asset_type":"bond","jurisdiction_id":"PK-PEZ","metadata":{"issuer":"Acme Corp","maturity":"2030-01-01"}}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let record: SmartAssetRecord = body_json(resp).await;
        assert_eq!(record.asset_type, "bond");
        assert_eq!(record.jurisdiction_id, "PK-PEZ");
        assert_eq!(record.status, AssetStatus::Genesis);
        assert!(record.genesis_digest.is_none());
        assert_eq!(record.compliance_status, AssetComplianceStatus::Unevaluated);
    }

    #[tokio::test]
    async fn handler_create_asset_empty_asset_type_returns_422() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/assets/genesis")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"asset_type":"","jurisdiction_id":"PK-PEZ"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_create_asset_empty_jurisdiction_returns_422() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/assets/genesis")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"asset_type":"equity","jurisdiction_id":""}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_create_asset_bad_json_returns_422() {
        // BUG-038: JSON parse errors now return 422 (Unprocessable Entity).
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/assets/genesis")
            .header("content-type", "application/json")
            .body(Body::from(r#"not valid json"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_get_asset_not_found_returns_404() {
        let app = test_app();
        let id = Uuid::new_v4();
        let req = Request::builder()
            .method("GET")
            .uri(format!("/v1/assets/{id}"))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn handler_create_then_get_asset_returns_200() {
        let state = AppState::new();
        let app = test_app_with_state(state.clone());

        // Create an asset.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/assets/genesis")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"asset_type":"equity","jurisdiction_id":"AE-DIFC","metadata":{}}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);

        let created: SmartAssetRecord = body_json(create_resp).await;

        // Get the asset.
        let get_req = Request::builder()
            .method("GET")
            .uri(format!("/v1/assets/{}", created.id))
            .body(Body::empty())
            .unwrap();
        let get_resp = app.oneshot(get_req).await.unwrap();
        assert_eq!(get_resp.status(), StatusCode::OK);

        let fetched: SmartAssetRecord = body_json(get_resp).await;
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.asset_type, "equity");
    }

    // ── Additional handler coverage ───────────────────────────────

    #[tokio::test]
    async fn handler_submit_registry_returns_501() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/assets/registry")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn handler_evaluate_compliance_returns_200() {
        let state = AppState::new();
        let app = test_app_with_state(state.clone());

        // Create an asset first.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/assets/genesis")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"asset_type":"bond","jurisdiction_id":"PK-PEZ","metadata":{}}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        let created: SmartAssetRecord = body_json(create_resp).await;

        // Evaluate compliance.
        let eval_req = Request::builder()
            .method("POST")
            .uri(format!("/v1/assets/{}/compliance/evaluate", created.id))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"domains":["aml","kyc","sanctions"],"context":{"entity_id":"12345"}}"#,
            ))
            .unwrap();
        let eval_resp = app.oneshot(eval_req).await.unwrap();
        assert_eq!(eval_resp.status(), StatusCode::OK);

        let result: ComplianceEvalResponse = body_json(eval_resp).await;
        assert_eq!(result.asset_id, created.id);
        // Without attestations, all 20 domains are Pending → overall is "pending".
        assert_eq!(result.overall_status, "pending");
        assert_eq!(result.domain_count, 20);
        assert_eq!(result.blocking_domains.len(), 20);
    }

    #[tokio::test]
    async fn handler_evaluate_compliance_asset_not_found_returns_404() {
        let app = test_app();
        let id = Uuid::new_v4();
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/assets/{id}/compliance/evaluate"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"domains":["aml"],"context":{}}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn handler_evaluate_compliance_bad_json_returns_422() {
        // BUG-038: JSON parse errors now return 422 (Unprocessable Entity).
        let app = test_app();
        let id = Uuid::new_v4();
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/assets/{id}/compliance/evaluate"))
            .header("content-type", "application/json")
            .body(Body::from(r#"not json"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_verify_anchor_returns_501() {
        let state = AppState::new();
        let app = test_app_with_state(state.clone());

        // Create an asset first.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/assets/genesis")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"asset_type":"bond","jurisdiction_id":"PK-PEZ"}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        let created: SmartAssetRecord = body_json(create_resp).await;

        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/assets/{}/anchors/corridor/verify", created.id))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"anchor_digest":"sha256:deadbeef","chain":"ethereum"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn handler_verify_anchor_empty_digest_returns_422() {
        let state = AppState::new();
        let app = test_app_with_state(state.clone());

        // Create an asset first.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/assets/genesis")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"asset_type":"bond","jurisdiction_id":"PK-PEZ"}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        let created: SmartAssetRecord = body_json(create_resp).await;

        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/assets/{}/anchors/corridor/verify", created.id))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"anchor_digest":"","chain":"ethereum"}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_verify_anchor_bad_json_returns_422() {
        // BUG-038: JSON parse errors now return 422 (Unprocessable Entity).
        let state = AppState::new();
        let app = test_app_with_state(state.clone());

        // Create an asset first.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/assets/genesis")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"asset_type":"bond","jurisdiction_id":"PK-PEZ"}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        let created: SmartAssetRecord = body_json(create_resp).await;

        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/assets/{}/anchors/corridor/verify", created.id))
            .header("content-type", "application/json")
            .body(Body::from(r#"broken"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_create_asset_default_metadata_returns_201() {
        let app = test_app();
        // Omit metadata field entirely; serde should use the default.
        let req = Request::builder()
            .method("POST")
            .uri("/v1/assets/genesis")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"asset_type":"commodity","jurisdiction_id":"PK-REZ"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let record: SmartAssetRecord = body_json(resp).await;
        assert_eq!(record.asset_type, "commodity");
        assert!(record.metadata.is_null());
    }
}
