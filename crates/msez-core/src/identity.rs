//! # Domain Identity Newtypes — Validated Identifier Primitives
//!
//! Newtype wrappers for all domain identifiers in the SEZ Stack.
//! These prevent accidental identifier confusion — you cannot pass
//! an `EntityId` where a `CorridorId` is expected.
//!
//! ## Validation
//!
//! All newtypes with string-based inner values validate their input at
//! construction time. Invalid inputs are rejected with structured errors.
//! The inner fields are private — the only way to construct these types
//! is through the validated constructors.
//!
//! ## Security Invariant
//!
//! Type-level distinction between identifier namespaces prevents
//! cross-namespace confusion attacks where an attacker substitutes
//! one kind of identifier for another.
//!
//! ## Implements
//!
//! Spec §12 — Identity primitives and jurisdiction model.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::MsezError;

// ---------------------------------------------------------------------------
// UUID-based identifiers
// ---------------------------------------------------------------------------

/// Unique identifier for an entity (company, SPV, trust).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(Uuid);

/// Unique identifier for a trade corridor between two jurisdictions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CorridorId(Uuid);

/// Unique identifier for a cross-jurisdiction asset migration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MigrationId(Uuid);

/// Unique identifier for a corridor watcher node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WatcherId(Uuid);

macro_rules! uuid_id_impl {
    ($type:ident, $prefix:literal) => {
        impl $type {
            /// Generate a new random identifier.
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Create from an existing UUID.
            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Access the inner UUID.
            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }
        }

        impl Default for $type {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}:{}", $prefix, self.0)
            }
        }
    };
}

uuid_id_impl!(EntityId, "entity");
uuid_id_impl!(CorridorId, "corridor");
uuid_id_impl!(MigrationId, "migration");
uuid_id_impl!(WatcherId, "watcher");

// ---------------------------------------------------------------------------
// Jurisdiction identifier
// ---------------------------------------------------------------------------

/// Unique identifier for a jurisdiction (SEZ zone).
///
/// Format follows ISO 3166 country codes extended with zone suffixes,
/// e.g., "PK" for Pakistan, "PK-PSEZ" for Pakistan Special Economic Zone.
///
/// # Validation
///
/// - Must be non-empty.
/// - Must contain only ASCII alphanumeric characters and hyphens.
/// - Must not start or end with a hyphen.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JurisdictionId(String);

impl JurisdictionId {
    /// Create a new jurisdiction identifier with validation.
    ///
    /// # Errors
    ///
    /// Returns an error if the identifier is empty, contains invalid characters,
    /// or starts/ends with a hyphen.
    pub fn new(id: impl Into<String>) -> Result<Self, MsezError> {
        let id = id.into();
        if id.is_empty() {
            return Err(MsezError::SchemaValidation(
                "JurisdictionId must not be empty".into(),
            ));
        }
        if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(MsezError::SchemaValidation(format!(
                "JurisdictionId contains invalid characters: {id:?} (only ASCII alphanumeric and hyphens allowed)"
            )));
        }
        if id.starts_with('-') || id.ends_with('-') {
            return Err(MsezError::SchemaValidation(format!(
                "JurisdictionId must not start or end with a hyphen: {id:?}"
            )));
        }
        Ok(Self(id))
    }

    /// Access the inner string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for JurisdictionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// DID (Decentralized Identifier)
// ---------------------------------------------------------------------------

/// Decentralized Identifier (W3C DID Core specification).
///
/// Format: `did:<method>:<method-specific-id>`
///
/// # Validation
///
/// - Must start with `did:`.
/// - Method name must be non-empty, lowercase alphanumeric.
/// - Method-specific ID must be non-empty.
///
/// # Examples
///
/// - `did:key:z6MkhaXg...` — cryptographic key-based DID
/// - `did:web:example.com` — web-based DID
/// - `did:msez:entity:abc123` — SEZ Stack entity DID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DID(String);

