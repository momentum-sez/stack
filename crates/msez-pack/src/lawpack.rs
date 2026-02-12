//! # Lawpack — Statute to Machine-Readable Rules
//!
//! Translates legislative statutes (e.g., Income Tax Ordinance 2001,
//! Sales Tax Act 1990) into machine-readable compliance rules with
//! content-addressed artifact generation.
//!
//! ## Implements
//!
//! Spec §10 — Lawpack structure, compilation, and canonicalization.

use serde::{Deserialize, Serialize};

/// A compiled lawpack bundle containing machine-readable rules
/// derived from legislative statutes.
///
/// Placeholder — full implementation will include rule sets,
/// effective dates, supersession chains, and content-addressed digests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lawpack {
    /// Human-readable name of the lawpack.
    pub name: String,
    /// Jurisdiction this lawpack applies to.
    pub jurisdiction: String,
    /// Version of the lawpack.
    pub version: String,
}
