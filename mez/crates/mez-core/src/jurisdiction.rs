//! # Jurisdiction & Corridor Identifiers
//!
//! Newtypes for jurisdiction and corridor identifiers. These are the
//! fundamental addressing primitives in the EZ Stack — a jurisdiction
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

// -- Validating Deserialize for JurisdictionId --------------------------------

impl<'de> Deserialize<'de> for JurisdictionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::new(raw).map_err(serde::de::Error::custom)
    }
}

/// A jurisdiction identifier, typically an ISO 3166-1 code or a
/// zone-specific identifier (e.g., "PK-RSEZ" for Pakistan Rashakai EZ).
///
/// # Validation
///
/// Must be a non-empty string. No further format restrictions are imposed
/// because jurisdiction naming varies across EZ configurations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct JurisdictionId(String);

impl JurisdictionId {
    /// Create a jurisdiction identifier from a string, validating non-emptiness.
    ///
    /// # Errors
    ///
    /// Returns [`ValidationError::InvalidJurisdictionId`] if the string is
    /// empty or whitespace-only.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let trimmed = value.into().trim().to_string();
        if trimmed.is_empty() {
            return Err(ValidationError::InvalidJurisdictionId);
        }
        Ok(Self(trimmed))
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

    #[test]
    fn jurisdiction_id_display() {
        let jid = JurisdictionId::new("PK-RSEZ").unwrap();
        assert_eq!(format!("{jid}"), "PK-RSEZ");
    }

    #[test]
    fn jurisdiction_id_serde_roundtrip() {
        let jid = JurisdictionId::new("US-DE").unwrap();
        let json = serde_json::to_string(&jid).unwrap();
        let deser: JurisdictionId = serde_json::from_str(&json).unwrap();
        assert_eq!(jid, deser);
    }

    #[test]
    fn corridor_id_default() {
        let cid = CorridorId::default();
        // default() calls new() which generates a UUID
        assert!(!cid.as_uuid().is_nil());
    }

    #[test]
    fn corridor_id_display() {
        let uuid = Uuid::nil();
        let cid = CorridorId::from_uuid(uuid);
        let display = format!("{cid}");
        assert_eq!(display, "00000000-0000-0000-0000-000000000000");
    }

    #[test]
    fn corridor_id_serde_roundtrip() {
        let cid = CorridorId::new();
        let json = serde_json::to_string(&cid).unwrap();
        let deser: CorridorId = serde_json::from_str(&json).unwrap();
        assert_eq!(cid, deser);
    }

    #[test]
    fn corridor_id_hash_works() {
        use std::collections::HashSet;
        let cid1 = CorridorId::new();
        let cid2 = CorridorId::new();
        let mut set = HashSet::new();
        set.insert(cid1.clone());
        set.insert(cid2.clone());
        assert_eq!(set.len(), 2);
        assert!(set.contains(&cid1));
    }

    #[test]
    fn jurisdiction_id_clone_and_eq() {
        let jid = JurisdictionId::new("SG").unwrap();
        let jid2 = jid.clone();
        assert_eq!(jid, jid2);
    }

    #[test]
    fn jurisdiction_id_hash_works() {
        use std::collections::HashSet;
        let j1 = JurisdictionId::new("PK").unwrap();
        let j2 = JurisdictionId::new("SG").unwrap();
        let mut set = HashSet::new();
        set.insert(j1.clone());
        set.insert(j2);
        assert_eq!(set.len(), 2);
        assert!(set.contains(&j1));
    }
}
