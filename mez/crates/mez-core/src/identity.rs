//! # Identity Newtypes
//!
//! Domain-primitive newtypes for identifiers throughout the EZ Stack.
//! Each identifier is a distinct type — you cannot pass an [`EntityId`]
//! where a [`WatcherId`] is expected.
//!
//! ## Validation
//!
//! String-based identifiers ([`Did`], [`Cnic`], [`Ntn`], [`PassportNumber`])
//! validate format at construction time. UUID-based identifiers ([`EntityId`],
//! [`MigrationId`], [`WatcherId`]) are always valid by construction.
//!
//! ## Spec Reference
//!
//! - CNIC: Pakistan NADRA Computerized National Identity Card (13 digits)
//! - NTN: Pakistan FBR National Tax Number (7 digits, IRIS integration)
//! - DID: W3C Decentralized Identifier (did:method:identifier)

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ValidationError;

/// Helper macro to implement `Deserialize` for string newtypes that must
/// validate their contents. Deserializes as a plain `String`, then routes
/// through the type's `new()` constructor so that invalid values are
/// rejected at deserialization time — not silently accepted.
macro_rules! impl_validating_deserialize {
    ($ty:ident) => {
        impl<'de> Deserialize<'de> for $ty {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let raw = String::deserialize(deserializer)?;
                Self::new(raw).map_err(serde::de::Error::custom)
            }
        }
    };
}

// ---------------------------------------------------------------------------
// UUID-based identifiers (always valid by construction)
// ---------------------------------------------------------------------------

/// A unique identifier for an entity (company, organization, individual)
/// registered within a Economic Zone.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(Uuid);

impl EntityId {
    /// Create a new random entity identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create an entity identifier from an existing UUID.
    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Access the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for EntityId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for EntityId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::from_str(s).map(Self)
    }
}

/// A unique identifier for a cross-asset migration saga.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MigrationId(Uuid);

impl MigrationId {
    /// Create a new random migration identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a migration identifier from an existing UUID.
    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Access the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for MigrationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MigrationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A unique identifier for a watcher node in the corridor economy.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WatcherId(Uuid);

impl WatcherId {
    /// Create a new random watcher identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a watcher identifier from an existing UUID.
    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Access the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for WatcherId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WatcherId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// String-based identifiers (validated at construction)
// ---------------------------------------------------------------------------

/// W3C Decentralized Identifier (DID).
///
/// Format: `did:<method>:<method-specific-id>`
/// where method is lowercase alphanumeric and method-specific-id is non-empty.
///
/// # Validation
///
/// - Must start with `did:`
/// - Method name must be at least 1 character, lowercase alphanumeric
/// - Must have a `:` separator after method
/// - Method-specific identifier must be non-empty
///
/// Reference: <https://www.w3.org/TR/did-core/#did-syntax>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Did(String);

impl_validating_deserialize!(Did);

impl Did {
    /// Create a DID from a string, validating format.
    ///
    /// # Errors
    ///
    /// Returns [`ValidationError::InvalidDid`] if the string does not
    /// match the `did:method:identifier` format.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let s = value.into();
        Self::validate(&s)?;
        Ok(Self(s))
    }

    /// Validate DID format without constructing.
    fn validate(s: &str) -> Result<(), ValidationError> {
        if !s.starts_with("did:") {
            return Err(ValidationError::InvalidDid(s.to_string()));
        }

        let rest = &s[4..]; // after "did:"
        let colon_pos = rest.find(':');
        match colon_pos {
            None => return Err(ValidationError::InvalidDid(s.to_string())),
            Some(pos) => {
                let method = &rest[..pos];
                let identifier = &rest[pos + 1..];

                // Method must be non-empty and lowercase alphanumeric
                if method.is_empty()
                    || !method
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                {
                    return Err(ValidationError::InvalidDid(s.to_string()));
                }

                // Identifier must be non-empty
                if identifier.is_empty() {
                    return Err(ValidationError::InvalidDid(s.to_string()));
                }
            }
        }

        Ok(())
    }

    /// Access the DID string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Return the DID method (the part between the first and second colons).
    pub fn method(&self) -> &str {
        let rest = &self.0[4..]; // after "did:"
        let colon_pos = rest.find(':').expect("validated at construction");
        &rest[..colon_pos]
    }

