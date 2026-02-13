//! Typed client for Mass investment-info API (OWNERSHIP primitive).
//!
//! Base URL: `investment-info-production-4f3779c81425.herokuapp.com`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// -- Types matching Mass API schemas ------------------------------------------

/// Cap table as returned by the Mass investment-info API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassCapTable {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub share_classes: Vec<MassShareClass>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Share class definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassShareClass {
    pub name: String,
    pub authorized_shares: u64,
    pub issued_shares: u64,
    pub par_value: Option<String>,
    pub voting_rights: bool,
}

/// Ownership transfer event.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Client for the Mass investment-info API.
#[derive(Debug, Clone)]
pub struct OwnershipClient {
    _http: reqwest::Client,
    _base_url: url::Url,
}

impl OwnershipClient {
    pub(crate) fn new(http: reqwest::Client, base_url: url::Url) -> Self {
        Self {
            _http: http,
            _base_url: base_url,
        }
    }
}
