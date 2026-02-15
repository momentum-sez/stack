//! # Identity Aggregation Facade (IDENTITY primitive)
//!
//! The Identity primitive does not yet have its own dedicated Mass API service.
//! Identity is currently split across:
//! - `consent-info`: identity verification, KYC/KYB workflows, attestation
//! - `organization-info`: entity-level identity (CNIC/NTN binding, beneficial ownership)
//!
//! This client acts as an **aggregation facade** that unifies both sources behind
//! a single `IdentityClient` interface. When a dedicated `identity-info.api.mass.inc`
//! is deployed, the client transparently routes to it instead.
//!
//! ## Pakistan GovOS Integration Points
//!
//! - **NADRA**: CNIC verification via identity verification endpoints
//! - **FBR IRIS**: NTN cross-reference via entity identity endpoints
//! - **SECP**: Corporate identity via organization-info entity records
//!
//! See CLAUDE.md §II and Architecture Audit v5.0 §4.1 (P1-005).

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

/// Aggregation facade for Mass identity services.
///
/// Unifies identity operations currently split across `consent-info` and
/// `organization-info`. When a dedicated `identity-info.api.mass.inc` is
/// deployed, set `dedicated_url` to route all requests there.
///
/// ## Data Sources
///
/// | Operation | Dedicated Service | Fallback (current) |
/// |-----------|------------------|--------------------|
/// | KYC/KYB verify | identity-info | consent-info |
/// | Get identity | identity-info | consent-info |
/// | CNIC verify | identity-info | organization-info |
/// | NTN verify | identity-info | organization-info |
/// | List by entity | identity-info | consent-info |
#[derive(Debug, Clone)]
pub struct IdentityClient {
    http: reqwest::Client,
    /// URL for consent-info (identity verification, KYC workflows).
    consent_url: url::Url,
    /// URL for organization-info (entity-level identity: CNIC/NTN binding).
    org_info_url: url::Url,
    /// Dedicated identity-info URL. When set, all requests route here instead
    /// of being split across consent-info and organization-info.
    dedicated_url: Option<url::Url>,
}

impl IdentityClient {
    pub(crate) fn new(
        http: reqwest::Client,
        consent_url: url::Url,
        org_info_url: url::Url,
        dedicated_url: Option<url::Url>,
    ) -> Self {
        Self {
            http,
            consent_url,
            org_info_url,
            dedicated_url,
        }
    }

    /// Return the base URL for identity verification operations.
    /// Uses the dedicated service if configured, otherwise consent-info.
    fn identity_base_url(&self) -> &url::Url {
        self.dedicated_url.as_ref().unwrap_or(&self.consent_url)
    }

    /// Return the base URL for entity-level identity operations (CNIC/NTN).
    /// Uses the dedicated service if configured, otherwise organization-info.
    fn entity_identity_base_url(&self) -> &url::Url {
        self.dedicated_url.as_ref().unwrap_or(&self.org_info_url)
    }

    /// Get an identity by ID.
    ///
    /// Routes to dedicated identity-info if configured, otherwise consent-info.
    pub async fn get_identity(&self, id: Uuid) -> Result<Option<MassIdentity>, MassApiError> {
        let base = self.identity_base_url();
        let endpoint = format!("GET /identities/{id}");

        let service_path = if self.dedicated_url.is_some() {
            "identity-info"
        } else {
            "consent-info"
        };
        let url = format!("{base}{service_path}/identities/{id}");

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
    /// Routes to dedicated identity-info if configured, otherwise consent-info.
    pub async fn verify(&self, req: &VerifyIdentityRequest) -> Result<MassIdentity, MassApiError> {
        let base = self.identity_base_url();
        let endpoint = "POST /identities/verify";

        let service_path = if self.dedicated_url.is_some() {
            "identity-info"
        } else {
            "consent-info"
        };
        let url = format!("{base}{service_path}/identities/verify");

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

        resp.json()
            .await
            .map_err(|e| MassApiError::Deserialization {
                endpoint: endpoint.into(),
                source: e,
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
