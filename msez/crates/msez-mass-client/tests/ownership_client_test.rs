//! Contract tests for OwnershipClient against the Mass consent-info Swagger spec
//! (cap tables, share classes, shareholders) and investment-info (future).
//!
//! ## Endpoints Tested (consent-info)
//!
//! | Method | Path | Test |
//! |--------|------|------|
//! | POST   | `/api/v1/capTables` | `create_cap_table_*` |
//! | GET    | `/api/v1/capTables/{id}` | `get_cap_table_*` |
//! | GET    | `/api/v1/capTables/organization/{orgId}` | `get_cap_table_by_org_*` |
//! | GET    | `/api/v1/shareClasses/organization/{orgId}` | `get_share_classes_*` |

use msez_mass_client::ownership::{CreateCapTableRequest, ShareholderAllocation};
use msez_mass_client::{MassApiConfig, MassClient};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn test_client(mock_server: &MockServer) -> MassClient {
    let config = MassApiConfig {
        organization_info_url: "http://127.0.0.1:19000".parse().unwrap(),
        investment_info_url: "http://127.0.0.1:19001".parse().unwrap(),
        treasury_info_url: "http://127.0.0.1:19002".parse().unwrap(),
        consent_info_url: mock_server.uri().parse().unwrap(),
        identity_info_url: None,
        templating_engine_url: "http://127.0.0.1:19004".parse().unwrap(),
        api_token: zeroize::Zeroizing::new("test-token".into()),
        timeout_secs: 5,
    };
    MassClient::new(config).unwrap()
}

// ── POST /api/v1/capTables ───────────────────────────────────────────

#[tokio::test]
async fn create_cap_table_sends_correct_path_and_returns_cap_table() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/consent-info/api/v1/capTables"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "organizationId": "org-001",
            "authorizedShares": 1000000,
            "outstandingShares": 0,
            "fullyDilutedShares": 0,
            "reservedShares": 0,
            "unreservedShares": 1000000,
            "shareClasses": [{
                "name": "Common",
                "authorizedShares": 1000000,
                "outstandingShares": 0,
                "votingRights": true,
                "restricted": false
            }],
            "shareholders": [],
            "optionsPools": [],
            "createdAt": "2026-01-15T12:00:00Z",
            "updatedAt": "2026-01-15T12:00:00Z"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let req = CreateCapTableRequest {
        organization_id: "org-001".into(),
        authorized_shares: Some(1000000),
        options_pool: None,
        par_value: Some("0.01".into()),
        shareholders: vec![],
    };

    let cap_table = client.ownership().create_cap_table(&req).await.unwrap();
    assert_eq!(cap_table.organization_id, "org-001");
    assert_eq!(cap_table.authorized_shares, Some(1000000));
    assert_eq!(cap_table.share_classes.len(), 1);
    assert_eq!(cap_table.share_classes[0].name, "Common");
    assert!(cap_table.share_classes[0].voting_rights);
}

#[tokio::test]
async fn create_cap_table_with_shareholders() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/consent-info/api/v1/capTables"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "organizationId": "org-001",
            "authorizedShares": 1000000,
            "shareClasses": [],
            "shareholders": [{"email": "alice@example.com"}],
            "optionsPools": [],
            "createdAt": "2026-01-15T12:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let req = CreateCapTableRequest {
        organization_id: "org-001".into(),
        authorized_shares: Some(1000000),
        options_pool: None,
        par_value: None,
        shareholders: vec![ShareholderAllocation {
            user_id: None,
            email: "alice@example.com".into(),
            first_name: Some("Alice".into()),
            last_name: Some("Khan".into()),
            percentage: Some(51.0),
        }],
    };

    let cap_table = client.ownership().create_cap_table(&req).await.unwrap();
    assert_eq!(cap_table.organization_id, "org-001");
}

#[tokio::test]
async fn create_cap_table_handles_api_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/consent-info/api/v1/capTables"))
        .respond_with(ResponseTemplate::new(400).set_body_string("organization_id is required"))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let req = CreateCapTableRequest {
        organization_id: "".into(),
        authorized_shares: None,
        options_pool: None,
        par_value: None,
        shareholders: vec![],
    };

    let result = client.ownership().create_cap_table(&req).await;
    assert!(result.is_err());
}

