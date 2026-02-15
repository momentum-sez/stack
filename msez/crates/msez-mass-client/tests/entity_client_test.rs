//! Contract tests for EntityClient against the Mass organization-info Swagger spec.
//!
//! These tests use wiremock to simulate the live Mass organization-info API at
//! `organization-info.api.mass.inc`. Every path, request shape, and response
//! shape is derived from the live `/v3/api-docs` spec (February 2026).
//!
//! ## Endpoints Tested
//!
//! | Method | Path (relative to context) | Test |
//! |--------|---------------------------|------|
//! | POST   | `/api/v1/organization/create` | `create_entity_*` |
//! | GET    | `/api/v1/organization/{id}` | `get_entity_*` |
//! | GET    | `/api/v1/organization` | `list_entities_*` |
//! | POST   | `/api/v1/organization/search` | `search_entities_*` |
//! | DELETE | `/api/v1/organization/{id}` | `delete_entity_*` |
//! | GET    | `/api/v1/organization/supported-jurisdictions` | `supported_jurisdictions_*` |

use msez_mass_client::entities::{
    CreateEntityRequest, MassEntityStatus, SearchOrganizationsRequest,
};
use msez_mass_client::{MassApiConfig, MassClient};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Build a MassClient with the organization-info URL pointed at a wiremock server.
async fn test_client(mock_server: &MockServer) -> MassClient {
    let config = MassApiConfig {
        organization_info_url: mock_server.uri().parse().unwrap(),
        investment_info_url: "http://127.0.0.1:19001".parse().unwrap(),
        treasury_info_url: "http://127.0.0.1:19002".parse().unwrap(),
        consent_info_url: "http://127.0.0.1:19003".parse().unwrap(),
        identity_info_url: None,
        templating_engine_url: "http://127.0.0.1:19004".parse().unwrap(),
        api_token: zeroize::Zeroizing::new("test-token".into()),
        timeout_secs: 5,
    };
    MassClient::new(config).unwrap()
}

// ── POST /api/v1/organization/create ─────────────────────────────────

#[tokio::test]
async fn create_entity_sends_correct_path_and_returns_entity() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/organization-info/api/v1/organization/create"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "name": "Test Corp",
            "jurisdiction": "pk-sez-01",
            "status": "ACTIVE",
            "tags": ["sez"],
            "createdAt": "2026-01-15T12:00:00Z",
            "updatedAt": "2026-01-15T12:00:00Z"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let req = CreateEntityRequest {
        name: "Test Corp".into(),
        jurisdiction: Some("pk-sez-01".into()),
        address: None,
        entity_type: Some("llc".into()),
        tags: vec!["sez".into()],
    };

    let entity = client.entities().create(&req).await.unwrap();
    assert_eq!(entity.name, "Test Corp");
    assert_eq!(entity.jurisdiction.as_deref(), Some("pk-sez-01"));
    assert_eq!(entity.status, Some(MassEntityStatus::Active));
    assert_eq!(entity.tags, vec!["sez"]);
}

#[tokio::test]
async fn create_entity_handles_422_validation_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/organization-info/api/v1/organization/create"))
        .respond_with(ResponseTemplate::new(422).set_body_string(r#"{"error":"name is required"}"#))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let req = CreateEntityRequest {
        name: "".into(),
        jurisdiction: None,
        address: None,
        entity_type: None,
        tags: vec![],
    };

    let result = client.entities().create(&req).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        msez_mass_client::MassApiError::ApiError { status, body, .. } => {
            assert_eq!(status, 422);
            assert!(body.contains("name is required"));
        }
        other => panic!("expected ApiError, got: {other:?}"),
    }
}

#[tokio::test]
async fn create_entity_with_address_and_tags() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/organization-info/api/v1/organization/create"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440099",
            "name": "Full Corp",
            "jurisdiction": "ae-difc",
            "status": "ACTIVE",
            "address": {"street": "123 Main St", "city": "Dubai"},
            "tags": ["sez", "tech"],
            "board": [{"name": "Alice"}],
            "members": [{"userId": "u1"}],
            "createdAt": "2026-01-15T12:00:00Z",
            "updatedAt": "2026-01-15T12:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let req = CreateEntityRequest {
        name: "Full Corp".into(),
        jurisdiction: Some("ae-difc".into()),
        address: Some(serde_json::json!({"street": "123 Main St", "city": "Dubai"})),
        entity_type: Some("company".into()),
        tags: vec!["sez".into(), "tech".into()],
    };

    let entity = client.entities().create(&req).await.unwrap();
    assert_eq!(entity.name, "Full Corp");
    assert!(entity.address.is_some());
    assert!(entity.board.is_some());
    assert!(entity.members.is_some());
    assert_eq!(entity.tags.len(), 2);
}