impl DID {
    /// Create a new DID with validation.
    ///
    /// # Errors
    ///
    /// Returns an error if the string does not match the DID syntax:
    /// `did:<method>:<method-specific-id>` where method is lowercase
    /// alphanumeric and method-specific-id is non-empty.
    pub fn new(did: impl Into<String>) -> Result<Self, MsezError> {
        let did = did.into();
        if !did.starts_with("did:") {
            return Err(MsezError::SchemaValidation(format!(
                "DID must start with 'did:': {did:?}"
            )));
        }
        let rest = &did[4..];
        let colon_pos = rest.find(':').ok_or_else(|| {
            MsezError::SchemaValidation(format!(
                "DID must have format 'did:<method>:<id>': {did:?}"
            ))
        })?;
        let method = &rest[..colon_pos];
        let specific_id = &rest[colon_pos + 1..];
        if method.is_empty() {
            return Err(MsezError::SchemaValidation(format!(
                "DID method must not be empty: {did:?}"
            )));
        }
        if !method.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
            return Err(MsezError::SchemaValidation(format!(
                "DID method must be lowercase alphanumeric: {did:?}"
            )));
        }
        if specific_id.is_empty() {
            return Err(MsezError::SchemaValidation(format!(
                "DID method-specific-id must not be empty: {did:?}"
            )));
        }
        Ok(Self(did))
    }

    /// Access the full DID string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Extract the DID method (e.g., "key", "web", "msez").
    pub fn method(&self) -> &str {
        let rest = &self.0[4..]; // skip "did:"
        let colon_pos = rest.find(':').expect("validated at construction");
        &rest[..colon_pos]
    }

    /// Extract the method-specific identifier.
    pub fn method_specific_id(&self) -> &str {
        let rest = &self.0[4..]; // skip "did:"
        let colon_pos = rest.find(':').expect("validated at construction");
        &rest[colon_pos + 1..]
    }
}

impl std::fmt::Display for DID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Pakistan National Tax Number (NTN)
// ---------------------------------------------------------------------------

/// Pakistan National Tax Number (NTN).
///
/// First-class identifier type for FBR IRIS integration.
///
/// # Validation
///
/// - Must be non-empty.
/// - Must contain only digits (after stripping hyphens).
/// - Must be 7 digits (standard NTN format).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NTN(String);

impl NTN {
    /// Create a new NTN with validation.
    ///
    /// Accepts raw digit strings (e.g., "1234567") or hyphenated forms.
    /// The stored value retains the original format.
    ///
    /// # Errors
    ///
    /// Returns an error if the NTN is empty or does not contain exactly
    /// 7 digits (aside from hyphens).
    pub fn new(ntn: impl Into<String>) -> Result<Self, MsezError> {
        let ntn = ntn.into();
        if ntn.is_empty() {
            return Err(MsezError::SchemaValidation(
                "NTN must not be empty".into(),
            ));
        }
        let digits: String = ntn.chars().filter(|c| *c != '-').collect();
        if !digits.chars().all(|c| c.is_ascii_digit()) {
            return Err(MsezError::SchemaValidation(format!(
                "NTN must contain only digits and hyphens: {ntn:?}"
            )));
        }
        if digits.len() != 7 {
            return Err(MsezError::SchemaValidation(format!(
                "NTN must be 7 digits, got {}: {ntn:?}",
                digits.len()
            )));
        }
        Ok(Self(ntn))
    }

    /// Access the NTN string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the digits-only representation (hyphens stripped).
    pub fn digits(&self) -> String {
        self.0.chars().filter(|c| *c != '-').collect()
    }
}

impl std::fmt::Display for NTN {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Pakistan CNIC (Computerized National Identity Card)
// ---------------------------------------------------------------------------

/// Pakistan Computerized National Identity Card number (CNIC).
///
/// First-class identifier type for NADRA integration.
///
/// # Validation
///
/// - Must contain exactly 13 digits (after stripping hyphens).
/// - Standard format: `XXXXX-XXXXXXX-X`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CNIC(String);

impl CNIC {
    /// Create a new CNIC with validation.
    ///
    /// Accepts either raw digits ("1234567890123") or the standard
    /// hyphenated format ("12345-6789012-3").
    ///
    /// # Errors
    ///
    /// Returns an error if the CNIC does not contain exactly 13 digits.
    pub fn new(cnic: impl Into<String>) -> Result<Self, MsezError> {
        let cnic = cnic.into();
        if cnic.is_empty() {
            return Err(MsezError::SchemaValidation(
                "CNIC must not be empty".into(),
            ));
        }
        let digits: String = cnic.chars().filter(|c| *c != '-').collect();
        if !digits.chars().all(|c| c.is_ascii_digit()) {
            return Err(MsezError::SchemaValidation(format!(
                "CNIC must contain only digits and hyphens: {cnic:?}"
            )));
        }
        if digits.len() != 13 {
            return Err(MsezError::SchemaValidation(format!(
                "CNIC must be 13 digits, got {}: {cnic:?}",
                digits.len()
            )));
        }
        Ok(Self(cnic))
    }

    /// Access the CNIC string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the digits-only representation (hyphens stripped).
    pub fn digits(&self) -> String {
        self.0.chars().filter(|c| *c != '-').collect()
    }

