//! # OWNERSHIP Primitive — Investment Info API
//!
//! Handles cap table management, ownership transfers, share class
//! definitions, and capital gains event tracking.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppError;
use crate::extractors::{extract_validated_json, Validate};
use crate::state::{AppState, CapTableRecord, OwnershipTransfer, ShareClass};
use axum::extract::rejection::JsonRejection;

/// Request to create/initialize a cap table.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCapTableRequest {
    pub entity_id: Uuid,
    #[serde(default)]
    pub share_classes: Vec<ShareClass>,
}

impl Validate for CreateCapTableRequest {
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

/// Request to record an ownership transfer.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RecordTransferRequest {
    pub from_holder: String,
    pub to_holder: String,
    pub share_class: String,
    pub quantity: u64,
    pub price_per_share: Option<String>,
}

impl Validate for RecordTransferRequest {
    fn validate(&self) -> Result<(), String> {
        if self.from_holder.trim().is_empty() || self.to_holder.trim().is_empty() {
            return Err("from_holder and to_holder must not be empty".to_string());
        }
        if self.quantity == 0 {
            return Err("quantity must be greater than 0".to_string());
        }
        Ok(())
    }
}

/// Build the ownership router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/ownership/cap-table", post(create_cap_table))
        .route("/v1/ownership/:entity_id/cap-table", get(get_cap_table))
        .route("/v1/ownership/:entity_id/transfers", post(record_transfer))
        .route(
            "/v1/ownership/:entity_id/share-classes",
            get(get_share_classes),
        )
}

/// POST /v1/ownership/cap-table — Initialize a cap table.
#[utoipa::path(
    post,
    path = "/v1/ownership/cap-table",
    request_body = CreateCapTableRequest,
    responses(
        (status = 201, description = "Cap table created", body = CapTableRecord),
    ),
    tag = "ownership"
)]
async fn create_cap_table(
    State(state): State<AppState>,
    body: Result<Json<CreateCapTableRequest>, JsonRejection>,
) -> Result<(axum::http::StatusCode, Json<CapTableRecord>), AppError> {
    let req = extract_validated_json(body)?;
    let now = Utc::now();
    let id = Uuid::new_v4();

    let record = CapTableRecord {
        id,
        entity_id: req.entity_id,
        share_classes: req.share_classes,
        transfers: Vec::new(),
        created_at: now,
        updated_at: now,
    };

    state.cap_tables.insert(id, record.clone());
    Ok((axum::http::StatusCode::CREATED, Json(record)))
}

/// GET /v1/ownership/:entity_id/cap-table — Get cap table for an entity.
#[utoipa::path(
    get,
    path = "/v1/ownership/{entity_id}/cap-table",
    params(("entity_id" = Uuid, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "Cap table found", body = CapTableRecord),
        (status = 404, description = "Not found", body = crate::error::ErrorBody),
    ),
    tag = "ownership"
)]
async fn get_cap_table(
    State(state): State<AppState>,
    Path(entity_id): Path<Uuid>,
) -> Result<Json<CapTableRecord>, AppError> {
    state
        .cap_tables
        .list()
        .into_iter()
        .find(|ct| ct.entity_id == entity_id)
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("cap table for entity {entity_id} not found")))
}

/// POST /v1/ownership/:entity_id/transfers — Record an ownership transfer.
#[utoipa::path(
    post,
    path = "/v1/ownership/{entity_id}/transfers",
    params(("entity_id" = Uuid, Path, description = "Entity ID")),
    request_body = RecordTransferRequest,
    responses(
        (status = 200, description = "Transfer recorded", body = CapTableRecord),
        (status = 404, description = "Not found", body = crate::error::ErrorBody),
    ),
    tag = "ownership"
)]
async fn record_transfer(
    State(state): State<AppState>,
    Path(entity_id): Path<Uuid>,
    body: Result<Json<RecordTransferRequest>, JsonRejection>,
) -> Result<Json<CapTableRecord>, AppError> {
    let req = extract_validated_json(body)?;
    let cap_table = state
        .cap_tables
        .list()
        .into_iter()
        .find(|ct| ct.entity_id == entity_id)
        .ok_or_else(|| AppError::NotFound(format!("cap table for entity {entity_id} not found")))?;

    let transfer = OwnershipTransfer {
        id: Uuid::new_v4(),
        from_holder: req.from_holder,
        to_holder: req.to_holder,
        share_class: req.share_class,
        quantity: req.quantity,
        price_per_share: req.price_per_share,
        transferred_at: Utc::now(),
    };

    state
        .cap_tables
        .update(&cap_table.id, |ct| {
            ct.transfers.push(transfer);
            ct.updated_at = Utc::now();
        })
        .map(Json)
        .ok_or_else(|| AppError::Internal("failed to update cap table".to_string()))
}

