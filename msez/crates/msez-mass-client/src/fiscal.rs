//! Typed client for Mass treasury-info API (FISCAL primitive).
//!
//! Base URL: `treasury-info.api.mass.inc`
//! Context path: `/treasury-info`
//! Swagger: `/treasury-info/swagger-ui/index.html`
//! API docs: `/treasury-info/v3/api-docs`
//!
//! ## Live API Paths (from Swagger spec, February 2026)
//!
//! ### Treasury
//! | Method | Path | Operation |
//! |--------|------|-----------|
//! | POST   | `/api/v1/treasury/create` | Create treasury |
//! | GET    | `/api/v1/treasury/{id}` | Get treasury by ID |
//! | GET    | `/api/v1/treasury/entity/{entityId}` | Get by entity |
//!
//! ### Accounts
//! | Method | Path | Operation |
//! |--------|------|-----------|
//! | POST   | `/api/v1/account/create` | Create account (requires treasuryId, idempotencyKey) |
//! | GET    | `/api/v1/account/{id}` | Get account by ID |
//!
//! ### Transactions/Payments
//! | Method | Path | Operation |
//! |--------|------|-----------|
//! | POST   | `/api/v1/transaction/create/payment` | Create payment |
//! | GET    | `/api/v1/transaction/{id}` | Get transaction by ID |

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::MassApiError;

/// API version path for treasury-info service.
const API_PREFIX: &str = "treasury-info/api/v1";

// -- Typed enums matching Mass API values ------------------------------------

/// Treasury context as defined by the Mass treasury-info API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MassTreasuryContext {
    UnitFinance,
    CurrencyCloud,
    Clowd9,
    Interlace,
    Paynetics,
    Mass,
    Tenet,
    NotWorthy,
    /// Forward-compatible catch-all.
    #[serde(other)]
    Unknown,
}

/// Payment status as defined by Mass treasury-info.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
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

/// Treasury record from the Mass treasury-info API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassTreasury {
    pub id: Uuid,
    #[serde(default)]
    pub reference_id: Option<String>,
    pub entity_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub status: Option<serde_json::Value>,
    #[serde(default)]
    pub context: Option<MassTreasuryContext>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Bank account as returned by the Mass treasury-info API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassFiscalAccount {
    pub id: Uuid,
    #[serde(default)]
    pub entity_id: Option<String>,
    #[serde(default)]
    pub treasury_id: Option<Uuid>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub balance: Option<String>,
    #[serde(default)]
    pub available: Option<String>,
    #[serde(default)]
    pub status: Option<serde_json::Value>,
    #[serde(default)]
    pub funding_details: Option<serde_json::Value>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Request to create a treasury.
///
/// Matches `POST /api/v1/treasury/create` on treasury-info.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTreasuryRequest {
    pub entity_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<MassTreasuryContext>,
}

/// Financial transaction as returned by the treasury-info API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassPayment {
    pub id: Uuid,
    #[serde(default)]
    pub account_id: Option<Uuid>,
    #[serde(default)]
    pub entity_id: Option<String>,
    #[serde(default)]
    pub transaction_type: Option<String>,
    #[serde(default)]
    pub status: Option<MassPaymentStatus>,
    #[serde(default)]
    pub direction: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub amount: Option<String>,
    #[serde(default)]
    pub reference: Option<String>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
}

/// Request to create a payment.
///
/// Matches `POST /api/v1/transaction/create/payment` on treasury-info.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePaymentRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_type: Option<String>,
    pub amount: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_entity: Option<serde_json::Value>,
    pub source_account_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

