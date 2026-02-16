//! Tests for MassClient::health_check().
//!
//! Verifies that the health check correctly identifies reachable and
//! unreachable Mass API services. Uses wiremock for the reachable case
//! and a bogus URL for the unreachable case.

use msez_mass_client::{MassApiConfig, MassClient};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Build a MassClient with configurable URLs per service.
fn test_client_with_urls(
    org_url: &str,
    treasury_url: &str,
    consent_url: &str,
) -> MassClient {
    let config = MassApiConfig {
        organization_info_url: org_url.parse().unwrap(),
        investment_info_url: "http://127.0.0.1:19001".parse().unwrap(),
        treasury_info_url: treasury_url.parse().unwrap(),
        consent_info_url: consent_url.parse().unwrap(),
        identity_info_url: None,
        templating_engine_url: "http://127.0.0.1:19004".parse().unwrap(),
        api_token: zeroize::Zeroizing::new("test-token".into()),
        timeout_secs: 5,
    };
    MassClient::new(config).unwrap()
}

#[tokio::test]
async fn health_check_all_reachable() {
    let org_server = MockServer::start().await;
    let treasury_server = MockServer::start().await;
    let consent_server = MockServer::start().await;

    // Mount health-check responses on the Swagger docs path.
    for server in [&org_server, &treasury_server, &consent_server] {
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
            .mount(server)
            .await;
    }

    let client = test_client_with_urls(
        &org_server.uri(),
        &treasury_server.uri(),
        &consent_server.uri(),
    );

    let result = client.health_check().await;
    assert!(result.all_healthy(), "All services should be reachable");
    assert_eq!(result.reachable.len(), 3);
    assert!(result.unreachable.is_empty());
}

#[tokio::test]
async fn health_check_one_unreachable() {
    let org_server = MockServer::start().await;
    let consent_server = MockServer::start().await;

    for server in [&org_server, &consent_server] {
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
            .mount(server)
            .await;
    }

    // treasury-info points to a closed port — unreachable.
    let client = test_client_with_urls(
        &org_server.uri(),
        "http://127.0.0.1:1",
        &consent_server.uri(),
    );

    let result = client.health_check().await;
    assert!(!result.all_healthy(), "Should have one unreachable service");
    assert_eq!(result.reachable.len(), 2);
    assert_eq!(result.unreachable.len(), 1);
    assert_eq!(result.unreachable[0].0, "treasury-info");
}

#[tokio::test]
async fn health_check_all_unreachable() {
    let client = test_client_with_urls(
        "http://127.0.0.1:1",
        "http://127.0.0.1:2",
        "http://127.0.0.1:3",
    );

    let result = client.health_check().await;
    assert!(!result.all_healthy());
    assert_eq!(result.reachable.len(), 0);
    assert_eq!(result.unreachable.len(), 3);
}

#[tokio::test]
async fn health_check_non_success_status_still_reachable() {
    // A 401 or 404 from the server means the service is alive (just the
    // specific endpoint may require auth). The health check should treat
    // any HTTP response as "reachable".
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let client = test_client_with_urls(
        &server.uri(),
        &server.uri(),
        &server.uri(),
    );

    let result = client.health_check().await;
    assert!(
        result.all_healthy(),
        "Non-success HTTP status should still count as reachable"
    );
}