// ── GET /api/v1/organization/{id} ────────────────────────────────────

#[tokio::test]
async fn get_entity_returns_entity_when_found() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440000";

    Mock::given(method("GET"))
        .and(path(format!("/organization-info/api/v1/organization/{id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": id,
            "name": "Fetched Corp",
            "jurisdiction": "ae-difc",
            "status": "ACTIVE",
            "tags": [],
            "createdAt": "2026-01-15T12:00:00Z",
            "updatedAt": "2026-01-15T12:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let entity = client.entities().get(id.parse::<uuid::Uuid>().unwrap()).await.unwrap();

    assert!(entity.is_some());
    let entity = entity.unwrap();
    assert_eq!(entity.name, "Fetched Corp");
    assert_eq!(entity.jurisdiction.as_deref(), Some("ae-difc"));
}

#[tokio::test]
async fn get_entity_returns_none_when_not_found() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440001";

    Mock::given(method("GET"))
        .and(path(format!("/organization-info/api/v1/organization/{id}")))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let entity = client.entities().get(id.parse::<uuid::Uuid>().unwrap()).await.unwrap();
    assert!(entity.is_none());
}

#[tokio::test]
async fn get_entity_returns_error_on_500() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440001";

    Mock::given(method("GET"))
        .and(path(format!("/organization-info/api/v1/organization/{id}")))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.entities().get(id.parse::<uuid::Uuid>().unwrap()).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        msez_mass_client::MassApiError::ApiError { status, .. } => {
            assert_eq!(status, 500);
        }
        other => panic!("expected ApiError, got: {other:?}"),
    }
}

// ── GET /api/v1/organization ─────────────────────────────────────────

#[tokio::test]
async fn list_entities_returns_list() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organization-info/api/v1/organization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "name": "Corp A",
                "jurisdiction": "pk-sez-01",
                "status": "ACTIVE",
                "tags": [],
                "createdAt": "2026-01-15T12:00:00Z"
            },
            {
                "id": "550e8400-e29b-41d4-a716-446655440001",
                "name": "Corp B",
                "jurisdiction": "ae-difc",
                "status": "INACTIVE",
                "tags": ["tech"],
                "createdAt": "2026-01-15T12:00:00Z"
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let entities = client.entities().list(None).await.unwrap();
    assert_eq!(entities.len(), 2);
    assert_eq!(entities[0].name, "Corp A");
    assert_eq!(entities[1].name, "Corp B");
    assert_eq!(entities[1].status, Some(MassEntityStatus::Inactive));
}

#[tokio::test]
async fn list_entities_by_ids() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organization-info/api/v1/organization"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!([{
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "name": "Corp A",
                "tags": []
            }])),
        )
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let ids = vec!["550e8400-e29b-41d4-a716-446655440000".parse().unwrap()];
    let entities = client.entities().list(Some(&ids)).await.unwrap();
    assert_eq!(entities.len(), 1);
}

// ── POST /api/v1/organization/search ─────────────────────────────────

#[tokio::test]
async fn search_entities_returns_paginated_results() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/organization-info/api/v1/organization/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "content": [{
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "name": "Search Corp",
                "tags": [],
                "status": "ACTIVE"
            }],
            "totalElements": 1,
            "totalPages": 1,
            "number": 0,
            "size": 10
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let req = SearchOrganizationsRequest {
        query: Some("Search".into()),
        page: Some(0),
        size: Some(10),
    };
    let resp = client.entities().search(&req).await.unwrap();
    assert_eq!(resp.content.len(), 1);
    assert_eq!(resp.content[0].name, "Search Corp");
    assert_eq!(resp.total_elements, Some(1));
    assert_eq!(resp.total_pages, Some(1));
}

// ── DELETE /api/v1/organization/{id} ─────────────────────────────────