    /// Return the method-specific identifier (everything after `did:method:`).
    pub fn method_specific_id(&self) -> &str {
        let rest = &self.0[4..]; // after "did:"
        let colon_pos = rest.find(':').expect("validated at construction");
        &rest[colon_pos + 1..]
    }
}

impl std::fmt::Display for Did {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Pakistan National Tax Number (NTN).
///
/// First-class identifier for FBR IRIS integration. Validated at construction
/// to be exactly 7 digits.
///
/// # Validation
///
/// - Must be exactly 7 digits (0-9)
/// - Leading zeros are significant (e.g., "0012345" is valid)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Ntn(String);

impl_validating_deserialize!(Ntn);

impl Ntn {
    /// Create an NTN from a string value, validating the 7-digit format.
    ///
    /// # Errors
    ///
    /// Returns [`ValidationError::InvalidNtn`] if the string is not exactly
    /// 7 digits.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let s = value.into();
        if s.len() != 7 || !s.chars().all(|c| c.is_ascii_digit()) {
            return Err(ValidationError::InvalidNtn(s));
        }
        Ok(Self(s))
    }

    /// Access the NTN string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Ntn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Pakistan Computerized National Identity Card (CNIC) number.
///
/// First-class identifier for NADRA cross-referencing. The canonical storage
/// format is 13 digits without dashes. The constructor accepts both:
/// - `"1234567890123"` (13 digits)
/// - `"12345-6789012-3"` (formatted with dashes: 5-7-1)
///
/// # Validation
///
/// - Must be exactly 13 digits after stripping dashes
/// - If dashes are present, must follow the 5-7-1 pattern
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Cnic(String);

impl_validating_deserialize!(Cnic);

impl Cnic {
    /// Create a CNIC from a string value, validating format.
    ///
    /// Accepts both `"1234567890123"` and `"12345-6789012-3"` formats.
    /// Stores in the canonical 13-digit format (dashes stripped).
    ///
    /// # Errors
    ///
    /// Returns [`ValidationError::InvalidCnic`] if the format is invalid.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let raw = value.into();
        let digits: String = raw.chars().filter(|c| *c != '-').collect();

        // Must be exactly 13 digits
        if digits.len() != 13 || !digits.chars().all(|c| c.is_ascii_digit()) {
            return Err(ValidationError::InvalidCnic(raw));
        }

        // If dashes were present, validate the pattern is 5-7-1
        if raw.contains('-') {
            let parts: Vec<&str> = raw.split('-').collect();
            if parts.len() != 3 || parts[0].len() != 5 || parts[1].len() != 7 || parts[2].len() != 1
            {
                return Err(ValidationError::InvalidCnic(raw));
            }
        }

        // Store canonical form (digits only)
        Ok(Self(digits))
    }

    /// Access the CNIC in canonical 13-digit format (no dashes).
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Return the CNIC in formatted form: XXXXX-XXXXXXX-X.
    pub fn formatted(&self) -> String {
        format!("{}-{}-{}", &self.0[..5], &self.0[5..12], &self.0[12..])
    }
}

impl std::fmt::Display for Cnic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.formatted())
    }
}

/// Passport number.
///
/// A travel document identifier. Format varies by issuing country, so
/// validation is intentionally lenient: alphanumeric, 5-20 characters.
///
/// # Validation
///
/// - Must be 5-20 characters
/// - Must be alphanumeric (ASCII letters and digits only)
/// - Stored in uppercase form for consistency
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct PassportNumber(String);

impl_validating_deserialize!(PassportNumber);

impl PassportNumber {
    /// Create a passport number, validating format.
    ///
    /// The value is converted to uppercase for storage consistency.
    ///
    /// # Errors
    ///
    /// Returns [`ValidationError::InvalidPassportNumber`] if the format
    /// is invalid.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let s = value.into();
        let upper = s.trim().to_uppercase();

