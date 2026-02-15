//! Contract tests for TemplatingClient against the Mass templating-engine Swagger spec.
//!
//! ## Endpoints Tested
//!
//! | Method | Path | Test |
//! |--------|------|------|
//! | POST   | `/api/v1/template/sign` | `sign_*` |
//! | GET    | `/api/v1/template/{id}` | `get_template_*` |
//! | GET    | `/api/v1/template/available` | `available_templates_*` |
//! | GET    | `/api/v1/submission/{id}` | `get_submission_*` |

use msez_mass_client::templating::{
    SignTemplateRequest, SigningOrder, SigningRole, TemplateSigner,
};
use msez_mass_client::{MassApiConfig, MassClient};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn test_client(mock_server: &MockServer) -> MassClient {
    let config = MassApiConfig {
        organization_info_url: "http://127.0.0.1:19000".parse().unwrap(),
        investment_info_url: "http://127.0.0.1:19001".parse().unwrap(),
        treasury_info_url: "http://127.0.0.1:19002".parse().unwrap(),
        consent_info_url: "http://127.0.0.1:19003".parse().unwrap(),
        identity_info_url: None,
        templating_engine_url: mock_server.uri().parse().unwrap(),
        api_token: "test-token".into(),
        timeout_secs: 5,
    };
    MassClient::new(config).unwrap()
}

// ── POST /api/v1/template/sign ───────────────────────────────────────

#[tokio::test]
async fn sign_sends_correct_path_and_returns_submission() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/templating-engine/api/v1/template/sign"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "sub-001",
            "entityId": "entity-001",
            "context": "FORMATION",
            "status": "PENDING",
            "signingOrder": "PRESERVED",
            "signers": [{"email": "alice@example.com", "role": "OFFICER"}],
            "documentUri": "https://docs.mass.inc/sub-001.pdf",
            "createdAt": "2026-01-15T12:00:00Z",
            "updatedAt": "2026-01-15T12:00:00Z"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let req = SignTemplateRequest {
        entity_id: "entity-001".into(),
        signing_order: Some(SigningOrder::Preserved),
        template_types: vec!["CERTIFICATE_OF_FORMATION".into()],
        signers: vec![TemplateSigner {
            id: None,
            reference_id: None,
            name: None,
            email: "alice@example.com".into(),
            signing_role: SigningRole::Officer,
        }],
        fields: None,
        tags: None,
        parent_submission_id: None,
    };

    let submission = client.templating().sign(&req).await.unwrap();
    assert_eq!(submission.id, "sub-001");
    assert_eq!(submission.entity_id.as_deref(), Some("entity-001"));
    assert_eq!(submission.status.as_deref(), Some("PENDING"));
    assert!(submission.document_uri.is_some());
}

#[tokio::test]
async fn sign_handles_api_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/templating-engine/api/v1/template/sign"))
        .respond_with(ResponseTemplate::new(400).set_body_string("invalid template type"))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let req = SignTemplateRequest {
        entity_id: "entity-001".into(),
        signing_order: None,
        template_types: vec![],
        signers: vec![],
        fields: None,
        tags: None,
        parent_submission_id: None,
    };

    let result = client.templating().sign(&req).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        msez_mass_client::MassApiError::ApiError { status, .. } => {
            assert_eq!(status, 400);
        }
        other => panic!("expected ApiError, got: {other:?}"),
    }
}

// ── GET /api/v1/template/{id} ────────────────────────────────────────

#[tokio::test]
async fn get_template_returns_template_when_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/templating-engine/api/v1/template/tpl-001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "tpl-001",
            "name": "Certificate of Formation",
            "context": "FORMATION",
            "type": "CERTIFICATE_OF_FORMATION",
            "grouping": "FORMATION",
            "status": "ACTIVE",
            "version": "1.0"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let template = client
        .templating()
        .get_template("tpl-001")
        .await
        .unwrap();
    assert!(template.is_some());
    let template = template.unwrap();
    assert_eq!(template.id, "tpl-001");
    assert_eq!(template.name.as_deref(), Some("Certificate of Formation"));
    assert_eq!(
        template.template_type.as_deref(),
        Some("CERTIFICATE_OF_FORMATION")
    );
}

#[tokio::test]
async fn get_template_returns_none_when_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/templating-engine/api/v1/template/nonexistent"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let template = client
        .templating()
        .get_template("nonexistent")
        .await
        .unwrap();
    assert!(template.is_none());
}

// ── GET /api/v1/template/available ───────────────────────────────────

#[tokio::test]
async fn available_templates_returns_list() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/templating-engine/api/v1/template/available"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "templateType": "CERTIFICATE_OF_FORMATION",
                "templateName": "Certificate of Formation",
                "entityId": "entity-001",
                "templateId": "tpl-001"
            },
            {
                "templateType": "EQUITY_OFFER",
                "templateName": "Equity Offer Agreement"
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let templates = client
        .templating()
        .available_templates(None, None)
        .await
        .unwrap();
    assert_eq!(templates.len(), 2);
    assert_eq!(templates[0].template_type, "CERTIFICATE_OF_FORMATION");
    assert_eq!(
        templates[0].template_name.as_deref(),
        Some("Certificate of Formation")
    );
}

#[tokio::test]
async fn available_templates_with_filters() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/templating-engine/api/v1/template/available"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "templateType": "CERTIFICATE_OF_FORMATION",
                "templateName": "Certificate of Formation",
                "entityId": "entity-001"
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let templates = client
        .templating()
        .available_templates(Some("entity-001"), Some("FORMATION"))
        .await
        .unwrap();
    assert_eq!(templates.len(), 1);
}

// ── GET /api/v1/submission/{id} ──────────────────────────────────────

#[tokio::test]
async fn get_submission_returns_submission_when_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/templating-engine/api/v1/submission/sub-001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "sub-001",
            "entityId": "entity-001",
            "context": "FORMATION",
            "status": "COMPLETED",
            "signingOrder": "RANDOM",
            "signers": [],
            "documentUri": "https://docs.mass.inc/sub-001.pdf",
            "createdAt": "2026-01-15T12:00:00Z",
            "updatedAt": "2026-01-16T12:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let submission = client
        .templating()
        .get_submission("sub-001")
        .await
        .unwrap();
    assert!(submission.is_some());
    let submission = submission.unwrap();
    assert_eq!(submission.id, "sub-001");
    assert_eq!(submission.status.as_deref(), Some("COMPLETED"));
}

#[tokio::test]
async fn get_submission_returns_none_when_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/templating-engine/api/v1/submission/nonexistent"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let submission = client
        .templating()
        .get_submission("nonexistent")
        .await
        .unwrap();
    assert!(submission.is_none());
}

// ── Forward compatibility ────────────────────────────────────────────

#[tokio::test]
async fn template_deserializes_with_unknown_fields() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/templating-engine/api/v1/template/tpl-001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "tpl-001",
            "name": "Test",
            "futureField": "should be ignored",
            "anotherFutureField": 42
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let template = client
        .templating()
        .get_template("tpl-001")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(template.id, "tpl-001");
    assert_eq!(template.name.as_deref(), Some("Test"));
}
