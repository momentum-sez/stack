//! Typed client for Mass OWNERSHIP primitive.
//!
//! The Ownership primitive spans TWO Mass API services:
//!
//! - **consent-info** (`consent.api.mass.inc`): Cap tables, share classes,
//!   shareholders, securities, equity offers, options pools, corporate actions.
//!   This is the primary service for cap table management.
//!
//! - **investment-info** (`investment-info-production-...herokuapp.com`):
//!   Investments, fundraisers, capital calls, fund administration.
//!
//! This client uses the consent-info base URL for cap table operations
//! (since that is where the live endpoints are deployed per Swagger spec).
//!
//! ## Live API Paths (consent-info, from Swagger spec)
//!
//! | Method | Path | Operation |
//! |--------|------|-----------|
//! | POST   | `/api/v1/capTables` | Create cap table |
//! | GET    | `/api/v1/capTables/{id}` | Get cap table by ID |
//! | GET    | `/api/v1/capTables/organization/{organizationId}` | Get by org |
//! | POST   | `/api/v1/shareClasses/request` | Request share class |
//! | GET    | `/api/v1/shareClasses/organization/{organizationId}` | Get share classes |
//! | POST   | `/api/v1/securities` | Create security |
//! | GET    | `/api/v1/securities/organization/{organizationId}` | Get securities |
//! | POST   | `/api/v1/shareholders` | Create shareholder |
//! | GET    | `/api/v1/shareholders/organization/{organizationId}` | Get shareholders |
//!
//! ## Live API Paths (investment-info, from Swagger spec)
//!
//! | Method | Path | Operation |
//! |--------|------|-----------|
//! | POST   | `/api/v1/investment` | Create investment |
//! | GET    | `/api/v1/investment/{id}` | Get investment |
//! | GET    | `/api/v1/fundraiser/{id}` | Get fundraiser |
//! | GET    | `/api/v1/fundraiser/organization/{organizationId}` | Get fundraisers |

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::MassApiError;

/// API version path for consent-info service (cap table endpoints).
const CONSENT_API_PREFIX: &str = "consent-info/api/v1";

// -- Types matching Mass consent-info API Swagger schemas ---------------------

/// Cap table as returned by the Mass consent-info API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassCapTable {
    pub id: Uuid,
    pub organization_id: String,
    #[serde(default)]
    pub authorized_shares: Option<u64>,
    #[serde(default)]
    pub outstanding_shares: Option<u64>,
    #[serde(default)]
    pub fully_diluted_shares: Option<u64>,
    #[serde(default)]
    pub reserved_shares: Option<u64>,
    #[serde(default)]
    pub unreserved_shares: Option<u64>,
    #[serde(default)]
    pub share_classes: Vec<MassShareClass>,
    #[serde(default)]
    pub shareholders: Vec<serde_json::Value>,
    #[serde(default)]
    pub options_pools: Vec<serde_json::Value>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Share class definition from consent-info API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassShareClass {
    #[serde(default)]
    pub id: Option<Uuid>,
    pub name: String,
    #[serde(default)]
    pub authorized_shares: u64,
    #[serde(default, alias = "issued_shares")]
    pub outstanding_shares: u64,
    #[serde(default)]
    pub par_value: Option<String>,
    #[serde(default)]
    pub voting_rights: bool,
    #[serde(default)]
    pub restricted: bool,
    #[serde(default, alias = "type")]
    pub class_type: Option<String>,
}

/// Request to create a cap table.
///
/// Matches `POST /api/v1/capTables` on consent-info.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCapTableRequest {
    pub organization_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorized_shares: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options_pool: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub par_value: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub shareholders: Vec<ShareholderAllocation>,
}

/// Shareholder allocation for cap table creation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareholderAllocation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percentage: Option<f64>,
}

/// Ownership transfer event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

/// Client for the Mass ownership primitives (cap tables via consent-info).
#[derive(Debug, Clone)]
pub struct OwnershipClient {
    http: reqwest::Client,
    /// consent-info base URL (cap tables, shareholders, securities).
    consent_base_url: url::Url,
    /// investment-info base URL (investments, fundraisers).
    _investment_base_url: url::Url,
}

impl OwnershipClient {
    pub(crate) fn new(
        http: reqwest::Client,
        consent_base_url: url::Url,
        investment_base_url: url::Url,
    ) -> Self {
        Self {
            http,
            consent_base_url,
            _investment_base_url: investment_base_url,
        }
    }

    /// Create a cap table for an organization.
    ///
    /// Calls `POST {consent_base}/consent-info/api/v1/capTables`.
    pub async fn create_cap_table(
        &self,
        req: &CreateCapTableRequest,
    ) -> Result<MassCapTable, MassApiError> {
        let endpoint = "POST /capTables";
        let url = format!("{}{}/capTables", self.consent_base_url, CONSENT_API_PREFIX);

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

    /// Get a cap table by its ID.
    ///
    /// Calls `GET {consent_base}/consent-info/api/v1/capTables/{id}`.
    pub async fn get_cap_table(&self, id: Uuid) -> Result<Option<MassCapTable>, MassApiError> {
        let endpoint = format!("GET /capTables/{id}");
        let url = format!(
            "{}{}/capTables/{id}",
            self.consent_base_url, CONSENT_API_PREFIX
        );

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

    /// Get a cap table by organization ID.
    ///
    /// Calls `GET {consent_base}/consent-info/api/v1/capTables/organization/{org_id}`.
    pub async fn get_cap_table_by_org(
        &self,
        organization_id: &str,
    ) -> Result<Option<MassCapTable>, MassApiError> {
        let encoded_org_id: String =
            url::form_urlencoded::byte_serialize(organization_id.as_bytes()).collect();
        let endpoint = format!("GET /capTables/organization/{organization_id}");
        let url = format!(
            "{}{}/capTables/organization/{encoded_org_id}",
            self.consent_base_url, CONSENT_API_PREFIX
        );

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

    /// Get share classes for an organization.
    ///
    /// Calls `GET {consent_base}/consent-info/api/v1/shareClasses/organization/{org_id}`.
    pub async fn get_share_classes(
        &self,
        organization_id: &str,
    ) -> Result<Vec<MassShareClass>, MassApiError> {
        let encoded_org_id: String =
            url::form_urlencoded::byte_serialize(organization_id.as_bytes()).collect();
        let endpoint = format!("GET /shareClasses/organization/{organization_id}");
        let url = format!(
            "{}{}/shareClasses/organization/{encoded_org_id}",
            self.consent_base_url, CONSENT_API_PREFIX
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
}
