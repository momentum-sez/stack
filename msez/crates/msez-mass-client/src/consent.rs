//! Typed client for Mass consent-info API (CONSENT primitive).
//!
//! Base URL: `consent.api.mass.inc`
//! Context path: `/consent-info`
//! Swagger: `/consent-info/swagger-ui/index.html`
//! API docs: `/consent-info/v3/api-docs`
//!
//! ## Live API Paths (from Swagger spec, February 2026)
//!
//! | Method | Path | Operation |
//! |--------|------|-----------|
//! | POST   | `/api/v1/consents` | Create consent |
//! | GET    | `/api/v1/consents/{id}` | Get consent by ID |
//! | DELETE | `/api/v1/consents/{id}` | Cancel consent |
//! | POST   | `/api/v1/consents/approve/{id}` | Approve consent |
//! | POST   | `/api/v1/consents/reject/{id}` | Reject consent |
//! | GET    | `/api/v1/consents/organization/{organizationId}` | Get org consents |

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::MassApiError;

/// API version path for consent-info service.
const API_PREFIX: &str = "consent-info/api/v1";

// -- Typed enums matching Mass API values ------------------------------------

/// Consent operation type as defined by the live consent-info API.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MassConsentOperationType {
    EquityOffer,
    IssueNewShares,
    AmendOptionsPool,
    CreateOptionsPool,
    CreateCommonClass,
    ModifyCompanyLegalName,
    ModifyBoardMemberDesignation,
    CertificateOfAmendment,
    /// Forward-compatible catch-all.
    #[serde(other)]
    Unknown,
}

/// Consent status as defined by the live consent-info API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MassConsentStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
    ForceApproved,
    Completed,
    Canceled,
    /// Forward-compatible catch-all.
    #[serde(other)]
    Unknown,
}

// -- Types matching Mass API schemas ------------------------------------------

/// Consent record from Mass consent-info API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassConsent {
    pub id: Uuid,
    pub organization_id: String,
    #[serde(default)]
    pub operation_id: Option<Uuid>,
    #[serde(default)]
    pub operation_type: Option<MassConsentOperationType>,
    #[serde(default)]
    pub status: Option<MassConsentStatus>,
    #[serde(default)]
    pub votes: Vec<MassConsentVote>,
    #[serde(default)]
    pub num_votes_required: Option<u32>,
    #[serde(default)]
    pub approval_count: Option<u32>,
    #[serde(default)]
    pub rejection_count: Option<u32>,
    #[serde(default)]
    pub document_url: Option<String>,
    #[serde(default)]
    pub signatory: Option<String>,
    #[serde(default)]
    pub jurisdiction: Option<String>,
    #[serde(default)]
    pub requested_by: Option<String>,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

/// A vote cast on a consent request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassConsentVote {
    #[serde(default)]
    pub vote: Option<String>,
    #[serde(default)]
    pub voted_by: Option<String>,
    #[serde(default)]
    pub board_member_id: Option<String>,
    #[serde(default)]
    pub approve: Option<bool>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
}

/// Vote response returned by approve/reject operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassConsentVoteResponse {
    pub consent_id: Uuid,
    #[serde(default)]
    pub operation_id: Option<Uuid>,
    pub organization_id: String,
    #[serde(default)]
    pub vote: Option<String>,
    #[serde(default)]
    pub voted_by: Option<String>,
    #[serde(default)]
    pub operation_type: Option<MassConsentOperationType>,
    #[serde(default)]
    pub majority_reached: Option<bool>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
}

