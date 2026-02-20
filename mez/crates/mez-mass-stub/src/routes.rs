// SPDX-License-Identifier: BUSL-1.1
//! Route definitions for the Mass API stub.
//!
//! Implements the endpoints that `mez-mass-client` actually calls, with
//! responses that deserialize cleanly into the client's types (camelCase
//! JSON, correct field shapes).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::store::AppState;

/// Build the complete router with all Mass API stub routes.
pub fn router(state: AppState) -> Router {
    Router::new()
        // Health
        .route("/health", get(health))
        // organization-info (ENTITIES)
        .route(
            "/organization-info/api/v1/organization/create",
            post(org_create),
        )
        .route(
            "/organization-info/api/v1/organization/search",
            post(org_search),
        )
        .route(
            "/organization-info/api/v1/organization/supported-jurisdictions",
            get(org_supported_jurisdictions),
        )
        .route(
            "/organization-info/api/v1/organization/:id",
            get(org_get).put(org_update).delete(org_delete),
        )
        .route(
            "/organization-info/api/v1/organization",
            get(org_list),
        )
        // treasury-info (FISCAL)
        .route(
            "/treasury-info/api/v1/treasury/create",
            post(treasury_create),
        )
        .route(
            "/treasury-info/api/v1/treasury/:id",
            get(treasury_get),
        )
        // consent-info (CONSENT)
        .route(
            "/consent-info/api/v1/consents",
            post(consent_create),
        )
        .route(
            "/consent-info/api/v1/consents/:id",
            get(consent_get),
        )
        // investment-info (OWNERSHIP)
        .route(
            "/investment-info/api/v1/investment",
            post(investment_create),
        )
        .route(
            "/investment-info/api/v1/investment/:id",
            get(investment_get),
        )
        // templating-engine
        .route(
            "/templating-engine/api/v1/templates",
            get(templates_list),
        )
        // Fallback: 501 Not Implemented
        .fallback(not_implemented)
        .with_state(state)
}

// ── Health ──────────────────────────────────────────────────────────

async fn health() -> StatusCode {
    StatusCode::OK
}

// ── Organization-info (ENTITIES) ────────────────────────────────────