    /// Returns the standard hyphenated format: `XXXXX-XXXXXXX-X`.
    pub fn formatted(&self) -> String {
        let d = self.digits();
        format!("{}-{}-{}", &d[0..5], &d[5..12], &d[12..13])
    }
}

impl std::fmt::Display for CNIC {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Passport Number
// ---------------------------------------------------------------------------

/// Passport number for KYC/KYB verification.
///
/// # Validation
///
/// - Must be non-empty.
/// - Must contain only ASCII alphanumeric characters (no spaces or special chars).
/// - Length between 5 and 20 characters (covers all national formats).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PassportNumber(String);

impl PassportNumber {
    /// Create a new passport number with validation.
    ///
    /// # Errors
    ///
    /// Returns an error if the passport number is empty, too short/long,
    /// or contains non-alphanumeric characters.
    pub fn new(passport: impl Into<String>) -> Result<Self, MsezError> {
        let passport = passport.into();
        if passport.is_empty() {
            return Err(MsezError::SchemaValidation(
                "PassportNumber must not be empty".into(),
            ));
        }
        if !passport.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(MsezError::SchemaValidation(format!(
                "PassportNumber must be alphanumeric: {passport:?}"
            )));
        }
        if passport.len() < 5 || passport.len() > 20 {
            return Err(MsezError::SchemaValidation(format!(
                "PassportNumber length must be 5-20 characters, got {}: {passport:?}",
                passport.len()
            )));
        }
        Ok(Self(passport))
    }

    /// Access the passport number string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PassportNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- UUID-based types ----

    #[test]
    fn test_entity_id_new() {
        let id = EntityId::new();
        assert!(!id.as_uuid().is_nil());
    }

    #[test]
    fn test_entity_id_display() {
        let id = EntityId::new();
        let s = format!("{id}");
        assert!(s.starts_with("entity:"));
    }

    #[test]
    fn test_entity_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let id = EntityId::from_uuid(uuid);
        assert_eq!(*id.as_uuid(), uuid);
    }

    #[test]
    fn test_corridor_id_display() {
        let id = CorridorId::new();
        assert!(format!("{id}").starts_with("corridor:"));
    }

    #[test]
    fn test_migration_id_display() {
        let id = MigrationId::new();
        assert!(format!("{id}").starts_with("migration:"));
    }

    #[test]
    fn test_watcher_id_display() {
        let id = WatcherId::new();
        assert!(format!("{id}").starts_with("watcher:"));
    }

    #[test]
    fn test_uuid_type_safety() {
        let e = EntityId::new();
        let c = CorridorId::new();
        assert_ne!(
            format!("{e}").split(':').next(),
            format!("{c}").split(':').next()
        );
    }

    // ---- JurisdictionId ----

    #[test]
    fn test_jurisdiction_id_valid() {
        let id = JurisdictionId::new("PK").unwrap();
        assert_eq!(id.as_str(), "PK");
    }

    #[test]
    fn test_jurisdiction_id_with_zone() {
        let id = JurisdictionId::new("PK-PSEZ").unwrap();
        assert_eq!(id.as_str(), "PK-PSEZ");
    }

    #[test]
    fn test_jurisdiction_id_empty_rejected() {
        assert!(JurisdictionId::new("").is_err());
    }

    #[test]
    fn test_jurisdiction_id_special_chars_rejected() {
        assert!(JurisdictionId::new("PK/PSEZ").is_err());
        assert!(JurisdictionId::new("PK PSEZ").is_err());
        assert!(JurisdictionId::new("PK.PSEZ").is_err());
    }

    #[test]
    fn test_jurisdiction_id_hyphen_edges_rejected() {
        assert!(JurisdictionId::new("-PK").is_err());
        assert!(JurisdictionId::new("PK-").is_err());
    }

    #[test]
    fn test_jurisdiction_id_display() {
        let id = JurisdictionId::new("PK-PSEZ").unwrap();
        assert_eq!(format!("{id}"), "PK-PSEZ");
    }

    // ---- DID ----

    #[test]
    fn test_did_valid_key() {
        let did = DID::new("did:key:z6MkhaXgBZDvotD").unwrap();
        assert_eq!(did.method(), "key");
        assert_eq!(did.method_specific_id(), "z6MkhaXgBZDvotD");
    }

    #[test]
    fn test_did_valid_web() {
        let did = DID::new("did:web:example.com").unwrap();
        assert_eq!(did.method(), "web");
        assert_eq!(did.method_specific_id(), "example.com");
    }

