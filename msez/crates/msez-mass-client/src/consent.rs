//! Typed client for Mass consent-info API (CONSENT primitive).
//!
//! Base URL: `consent.api.mass.inc`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::MassApiError;

// -- Types matching Mass API schemas ------------------------------------------

/// Consent record from Mass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassConsent {
    pub id: Uuid,
    pub consent_type: String,
    pub description: String,
    pub parties: Vec<MassConsentParty>,
    pub status: String,
    pub audit_trail: Vec<MassConsentAuditEntry>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Party involved in a consent request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassConsentParty {
    pub entity_id: Uuid,
    pub role: String,
    pub decision: Option<String>,
    pub decided_at: Option<DateTime<Utc>>,
}

/// Audit trail entry for a consent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassConsentAuditEntry {
    pub action: String,
    pub actor_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub details: Option<String>,
}

/// Request to create a consent request.
#[derive(Debug, Serialize)]
pub struct CreateConsentRequest {
    pub consent_type: String,
    pub description: String,
    pub parties: Vec<ConsentPartyInput>,
}

/// Input for a consent party when creating a request.
#[derive(Debug, Serialize)]
pub struct ConsentPartyInput {
    pub entity_id: Uuid,
    pub role: String,
}

// -- Client -------------------------------------------------------------------

/// Client for the Mass consent-info API.
#[derive(Debug, Clone)]
pub struct ConsentClient {
    http: reqwest::Client,
    base_url: url::Url,
}

impl ConsentClient {
    pub(crate) fn new(http: reqwest::Client, base_url: url::Url) -> Self {
        Self { http, base_url }
    }

    /// Create a consent request.
    ///
    /// Calls `POST {base_url}/consent-info/consents`.
    pub async fn create(
        &self,
        req: &CreateConsentRequest,
    ) -> Result<MassConsent, MassApiError> {
        let endpoint = "POST /consents";
        let url = format!("{}consent-info/consents", self.base_url);

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

    /// Get a consent request by ID.
    ///
    /// Calls `GET {base_url}/consent-info/consents/{id}`.
    pub async fn get(&self, id: Uuid) -> Result<Option<MassConsent>, MassApiError> {
        let endpoint = format!("GET /consents/{id}");
        let url = format!("{}consent-info/consents/{id}", self.base_url);

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