async fn org_create(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Response {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("");

    let entity = json!({
        "id": id.to_string(),
        "name": name,
        "jurisdiction": body.get("jurisdiction"),
        "status": "ACTIVE",
        "tags": body.get("tags").cloned().unwrap_or_else(|| json!([])),
        "address": body.get("address"),
        "createdAt": now,
        "updatedAt": now
    });

    state.organizations().insert(id, entity.clone());
    (StatusCode::CREATED, Json(entity)).into_response()
}

async fn org_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.organizations().get(&id) {
        Some(entry) => Json(entry.value().clone()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn org_update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<Value>,
) -> Response {
    match state.organizations().get_mut(&id) {
        Some(mut entry) => {
            let val = entry.value_mut();
            // Merge fields from body into existing entity.
            if let (Some(existing), Some(updates)) = (val.as_object_mut(), body.as_object()) {
                for (k, v) in updates {
                    existing.insert(k.clone(), v.clone());
                }
                existing.insert(
                    "updatedAt".to_string(),
                    json!(Utc::now().to_rfc3339()),
                );
            }
            Json(val.clone()).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn org_delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.organizations().remove(&id) {
        Some(_) => StatusCode::OK.into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(Deserialize)]
struct OrgListQuery {
    ids: Option<String>,
}

async fn org_list(
    State(state): State<AppState>,
    Query(query): Query<OrgListQuery>,
) -> Json<Value> {
    let results: Vec<Value> = match query.ids {
        Some(ids_str) => {
            let ids: Vec<Uuid> = ids_str
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            ids.iter()
                .filter_map(|id| state.organizations().get(id).map(|e| e.value().clone()))
                .collect()
        }
        None => state
            .organizations()
            .iter()
            .map(|e| e.value().clone())
            .collect(),
    };
    Json(json!(results))
}

async fn org_search(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let query = body
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let page = body
        .get("page")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let size = body
        .get("size")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;

    let all: Vec<Value> = state
        .organizations()
        .iter()
        .map(|e| e.value().clone())
        .filter(|v| {
            if query.is_empty() {
                return true;
            }
            v.get("name")
                .and_then(|n| n.as_str())
                .map(|n| n.to_lowercase().contains(&query))
                .unwrap_or(false)
        })
        .collect();

    let total = all.len();
    let start = page * size;
    let content: Vec<Value> = all.into_iter().skip(start).take(size).collect();
    let total_pages = (total + size - 1) / size;

    Json(json!({
        "content": content,
        "totalElements": total,
        "totalPages": total_pages,
        "number": page,
        "size": size
    }))
}

async fn org_supported_jurisdictions() -> Json<Value> {
    Json(json!([
        {"code": "pk", "name": "Pakistan"},
        {"code": "pk-sifc", "name": "Pakistan SIFC"},
        {"code": "ae-difc", "name": "UAE DIFC"}
    ]))
}

// ── Treasury-info (FISCAL) ──────────────────────────────────────────

async fn treasury_create(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Response {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    let entity_id = body
        .get("entityId")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let treasury = json!({
        "id": id.to_string(),
        "referenceId": null,
        "entityId": entity_id,
        "name": body.get("entityName"),
        "status": "ACTIVE",
        "context": body.get("context").cloned().unwrap_or(json!("MASS")),
        "createdAt": now,
        "updatedAt": now
    });

    state.treasuries().insert(id, treasury.clone());
    (StatusCode::CREATED, Json(treasury)).into_response()
}

async fn treasury_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.treasuries().get(&id) {
        Some(entry) => Json(entry.value().clone()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// ── Consent-info (CONSENT) ──────────────────────────────────────────

async fn consent_create(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Response {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    let org_id = body
        .get("organizationId")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let consent = json!({
        "id": id.to_string(),
        "organizationId": org_id,
        "operationId": body.get("operationId"),
        "operationType": body.get("operationType"),
        "status": "PENDING",
        "votes": [],
        "numVotesRequired": body.get("numBoardMemberApprovalsRequired"),
        "approvalCount": 0,
        "rejectionCount": 0,
        "requestedBy": body.get("requestedBy"),
        "createdAt": now,
        "updatedAt": now
    });

    state.consents().insert(id, consent.clone());
    (StatusCode::CREATED, Json(consent)).into_response()
}

async fn consent_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.consents().get(&id) {
        Some(entry) => Json(entry.value().clone()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// ── Investment-info (OWNERSHIP) ─────────────────────────────────────

async fn investment_create(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Response {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();

    let mut investment = body.clone();
    if let Some(obj) = investment.as_object_mut() {
        obj.insert("id".to_string(), json!(id.to_string()));
        obj.insert("createdAt".to_string(), json!(now));
        obj.insert("updatedAt".to_string(), json!(now.clone()));
    }

    state.investments().insert(id, investment.clone());
    (StatusCode::CREATED, Json(investment)).into_response()
}

async fn investment_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.investments().get(&id) {
        Some(entry) => Json(entry.value().clone()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// ── Templating-engine ───────────────────────────────────────────────

async fn templates_list() -> Json<Value> {
    Json(json!([]))
}

// ── Fallback ────────────────────────────────────────────────────────

async fn not_implemented() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_app() -> Router {
        router(AppState::new())
    }

    async fn body_json(resp: Response) -> Value {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn health_returns_200() {
        let app = test_app();
        let req = axum::http::Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn org_crud_lifecycle() {
        let state = AppState::new();
        let app = router(state);

        // Create
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/organization-info/api/v1/organization/create")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "name": "Test Corp",
                    "jurisdiction": "pk-sifc",
                    "tags": ["ez"]
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let created = body_json(resp).await;
        let id = created["id"].as_str().unwrap();
        assert_eq!(created["name"], "Test Corp");
        assert_eq!(created["status"], "ACTIVE");

        // Get
        let get_uri = format!("/organization-info/api/v1/organization/{id}");
        let req = axum::http::Request::builder()
            .uri(&get_uri)
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let fetched = body_json(resp).await;
        assert_eq!(fetched["name"], "Test Corp");

        // Update
        let req = axum::http::Request::builder()
            .method("PUT")
            .uri(&get_uri)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({"name": "Updated Corp"})).unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let updated = body_json(resp).await;
        assert_eq!(updated["name"], "Updated Corp");

        // Delete
        let req = axum::http::Request::builder()
            .method("DELETE")
            .uri(&get_uri)
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Get after delete → 404
        let req = axum::http::Request::builder()
            .uri(&get_uri)
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn org_search_filters_by_name() {
        let state = AppState::new();
        let app = router(state.clone());

        // Create two orgs
        for name in &["Alpha Corp", "Beta Inc"] {
            let req = axum::http::Request::builder()
                .method("POST")
                .uri("/organization-info/api/v1/organization/create")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&json!({"name": name, "tags": []})).unwrap(),
                ))
                .unwrap();
            app.clone().oneshot(req).await.unwrap();
        }

        // Search for "alpha"
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/organization-info/api/v1/organization/search")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({"query": "alpha", "page": 0, "size": 10})).unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let result = body_json(resp).await;
        assert_eq!(result["totalElements"], 1);
        assert_eq!(result["content"][0]["name"], "Alpha Corp");
    }

    #[tokio::test]
    async fn treasury_create_and_get() {
        let state = AppState::new();
        let app = router(state);

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/treasury-info/api/v1/treasury/create")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "entityId": "some-entity",
                    "entityName": "Test Treasury"
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let created = body_json(resp).await;
        let id = created["id"].as_str().unwrap();

        let req = axum::http::Request::builder()
            .uri(format!("/treasury-info/api/v1/treasury/{id}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn consent_create_and_get() {
        let state = AppState::new();
        let app = router(state);

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/consent-info/api/v1/consents")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "organizationId": "org-1",
                    "operationType": "EQUITY_OFFER"
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let created = body_json(resp).await;
        let id = created["id"].as_str().unwrap();

        let req = axum::http::Request::builder()
            .uri(format!("/consent-info/api/v1/consents/{id}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn investment_create_and_get() {
        let state = AppState::new();
        let app = router(state);

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/investment-info/api/v1/investment")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({"amount": "10000", "currency": "PKR"})).unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let created = body_json(resp).await;
        let id = created["id"].as_str().unwrap();

        let req = axum::http::Request::builder()
            .uri(format!("/investment-info/api/v1/investment/{id}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn templates_returns_empty_list() {
        let app = test_app();
        let req = axum::http::Request::builder()
            .uri("/templating-engine/api/v1/templates")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body, json!([]));
    }

    #[tokio::test]
    async fn unknown_path_returns_501() {
        let app = test_app();
        let req = axum::http::Request::builder()
            .uri("/some/unknown/path")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn supported_jurisdictions_returns_list() {
        let app = test_app();
        let req = axum::http::Request::builder()
            .uri("/organization-info/api/v1/organization/supported-jurisdictions")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert!(body.as_array().unwrap().len() >= 2);
    }

    #[tokio::test]
    async fn org_list_returns_all_when_no_ids() {
        let state = AppState::new();
        let app = router(state.clone());

        // Create one org
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/organization-info/api/v1/organization/create")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({"name": "Listed Corp", "tags": []})).unwrap(),
            ))
            .unwrap();
        app.clone().oneshot(req).await.unwrap();

        // List all
        let req = axum::http::Request::builder()
            .uri("/organization-info/api/v1/organization")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let body = body_json(resp).await;
        assert_eq!(body.as_array().unwrap().len(), 1);
    }
}
