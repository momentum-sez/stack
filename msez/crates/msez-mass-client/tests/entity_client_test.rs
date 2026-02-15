//! Integration tests for the entity client using wiremock.
//!
//! These tests verify the typed client correctly communicates with a mock
//! Mass organization-info API, replacing the in-process CRUD tests that
//! previously lived in msez-api/src/routes/entities.rs.

use msez_mass_client::entities::{CreateEntityRequest, MassEntityType, MassEntityStatus};
use msez_mass_client::{MassApiConfig, MassClient};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper: build a MassClient pointing at a wiremock server.
async fn test_client(mock_server: &MockServer) -> MassClient {
    let config = MassApiConfig {
        organization_info_url: mock_server.uri().parse().unwrap(),
        investment_info_url: "http://127.0.0.1:19001".parse().unwrap(),
        treasury_info_url: "http://127.0.0.1:19002".parse().unwrap(),
        consent_info_url: "http://127.0.0.1:19003".parse().unwrap(),
        identity_info_url: None,
        templating_engine_url: "http://127.0.0.1:19004".parse().unwrap(),
        api_token: "test-token".into(),
        timeout_secs: 5,
    };
    MassClient::new(config).unwrap()
}

#[tokio::test]
async fn create_entity_returns_created_entity() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/organization-info/organizations"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "entity_type": "llc",
            "legal_name": "Test Corp",
            "jurisdiction_id": "pk-sez-01",
            "status": "active",
            "beneficial_owners": [],
            "created_at": "2026-01-15T12:00:00Z",
            "updated_at": "2026-01-15T12:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    let req = CreateEntityRequest {
        entity_type: MassEntityType::Llc,
        legal_name: "Test Corp".into(),
        jurisdiction_id: "pk-sez-01".into(),
        beneficial_owners: vec![],
    };

    let entity = client.entities().create(&req).await.unwrap();
    assert_eq!(entity.legal_name, "Test Corp");
    assert_eq!(entity.jurisdiction_id, "pk-sez-01");
    assert_eq!(entity.entity_type, MassEntityType::Llc);
    assert_eq!(entity.status, MassEntityStatus::Active);
}

#[tokio::test]
async fn get_entity_returns_entity_when_found() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440000";

    Mock::given(method("GET"))
        .and(path(format!(
            "/organization-info/organizations/{id}"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": id,
            "entity_type": "company",
            "legal_name": "Fetched Corp",
            "jurisdiction_id": "ae-difc",
            "status": "active",
            "beneficial_owners": [],
            "created_at": "2026-01-15T12:00:00Z",
            "updated_at": "2026-01-15T12:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let entity = client
        .entities()
        .get("550e8400-e29b-41d4-a716-446655440000".parse().unwrap())
        .await
        .unwrap();

    assert!(entity.is_some());
    let entity = entity.unwrap();
    assert_eq!(entity.legal_name, "Fetched Corp");
}

#[tokio::test]
async fn get_entity_returns_none_when_not_found() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440001";

    Mock::given(method("GET"))
        .and(path(format!(
            "/organization-info/organizations/{id}"
        )))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let entity = client
        .entities()
        .get(id.parse().unwrap())
        .await
        .unwrap();

    assert!(entity.is_none());
}

#[tokio::test]
async fn list_entities_returns_list() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organization-info/organizations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "entity_type": "llc",
                "legal_name": "Corp A",
                "jurisdiction_id": "pk-sez-01",
                "status": "active",
                "beneficial_owners": [],
                "created_at": "2026-01-15T12:00:00Z",
                "updated_at": "2026-01-15T12:00:00Z"
            },
            {
                "id": "550e8400-e29b-41d4-a716-446655440001",
                "entity_type": "company",
                "legal_name": "Corp B",
                "jurisdiction_id": "ae-difc",
                "status": "active",
                "beneficial_owners": [],
                "created_at": "2026-01-15T12:00:00Z",
                "updated_at": "2026-01-15T12:00:00Z"
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let entities = client.entities().list(None, None).await.unwrap();

    assert_eq!(entities.len(), 2);
    assert_eq!(entities[0].legal_name, "Corp A");
    assert_eq!(entities[1].legal_name, "Corp B");
}

#[tokio::test]
async fn create_entity_handles_api_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/organization-info/organizations"))
        .respond_with(ResponseTemplate::new(422).set_body_string("validation failed"))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    let req = CreateEntityRequest {
        entity_type: MassEntityType::Llc,
        legal_name: "".into(),
        jurisdiction_id: "".into(),
        beneficial_owners: vec![],
    };

    let result = client.entities().create(&req).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        msez_mass_client::MassApiError::ApiError {
            status, body, ..
        } => {
            assert_eq!(status, 422);
            assert!(body.contains("validation failed"));
        }
        other => panic!("expected ApiError, got: {other:?}"),
    }
}

#[tokio::test]
async fn create_entity_with_beneficial_owners() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/organization-info/organizations"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440099",
            "entity_type": "llc",
            "legal_name": "Owner Corp",
            "jurisdiction_id": "pk-sez-01",
            "status": "active",
            "beneficial_owners": [{
                "name": "Alice Khan",
                "ownership_percentage": "51.0",
                "cnic": "12345-1234567-1",
                "ntn": "1234567"
            }],
            "created_at": "2026-01-15T12:00:00Z",
            "updated_at": "2026-01-15T12:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    let req = CreateEntityRequest {
        entity_type: MassEntityType::Llc,
        legal_name: "Owner Corp".into(),
        jurisdiction_id: "pk-sez-01".into(),
        beneficial_owners: vec![msez_mass_client::entities::MassBeneficialOwner {
            name: "Alice Khan".into(),
            ownership_percentage: "51.0".into(),
            cnic: Some("12345-1234567-1".into()),
            ntn: Some("1234567".into()),
        }],
    };

    let entity = client.entities().create(&req).await.unwrap();
    assert_eq!(entity.beneficial_owners.len(), 1);
    assert_eq!(entity.beneficial_owners[0].name, "Alice Khan");
}
