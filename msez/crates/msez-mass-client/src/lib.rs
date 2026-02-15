//! # msez-mass-client — Typed Rust client for Mass Protocol APIs
//!
//! Provides ergonomic, typed access to the five Mass programmable primitives:
//! - **Entities** via `organization-info.api.mass.inc`
//! - **Ownership** via `investment-info`
//! - **Fiscal** via `treasury-info.api.mass.inc`
//! - **Identity** via Mass identity services
//! - **Consent** via `consent.api.mass.inc`
//!
//! Plus the **Templating Engine** for document generation.
//!
//! ## Architecture
//!
//! This crate is the ONLY authorized path for the SEZ Stack to interact with
//! Mass primitive data. Direct HTTP requests to Mass endpoints from any other
//! crate are forbidden (see CLAUDE.md Section II).

pub mod config;
pub mod consent;
pub mod entities;
pub mod error;
pub mod fiscal;
pub mod identity;
pub mod ownership;
pub(crate) mod retry;
pub mod templating;

pub use config::MassApiConfig;
pub use error::MassApiError;

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
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {}", config.api_token))
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
            entities: entities::EntityClient::new(http.clone(), config.organization_info_url.clone()),
            ownership: ownership::OwnershipClient::new(http.clone(), config.investment_info_url),
            fiscal: fiscal::FiscalClient::new(http.clone(), config.treasury_info_url),
            identity: identity::IdentityClient::new(
                http.clone(),
                config.consent_info_url.clone(),
                config.organization_info_url,
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

    /// Access the ownership (investment-info) client.
    pub fn ownership(&self) -> &ownership::OwnershipClient {
        &self.ownership
    }

    /// Access the fiscal (treasury-info) client.
    pub fn fiscal(&self) -> &fiscal::FiscalClient {
        &self.fiscal
    }

    /// Access the identity client.
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
