//! Typed client for Mass treasury-info API (FISCAL primitive).
//!
//! Base URL: `treasury-info.api.mass.inc`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// -- Types matching Mass API schemas ------------------------------------------

/// Fiscal account as returned by the Mass treasury-info API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassFiscalAccount {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub account_type: String,
    pub currency: String,
    pub balance: String,
    pub ntn: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
    pub status: String,
    pub created_at: DateTime<Utc>,
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
    _http: reqwest::Client,
    _base_url: url::Url,
}

impl FiscalClient {
    pub(crate) fn new(http: reqwest::Client, base_url: url::Url) -> Self {
        Self {
            _http: http,
            _base_url: base_url,
        }
    }
}
