//! # Jurisdiction & Corridor Identifiers
//!
//! Newtypes for jurisdiction and corridor identifiers. These are the
//! fundamental addressing primitives in the SEZ Stack — a jurisdiction
//! identifies a zone's legal context, and a corridor identifies a
//! bilateral trade channel between two jurisdictions.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A jurisdiction identifier, typically an ISO 3166-1 code or a
/// zone-specific identifier (e.g., "PK-RSEZ" for Pakistan Rashakai SEZ).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JurisdictionId(String);

impl JurisdictionId {
    /// Create a jurisdiction identifier from a string.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Access the jurisdiction identifier string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for JurisdictionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A unique identifier for a trade corridor between two jurisdictions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CorridorId(Uuid);

impl CorridorId {
    /// Create a new random corridor identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a corridor identifier from an existing UUID.
    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Access the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for CorridorId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CorridorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
