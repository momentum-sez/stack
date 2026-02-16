//! # msez-mass-client -- Typed Rust client for Mass Protocol APIs
//!
//! Provides ergonomic, typed access to the five Mass programmable primitives:
//! - **Entities** via `organization-info.api.mass.inc`
//! - **Ownership** via `consent.api.mass.inc` (cap tables) + `investment-info` (investments)
//! - **Fiscal** via `treasury-info.api.mass.inc`
//! - **Identity** via aggregation of `organization-info` + `consent-info`
//! - **Consent** via `consent.api.mass.inc`
//!
//! Plus the **Templating Engine** for document generation.
//!
//! ## Architecture
//!
//! This crate is the ONLY authorized path for the SEZ Stack to interact with
//! Mass primitive data. Direct HTTP requests to Mass endpoints from any other
//! crate are forbidden (see CLAUDE.md Section II).
//!
//! ## API Path Convention
//!
//! All Mass APIs are Spring Boot services with context paths. The full URL
//! pattern is: `{base_url}/{context-path}/api/v1/{resource}`.
//! For example: `https://consent.api.mass.inc/consent-info/api/v1/consents`.

pub mod config;
pub mod consent;
pub mod entities;
pub mod error;
pub mod fiscal;
pub mod identity;
pub mod nadra;
pub mod ownership;
pub(crate) mod retry;
pub mod templating;
pub mod types;

pub use config::MassApiConfig;
pub use error::MassApiError;
pub use types::MassEntityId;

// Re-export msez-core identifier newtypes for callers that need type-safe
// identifiers when working with Mass API data. Per CLAUDE.md §V.2, this
// crate depends on msez-core ONLY for these identifier types.
pub use msez_core::{Cnic, Did, EntityId, JurisdictionId, Ntn};

use std::time::Duration;

/// Result of a health check against the Mass API services.
///
/// Reports which core services are reachable and which are not.
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    /// Services that responded successfully.
    pub reachable: Vec<String>,
    /// Services that failed with error details.
    pub unreachable: Vec<(String, String)>,
}

impl HealthCheckResult {
    /// Whether all checked services are reachable.
    pub fn all_healthy(&self) -> bool {
        self.unreachable.is_empty()
    }
}

/// Top-level Mass API client. Holds sub-clients for each primitive.
#[derive(Debug, Clone)]
pub struct MassClient {
    entities: entities::EntityClient,
    ownership: ownership::OwnershipClient,
    fiscal: fiscal::FiscalClient,
    identity: identity::IdentityClient,
    consent: consent::ConsentClient,
    templating: templating::TemplatingClient,
    /// HTTP client for health checks (short timeout, no auth required).
    health_http: reqwest::Client,
    /// Base URLs for health check probing (service_name → URL).
    health_urls: Vec<(String, url::Url)>,
}

impl MassClient {
    /// Create a new Mass API client from configuration.
    pub fn new(config: MassApiConfig) -> Result<Self, MassApiError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!(
                        "Bearer {}",
                        config.api_token.as_str()
                    ))
                    .map_err(|_| MassApiError::Config(config::ConfigError::MissingToken))?,
                );
                headers
            })
            .build()
            .map_err(|e| MassApiError::Http {
                endpoint: "client_init".into(),
                source: e,
            })?;

        // Short-timeout client for health checks (no auth headers needed).
        let health_http = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .map_err(|e| MassApiError::Http {
                endpoint: "health_client_init".into(),
                source: e,
            })?;

        // Collect the distinct Mass API base URLs for health probing.
        // Use the Swagger docs endpoint as the health target — it's
        // lightweight and always available on Spring Boot services.
        let health_urls = vec![
            (
                "organization-info".to_string(),
                config.organization_info_url.clone(),
            ),
            ("treasury-info".to_string(), config.treasury_info_url.clone()),
            ("consent-info".to_string(), config.consent_info_url.clone()),
        ];

        Ok(Self {
            entities: entities::EntityClient::new(
                http.clone(),
                config.organization_info_url.clone(),
            ),
            // Ownership: cap tables live on consent-info, investments on investment-info.
            ownership: ownership::OwnershipClient::new(
                http.clone(),
                config.consent_info_url.clone(),
                config.investment_info_url,
            ),
            fiscal: fiscal::FiscalClient::new(http.clone(), config.treasury_info_url),
            // Identity: aggregation facade across org-info and consent-info,
            // with optional dedicated identity-info service for Pakistan GovOS (P1-005).
            identity: identity::IdentityClient::new(
                http.clone(),
                config.organization_info_url,
                config.consent_info_url.clone(),
                config.identity_info_url,
            ),
            consent: consent::ConsentClient::new(http.clone(), config.consent_info_url),
            templating: templating::TemplatingClient::new(http, config.templating_engine_url),
            health_http,
            health_urls,
        })
    }

    /// Access the entities (organization-info) client.
    pub fn entities(&self) -> &entities::EntityClient {
        &self.entities
    }

    /// Access the ownership (cap tables via consent-info, investments via investment-info) client.
    pub fn ownership(&self) -> &ownership::OwnershipClient {
        &self.ownership
    }

    /// Access the fiscal (treasury-info) client.
    pub fn fiscal(&self) -> &fiscal::FiscalClient {
        &self.fiscal
    }

    /// Access the identity client (aggregation facade).
    pub fn identity(&self) -> &identity::IdentityClient {
        &self.identity
    }

    /// Access the consent (consent-info) client.
    pub fn consent(&self) -> &consent::ConsentClient {
        &self.consent
    }

    /// Access the templating-engine client.
    pub fn templating(&self) -> &templating::TemplatingClient {
        &self.templating
    }

    /// Probe connectivity to the core Mass API services.
    ///
    /// Sends a lightweight GET request to each service's Swagger/API docs
    /// endpoint (which responds without authentication on Spring Boot services).
    /// Uses a 3-second timeout to avoid blocking the readiness probe.
    ///
    /// Returns a [`HealthCheckResult`] with reachable and unreachable services.
    /// Does not retry — the readiness probe will be called again by the
    /// orchestrator (K8s, Docker health check).
    pub async fn health_check(&self) -> HealthCheckResult {
        let mut reachable = Vec::new();
        let mut unreachable = Vec::new();

        for (name, base_url) in &self.health_urls {
            // Spring Boot actuator health endpoint. Falls back to a
            // simple GET to the context path if actuator is not configured.
            let health_url = format!("{}{}/v3/api-docs", base_url, name);
            match self.health_http.get(&health_url).send().await {
                Ok(resp) if resp.status().is_success() || resp.status().is_redirection() => {
                    reachable.push(name.clone());
                }
                Ok(resp) => {
                    // Got an HTTP response (service is reachable) but not
                    // a success status. For health purposes, any response
                    // from the server means the service is alive.
                    tracing::debug!(
                        service = %name,
                        status = %resp.status(),
                        "Mass API responded with non-success status (still reachable)"
                    );
                    reachable.push(name.clone());
                }
                Err(e) => {
                    unreachable.push((name.clone(), e.to_string()));
                }
            }
        }

        HealthCheckResult {
            reachable,
            unreachable,
        }
    }
}
