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
pub mod ownership;
pub(crate) mod retry;
pub mod templating;
pub mod types;

pub use config::MassApiConfig;
pub use error::MassApiError;
pub use types::MassEntityId;

use std::time::Duration;

/// Top-level Mass API client. Holds sub-clients for each primitive.
#[derive(Debug, Clone)]
pub struct MassClient {
    entities: entities::EntityClient,
    ownership: ownership::OwnershipClient,
    fiscal: fiscal::FiscalClient,
    identity: identity::IdentityClient,
    consent: consent::ConsentClient,
    templating: templating::TemplatingClient,
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
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {}", config.api_token.as_str()))
                        .map_err(|_| MassApiError::Config(config::ConfigError::MissingToken))?,
                );
                headers
            })
            .build()
            .map_err(|e| MassApiError::Http {
                endpoint: "client_init".into(),
                source: e,
            })?;

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
}