/// GET /v1/ownership/:entity_id/share-classes — Get share classes.
#[utoipa::path(
    get,
    path = "/v1/ownership/{entity_id}/share-classes",
    params(("entity_id" = Uuid, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "Share classes", body = Vec<ShareClass>),
        (status = 404, description = "Not found", body = crate::error::ErrorBody),
    ),
    tag = "ownership"
)]
async fn get_share_classes(
    State(state): State<AppState>,
    Path(entity_id): Path<Uuid>,
) -> Result<Json<Vec<ShareClass>>, AppError> {
    state
        .cap_tables
        .list()
        .into_iter()
        .find(|ct| ct.entity_id == entity_id)
        .map(|ct| Json(ct.share_classes))
        .ok_or_else(|| AppError::NotFound(format!("cap table for entity {entity_id} not found")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractors::Validate;

    // ── CreateCapTableRequest validation ──────────────────────────

    #[test]
    fn test_create_cap_table_request_valid() {
        let req = CreateCapTableRequest {
            entity_id: Uuid::new_v4(),
            share_classes: vec![],
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_cap_table_request_valid_with_classes() {
        let req = CreateCapTableRequest {
            entity_id: Uuid::new_v4(),
            share_classes: vec![ShareClass {
                name: "Class A".to_string(),
                authorized_shares: 1_000_000,
                issued_shares: 500_000,
                par_value: Some("1.00".to_string()),
                voting_rights: true,
            }],
        };
        assert!(req.validate().is_ok());
    }

    // ── RecordTransferRequest validation ──────────────────────────

    #[test]
    fn test_record_transfer_request_valid() {
        let req = RecordTransferRequest {
            from_holder: "Alice".to_string(),
            to_holder: "Bob".to_string(),
            share_class: "Class A".to_string(),
            quantity: 100,
            price_per_share: Some("10.00".to_string()),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_record_transfer_request_empty_from_holder() {
        let req = RecordTransferRequest {
            from_holder: "".to_string(),
            to_holder: "Bob".to_string(),
            share_class: "Class A".to_string(),
            quantity: 100,
            price_per_share: None,
        };
        let err = req.validate().unwrap_err();
        assert!(
            err.contains("from_holder") || err.contains("to_holder"),
            "error should mention holder fields: {err}"
        );
    }

    #[test]
    fn test_record_transfer_request_empty_to_holder() {
        let req = RecordTransferRequest {
            from_holder: "Alice".to_string(),
            to_holder: "  ".to_string(),
            share_class: "Class A".to_string(),
            quantity: 100,
            price_per_share: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_record_transfer_request_zero_quantity() {
        let req = RecordTransferRequest {
            from_holder: "Alice".to_string(),
            to_holder: "Bob".to_string(),
            share_class: "Class A".to_string(),
            quantity: 0,
            price_per_share: None,
        };
        let err = req.validate().unwrap_err();
        assert!(
            err.contains("quantity"),
            "error should mention quantity: {err}"
        );
    }

    #[test]
    fn test_record_transfer_request_both_holders_empty() {
        let req = RecordTransferRequest {
            from_holder: "".to_string(),
            to_holder: "".to_string(),
            share_class: "Class A".to_string(),
            quantity: 100,
            price_per_share: None,
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

    /// Helper: build the ownership router with a fresh AppState.
    fn test_app() -> Router<()> {
        router().with_state(AppState::new())
    }

    /// Helper: read the response body as bytes and deserialize from JSON.
    async fn body_json<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn handler_create_cap_table_returns_201() {
        let app = test_app();
        let entity_id = Uuid::new_v4();
        let body_str = format!(
            r#"{{"entity_id":"{}","share_classes":[{{"name":"Class A","authorized_shares":1000000,"issued_shares":500000,"par_value":"1.00","voting_rights":true}}]}}"#,
            entity_id
        );
        let req = Request::builder()
            .method("POST")
            .uri("/v1/ownership/cap-table")
            .header("content-type", "application/json")
            .body(Body::from(body_str))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let record: CapTableRecord = body_json(resp).await;
        assert_eq!(record.entity_id, entity_id);
        assert_eq!(record.share_classes.len(), 1);
        assert_eq!(record.share_classes[0].name, "Class A");
        assert!(record.transfers.is_empty());
    }

    #[tokio::test]
    async fn handler_create_cap_table_empty_classes_returns_201() {
        let app = test_app();
        let entity_id = Uuid::new_v4();
        let body_str = format!(r#"{{"entity_id":"{}"}}"#, entity_id);
        let req = Request::builder()
            .method("POST")
            .uri("/v1/ownership/cap-table")
            .header("content-type", "application/json")
            .body(Body::from(body_str))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let record: CapTableRecord = body_json(resp).await;
        assert!(record.share_classes.is_empty());
    }

    #[tokio::test]
    async fn handler_record_transfer_returns_200() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        let entity_id = Uuid::new_v4();

        // Create a cap table first.
        let create_body = format!(
            r#"{{"entity_id":"{}","share_classes":[{{"name":"Common","authorized_shares":10000,"issued_shares":5000,"voting_rights":true}}]}}"#,
            entity_id
        );
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/ownership/cap-table")
            .header("content-type", "application/json")
            .body(Body::from(create_body))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);

        // Record a transfer.
        let transfer_body = r#"{"from_holder":"Alice","to_holder":"Bob","share_class":"Common","quantity":100,"price_per_share":"10.00"}"#;
        let transfer_req = Request::builder()
            .method("POST")
            .uri(format!("/v1/ownership/{entity_id}/transfers"))
            .header("content-type", "application/json")
            .body(Body::from(transfer_body))
            .unwrap();
        let transfer_resp = app.oneshot(transfer_req).await.unwrap();
        assert_eq!(transfer_resp.status(), StatusCode::OK);

        let updated: CapTableRecord = body_json(transfer_resp).await;
        assert_eq!(updated.transfers.len(), 1);
        assert_eq!(updated.transfers[0].from_holder, "Alice");
        assert_eq!(updated.transfers[0].to_holder, "Bob");
        assert_eq!(updated.transfers[0].quantity, 100);
    }

    #[tokio::test]
    async fn handler_record_transfer_empty_holder_returns_422() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        let entity_id = Uuid::new_v4();

        // Create a cap table first.
        let create_body = format!(r#"{{"entity_id":"{}"}}"#, entity_id);
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/ownership/cap-table")
            .header("content-type", "application/json")
            .body(Body::from(create_body))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);

        // Record a transfer with empty from_holder.
        let transfer_body =
            r#"{"from_holder":"","to_holder":"Bob","share_class":"Common","quantity":100}"#;
        let transfer_req = Request::builder()
            .method("POST")
            .uri(format!("/v1/ownership/{entity_id}/transfers"))
            .header("content-type", "application/json")
            .body(Body::from(transfer_body))
            .unwrap();
        let transfer_resp = app.oneshot(transfer_req).await.unwrap();
        assert_eq!(transfer_resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_record_transfer_zero_quantity_returns_422() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        let entity_id = Uuid::new_v4();

        // Create a cap table first.
        let create_body = format!(r#"{{"entity_id":"{}"}}"#, entity_id);
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/ownership/cap-table")
            .header("content-type", "application/json")
            .body(Body::from(create_body))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);

        // Record a transfer with zero quantity.
        let transfer_body =
            r#"{"from_holder":"Alice","to_holder":"Bob","share_class":"Common","quantity":0}"#;
        let transfer_req = Request::builder()
            .method("POST")
            .uri(format!("/v1/ownership/{entity_id}/transfers"))
            .header("content-type", "application/json")
            .body(Body::from(transfer_body))
            .unwrap();
        let transfer_resp = app.oneshot(transfer_req).await.unwrap();
        assert_eq!(transfer_resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_record_transfer_no_cap_table_returns_404() {
        let app = test_app();
        let entity_id = Uuid::new_v4();

        let transfer_body =
            r#"{"from_holder":"Alice","to_holder":"Bob","share_class":"Common","quantity":100}"#;
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/ownership/{entity_id}/transfers"))
            .header("content-type", "application/json")
            .body(Body::from(transfer_body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // ── Additional handler coverage ───────────────────────────────

    #[tokio::test]
    async fn handler_get_cap_table_found_returns_200() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        let entity_id = Uuid::new_v4();

        // Create a cap table.
        let create_body = format!(
            r#"{{"entity_id":"{}","share_classes":[{{"name":"Class A","authorized_shares":1000000,"issued_shares":500000,"par_value":"1.00","voting_rights":true}},{{"name":"Class B","authorized_shares":500000,"issued_shares":100000,"voting_rights":false}}]}}"#,
            entity_id
        );
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/ownership/cap-table")
            .header("content-type", "application/json")
            .body(Body::from(create_body))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);

        // Get the cap table.
        let get_req = Request::builder()
            .method("GET")
            .uri(format!("/v1/ownership/{entity_id}/cap-table"))
            .body(Body::empty())
            .unwrap();
        let get_resp = app.oneshot(get_req).await.unwrap();
        assert_eq!(get_resp.status(), StatusCode::OK);

        let fetched: CapTableRecord = body_json(get_resp).await;
        assert_eq!(fetched.entity_id, entity_id);
        assert_eq!(fetched.share_classes.len(), 2);
        assert_eq!(fetched.share_classes[0].name, "Class A");
        assert_eq!(fetched.share_classes[1].name, "Class B");
    }

    #[tokio::test]
    async fn handler_get_cap_table_not_found_returns_404() {
        let app = test_app();
        let entity_id = Uuid::new_v4();
        let req = Request::builder()
            .method("GET")
            .uri(format!("/v1/ownership/{entity_id}/cap-table"))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn handler_get_share_classes_found_returns_200() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        let entity_id = Uuid::new_v4();

        // Create a cap table with share classes.
        let create_body = format!(
            r#"{{"entity_id":"{}","share_classes":[{{"name":"Preferred","authorized_shares":100000,"issued_shares":50000,"par_value":"5.00","voting_rights":true}}]}}"#,
            entity_id
        );
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/ownership/cap-table")
            .header("content-type", "application/json")
            .body(Body::from(create_body))
            .unwrap();
        app.clone().oneshot(create_req).await.unwrap();

        // Get share classes.
        let get_req = Request::builder()
            .method("GET")
            .uri(format!("/v1/ownership/{entity_id}/share-classes"))
            .body(Body::empty())
            .unwrap();
        let get_resp = app.oneshot(get_req).await.unwrap();
        assert_eq!(get_resp.status(), StatusCode::OK);

        let classes: Vec<ShareClass> = body_json(get_resp).await;
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].name, "Preferred");
        assert_eq!(classes[0].authorized_shares, 100000);
        assert_eq!(classes[0].issued_shares, 50000);
        assert_eq!(classes[0].par_value.as_deref(), Some("5.00"));
        assert!(classes[0].voting_rights);
    }

    #[tokio::test]
    async fn handler_get_share_classes_not_found_returns_404() {
        let app = test_app();
        let entity_id = Uuid::new_v4();
        let req = Request::builder()
            .method("GET")
            .uri(format!("/v1/ownership/{entity_id}/share-classes"))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn handler_create_cap_table_bad_json_returns_400() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/ownership/cap-table")
            .header("content-type", "application/json")
            .body(Body::from(r#"not valid json"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn handler_record_transfer_bad_json_returns_400() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        let entity_id = Uuid::new_v4();

        // Create a cap table first.
        let create_body = format!(r#"{{"entity_id":"{}"}}"#, entity_id);
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/ownership/cap-table")
            .header("content-type", "application/json")
            .body(Body::from(create_body))
            .unwrap();
        app.clone().oneshot(create_req).await.unwrap();

        // Send bad JSON for transfer.
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/ownership/{entity_id}/transfers"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{{bad json"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn handler_record_transfer_with_price_per_share() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        let entity_id = Uuid::new_v4();

        // Create a cap table.
        let create_body = format!(
            r#"{{"entity_id":"{}","share_classes":[{{"name":"Common","authorized_shares":10000,"issued_shares":5000,"voting_rights":true}}]}}"#,
            entity_id
        );
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/ownership/cap-table")
            .header("content-type", "application/json")
            .body(Body::from(create_body))
            .unwrap();
        app.clone().oneshot(create_req).await.unwrap();

        // Record a transfer without price_per_share.
        let transfer_body =
            r#"{"from_holder":"Alice","to_holder":"Bob","share_class":"Common","quantity":50}"#;
        let transfer_req = Request::builder()
            .method("POST")
            .uri(format!("/v1/ownership/{entity_id}/transfers"))
            .header("content-type", "application/json")
            .body(Body::from(transfer_body))
            .unwrap();
        let transfer_resp = app.oneshot(transfer_req).await.unwrap();
        assert_eq!(transfer_resp.status(), StatusCode::OK);

        let updated: CapTableRecord = body_json(transfer_resp).await;
        assert_eq!(updated.transfers.len(), 1);
        assert!(updated.transfers[0].price_per_share.is_none());
        assert_eq!(updated.transfers[0].quantity, 50);
    }
}
