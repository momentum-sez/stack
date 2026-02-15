//! Typed client for Mass organization-info API (ENTITIES primitive).
//!
//! Base URL: `organization-info.api.mass.inc`
//! Context path: `/organization-info`
//! Swagger: `/organization-info/swagger-ui/index.html`
//! API docs: `/organization-info/v3/api-docs`
//!
//! ## Live API Paths (from Swagger spec, February 2026)
//!
//! | Method | Path (relative to context) | Operation |
//! |--------|---------------------------|-----------|
//! | POST   | `/api/v1/organization/create` | Create organization |
//! | GET    | `/api/v1/organization/{organizationId}` | Get by ID |
//! | PUT    | `/api/v1/organization/{organizationId}` | Update organization |
//! | DELETE | `/api/v1/organization/{organizationId}` | Delete organization |
//! | GET    | `/api/v1/organization` | Get by IDs (query param) |
//! | POST   | `/api/v1/organization/search` | Search with pagination |
//! | GET    | `/api/v1/organization/supported-jurisdictions` | List jurisdictions |

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::MassApiError;

/// API version path segment for organization-info service.
const API_PREFIX: &str = "organization-info/api/v1";

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
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
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

/// Organization as returned by the Mass organization-info API.
///
/// Fields use `#[serde(default)]` for resilience against schema evolution
/// in the live Mass API. The live API may return additional fields not
/// modeled here — `serde(deny_unknown_fields)` is intentionally NOT used.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassEntity {
    pub id: Uuid,
    /// Organization name (mapped from `name` in the live API).
    #[serde(alias = "legal_name")]
    pub name: String,
    #[serde(default)]
    pub jurisdiction: Option<String>,
    #[serde(default)]
    pub status: Option<MassEntityStatus>,
    #[serde(default)]
    pub address: Option<serde_json::Value>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
    /// Board of directors, if returned by the API.
    #[serde(default)]
    pub board: Option<serde_json::Value>,
    /// Members list, if returned by the API.
    #[serde(default)]
    pub members: Option<serde_json::Value>,
}

/// Request to create an organization via Mass.
///
/// Matches the live `POST /api/v1/organization/create` request schema.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEntityRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jurisdiction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Search request for querying organizations.
///
/// Matches the live `POST /api/v1/organization/search` request schema.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchOrganizationsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,
}

/// Paginated search response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchOrganizationsResponse {
    #[serde(default)]
    pub content: Vec<MassEntity>,
    #[serde(default)]
    pub total_elements: Option<u64>,
    #[serde(default)]
    pub total_pages: Option<u32>,
    #[serde(default)]
    pub number: Option<u32>,
    #[serde(default)]
    pub size: Option<u32>,
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

    /// Create a new organization in Mass.
    ///
    /// Calls `POST {base_url}/organization-info/api/v1/organization/create`.
    pub async fn create(&self, req: &CreateEntityRequest) -> Result<MassEntity, MassApiError> {
        let endpoint = "POST /organization/create";
        let url = format!("{}{}/organization/create", self.base_url, API_PREFIX);

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

    /// Get an organization by ID from Mass.
    ///
    /// Calls `GET {base_url}/organization-info/api/v1/organization/{id}`.
    pub async fn get(&self, id: Uuid) -> Result<Option<MassEntity>, MassApiError> {
        let endpoint = format!("GET /organization/{id}");
        let url = format!("{}{}/organization/{id}", self.base_url, API_PREFIX);

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

    /// List organizations by IDs from Mass.
    ///
    /// Calls `GET {base_url}/organization-info/api/v1/organization?ids={ids}`.
    pub async fn list(
        &self,
        ids: Option<&[Uuid]>,
    ) -> Result<Vec<MassEntity>, MassApiError> {
        let endpoint = "GET /organization";
        let mut url = format!("{}{}/organization", self.base_url, API_PREFIX);

        if let Some(ids) = ids {
            if !ids.is_empty() {
                let id_str: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
                url.push_str(&format!("?ids={}", id_str.join(",")));
            }
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

    /// Search organizations with pagination.
    ///
    /// Calls `POST {base_url}/organization-info/api/v1/organization/search`.
    pub async fn search(
        &self,
        req: &SearchOrganizationsRequest,
    ) -> Result<SearchOrganizationsResponse, MassApiError> {
        let endpoint = "POST /organization/search";
        let url = format!("{}{}/organization/search", self.base_url, API_PREFIX);

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

    /// Delete an organization by ID.
    ///
    /// Calls `DELETE {base_url}/organization-info/api/v1/organization/{id}`.
    pub async fn delete(&self, id: Uuid) -> Result<(), MassApiError> {
        let endpoint = format!("DELETE /organization/{id}");
        let url = format!("{}{}/organization/{id}", self.base_url, API_PREFIX);

        let resp = crate::retry::retry_send(|| self.http.delete(&url).send())
            .await
            .map_err(|e| MassApiError::Http {
                endpoint: endpoint.clone(),
                source: e,
            })?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(MassApiError::ApiError {
                endpoint,
                status,
                body,
            });
        }

        Ok(())
    }

    /// Get supported jurisdictions from Mass.
    ///
    /// Calls `GET {base_url}/organization-info/api/v1/organization/supported-jurisdictions`.
    pub async fn supported_jurisdictions(&self) -> Result<Vec<serde_json::Value>, MassApiError> {
        let endpoint = "GET /organization/supported-jurisdictions";
        let url = format!(
            "{}{}/organization/supported-jurisdictions",
            self.base_url, API_PREFIX
        );

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
