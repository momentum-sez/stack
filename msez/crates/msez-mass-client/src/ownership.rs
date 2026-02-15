//! Typed client for Mass investment-info API (OWNERSHIP primitive).
//!
//! Base URL: `investment-info-production-4f3779c81425.herokuapp.com`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::MassApiError;

// -- Types matching Mass API schemas ------------------------------------------

/// Cap table as returned by the Mass investment-info API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassCapTable {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub share_classes: Vec<MassShareClass>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Share class definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassShareClass {
    pub name: String,
    pub authorized_shares: u64,
    pub issued_shares: u64,
    pub par_value: Option<String>,
    pub voting_rights: bool,
}

/// Request to create a cap table for an entity.
#[derive(Debug, Serialize)]
pub struct CreateCapTableRequest {
    pub entity_id: Uuid,
    pub share_classes: Vec<MassShareClass>,
}

/// Ownership transfer event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassOwnershipTransfer {
    pub id: Uuid,
    pub from_holder: String,
    pub to_holder: String,
    pub share_class: String,
    pub quantity: u64,
    pub price_per_share: Option<String>,
    pub transferred_at: DateTime<Utc>,
}

// -- Client -------------------------------------------------------------------

/// Client for the Mass investment-info API.
#[derive(Debug, Clone)]
pub struct OwnershipClient {
    http: reqwest::Client,
    base_url: url::Url,
}

impl OwnershipClient {
    pub(crate) fn new(http: reqwest::Client, base_url: url::Url) -> Self {
        Self { http, base_url }
    }

    /// Create a cap table for an entity.
    ///
    /// Calls `POST {base_url}/investment-info/cap-tables`.
    pub async fn create_cap_table(
        &self,
        req: &CreateCapTableRequest,
    ) -> Result<MassCapTable, MassApiError> {
        let endpoint = "POST /cap-tables";
        let url = format!("{}investment-info/cap-tables", self.base_url);

        let resp = self
            .http
            .post(&url)
            .json(req)
            .send()
            .await
            .map_err(|e| MassApiError::Http {
                endpoint: endpoint.into(),
                source: e,
            })?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(MassApiError::ApiError {
                endpoint: endpoint.into(),
                status,
                body,
            });
        }

        resp.json().await.map_err(|e| MassApiError::Deserialization {
            endpoint: endpoint.into(),
            source: e,
        })
    }

    /// Get a cap table by entity ID.
    ///
    /// Calls `GET {base_url}/investment-info/cap-tables/{entity_id}`.
    pub async fn get_cap_table(
        &self,
        entity_id: Uuid,
    ) -> Result<Option<MassCapTable>, MassApiError> {
        let endpoint = format!("GET /cap-tables/{entity_id}");
        let url = format!(
            "{}investment-info/cap-tables/{entity_id}",
            self.base_url
        );

        let resp =
            self.http
                .get(&url)
                .send()
                .await
                .map_err(|e| MassApiError::Http {
                    endpoint: endpoint.clone(),
                    source: e,
                })?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(MassApiError::ApiError {
                endpoint,
                status,
                body,
            });
        }

        resp.json()
            .await
            .map(Some)
            .map_err(|e| MassApiError::Deserialization {
                endpoint,
                source: e,
            })
    }
}