/// Request to create a consent request.
///
/// Matches `POST /api/v1/consents` on consent-info.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateConsentRequest {
    pub organization_id: String,
    pub operation_type: MassConsentOperationType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_board_member_approvals_required: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signatory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_date: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
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
    /// Calls `POST {base_url}/consent-info/api/v1/consents`.
    pub async fn create(&self, req: &CreateConsentRequest) -> Result<MassConsent, MassApiError> {
        let endpoint = "POST /consents";
        let url = format!("{}{}/consents", self.base_url, API_PREFIX);

        let resp = crate::retry::retry_send(|| self.http.post(&url).json(req).send())
            .await
            .map_err(|e| MassApiError::Http {
                endpoint: endpoint.into(),
                source: e,
            })?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_else(|e| format!("<failed to read response body: {e}>"));
            return Err(MassApiError::ApiError {
                endpoint: endpoint.into(),
                status,
                body,
            });
        }

        resp.json()
            .await
            .map_err(|e| MassApiError::Deserialization {
                endpoint: endpoint.into(),
                source: e,
            })
    }

    /// Get a consent by ID.
    ///
    /// Calls `GET {base_url}/consent-info/api/v1/consents/{id}`.
    pub async fn get(&self, id: Uuid) -> Result<Option<MassConsent>, MassApiError> {
        let endpoint = format!("GET /consents/{id}");
        let url = format!("{}{}/consents/{id}", self.base_url, API_PREFIX);

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
            let body = resp.text().await.unwrap_or_else(|e| format!("<failed to read response body: {e}>"));
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

    /// Approve a consent request.
    ///
    /// Calls `POST {base_url}/consent-info/api/v1/consents/approve/{id}`.
    pub async fn approve(
        &self,
        id: Uuid,
        force: bool,
    ) -> Result<MassConsentVoteResponse, MassApiError> {
        let endpoint = format!("POST /consents/approve/{id}");
        let mut url = format!("{}{}/consents/approve/{id}", self.base_url, API_PREFIX);

        if force {
            url.push_str("?force=true");
        }

        let resp = crate::retry::retry_send(|| self.http.post(&url).send())
            .await
            .map_err(|e| MassApiError::Http {
                endpoint: endpoint.clone(),
                source: e,
            })?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_else(|e| format!("<failed to read response body: {e}>"));
            return Err(MassApiError::ApiError {
                endpoint,
                status,
                body,
            });
        }

        resp.json()
            .await
            .map_err(|e| MassApiError::Deserialization {
                endpoint,
                source: e,
            })
    }

    /// Reject a consent request.
    ///
    /// Calls `POST {base_url}/consent-info/api/v1/consents/reject/{id}`.
    pub async fn reject(&self, id: Uuid) -> Result<MassConsentVoteResponse, MassApiError> {
        let endpoint = format!("POST /consents/reject/{id}");
        let url = format!("{}{}/consents/reject/{id}", self.base_url, API_PREFIX);

        let resp = crate::retry::retry_send(|| self.http.post(&url).send())
            .await
            .map_err(|e| MassApiError::Http {
                endpoint: endpoint.clone(),
                source: e,
            })?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_else(|e| format!("<failed to read response body: {e}>"));
            return Err(MassApiError::ApiError {
                endpoint,
                status,
                body,
            });
        }

        resp.json()
            .await
            .map_err(|e| MassApiError::Deserialization {
                endpoint,
                source: e,
            })
    }

    /// Get all consents for an organization.
    ///
    /// Calls `GET {base_url}/consent-info/api/v1/consents/organization/{org_id}`.
    pub async fn list_by_organization(
        &self,
        organization_id: &str,
    ) -> Result<Vec<MassConsent>, MassApiError> {
        let encoded_org_id: String =
            url::form_urlencoded::byte_serialize(organization_id.as_bytes()).collect();
        let endpoint = format!("GET /consents/organization/{organization_id}");
        let url = format!(
            "{}{}/consents/organization/{encoded_org_id}",
            self.base_url, API_PREFIX
        );

        let resp = crate::retry::retry_send(|| self.http.get(&url).send())
            .await
            .map_err(|e| MassApiError::Http {
                endpoint: endpoint.clone(),
                source: e,
            })?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_else(|e| format!("<failed to read response body: {e}>"));
            return Err(MassApiError::ApiError {
                endpoint,
                status,
                body,
            });
        }

        resp.json()
            .await
            .map_err(|e| MassApiError::Deserialization {
                endpoint,
                source: e,
            })
    }

    /// Cancel a consent request.
    ///
    /// Calls `DELETE {base_url}/consent-info/api/v1/consents/{id}`.
    pub async fn cancel(&self, id: Uuid) -> Result<(), MassApiError> {
        let endpoint = format!("DELETE /consents/{id}");
        let url = format!("{}{}/consents/{id}", self.base_url, API_PREFIX);

        let resp = crate::retry::retry_send(|| self.http.delete(&url).send())
            .await
            .map_err(|e| MassApiError::Http {
                endpoint: endpoint.clone(),
                source: e,
            })?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_else(|e| format!("<failed to read response body: {e}>"));
            return Err(MassApiError::ApiError {
                endpoint,
                status,
                body,
            });
        }

        Ok(())
    }
}