/// Tax event record from Mass treasury-info.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassTaxEvent {
    pub id: Uuid,
    pub entity_id: String,
    pub event_type: String,
    pub amount: String,
    pub currency: String,
    #[serde(default)]
    pub tax_year: Option<String>,
    #[serde(default)]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

    /// Create a treasury for an entity.
    ///
    /// Calls `POST {base_url}/treasury-info/api/v1/treasury/create`.
    pub async fn create_treasury(
        &self,
        req: &CreateTreasuryRequest,
    ) -> Result<MassTreasury, MassApiError> {
        let endpoint = "POST /treasury/create";
        let url = format!("{}{}/treasury/create", self.base_url, API_PREFIX);

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

    /// Create a bank account within a treasury.
    ///
    /// Calls `POST {base_url}/treasury-info/api/v1/account/create?treasuryId={}&idempotencyKey={}`.
    pub async fn create_account(
        &self,
        treasury_id: Uuid,
        idempotency_key: &str,
        name: Option<&str>,
    ) -> Result<MassFiscalAccount, MassApiError> {
        let endpoint = "POST /account/create";
        let encoded_key: String =
            url::form_urlencoded::byte_serialize(idempotency_key.as_bytes()).collect();
        let mut url = format!(
            "{}{}/account/create?treasuryId={}&idempotencyKey={}",
            self.base_url, API_PREFIX, treasury_id, encoded_key
        );

        if let Some(n) = name {
            let encoded_name: String =
                url::form_urlencoded::byte_serialize(n.as_bytes()).collect();
            url.push_str(&format!("&name={encoded_name}"));
        }

        let resp = crate::retry::retry_send(|| self.http.post(&url).send())
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

    /// Get a bank account by ID.
    ///
    /// Calls `GET {base_url}/treasury-info/api/v1/account/{id}`.
    pub async fn get_account(&self, id: Uuid) -> Result<Option<MassFiscalAccount>, MassApiError> {
        let endpoint = format!("GET /account/{id}");
        let url = format!("{}{}/account/{id}", self.base_url, API_PREFIX);

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

    /// Create a payment transaction.
    ///
    /// Calls `POST {base_url}/treasury-info/api/v1/transaction/create/payment`.
    pub async fn create_payment(
        &self,
        req: &CreatePaymentRequest,
    ) -> Result<MassPayment, MassApiError> {
        let endpoint = "POST /transaction/create/payment";
        let url = format!("{}{}/transaction/create/payment", self.base_url, API_PREFIX);

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

    /// Get a transaction by ID.
    ///
    /// Calls `GET {base_url}/treasury-info/api/v1/transaction/{id}`.
    pub async fn get_transaction(&self, id: Uuid) -> Result<Option<MassPayment>, MassApiError> {
        let endpoint = format!("GET /transaction/{id}");
        let url = format!("{}{}/transaction/{id}", self.base_url, API_PREFIX);

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

    /// Record a tax event for an entity.
    ///
    /// Calls `POST {base_url}/treasury-info/api/v1/tax-events`.
    pub async fn record_tax_event(
        &self,
        req: &RecordTaxEventRequest,
    ) -> Result<MassTaxEvent, MassApiError> {
        let endpoint = "POST /tax-events";
        let url = format!("{}{}/tax-events", self.base_url, API_PREFIX);

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

    /// List tax events for an entity.
    ///
    /// Calls `GET {base_url}/treasury-info/api/v1/tax-events?entity_id={entity_id}`.
    pub async fn list_tax_events(
        &self,
        entity_id: Uuid,
        tax_year: Option<&str>,
    ) -> Result<Vec<MassTaxEvent>, MassApiError> {
        let endpoint = format!("GET /tax-events?entity_id={entity_id}");
        let mut url = format!(
            "{}{}/tax-events?entity_id={entity_id}",
            self.base_url, API_PREFIX
        );
        if let Some(year) = tax_year {
            let encoded_year: String =
                url::form_urlencoded::byte_serialize(year.as_bytes()).collect();
            url.push_str(&format!("&tax_year={encoded_year}"));
        }

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

    /// Compute withholding tax for a transaction.
    ///
    /// Calls `POST {base_url}/treasury-info/api/v1/withholding/compute`.
    pub async fn compute_withholding(
        &self,
        req: &WithholdingComputeRequest,
    ) -> Result<WithholdingResult, MassApiError> {
        let endpoint = "POST /withholding/compute";
        let url = format!("{}{}/withholding/compute", self.base_url, API_PREFIX);

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
}
