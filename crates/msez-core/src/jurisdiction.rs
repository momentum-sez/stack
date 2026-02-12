//! # Jurisdiction Types
//!
//! Defines jurisdiction configuration types for SEZ zone identification
//! and multi-jurisdiction composition.
//!
//! The `JurisdictionId` type itself lives in [`crate::identity`] alongside
//! other validated identity newtypes. This module provides higher-level
//! configuration structures.
//!
//! ## Implements
//!
//! Spec §3 — Jurisdiction model and zone configuration.

use serde::{Deserialize, Serialize};

pub use crate::identity::JurisdictionId;

/// Configuration for a jurisdiction, loaded from zone YAML files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionConfig {
    /// The jurisdiction identifier.
    pub id: JurisdictionId,
    /// Human-readable name.
    pub name: String,
}