        if upper.len() < 5 || upper.len() > 20 {
            return Err(ValidationError::InvalidPassportNumber(s));
        }
        if !upper.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(ValidationError::InvalidPassportNumber(s));
        }

        Ok(Self(upper))
    }

    /// Access the passport number string (uppercase).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PassportNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- EntityId --

    #[test]
    fn entity_id_unique() {
        let a = EntityId::new();
        let b = EntityId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn entity_id_from_uuid_roundtrip() {
        let uuid = Uuid::new_v4();
        let id = EntityId::from_uuid(uuid);
        assert_eq!(*id.as_uuid(), uuid);
    }

    // -- DID --

    #[test]
    fn did_valid_examples() {
        assert!(Did::new("did:web:example.com").is_ok());
        assert!(Did::new("did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK").is_ok());
        assert!(Did::new("did:ethr:0xb9c5714089478a327f09197987f16f9e5d936e8a").is_ok());
    }

    #[test]
    fn did_method_extraction() {
        let did = Did::new("did:web:example.com").unwrap();
        assert_eq!(did.method(), "web");
        assert_eq!(did.method_specific_id(), "example.com");
    }

    #[test]
    fn did_rejects_invalid() {
        assert!(Did::new("").is_err());
        assert!(Did::new("notadid").is_err());
        assert!(Did::new("did:").is_err());
        assert!(Did::new("did::something").is_err()); // empty method
        assert!(Did::new("did:Web:id").is_err()); // uppercase method
        assert!(Did::new("did:method:").is_err()); // empty identifier
    }

    // -- CNIC --

    #[test]
    fn cnic_valid_13_digits() {
        let cnic = Cnic::new("1234567890123").unwrap();
        assert_eq!(cnic.as_str(), "1234567890123");
    }

    #[test]
    fn cnic_valid_formatted() {
        let cnic = Cnic::new("12345-6789012-3").unwrap();
        assert_eq!(cnic.as_str(), "1234567890123"); // stored without dashes
        assert_eq!(cnic.formatted(), "12345-6789012-3");
    }

    #[test]
    fn cnic_rejects_invalid() {
        assert!(Cnic::new("").is_err());
        assert!(Cnic::new("123456789012").is_err()); // 12 digits
        assert!(Cnic::new("12345678901234").is_err()); // 14 digits
        assert!(Cnic::new("12345-678901-23").is_err()); // wrong dash pattern
        assert!(Cnic::new("1234a67890123").is_err()); // non-digit
    }

    // -- NTN --

    #[test]
    fn ntn_valid() {
        let ntn = Ntn::new("1234567").unwrap();
        assert_eq!(ntn.as_str(), "1234567");
    }

    #[test]
    fn ntn_leading_zeros() {
        let ntn = Ntn::new("0012345").unwrap();
        assert_eq!(ntn.as_str(), "0012345");
    }

    #[test]
    fn ntn_rejects_invalid() {
        assert!(Ntn::new("").is_err());
        assert!(Ntn::new("123456").is_err()); // 6 digits
        assert!(Ntn::new("12345678").is_err()); // 8 digits
        assert!(Ntn::new("123456a").is_err()); // non-digit
    }

    // -- PassportNumber --

    #[test]
    fn passport_valid() {
        let pp = PassportNumber::new("AB123456").unwrap();
        assert_eq!(pp.as_str(), "AB123456");
    }

    #[test]
    fn passport_lowercased_to_upper() {
        let pp = PassportNumber::new("ab123456").unwrap();
        assert_eq!(pp.as_str(), "AB123456");
    }

    #[test]
    fn passport_rejects_invalid() {
        assert!(PassportNumber::new("").is_err());
        assert!(PassportNumber::new("ABCD").is_err()); // too short (4)
        assert!(PassportNumber::new("AB12-3456").is_err()); // non-alphanumeric dash
        assert!(PassportNumber::new("A".repeat(21)).is_err()); // too long
    }

    // ── Coverage expansion tests ─────────────────────────────────────

    // -- EntityId --

    #[test]
    fn entity_id_default() {
        let id1 = EntityId::default();
        let id2 = EntityId::default();
        assert_ne!(id1, id2);
    }

    #[test]
    fn entity_id_display() {
        let id = EntityId::new();
        let display = format!("{id}");
        assert!(!display.is_empty());
        // UUID format: 8-4-4-4-12 = 36 chars
        assert_eq!(display.len(), 36);
    }

    // -- MigrationId --

    #[test]
    fn migration_id_unique() {
        let a = MigrationId::new();
        let b = MigrationId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn migration_id_default() {
        let id1 = MigrationId::default();
        let id2 = MigrationId::default();
        assert_ne!(id1, id2);
    }

    #[test]
    fn migration_id_display() {
        let id = MigrationId::new();
        let display = format!("{id}");
        assert_eq!(display.len(), 36);
    }

    #[test]
    fn migration_id_from_uuid_roundtrip() {
        let uuid = Uuid::new_v4();
        let id = MigrationId::from_uuid(uuid);
        assert_eq!(*id.as_uuid(), uuid);
    }

    // -- WatcherId --

    #[test]
    fn watcher_id_unique() {
        let a = WatcherId::new();
        let b = WatcherId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn watcher_id_default() {
        let id1 = WatcherId::default();
        let id2 = WatcherId::default();
        assert_ne!(id1, id2);
    }

    #[test]
    fn watcher_id_display() {
        let id = WatcherId::new();
        let display = format!("{id}");
        assert_eq!(display.len(), 36);
    }

    #[test]
    fn watcher_id_from_uuid_roundtrip() {
        let uuid = Uuid::new_v4();
        let id = WatcherId::from_uuid(uuid);
        assert_eq!(*id.as_uuid(), uuid);
    }

    // -- Did --

    #[test]
    fn did_display() {
        let did = Did::new("did:web:example.com").unwrap();
        assert_eq!(format!("{did}"), "did:web:example.com");
    }

    #[test]
    fn did_as_str() {
        let did = Did::new("did:key:z6MkTestKey").unwrap();
        assert_eq!(did.as_str(), "did:key:z6MkTestKey");
    }

    #[test]
    fn did_method_extraction_multiple() {
        let did = Did::new("did:ethr:0xabcdef1234567890").unwrap();
        assert_eq!(did.method(), "ethr");
        assert_eq!(did.method_specific_id(), "0xabcdef1234567890");
    }

    #[test]
    fn did_method_with_colons_in_id() {
        // Method-specific ID can contain colons
        let did = Did::new("did:web:example.com:path:to:resource").unwrap();
        assert_eq!(did.method(), "web");
        assert_eq!(did.method_specific_id(), "example.com:path:to:resource");
    }

    // -- Ntn --

    #[test]
    fn ntn_display() {
        let ntn = Ntn::new("1234567").unwrap();
        assert_eq!(format!("{ntn}"), "1234567");
    }

    // -- Cnic --

    #[test]
    fn cnic_display_formatted() {
        let cnic = Cnic::new("1234567890123").unwrap();
        assert_eq!(format!("{cnic}"), "12345-6789012-3");
    }

    #[test]
    fn cnic_formatted_from_digits() {
        let cnic = Cnic::new("1234567890123").unwrap();
        assert_eq!(cnic.formatted(), "12345-6789012-3");
    }

    // -- PassportNumber --

    #[test]
    fn passport_display() {
        let pp = PassportNumber::new("ab123456").unwrap();
        assert_eq!(format!("{pp}"), "AB123456");
    }

    #[test]
    fn passport_boundary_lengths() {
        // Exactly 5 chars (minimum)
        assert!(PassportNumber::new("ABCDE").is_ok());
        // Exactly 20 chars (maximum)
        assert!(PassportNumber::new("A".repeat(20)).is_ok());
    }

    #[test]
    fn passport_with_whitespace() {
        // Leading/trailing whitespace is trimmed
        let pp = PassportNumber::new("  AB123456  ").unwrap();
        assert_eq!(pp.as_str(), "AB123456");
    }

    // -- Serde roundtrips --

    #[test]
    fn entity_id_serde_roundtrip() {
        let id = EntityId::new();
        let json_str = serde_json::to_string(&id).unwrap();
        let deserialized: EntityId = serde_json::from_str(&json_str).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn migration_id_serde_roundtrip() {
        let id = MigrationId::new();
        let json_str = serde_json::to_string(&id).unwrap();
        let deserialized: MigrationId = serde_json::from_str(&json_str).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn watcher_id_serde_roundtrip() {
        let id = WatcherId::new();
        let json_str = serde_json::to_string(&id).unwrap();
        let deserialized: WatcherId = serde_json::from_str(&json_str).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn did_serde_roundtrip() {
        let did = Did::new("did:web:example.com").unwrap();
        let json_str = serde_json::to_string(&did).unwrap();
        let deserialized: Did = serde_json::from_str(&json_str).unwrap();
        assert_eq!(did, deserialized);
    }

    #[test]
    fn ntn_serde_roundtrip() {
        let ntn = Ntn::new("1234567").unwrap();
        let json_str = serde_json::to_string(&ntn).unwrap();
        let deserialized: Ntn = serde_json::from_str(&json_str).unwrap();
        assert_eq!(ntn, deserialized);
    }

    #[test]
    fn cnic_serde_roundtrip() {
        let cnic = Cnic::new("1234567890123").unwrap();
        let json_str = serde_json::to_string(&cnic).unwrap();
        let deserialized: Cnic = serde_json::from_str(&json_str).unwrap();
        assert_eq!(cnic, deserialized);
    }

    #[test]
    fn passport_serde_roundtrip() {
        let pp = PassportNumber::new("AB123456").unwrap();
        let json_str = serde_json::to_string(&pp).unwrap();
        let deserialized: PassportNumber = serde_json::from_str(&json_str).unwrap();
        assert_eq!(pp, deserialized);
    }

    // -- Hash collections --

    #[test]
    fn entity_id_in_hashset() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        let id1 = EntityId::new();
        let id2 = EntityId::new();
        set.insert(id1.clone());
        set.insert(id2);
        assert_eq!(set.len(), 2);
        assert!(set.contains(&id1));
    }

    #[test]
    fn did_in_hashset() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Did::new("did:web:a.com").unwrap());
        set.insert(Did::new("did:web:b.com").unwrap());
        set.insert(Did::new("did:web:a.com").unwrap());
        assert_eq!(set.len(), 2);
    }
}
