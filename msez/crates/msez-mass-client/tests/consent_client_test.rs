//! Contract tests for ConsentClient against the Mass consent-info Swagger spec.
//!
//! ## Endpoints Tested
//!
//! | Method | Path | Test |
//! |--------|------|------|
//! | POST   | `/api/v1/consents` | `create_consent_*` |
//! | GET    | `/api/v1/consents/{id}` | `get_consent_*` |
//! | DELETE | `/api/v1/consents/{id}` | `cancel_consent_*` |
//! | POST   | `/api/v1/consents/approve/{id}` | `approve_consent_*` |
//! | POST   | `/api/v1/consents/reject/{id}` | `reject_consent_*` |
//! | GET    | `/api/v1/consents/organization/{orgId}` | `list_by_org_*` |

use msez_mass_client::consent::{
    CreateConsentRequest, MassConsentOperationType, MassConsentStatus,
};
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
        api_token: "test-token".into(),
        timeout_secs: 5,
    };
    MassClient::new(config).unwrap()
}

// ── POST /api/v1/consents ────────────────────────────────────────────

#[tokio::test]
async fn create_consent_sends_correct_path_and_returns_consent() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/consent-info/api/v1/consents"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "organizationId": "org-001",
            "operationType": "EQUITY_OFFER",
            "status": "PENDING",
            "votes": [],
            "numVotesRequired": 2,
            "createdAt": "2026-01-15T12:00:00Z",
            "updatedAt": "2026-01-15T12:00:00Z"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let req = CreateConsentRequest {
        organization_id: "org-001".into(),
        operation_type: MassConsentOperationType::EquityOffer,
        operation_id: None,
        num_board_member_approvals_required: Some(2),
        requested_by: None,
        signatory: None,
        expiry_date: None,
        details: None,
    };

    let consent = client.consent().create(&req).await.unwrap();
    assert_eq!(consent.organization_id, "org-001");
    assert_eq!(
        consent.operation_type,
        Some(MassConsentOperationType::EquityOffer)
    );
    assert_eq!(consent.status, Some(MassConsentStatus::Pending));
    assert_eq!(consent.num_votes_required, Some(2));
}

#[tokio::test]
async fn create_consent_handles_api_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/consent-info/api/v1/consents"))
        .respond_with(ResponseTemplate::new(400).set_body_string("bad request"))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let req = CreateConsentRequest {
        organization_id: "".into(),
        operation_type: MassConsentOperationType::EquityOffer,
        operation_id: None,
        num_board_member_approvals_required: None,
        requested_by: None,
        signatory: None,
        expiry_date: None,
        details: None,
    };

    let result = client.consent().create(&req).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        msez_mass_client::MassApiError::ApiError { status, .. } => {
            assert_eq!(status, 400);
        }
        other => panic!("expected ApiError, got: {other:?}"),
    }
}

// ── GET /api/v1/consents/{id} ────────────────────────────────────────

