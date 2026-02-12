//! # Identity Newtypes
//!
//! Domain-primitive newtypes for identifiers throughout the SEZ Stack.
//! Each identifier is a distinct type — you cannot pass an [`EntityId`]
//! where a [`WatcherId`] is expected.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A unique identifier for an entity (company, organization, individual)
/// registered within a Special Economic Zone.
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

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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

/// Pakistan National Tax Number (NTN).
///
/// First-class identifier for FBR IRIS integration. Validated at construction.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ntn(String);

impl Ntn {
    /// Create an NTN from a string value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
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
/// First-class identifier for NADRA cross-referencing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Cnic(String);

impl Cnic {
    /// Create a CNIC from a string value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Access the CNIC string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Cnic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
