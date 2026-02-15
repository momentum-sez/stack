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

/// Type of tax event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaxEventType {
    /// Withholding tax deducted at source.
    WithholdingAtSource,
    /// Income tax assessment.
    IncomeTaxAssessment,
    /// Sales tax on goods/services.
    SalesTax,
    /// Capital gains tax on asset disposal.
    CapitalGainsTax,
    /// Annual tax year-end event.
    TaxYearEnd,
    /// Tax payment made to FBR.
    TaxPayment,
    /// Forward-compatible catch-all.
    #[serde(other)]
    Unknown,
}

/// Request to record a tax event.
#[derive(Debug, Serialize)]
pub struct RecordTaxEventRequest {
    pub entity_id: Uuid,
    pub event_type: TaxEventType,
    pub amount: String,
    pub currency: String,
    pub tax_year: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_transaction_id: Option<Uuid>,
    #[serde(default)]
    pub details: serde_json::Value,
}

/// Request to compute withholding for a transaction.
#[derive(Debug, Serialize)]
pub struct WithholdingComputeRequest {
    pub entity_id: Uuid,
    pub transaction_amount: String,
    pub currency: String,
    pub transaction_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ntn: Option<String>,
    pub jurisdiction_id: String,
}

/// Withholding computation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithholdingResult {
    pub entity_id: Uuid,
    pub gross_amount: String,
    pub withholding_amount: String,
    pub withholding_rate: String,
    pub net_amount: String,
    pub currency: String,
    pub withholding_type: String,
    pub ntn_status: String,
    pub computed_at: DateTime<Utc>,
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

    /// Record a tax event for an entity.
    ///
    /// Calls `POST {base_url}/treasury-info/tax-events`.
    pub async fn record_tax_event(
        &self,
        req: &RecordTaxEventRequest,
    ) -> Result<MassTaxEvent, MassApiError> {
        let endpoint = "POST /tax-events";
        let url = format!("{}treasury-info/tax-events", self.base_url);

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

    /// List tax events for an entity.
    ///
    /// Calls `GET {base_url}/treasury-info/tax-events?entity_id={entity_id}`.
    pub async fn list_tax_events(
        &self,
        entity_id: Uuid,
        tax_year: Option<&str>,
    ) -> Result<Vec<MassTaxEvent>, MassApiError> {
        let endpoint = format!("GET /tax-events?entity_id={entity_id}");
        let mut url = format!(
            "{}treasury-info/tax-events?entity_id={entity_id}",
            self.base_url
        );
        if let Some(year) = tax_year {
            url.push_str(&format!("&tax_year={year}"));
        }

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

    /// Compute withholding tax for a transaction.
    ///
    /// Calls `POST {base_url}/treasury-info/withholding/compute`.
    pub async fn compute_withholding(
        &self,
        req: &WithholdingComputeRequest,
    ) -> Result<WithholdingResult, MassApiError> {
        let endpoint = "POST /withholding/compute";
        let url = format!("{}treasury-info/withholding/compute", self.base_url);

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
