//! # Identity Aggregation Facade (IDENTITY primitive)
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
//!
//! ## Pakistan GovOS Integration Points
//!
//! - **NADRA**: CNIC verification via identity verification endpoints
//! - **FBR IRIS**: NTN cross-reference via entity identity endpoints
//! - **SECP**: Corporate identity via organization-info entity records

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

/// Type of identity document or credential being verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IdentityDocumentType {
    /// Pakistan NADRA Computerized National Identity Card (13 digits).
    Cnic,
    /// Pakistan FBR National Tax Number (7 digits).
    Ntn,
    /// Travel document (passport).
    Passport,
    /// Corporate registration (SECP).
    CorporateRegistration,
    /// W3C Decentralized Identifier.
    Did,
    /// Forward-compatible catch-all.
    #[serde(other)]
    Other,
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

/// CNIC verification request for NADRA integration.
#[derive(Debug, Serialize)]
pub struct CnicVerificationRequest {
    /// CNIC number (13 digits, with or without dashes).
    pub cnic: String,
    /// Full name for cross-reference.
    pub full_name: String,
    /// Date of birth for validation (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_of_birth: Option<String>,
    /// Entity ID to bind the verified CNIC to (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<Uuid>,
}

/// CNIC verification response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CnicVerificationResponse {
    pub cnic: String,
    pub verified: bool,
    pub full_name: Option<String>,
    pub identity_id: Option<Uuid>,
    pub verification_timestamp: DateTime<Utc>,
    pub details: serde_json::Value,
}

/// NTN verification request for FBR IRIS integration.
#[derive(Debug, Serialize)]
pub struct NtnVerificationRequest {
    /// NTN number (7 digits).
    pub ntn: String,
    /// Entity name for cross-reference.
    pub entity_name: String,
    /// Entity ID to bind the verified NTN to (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<Uuid>,
}

/// NTN verification response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtnVerificationResponse {
    pub ntn: String,
    pub verified: bool,
    pub registered_name: Option<String>,
    pub tax_status: Option<String>,
    pub identity_id: Option<Uuid>,
    pub verification_timestamp: DateTime<Utc>,
    pub details: serde_json::Value,
}

/// Consolidated identity view aggregating data from consent-info and
/// organization-info for a single entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidatedIdentity {
    /// The entity this identity belongs to.
    pub entity_id: Uuid,
    /// Identity records from the identity/consent service.
    pub identities: Vec<MassIdentity>,
    /// CNIC numbers linked to this entity (from organization-info).
    pub cnic_numbers: Vec<String>,
    /// NTN numbers linked to this entity (from organization-info).
    pub ntn_numbers: Vec<String>,
    /// Aggregated verification status.
    pub overall_status: MassIdentityStatus,
    /// Timestamp of this consolidation snapshot.
    pub consolidated_at: DateTime<Utc>,
}

// -- Client -------------------------------------------------------------------

/// Client for Mass identity services (aggregation facade).
///
/// Identity is split across organization-info and consent-info. This client
/// provides a unified interface. When a dedicated `identity-info.api.mass.inc`
/// is deployed, set `dedicated_url` to route verification requests there.
#[derive(Debug, Clone)]
pub struct IdentityClient {
    http: reqwest::Client,
    org_base_url: url::Url,
    consent_base_url: url::Url,
    /// Dedicated identity-info URL. When set, verification and list requests
    /// route here instead of being split across consent-info and organization-info.
    dedicated_url: Option<url::Url>,
}

impl IdentityClient {
    pub(crate) fn new(
        http: reqwest::Client,
        org_base_url: url::Url,
        consent_base_url: url::Url,
        dedicated_url: Option<url::Url>,
    ) -> Self {
        Self {
            http,
            org_base_url,
            consent_base_url,
            dedicated_url,
        }
    }

    /// Return the base URL for identity verification operations.
    /// Uses the dedicated service if configured, otherwise consent-info.
    fn identity_base_url(&self) -> &url::Url {
        self.dedicated_url.as_ref().unwrap_or(&self.consent_base_url)
    }

    /// Return the base URL for entity-level identity operations (CNIC/NTN).
    /// Uses the dedicated service if configured, otherwise organization-info.
    fn entity_identity_base_url(&self) -> &url::Url {
        self.dedicated_url.as_ref().unwrap_or(&self.org_base_url)
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

    /// Verify a CNIC number against NADRA records.
    ///
    /// Routes to dedicated identity-info if configured, otherwise organization-info.
    pub async fn verify_cnic(
        &self,
        req: &CnicVerificationRequest,
    ) -> Result<CnicVerificationResponse, MassApiError> {
        let base = self.entity_identity_base_url();
        let endpoint = "POST /identity/cnic/verify";

        let service_path = if self.dedicated_url.is_some() {
            "identity-info"
        } else {
            "organization-info"
        };
        let url = format!("{base}{service_path}/identity/cnic/verify");

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

    /// Verify an NTN number against FBR IRIS records.
    ///
    /// Routes to dedicated identity-info if configured, otherwise organization-info.
    pub async fn verify_ntn(
        &self,
        req: &NtnVerificationRequest,
    ) -> Result<NtnVerificationResponse, MassApiError> {
        let base = self.entity_identity_base_url();
        let endpoint = "POST /identity/ntn/verify";

        let service_path = if self.dedicated_url.is_some() {
            "identity-info"
        } else {
            "organization-info"
        };
        let url = format!("{base}{service_path}/identity/ntn/verify");

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

    /// List all identity records associated with an entity.
    ///
    /// When using the dedicated service, fetches from a single endpoint.
    /// When split, queries consent-info for identity records.
    pub async fn list_by_entity(
        &self,
        entity_id: Uuid,
    ) -> Result<Vec<MassIdentity>, MassApiError> {
        let base = self.identity_base_url();
        let endpoint = format!("GET /identities?entity_id={entity_id}");

        let service_path = if self.dedicated_url.is_some() {
            "identity-info"
        } else {
            "consent-info"
        };
        let url = format!(
            "{base}{service_path}/identities?entity_id={entity_id}"
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

    /// Whether the client is configured with a dedicated identity-info service.
    pub fn has_dedicated_service(&self) -> bool {
        self.dedicated_url.is_some()
    }
}
