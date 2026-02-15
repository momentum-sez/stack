//! Typed client for Mass identity services (IDENTITY primitive).
//!
//! ## Architecture Note (P1-005)
//!
//! The Identity primitive does NOT have a dedicated API service. Identity is
//! currently split across:
//!
//! - **organization-info**: Handles entity-level identity (membership, board,
//!   beneficial ownership verification as part of formation).
//!
//! - **consent-info**: Handles governance identity (shareholders, equity holders,
//!   signatory verification, consent voting authority).
//!
//! This client acts as an **aggregation facade** that composes identity-related
//! operations from both underlying services. A dedicated `identity-info.api.mass.inc`
//! is recommended for the Pakistan GovOS deployment where NADRA integration
//! demands a clear identity service boundary.
//!
//! ## Identity-Related Endpoints (organization-info)
//!
//! | Method | Path | Operation |
//! |--------|------|-----------|
//! | GET    | `/api/v1/membership/{orgId}/members` | Get organization members |
//! | GET    | `/api/v1/board/{orgId}` | Get board of directors |
//!
//! ## Identity-Related Endpoints (consent-info)
//!
//! | Method | Path | Operation |
//! |--------|------|-----------|
//! | POST   | `/api/v1/shareholders` | Create shareholder (identity) |
//! | GET    | `/api/v1/shareholders/organization/{orgId}` | Get shareholders |

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::MassApiError;

/// API version path for organization-info (identity endpoints).
const ORG_API_PREFIX: &str = "organization-info/api/v1";

// -- Typed enums matching Mass API values ------------------------------------

/// Identity type -- individual or corporate entity.
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
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MassIdentityStatus {
    Active,
    Inactive,
    Pending,
    Verified,
    Rejected,
    Expired,
    Deleted,
    /// Forward-compatible catch-all.
    #[serde(other)]
    Unknown,
}

// -- Types matching Mass API schemas ------------------------------------------

/// Organization member from organization-info API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassMember {
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub profile_image: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
}

/// Board director from organization-info API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassDirector {
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub shares: Option<u64>,
    #[serde(default)]
    pub ownership_percentage: Option<String>,
    #[serde(default)]
    pub active: Option<bool>,
}

/// Shareholder identity from consent-info API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassShareholder {
    pub id: Uuid,
    pub organization_id: String,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub business_name: Option<String>,
    #[serde(default)]
    pub is_entity: Option<bool>,
    #[serde(default)]
    pub status: Option<MassIdentityStatus>,
    #[serde(default)]
    pub outstanding_shares: Option<u64>,
    #[serde(default)]
    pub fully_diluted_shares: Option<u64>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Composite identity record assembled from multiple Mass services.
///
/// This is an SEZ Stack-side aggregation, not a direct Mass API response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassIdentity {
    pub organization_id: String,
    #[serde(default)]
    pub members: Vec<MassMember>,
    #[serde(default)]
    pub directors: Vec<MassDirector>,
    #[serde(default)]
    pub shareholders: Vec<MassShareholder>,
}

// -- Client -------------------------------------------------------------------

/// Client for Mass identity services (aggregation facade).
///
/// Identity is split across organization-info and consent-info. This client
/// provides a unified interface.
#[derive(Debug, Clone)]
pub struct IdentityClient {
    http: reqwest::Client,
    org_base_url: url::Url,
    consent_base_url: url::Url,
}

impl IdentityClient {
    pub(crate) fn new(
        http: reqwest::Client,
        org_base_url: url::Url,
        consent_base_url: url::Url,
    ) -> Self {
        Self {
            http,
            org_base_url,
            consent_base_url,
        }
    }

    /// Get members of an organization.
    ///
    /// Calls `GET {org_base}/organization-info/api/v1/membership/{orgId}/members`.
    pub async fn get_members(
        &self,
        organization_id: &str,
    ) -> Result<Vec<MassMember>, MassApiError> {
        let endpoint = format!("GET /membership/{organization_id}/members");
        let url = format!(
            "{}{}/membership/{organization_id}/members",
            self.org_base_url, ORG_API_PREFIX
        );

        let resp = crate::retry::retry_send(|| self.http.get(&url).send())
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

        resp.json().await.map_err(|e| MassApiError::Deserialization {
            endpoint,
            source: e,
        })
    }

    /// Get board of directors for an organization.
    ///
    /// Calls `GET {org_base}/organization-info/api/v1/board/{orgId}`.
    pub async fn get_board(
        &self,
        organization_id: &str,
    ) -> Result<Vec<MassDirector>, MassApiError> {
        let endpoint = format!("GET /board/{organization_id}");
        let url = format!(
            "{}{}/board/{organization_id}",
            self.org_base_url, ORG_API_PREFIX
        );

        let resp = crate::retry::retry_send(|| self.http.get(&url).send())
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

        resp.json().await.map_err(|e| MassApiError::Deserialization {
            endpoint,
            source: e,
        })
    }

    /// Get shareholders for an organization.
    ///
    /// Calls `GET {consent_base}/consent-info/api/v1/shareholders/organization/{orgId}`.
    pub async fn get_shareholders(
        &self,
        organization_id: &str,
    ) -> Result<Vec<MassShareholder>, MassApiError> {
        let endpoint = format!("GET /shareholders/organization/{organization_id}");
        let url = format!(
            "{}consent-info/api/v1/shareholders/organization/{organization_id}",
            self.consent_base_url
        );

        let resp = crate::retry::retry_send(|| self.http.get(&url).send())
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

        resp.json().await.map_err(|e| MassApiError::Deserialization {
            endpoint,
            source: e,
        })
    }

    /// Assemble a composite identity for an organization by calling
    /// members, board, and shareholders endpoints.
    pub async fn get_composite_identity(
        &self,
        organization_id: &str,
    ) -> Result<MassIdentity, MassApiError> {
        // Fire all three requests concurrently.
        let (members, directors, shareholders) = tokio::join!(
            self.get_members(organization_id),
            self.get_board(organization_id),
            self.get_shareholders(organization_id),
        );

        Ok(MassIdentity {
            organization_id: organization_id.to_string(),
            members: members.unwrap_or_default(),
            directors: directors.unwrap_or_default(),
            shareholders: shareholders.unwrap_or_default(),
        })
    }
}
