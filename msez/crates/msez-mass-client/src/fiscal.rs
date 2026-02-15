//! Typed client for Mass treasury-info API (FISCAL primitive).
//!
//! Base URL: `treasury-info.api.mass.inc`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::MassApiError;

// -- Typed enums matching Mass API values ------------------------------------

/// Fiscal account type as defined by Mass treasury-info.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MassAccountType {
    Operating,
    Escrow,
    Tax,
    Settlement,
    /// Forward-compatible catch-all.
    #[serde(other)]
    Unknown,
}

/// Payment status as defined by Mass treasury-info.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MassPaymentStatus {
    Pending,
    Completed,
    Failed,
    Reversed,
    /// Forward-compatible catch-all.
    #[serde(other)]
    Unknown,
}

// -- Types matching Mass API schemas ------------------------------------------

/// Fiscal account as returned by the Mass treasury-info API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassFiscalAccount {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub account_type: MassAccountType,
    pub currency: String,
    pub balance: String,
    pub ntn: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a fiscal account.
#[derive(Debug, Serialize)]
pub struct CreateAccountRequest {
    pub entity_id: Uuid,
    pub account_type: MassAccountType,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ntn: Option<String>,
}

/// Payment record from Mass treasury-info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassPayment {
    pub id: Uuid,
    pub from_account_id: Uuid,
    pub to_account_id: Option<Uuid>,
    pub amount: String,
    pub currency: String,
    pub reference: String,
    pub status: MassPaymentStatus,
    pub created_at: DateTime<Utc>,
}

/// Request to create a payment.
#[derive(Debug, Serialize)]
pub struct CreatePaymentRequest {
    pub from_account_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_account_id: Option<Uuid>,
    pub amount: String,
    pub currency: String,
    pub reference: String,
}

/// Tax event record from Mass treasury-info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassTaxEvent {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub event_type: String,
    pub amount: String,
    pub currency: String,
    pub tax_year: String,
    pub details: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

// -- Client -------------------------------------------------------------------

/// Client for the Mass treasury-info API.
#[derive(Debug, Clone)]
pub struct FiscalClient {
    http: reqwest::Client,
    base_url: url::Url,
}

impl FiscalClient {
    pub(crate) fn new(http: reqwest::Client, base_url: url::Url) -> Self {
        Self { http, base_url }
    }

    /// Create a fiscal account for an entity.
    ///
    /// Calls `POST {base_url}/treasury-info/accounts`.
    pub async fn create_account(
        &self,
        req: &CreateAccountRequest,
    ) -> Result<MassFiscalAccount, MassApiError> {
        let endpoint = "POST /accounts";
        let url = format!("{}treasury-info/accounts", self.base_url);

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

    /// Get a fiscal account by ID.
    ///
    /// Calls `GET {base_url}/treasury-info/accounts/{id}`.
    pub async fn get_account(&self, id: Uuid) -> Result<Option<MassFiscalAccount>, MassApiError> {
        let endpoint = format!("GET /accounts/{id}");
        let url = format!("{}treasury-info/accounts/{id}", self.base_url);

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

    /// Create a payment.
    ///
    /// Calls `POST {base_url}/treasury-info/payments`.
    pub async fn create_payment(
        &self,
        req: &CreatePaymentRequest,
    ) -> Result<MassPayment, MassApiError> {
        let endpoint = "POST /payments";
        let url = format!("{}treasury-info/payments", self.base_url);

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
}
