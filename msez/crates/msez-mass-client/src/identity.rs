//! Typed client for Mass identity services (IDENTITY primitive).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// -- Types matching Mass API schemas ------------------------------------------

/// Identity record from Mass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassIdentity {
    pub id: Uuid,
    pub identity_type: String,
    pub status: String,
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

// -- Client -------------------------------------------------------------------

/// Client for Mass identity services.
#[derive(Debug, Clone)]
pub struct IdentityClient {
    _http: reqwest::Client,
    _base_url: url::Url,
}

impl IdentityClient {
    pub(crate) fn new(http: reqwest::Client, base_url: url::Url) -> Self {
        Self {
            _http: http,
            _base_url: base_url,
        }
    }
}
