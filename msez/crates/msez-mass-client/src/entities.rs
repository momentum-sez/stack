//! Typed client for Mass organization-info API (ENTITIES primitive).
//!
//! Base URL: `organization-info.api.mass.inc`
//! Swagger: `/organization-info/swagger-ui/index.html`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::MassApiError;

// -- Typed enums matching Mass API values ------------------------------------

/// Entity type as defined by the Mass organization-info API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MassEntityType {
    Llc,
    Corporation,
    Company,
    Partnership,
    #[serde(rename = "sole_proprietor")]
    SoleProprietor,
    Trust,
    /// Forward-compatible catch-all for entity types the Mass API introduces
    /// after this client version is deployed.
    #[serde(other)]
    Unknown,
}

/// Entity status as defined by the Mass organization-info API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MassEntityStatus {
    Active,
    Inactive,
    Suspended,
    Dissolved,
    /// Forward-compatible catch-all.
    #[serde(other)]
    Unknown,
}

// -- Request/Response types matching Mass API schemas -------------------------

/// Entity as returned by the Mass organization-info API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassEntity {
    pub id: Uuid,
    pub entity_type: MassEntityType,
    pub legal_name: String,
    pub jurisdiction_id: String,
    pub status: MassEntityStatus,
    #[serde(default)]
    pub beneficial_owners: Vec<MassBeneficialOwner>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Beneficial owner as represented in the Mass API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassBeneficialOwner {
    pub name: String,
    pub ownership_percentage: String,
    pub cnic: Option<String>,
    pub ntn: Option<String>,
}

/// Request to create an entity via Mass.
#[derive(Debug, Serialize)]
pub struct CreateEntityRequest {
    pub entity_type: MassEntityType,
    pub legal_name: String,
    pub jurisdiction_id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub beneficial_owners: Vec<MassBeneficialOwner>,
}

// -- Client -------------------------------------------------------------------

/// Client for the Mass organization-info API.
#[derive(Debug, Clone)]
pub struct EntityClient {
    http: reqwest::Client,
    base_url: url::Url,
}

impl EntityClient {
    pub(crate) fn new(http: reqwest::Client, base_url: url::Url) -> Self {
        Self { http, base_url }
    }

    /// Create a new entity in Mass.
    pub async fn create(&self, req: &CreateEntityRequest) -> Result<MassEntity, MassApiError> {
        let endpoint = "POST /organizations";
        let url = format!("{}organization-info/organizations", self.base_url);

        let resp = crate::retry::retry_send(|| self.http.post(&url).json(req).send())
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

    /// Get an entity by ID from Mass.
    pub async fn get(&self, id: Uuid) -> Result<Option<MassEntity>, MassApiError> {
        let endpoint = format!("GET /organizations/{id}");
        let url = format!("{}organization-info/organizations/{id}", self.base_url);

        let resp = crate::retry::retry_send(|| self.http.get(&url).send())
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

    /// List entities from Mass (with optional pagination).
    pub async fn list(
        &self,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> Result<Vec<MassEntity>, MassApiError> {
        let endpoint = "GET /organizations";
        let mut url = format!("{}organization-info/organizations", self.base_url);

        let mut params = Vec::new();
        if let Some(o) = offset {
            params.push(format!("offset={o}"));
        }
        if let Some(l) = limit {
            params.push(format!("limit={l}"));
        }
        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        let resp = crate::retry::retry_send(|| self.http.get(&url).send())
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
}
