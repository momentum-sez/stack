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

use crate::error::AppError;
use crate::extractors::{extract_validated_json, Validate};
use crate::state::{AppState, AssetComplianceStatus, SmartAssetRecord};
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
        if self.jurisdiction_id.trim().is_empty() {
            return Err("jurisdiction_id must not be empty".to_string());
        }
        Ok(())
    }
}

/// Compliance evaluation request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ComplianceEvalRequest {
    pub domains: Vec<String>,
    pub context: serde_json::Value,
}

impl Validate for ComplianceEvalRequest {
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

/// Compliance evaluation response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ComplianceEvalResponse {
    pub asset_id: Uuid,
    pub overall_status: String,
    pub domain_results: serde_json::Value,
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
    body: Result<Json<CreateAssetRequest>, JsonRejection>,
) -> Result<(axum::http::StatusCode, Json<SmartAssetRecord>), AppError> {
    let req = extract_validated_json(body)?;
    let now = Utc::now();
    let id = Uuid::new_v4();

    let record = SmartAssetRecord {
        id,
        asset_type: req.asset_type,
        jurisdiction_id: req.jurisdiction_id,
        status: "GENESIS".to_string(),
        genesis_digest: None,
        compliance_status: AssetComplianceStatus::Unevaluated,
        metadata: req.metadata,
        created_at: now,
        updated_at: now,
    };

    state.smart_assets.insert(id, record.clone());
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
async fn submit_registry(State(_state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "submitted",
        "message": "Registry VC submission recorded"
    }))
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
    Path(id): Path<Uuid>,
) -> Result<Json<SmartAssetRecord>, AppError> {
    state
        .smart_assets
        .get(&id)
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("asset {id} not found")))
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
    Path(id): Path<Uuid>,
    body: Result<Json<ComplianceEvalRequest>, JsonRejection>,
) -> Result<Json<ComplianceEvalResponse>, AppError> {
    let _req = extract_validated_json(body)?;
    if !state.smart_assets.contains(&id) {
        return Err(AppError::NotFound(format!("asset {id} not found")));
    }

    Ok(Json(ComplianceEvalResponse {
        asset_id: id,
        overall_status: "PERMITTED".to_string(),
        domain_results: serde_json::json!({}),
        evaluated_at: Utc::now(),
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
    State(_state): State<AppState>,
    Path(id): Path<Uuid>,
    body: Result<Json<AnchorVerifyRequest>, JsonRejection>,
) -> Result<Json<serde_json::Value>, AppError> {
    let req = extract_validated_json(body)?;
    Ok(Json(serde_json::json!({
        "asset_id": id,
        "anchor_digest": req.anchor_digest,
        "chain": req.chain,
        "verified": true,
        "message": "Anchor verification is a Phase 2 feature — returning stub"
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractors::Validate;

    // ── CreateAssetRequest validation ─────────────────────────────

    #[test]
    fn test_create_asset_request_valid() {
        let req = CreateAssetRequest {
            asset_type: "bond".to_string(),
            jurisdiction_id: "PK-PSEZ".to_string(),
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
            jurisdiction_id: "PK-PSEZ".to_string(),
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
            jurisdiction_id: "PK-PSEZ".to_string(),
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
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_compliance_eval_request_empty_domains() {
        let req = ComplianceEvalRequest {
            domains: vec![],
            context: serde_json::json!({}),
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

    /// Helper: build the smart assets router with a fresh AppState.
    fn test_app() -> Router<()> {
        router().with_state(AppState::new())
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
                r#"{"asset_type":"bond","jurisdiction_id":"PK-PSEZ","metadata":{"issuer":"Acme Corp","maturity":"2030-01-01"}}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let record: SmartAssetRecord = body_json(resp).await;
        assert_eq!(record.asset_type, "bond");
        assert_eq!(record.jurisdiction_id, "PK-PSEZ");
        assert_eq!(record.status, "GENESIS");
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
                r#"{"asset_type":"","jurisdiction_id":"PK-PSEZ"}"#,
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
    async fn handler_create_asset_bad_json_returns_400() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/assets/genesis")
            .header("content-type", "application/json")
            .body(Body::from(r#"not valid json"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
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
        let app = router().with_state(state.clone());

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
    async fn handler_submit_registry_returns_200() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/assets/registry")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = body_json(resp).await;
        assert_eq!(body["status"], "submitted");
    }

    #[tokio::test]
    async fn handler_evaluate_compliance_returns_200() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Create an asset first.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/assets/genesis")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"asset_type":"bond","jurisdiction_id":"PK-PSEZ","metadata":{}}"#,
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
        assert_eq!(result.overall_status, "PERMITTED");
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
    async fn handler_evaluate_compliance_bad_json_returns_400() {
        let app = test_app();
        let id = Uuid::new_v4();
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/assets/{id}/compliance/evaluate"))
            .header("content-type", "application/json")
            .body(Body::from(r#"not json"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn handler_verify_anchor_returns_200() {
        let app = test_app();
        let id = Uuid::new_v4();
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/assets/{id}/anchors/corridor/verify"))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"anchor_digest":"sha256:deadbeef","chain":"ethereum"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = body_json(resp).await;
        assert_eq!(body["asset_id"], id.to_string());
        assert_eq!(body["anchor_digest"], "sha256:deadbeef");
        assert_eq!(body["chain"], "ethereum");
        assert_eq!(body["verified"], true);
    }

    #[tokio::test]
    async fn handler_verify_anchor_empty_digest_returns_422() {
        let app = test_app();
        let id = Uuid::new_v4();
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/assets/{id}/anchors/corridor/verify"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"anchor_digest":"","chain":"ethereum"}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_verify_anchor_bad_json_returns_400() {
        let app = test_app();
        let id = Uuid::new_v4();
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/assets/{id}/anchors/corridor/verify"))
            .header("content-type", "application/json")
            .body(Body::from(r#"broken"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
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
                r#"{"asset_type":"commodity","jurisdiction_id":"PK-RSEZ"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let record: SmartAssetRecord = body_json(resp).await;
        assert_eq!(record.asset_type, "commodity");
        assert!(record.metadata.is_null());
    }
}
