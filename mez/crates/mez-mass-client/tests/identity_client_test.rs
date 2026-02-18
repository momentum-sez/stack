//! Contract tests for IdentityClient — the aggregation facade across
//! organization-info (members, board) and consent-info (shareholders).
//!
//! ## Endpoints Tested
//!
//! | Service | Method | Path | Test |
//! |---------|--------|------|------|
//! | org-info | GET | `/api/v1/membership/{orgId}/members` | `get_members_*` |
//! | org-info | GET | `/api/v1/board/{orgId}` | `get_board_*` |
//! | consent-info | GET | `/api/v1/shareholders/organization/{orgId}` | `get_shareholders_*` |
//! | (composite) | — | — | `get_composite_identity_*` |

use mez_mass_client::{MassApiConfig, MassClient};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Build a MassClient with org-info and consent-info pointed at separate
/// wiremock servers for isolation.
async fn test_client_dual(org_server: &MockServer, consent_server: &MockServer) -> MassClient {
    let config = MassApiConfig {
        organization_info_url: org_server.uri().parse().unwrap(),
        investment_info_url: "http://127.0.0.1:19001".parse().unwrap(),
        treasury_info_url: "http://127.0.0.1:19002".parse().unwrap(),
        consent_info_url: consent_server.uri().parse().unwrap(),
        identity_info_url: None,
        templating_engine_url: "http://127.0.0.1:19004".parse().unwrap(),
        api_token: zeroize::Zeroizing::new("test-token".into()),
        timeout_secs: 5,
    };
    MassClient::new(config).unwrap()
}

/// Build a MassClient with only org-info pointed at a mock server.
async fn test_client_org(mock_server: &MockServer) -> MassClient {
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

// ── GET /api/v1/membership/{orgId}/members ───────────────────────────

#[tokio::test]
async fn get_members_returns_member_list() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organization-info/api/v1/membership/org-001/members"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "userId": "user-1",
                "name": "Alice Khan",
                "email": "alice@example.com",
                "roles": ["ADMIN", "MEMBER"]
            },
            {
                "userId": "user-2",
                "name": "Bob Ahmad",
                "email": "bob@example.com",
                "roles": ["MEMBER"]
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = test_client_org(&mock_server).await;
    let members = client.identity().get_members("org-001").await.unwrap();
    assert_eq!(members.len(), 2);
    assert_eq!(members[0].name.as_deref(), Some("Alice Khan"));
    assert_eq!(members[0].roles, vec!["ADMIN", "MEMBER"]);
    assert_eq!(members[1].name.as_deref(), Some("Bob Ahmad"));
}

#[tokio::test]
async fn get_members_handles_api_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organization-info/api/v1/membership/bad-org/members"))
        .respond_with(ResponseTemplate::new(404).set_body_string("organization not found"))
        .mount(&mock_server)
        .await;

    let client = test_client_org(&mock_server).await;
    let result = client.identity().get_members("bad-org").await;
    assert!(result.is_err());
}

// ── GET /api/v1/board/{orgId} ────────────────────────────────────────

#[tokio::test]
async fn get_board_returns_directors() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organization-info/api/v1/board/org-001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "userId": "user-1",
                "name": "Alice Khan",
                "email": "alice@example.com",
                "roles": ["DIRECTOR", "CHAIRMAN"],
                "shares": 500000,
                "ownershipPercentage": "51.0",
                "active": true
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = test_client_org(&mock_server).await;
    let directors = client.identity().get_board("org-001").await.unwrap();
    assert_eq!(directors.len(), 1);
    assert_eq!(directors[0].name.as_deref(), Some("Alice Khan"));
    assert_eq!(directors[0].shares, Some(500000));
    assert_eq!(directors[0].ownership_percentage.as_deref(), Some("51.0"));
    assert_eq!(directors[0].active, Some(true));
}

// ── GET /api/v1/shareholders/organization/{orgId} ────────────────────