#[tokio::test]
async fn get_consent_returns_consent_when_found() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440000";

    Mock::given(method("GET"))
        .and(path(format!("/consent-info/api/v1/consents/{id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": id,
            "organizationId": "org-001",
            "operationType": "ISSUE_NEW_SHARES",
            "status": "APPROVED",
            "votes": [{"vote": "APPROVE", "votedBy": "user-1", "approve": true}],
            "approvalCount": 1,
            "rejectionCount": 0,
            "createdAt": "2026-01-15T12:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let consent = client.consent().get(id.parse().unwrap()).await.unwrap();
    assert!(consent.is_some());
    let consent = consent.unwrap();
    assert_eq!(consent.organization_id, "org-001");
    assert_eq!(consent.status, Some(MassConsentStatus::Approved));
    assert_eq!(consent.votes.len(), 1);
    assert_eq!(consent.approval_count, Some(1));
}

#[tokio::test]
async fn get_consent_returns_none_when_not_found() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440001";

    Mock::given(method("GET"))
        .and(path(format!("/consent-info/api/v1/consents/{id}")))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let consent = client.consent().get(id.parse().unwrap()).await.unwrap();
    assert!(consent.is_none());
}

// ── POST /api/v1/consents/approve/{id} ───────────────────────────────

#[tokio::test]
async fn approve_consent_returns_vote_response() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440000";

    Mock::given(method("POST"))
        .and(path(format!("/consent-info/api/v1/consents/approve/{id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "consentId": id,
            "organizationId": "org-001",
            "vote": "APPROVE",
            "votedBy": "user-1",
            "operationType": "EQUITY_OFFER",
            "majorityReached": true,
            "createdAt": "2026-01-15T12:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let vote = client
        .consent()
        .approve(id.parse().unwrap(), false)
        .await
        .unwrap();
    assert_eq!(vote.consent_id.to_string(), id);
    assert_eq!(vote.organization_id, "org-001");
    assert_eq!(vote.majority_reached, Some(true));
}

#[tokio::test]
async fn approve_consent_with_force() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440000";

    // When force=true, the query param should be appended.
    Mock::given(method("POST"))
        .and(path(format!("/consent-info/api/v1/consents/approve/{id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "consentId": id,
            "organizationId": "org-001",
            "operationType": "EQUITY_OFFER",
            "majorityReached": true,
            "createdAt": "2026-01-15T12:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let vote = client
        .consent()
        .approve(id.parse().unwrap(), true)
        .await
        .unwrap();
    assert_eq!(vote.majority_reached, Some(true));
}

// ── POST /api/v1/consents/reject/{id} ────────────────────────────────

#[tokio::test]
async fn reject_consent_returns_vote_response() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440000";

    Mock::given(method("POST"))
        .and(path(format!("/consent-info/api/v1/consents/reject/{id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "consentId": id,
            "organizationId": "org-001",
            "vote": "REJECT",
            "votedBy": "user-2",
            "majorityReached": false,
            "createdAt": "2026-01-15T12:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let vote = client.consent().reject(id.parse().unwrap()).await.unwrap();
    assert_eq!(vote.consent_id.to_string(), id);
    assert_eq!(vote.majority_reached, Some(false));
}

// ── GET /api/v1/consents/organization/{orgId} ────────────────────────

#[tokio::test]
async fn list_by_organization_returns_consents() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/consent-info/api/v1/consents/organization/org-001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "organizationId": "org-001",
                "operationType": "EQUITY_OFFER",
                "status": "PENDING",
                "votes": [],
                "createdAt": "2026-01-15T12:00:00Z"
            },
            {
                "id": "550e8400-e29b-41d4-a716-446655440001",
                "organizationId": "org-001",
                "operationType": "ISSUE_NEW_SHARES",
                "status": "APPROVED",
                "votes": [],
                "createdAt": "2026-01-14T12:00:00Z"
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let consents = client
        .consent()
        .list_by_organization("org-001")
        .await
        .unwrap();
    assert_eq!(consents.len(), 2);
    assert_eq!(consents[0].status, Some(MassConsentStatus::Pending));
    assert_eq!(consents[1].status, Some(MassConsentStatus::Approved));
}

// ── DELETE /api/v1/consents/{id} ─────────────────────────────────────

#[tokio::test]
async fn cancel_consent_succeeds() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440000";

    Mock::given(method("DELETE"))
        .and(path(format!("/consent-info/api/v1/consents/{id}")))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.consent().cancel(id.parse().unwrap()).await;
    assert!(result.is_ok());
}

// ── Forward compatibility ────────────────────────────────────────────

#[tokio::test]
async fn consent_deserializes_with_unknown_operation_type() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440000";

    Mock::given(method("GET"))
        .and(path(format!("/consent-info/api/v1/consents/{id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": id,
            "organizationId": "org-001",
            "operationType": "FUTURE_OP_TYPE",
            "status": "FORCE_APPROVED",
            "votes": [],
            "createdAt": "2026-01-15T12:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let consent = client
        .consent()
        .get(id.parse().unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        consent.operation_type,
        Some(MassConsentOperationType::Unknown)
    );
    assert_eq!(consent.status, Some(MassConsentStatus::ForceApproved));
}
