//! # Jurisdiction Types
//!
//! Defines `JurisdictionId` and associated configuration types for
//! SEZ zone identification and multi-jurisdiction composition.
//!
//! ## Implements
//!
//! Spec §3 — Jurisdiction model and zone configuration.

use serde::{Deserialize, Serialize};

/// Unique identifier for a jurisdiction (SEZ zone).
///
/// Format follows ISO 3166 country codes extended with zone suffixes,
/// e.g., "PK" for Pakistan, "PK-PSEZ" for Pakistan Special Economic Zone.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JurisdictionId(pub String);

impl JurisdictionId {
    /// Create a new jurisdiction identifier.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
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

/// Configuration for a jurisdiction, loaded from zone YAML files.
///
/// Placeholder — full fields to be populated during implementation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionConfig {
    /// The jurisdiction identifier.
    pub id: JurisdictionId,
    /// Human-readable name.
    pub name: String,
}