#[tokio::test]
async fn delete_entity_succeeds() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440000";

    Mock::given(method("DELETE"))
        .and(path(format!("/organization-info/api/v1/organization/{id}")))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.entities().delete(id.parse::<uuid::Uuid>().unwrap()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn delete_entity_returns_error_on_404() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440000";

    Mock::given(method("DELETE"))
        .and(path(format!("/organization-info/api/v1/organization/{id}")))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.entities().delete(id.parse::<uuid::Uuid>().unwrap()).await;
    assert!(result.is_err());
}

// ── GET /api/v1/organization/supported-jurisdictions ─────────────────

#[tokio::test]
async fn supported_jurisdictions_returns_list() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(
            "/organization-info/api/v1/organization/supported-jurisdictions",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"code": "pk-sez-01", "name": "Pakistan SEZ"},
            {"code": "ae-difc", "name": "DIFC"}
        ])))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let jurisdictions = client.entities().supported_jurisdictions().await.unwrap();
    assert_eq!(jurisdictions.len(), 2);
}

// ── Serde resilience (forward compatibility) ─────────────────────────

#[tokio::test]
async fn entity_deserializes_with_unknown_fields() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440000";

    Mock::given(method("GET"))
        .and(path(format!("/organization-info/api/v1/organization/{id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": id,
            "name": "Forward Corp",
            "tags": [],
            "futureField": "should be ignored",
            "status": "NEVER_SEEN_STATUS"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let entity = client.entities().get(id.parse::<uuid::Uuid>().unwrap()).await.unwrap();
    assert!(entity.is_some());
    let entity = entity.unwrap();
    assert_eq!(entity.name, "Forward Corp");
    // Unknown status should map to the catch-all Unknown variant.
    assert_eq!(entity.status, Some(MassEntityStatus::Unknown));
}

#[tokio::test]
async fn entity_deserializes_with_missing_optional_fields() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440000";

    Mock::given(method("GET"))
        .and(path(format!("/organization-info/api/v1/organization/{id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": id,
            "name": "Minimal Corp"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let entity = client
        .entities()
        .get(id.parse::<uuid::Uuid>().unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(entity.name, "Minimal Corp");
    assert!(entity.jurisdiction.is_none());
    assert!(entity.status.is_none());
    assert!(entity.address.is_none());
    assert!(entity.tags.is_empty());
    assert!(entity.created_at.is_none());
    assert!(entity.board.is_none());
    assert!(entity.members.is_none());
}

// ── PUT /api/v1/organization/{id} ────────────────────────────────────

#[tokio::test]
async fn update_entity_sends_correct_path_and_returns_entity() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440000";

    Mock::given(method("PUT"))
        .and(path(format!(
            "/organization-info/api/v1/organization/{id}"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": id,
            "name": "Updated Corp",
            "jurisdiction": "pk-sez-01",
            "status": "ACTIVE",
            "tags": ["updated"],
            "createdAt": "2026-01-15T12:00:00Z",
            "updatedAt": "2026-02-15T10:00:00Z"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let body = serde_json::json!({"name": "Updated Corp"});
    let entity = client
        .entities()
        .update(id.parse().unwrap(), &body)
        .await
        .unwrap();
    assert_eq!(entity.name, "Updated Corp");
    assert_eq!(entity.jurisdiction.as_deref(), Some("pk-sez-01"));
}

#[tokio::test]
async fn update_entity_returns_error_on_404() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440099";

    Mock::given(method("PUT"))
        .and(path(format!(
            "/organization-info/api/v1/organization/{id}"
        )))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let body = serde_json::json!({"name": "Ghost Corp"});
    let result = client.entities().update(id.parse().unwrap(), &body).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        msez_mass_client::MassApiError::ApiError { status, .. } => {
            assert_eq!(status, 404);
        }
        other => panic!("expected ApiError, got: {other:?}"),
    }
}

#[tokio::test]
async fn update_entity_returns_error_on_422() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440000";

    Mock::given(method("PUT"))
        .and(path(format!(
            "/organization-info/api/v1/organization/{id}"
        )))
        .respond_with(
            ResponseTemplate::new(422).set_body_string("Validation failed: name is required"),
        )
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let body = serde_json::json!({});
    let result = client.entities().update(id.parse().unwrap(), &body).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        msez_mass_client::MassApiError::ApiError { status, body, .. } => {
            assert_eq!(status, 422);
            assert!(body.contains("Validation failed"));
        }
        other => panic!("expected ApiError, got: {other:?}"),
    }
}
