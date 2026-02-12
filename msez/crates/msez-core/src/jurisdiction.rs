//! # Jurisdiction & Corridor Identifiers
//!
//! Newtypes for jurisdiction and corridor identifiers. These are the
//! fundamental addressing primitives in the SEZ Stack — a jurisdiction
//! identifies a zone's legal context, and a corridor identifies a
//! bilateral trade channel between two jurisdictions.
//!
//! ## Validation
//!
//! [`JurisdictionId`] is validated to be non-empty at construction time.
//! [`CorridorId`] is UUID-based and always valid by construction.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ValidationError;

/// A jurisdiction identifier, typically an ISO 3166-1 code or a
/// zone-specific identifier (e.g., "PK-RSEZ" for Pakistan Rashakai SEZ).
///
/// # Validation
///
/// Must be a non-empty string. No further format restrictions are imposed
/// because jurisdiction naming varies across SEZ configurations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JurisdictionId(String);

impl JurisdictionId {
    /// Create a jurisdiction identifier from a string, validating non-emptiness.
    ///
    /// # Errors
    ///
    /// Returns [`ValidationError::InvalidJurisdictionId`] if the string is
    /// empty or whitespace-only.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let s = value.into();
        if s.trim().is_empty() {
            return Err(ValidationError::InvalidJurisdictionId);
        }
        Ok(Self(s))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jurisdiction_id_valid() {
        let jid = JurisdictionId::new("PK-RSEZ").unwrap();
        assert_eq!(jid.as_str(), "PK-RSEZ");
    }

    #[test]
    fn jurisdiction_id_rejects_empty() {
        assert!(JurisdictionId::new("").is_err());
        assert!(JurisdictionId::new("   ").is_err());
    }

    #[test]
    fn corridor_id_unique() {
        let a = CorridorId::new();
        let b = CorridorId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn corridor_id_from_uuid_roundtrip() {
        let uuid = Uuid::new_v4();
        let cid = CorridorId::from_uuid(uuid);
        assert_eq!(*cid.as_uuid(), uuid);
    }
}