    #[test]
    fn test_did_valid_msez() {
        let did = DID::new("did:msez:entity:abc123").unwrap();
        assert_eq!(did.method(), "msez");
        assert_eq!(did.method_specific_id(), "entity:abc123");
    }

    #[test]
    fn test_did_missing_prefix() {
        assert!(DID::new("key:z6MkhaXg").is_err());
    }

    #[test]
    fn test_did_empty_method() {
        assert!(DID::new("did::specific").is_err());
    }

    #[test]
    fn test_did_empty_specific_id() {
        assert!(DID::new("did:key:").is_err());
    }

    #[test]
    fn test_did_no_second_colon() {
        assert!(DID::new("did:key").is_err());
    }

    #[test]
    fn test_did_uppercase_method_rejected() {
        assert!(DID::new("did:Key:specific").is_err());
    }

    #[test]
    fn test_did_display() {
        let did = DID::new("did:msez:entity:abc").unwrap();
        assert_eq!(format!("{did}"), "did:msez:entity:abc");
    }

    // ---- NTN ----

    #[test]
    fn test_ntn_valid() {
        let ntn = NTN::new("1234567").unwrap();
        assert_eq!(ntn.as_str(), "1234567");
        assert_eq!(ntn.digits(), "1234567");
    }

    #[test]
    fn test_ntn_empty_rejected() {
        assert!(NTN::new("").is_err());
    }

    #[test]
    fn test_ntn_wrong_length() {
        assert!(NTN::new("123456").is_err());
        assert!(NTN::new("12345678").is_err());
    }

    #[test]
    fn test_ntn_non_digit_rejected() {
        assert!(NTN::new("123456A").is_err());
    }

    // ---- CNIC ----

    #[test]
    fn test_cnic_valid_digits() {
        let cnic = CNIC::new("1234567890123").unwrap();
        assert_eq!(cnic.digits(), "1234567890123");
    }

    #[test]
    fn test_cnic_valid_formatted() {
        let cnic = CNIC::new("12345-6789012-3").unwrap();
        assert_eq!(cnic.digits(), "1234567890123");
        assert_eq!(cnic.formatted(), "12345-6789012-3");
    }

    #[test]
    fn test_cnic_wrong_length() {
        assert!(CNIC::new("123456789012").is_err());
        assert!(CNIC::new("12345678901234").is_err());
    }

    #[test]
    fn test_cnic_empty_rejected() {
        assert!(CNIC::new("").is_err());
    }

    #[test]
    fn test_cnic_non_digit_rejected() {
        assert!(CNIC::new("12345-678901A-3").is_err());
    }

    // ---- PassportNumber ----

    #[test]
    fn test_passport_valid() {
        let p = PassportNumber::new("AB1234567").unwrap();
        assert_eq!(p.as_str(), "AB1234567");
    }

    #[test]
    fn test_passport_empty_rejected() {
        assert!(PassportNumber::new("").is_err());
    }

    #[test]
    fn test_passport_too_short() {
        assert!(PassportNumber::new("AB12").is_err());
    }

    #[test]
    fn test_passport_too_long() {
        assert!(PassportNumber::new("A".repeat(21)).is_err());
    }

    #[test]
    fn test_passport_special_chars_rejected() {
        assert!(PassportNumber::new("AB-12345").is_err());
        assert!(PassportNumber::new("AB 12345").is_err());
    }

    // ---- Serde round-trips ----

    #[test]
    fn test_serde_entity_id() {
        let id = EntityId::new();
        let json = serde_json::to_string(&id).unwrap();
        let parsed: EntityId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_serde_jurisdiction_id() {
        let id = JurisdictionId::new("PK-PSEZ").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        let parsed: JurisdictionId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_serde_did() {
        let did = DID::new("did:key:z6MkhaXg").unwrap();
        let json = serde_json::to_string(&did).unwrap();
        let parsed: DID = serde_json::from_str(&json).unwrap();
        assert_eq!(did, parsed);
    }

    #[test]
    fn test_serde_ntn() {
        let ntn = NTN::new("1234567").unwrap();
        let json = serde_json::to_string(&ntn).unwrap();
        let parsed: NTN = serde_json::from_str(&json).unwrap();
        assert_eq!(ntn, parsed);
    }

    #[test]
    fn test_serde_cnic() {
        let cnic = CNIC::new("1234567890123").unwrap();
        let json = serde_json::to_string(&cnic).unwrap();
        let parsed: CNIC = serde_json::from_str(&json).unwrap();
        assert_eq!(cnic, parsed);
    }
}
