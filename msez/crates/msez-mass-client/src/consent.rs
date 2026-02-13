//! Typed client for Mass consent-info API (CONSENT primitive).
//!
//! Base URL: `consent.api.mass.inc`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// -- Types matching Mass API schemas ------------------------------------------

/// Consent record from Mass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassConsent {
    pub id: Uuid,
    pub consent_type: String,
    pub description: String,
    pub parties: Vec<MassConsentParty>,
    pub status: String,
    pub audit_trail: Vec<MassConsentAuditEntry>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Party involved in a consent request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassConsentParty {
    pub entity_id: Uuid,
    pub role: String,
    pub decision: Option<String>,
    pub decided_at: Option<DateTime<Utc>>,
}

/// Audit trail entry for a consent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassConsentAuditEntry {
    pub action: String,
    pub actor_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub details: Option<String>,
}

// -- Client -------------------------------------------------------------------

/// Client for the Mass consent-info API.
#[derive(Debug, Clone)]
pub struct ConsentClient {
    _http: reqwest::Client,
    _base_url: url::Url,
}

impl ConsentClient {
    pub(crate) fn new(http: reqwest::Client, base_url: url::Url) -> Self {
        Self {
            _http: http,
            _base_url: base_url,
        }
    }
}
