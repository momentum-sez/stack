//! # Domain Identity Newtypes
//!
//! Newtype wrappers for all domain identifiers in the SEZ Stack.
//! These prevent accidental identifier confusion — you cannot pass
//! an `EntityId` where a `CorridorId` is expected.
//!
//! ## Security Invariant
//!
//! Type-level distinction between identifier namespaces prevents
//! cross-namespace confusion attacks where an attacker substitutes
//! one kind of identifier for another.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for an entity (company, SPV, trust).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub Uuid);

/// Unique identifier for a trade corridor between two jurisdictions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CorridorId(pub Uuid);

/// Unique identifier for a cross-jurisdiction asset migration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MigrationId(pub Uuid);

/// Unique identifier for a corridor watcher node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WatcherId(pub Uuid);

/// Pakistan National Tax Number (NTN).
///
/// First-class identifier type for FBR IRIS integration.
/// Format: 7-digit numeric string with optional suffix.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NTN(pub String);

/// Pakistan Computerized National Identity Card number (CNIC).
///
/// First-class identifier type for NADRA integration.
/// Format: 13-digit numeric string (XXXXX-XXXXXXX-X).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CNIC(pub String);

/// Passport number for KYC/KYB verification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PassportNumber(pub String);

impl EntityId {
    /// Generate a new random entity identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Access the inner UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl CorridorId {
    /// Generate a new random corridor identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Access the inner UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl MigrationId {
    /// Generate a new random migration identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Access the inner UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl WatcherId {
    /// Generate a new random watcher identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Access the inner UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "entity:{}", self.0)
    }
}

impl std::fmt::Display for CorridorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "corridor:{}", self.0)
    }
}

impl std::fmt::Display for MigrationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "migration:{}", self.0)
    }
}

impl std::fmt::Display for WatcherId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "watcher:{}", self.0)
    }
}