#[tokio::test]
async fn get_shareholders_returns_shareholder_list() {
    let consent_server = MockServer::start().await;
    let org_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(
            "/consent-info/api/v1/shareholders/organization/org-001",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "organizationId": "org-001",
                "email": "alice@example.com",
                "firstName": "Alice",
                "lastName": "Khan",
                "isEntity": false,
                "outstandingShares": 500000,
                "fullyDilutedShares": 510000,
                "status": "ACTIVE",
                "createdAt": "2026-01-15T12:00:00Z"
            }
        ])))
        .mount(&consent_server)
        .await;

    let client = test_client_dual(&org_server, &consent_server).await;
    let shareholders = client.identity().get_shareholders("org-001").await.unwrap();
    assert_eq!(shareholders.len(), 1);
    assert_eq!(shareholders[0].first_name.as_deref(), Some("Alice"));
    assert_eq!(shareholders[0].outstanding_shares, Some(500000));
    assert_eq!(shareholders[0].is_entity, Some(false));
}

// ── Composite identity (aggregation across both services) ────────────

#[tokio::test]
async fn get_composite_identity_aggregates_all_three_sources() {
    let org_server = MockServer::start().await;
    let consent_server = MockServer::start().await;

    // Members (organization-info)
    Mock::given(method("GET"))
        .and(path("/organization-info/api/v1/membership/org-001/members"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"userId": "user-1", "name": "Alice", "roles": ["ADMIN"]}
        ])))
        .mount(&org_server)
        .await;

    // Board (organization-info)
    Mock::given(method("GET"))
        .and(path("/organization-info/api/v1/board/org-001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"userId": "user-1", "name": "Alice", "roles": ["DIRECTOR"], "active": true}
        ])))
        .mount(&org_server)
        .await;

    // Shareholders (consent-info)
    Mock::given(method("GET"))
        .and(path(
            "/consent-info/api/v1/shareholders/organization/org-001",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "organizationId": "org-001",
                "email": "alice@example.com",
                "firstName": "Alice"
            }
        ])))
        .mount(&consent_server)
        .await;

    let client = test_client_dual(&org_server, &consent_server).await;
    let identity = client
        .identity()
        .get_composite_identity("org-001")
        .await
        .unwrap();

    assert_eq!(identity.organization_id, "org-001");
    assert_eq!(identity.members.len(), 1);
    assert_eq!(identity.directors.len(), 1);
    assert_eq!(identity.shareholders.len(), 1);
}

#[tokio::test]
async fn get_composite_identity_propagates_partial_failures() {
    let org_server = MockServer::start().await;
    let consent_server = MockServer::start().await;

    // Members succeeds
    Mock::given(method("GET"))
        .and(path("/organization-info/api/v1/membership/org-001/members"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"userId": "user-1", "name": "Alice", "roles": ["ADMIN"]}
        ])))
        .mount(&org_server)
        .await;

    // Board returns 500 (error)
    Mock::given(method("GET"))
        .and(path("/organization-info/api/v1/board/org-001"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&org_server)
        .await;

    // Shareholders returns 500 (error)
    Mock::given(method("GET"))
        .and(path(
            "/consent-info/api/v1/shareholders/organization/org-001",
        ))
        .respond_with(ResponseTemplate::new(500))
        .mount(&consent_server)
        .await;

    let client = test_client_dual(&org_server, &consent_server).await;
    let result = client.identity().get_composite_identity("org-001").await;

    // Graceful degradation (P1-005): when some sub-queries fail, the facade
    // returns partial results rather than a hard failure. This allows callers
    // to serve available data while logging warnings for the failed sub-queries.
    // Complete failures (all sub-queries empty) still return an error.
    let identity = result.expect("partial success should return Ok with available data");
    assert_eq!(identity.members.len(), 1, "members sub-query succeeded");
    assert!(
        identity.directors.is_empty(),
        "directors sub-query failed gracefully"
    );
    assert!(
        identity.shareholders.is_empty(),
        "shareholders sub-query failed gracefully"
    );
}

// ── Forward compatibility ────────────────────────────────────────────

#[tokio::test]
async fn member_deserializes_with_unknown_fields() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organization-info/api/v1/membership/org-001/members"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "userId": "user-1",
                "name": "Alice",
                "futureField": "ignored",
                "roles": []
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = test_client_org(&mock_server).await;
    let members = client.identity().get_members("org-001").await.unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].name.as_deref(), Some("Alice"));
}
