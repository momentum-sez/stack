//! Typed client for Mass identity services (IDENTITY primitive).
//!
//! Identity is embedded within the organization-info and consent-info APIs.
//! This client wraps the identity-specific endpoints.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::MassApiError;

// -- Typed enums matching Mass API values ------------------------------------

/// Identity type as defined by Mass identity services.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MassIdentityType {
    Individual,
    Corporate,
    /// Forward-compatible catch-all.
    #[serde(other)]
    Unknown,
}

/// Identity verification status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MassIdentityStatus {
    Pending,
    Verified,
    Rejected,
    Expired,
    /// Forward-compatible catch-all.
    #[serde(other)]
    Unknown,
}

// -- Types matching Mass API schemas ------------------------------------------

/// Identity record from Mass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassIdentity {
    pub id: Uuid,
    pub identity_type: MassIdentityType,
    pub status: MassIdentityStatus,
    pub linked_ids: Vec<MassLinkedExternalId>,
    pub attestations: Vec<MassIdentityAttestation>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// External ID linked to an identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassLinkedExternalId {
    pub id_type: String,
    pub id_value: String,
    pub verified: bool,
    pub linked_at: DateTime<Utc>,
}

/// Identity attestation from Mass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassIdentityAttestation {
    pub id: Uuid,
    pub attestation_type: String,
    pub issuer: String,
    pub status: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Request to verify an identity (KYC/KYB).
#[derive(Debug, Serialize)]
pub struct VerifyIdentityRequest {
    pub identity_type: MassIdentityType,
    pub linked_ids: Vec<LinkedIdInput>,
}

/// Input for linking an external ID during verification.
#[derive(Debug, Serialize)]
pub struct LinkedIdInput {
    pub id_type: String,
    pub id_value: String,
}

// -- Client -------------------------------------------------------------------

/// Client for Mass identity services.
#[derive(Debug, Clone)]
pub struct IdentityClient {
    http: reqwest::Client,
    base_url: url::Url,
}

impl IdentityClient {
    pub(crate) fn new(http: reqwest::Client, base_url: url::Url) -> Self {
        Self { http, base_url }
    }

    /// Get an identity by ID.
    ///
    /// Calls `GET {base_url}/consent-info/identities/{id}`.
    pub async fn get_identity(
        &self,
        id: Uuid,
    ) -> Result<Option<MassIdentity>, MassApiError> {
        let endpoint = format!("GET /identities/{id}");
        let url = format!("{}consent-info/identities/{id}", self.base_url);

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

    /// Submit a verification request (KYC/KYB).
    ///
    /// Calls `POST {base_url}/consent-info/identities/verify`.
    pub async fn verify(
        &self,
        req: &VerifyIdentityRequest,
    ) -> Result<MassIdentity, MassApiError> {
        let endpoint = "POST /identities/verify";
        let url = format!("{}consent-info/identities/verify", self.base_url);

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
}