// ── GET /api/v1/capTables/{id} ───────────────────────────────────────

#[tokio::test]
async fn get_cap_table_returns_cap_table_when_found() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440000";

    Mock::given(method("GET"))
        .and(path(format!("/consent-info/api/v1/capTables/{id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": id,
            "organizationId": "org-001",
            "authorizedShares": 1000000,
            "outstandingShares": 500000,
            "shareClasses": [{
                "name": "Common",
                "authorizedShares": 1000000,
                "outstandingShares": 500000,
                "votingRights": true,
                "restricted": false
            }],
            "shareholders": [],
            "optionsPools": [],
            "createdAt": "2026-01-15T12:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let cap_table = client
        .ownership()
        .get_cap_table(id.parse().unwrap())
        .await
        .unwrap();
    assert!(cap_table.is_some());
    let cap_table = cap_table.unwrap();
    assert_eq!(cap_table.authorized_shares, Some(1000000));
    assert_eq!(cap_table.outstanding_shares, Some(500000));
}

#[tokio::test]
async fn get_cap_table_returns_none_when_not_found() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440099";

    Mock::given(method("GET"))
        .and(path(format!("/consent-info/api/v1/capTables/{id}")))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let cap_table = client
        .ownership()
        .get_cap_table(id.parse().unwrap())
        .await
        .unwrap();
    assert!(cap_table.is_none());
}

// ── GET /api/v1/capTables/organization/{orgId} ───────────────────────

#[tokio::test]
async fn get_cap_table_by_org_returns_cap_table_when_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/consent-info/api/v1/capTables/organization/org-001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "organizationId": "org-001",
            "authorizedShares": 1000000,
            "shareClasses": [],
            "shareholders": [],
            "optionsPools": [],
            "createdAt": "2026-01-15T12:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let cap_table = client
        .ownership()
        .get_cap_table_by_org("org-001")
        .await
        .unwrap();
    assert!(cap_table.is_some());
    assert_eq!(cap_table.unwrap().organization_id, "org-001");
}

#[tokio::test]
async fn get_cap_table_by_org_returns_none_when_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(
            "/consent-info/api/v1/capTables/organization/nonexistent",
        ))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let cap_table = client
        .ownership()
        .get_cap_table_by_org("nonexistent")
        .await
        .unwrap();
    assert!(cap_table.is_none());
}

// ── GET /api/v1/shareClasses/organization/{orgId} ────────────────────

#[tokio::test]
async fn get_share_classes_returns_list() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(
            "/consent-info/api/v1/shareClasses/organization/org-001",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": "550e8400-e29b-41d4-a716-446655440010",
                "name": "Common A",
                "authorizedShares": 800000,
                "outstandingShares": 400000,
                "votingRights": true,
                "restricted": false
            },
            {
                "id": "550e8400-e29b-41d4-a716-446655440011",
                "name": "Preferred B",
                "authorizedShares": 200000,
                "outstandingShares": 100000,
                "votingRights": false,
                "restricted": true,
                "parValue": "1.00"
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let classes = client
        .ownership()
        .get_share_classes("org-001")
        .await
        .unwrap();
    assert_eq!(classes.len(), 2);
    assert_eq!(classes[0].name, "Common A");
    assert!(classes[0].voting_rights);
    assert!(!classes[0].restricted);
    assert_eq!(classes[1].name, "Preferred B");
    assert!(!classes[1].voting_rights);
    assert!(classes[1].restricted);
    assert_eq!(classes[1].par_value.as_deref(), Some("1.00"));
}

// ── Forward compatibility ────────────────────────────────────────────

#[tokio::test]
async fn cap_table_deserializes_with_minimal_fields() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440000";

    Mock::given(method("GET"))
        .and(path(format!("/consent-info/api/v1/capTables/{id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": id,
            "organizationId": "org-001"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let cap_table = client
        .ownership()
        .get_cap_table(id.parse().unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(cap_table.organization_id, "org-001");
    assert!(cap_table.authorized_shares.is_none());
    assert!(cap_table.share_classes.is_empty());
    assert!(cap_table.shareholders.is_empty());
}
