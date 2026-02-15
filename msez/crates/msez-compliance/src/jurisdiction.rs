//! # Regpack-Driven Jurisdiction Configuration
//!
//! Implements `JurisdictionConfig` by reading applicable compliance domains
//! from a compiled regpack. This replaces `DefaultJurisdiction` (which treats
//! all 20 domains as applicable everywhere) with jurisdiction-specific
//! domain scoping.

use msez_core::{ComplianceDomain, JurisdictionId};
use msez_tensor::JurisdictionConfig;

/// A jurisdiction configured by a regpack.
///
/// The regpack's metadata declares which compliance domains are applicable
/// in this jurisdiction. Domains not covered by the regpack are initialized
/// as `NotApplicable` in the compliance tensor, correctly narrowing the
/// evaluation scope.
///
/// ## Example
///
/// A Pakistani SEZ regpack covering AML, KYC, SANCTIONS, TAX, LICENSING,
/// CORPORATE, and TRADE produces a jurisdiction where only those 7 domains
/// are evaluated. The remaining 13 domains (CUSTODY, DATA_PRIVACY, BANKING,
/// PAYMENTS, CLEARING, SETTLEMENT, DIGITAL_ASSETS, EMPLOYMENT, IMMIGRATION,
/// IP, CONSUMER_PROTECTION, ARBITRATION, SECURITIES) are `NotApplicable`.
#[derive(Debug, Clone)]
pub struct RegpackJurisdiction {
    id: JurisdictionId,
    applicable: Vec<ComplianceDomain>,
}

impl RegpackJurisdiction {
    /// Create a jurisdiction from a regpack's declared domain coverage.
    ///
    /// `domain_names` is the list of domain strings from the regpack metadata
    /// (e.g., `["aml", "kyc", "sanctions", "tax"]`). Each string is parsed
    /// into a `ComplianceDomain`. Unrecognized domains are logged as warnings
    /// and skipped — they do not cause failure.
    pub fn from_domain_names(id: JurisdictionId, domain_names: &[String]) -> Self {
        let mut applicable = Vec::new();
        for name in domain_names {
            match name.parse::<ComplianceDomain>() {
                Ok(domain) => applicable.push(domain),
                Err(_) => {
                    tracing::warn!(
                        domain = %name,
                        jurisdiction = %id,
                        "regpack references unknown compliance domain — skipping"
                    );
                }
            }
        }

        if applicable.is_empty() {
            tracing::warn!(
                jurisdiction = %id,
                "regpack declares no recognized compliance domains — \
                 falling back to all 20 domains"
            );
            applicable = ComplianceDomain::all().to_vec();
        }

        Self { id, applicable }
    }

    /// Create a jurisdiction directly from a slice of domains.
    /// Useful in tests and when the domain set is already resolved.
    pub fn from_domains(id: JurisdictionId, domains: Vec<ComplianceDomain>) -> Self {
        Self {
            id,
            applicable: domains,
        }
    }

    /// Return the number of applicable domains.
    pub fn domain_count(&self) -> usize {
        self.applicable.len()
    }
}

impl JurisdictionConfig for RegpackJurisdiction {
    fn jurisdiction_id(&self) -> &JurisdictionId {
        &self.id
    }

    fn applicable_domains(&self) -> &[ComplianceDomain] {
        &self.applicable
    }
}
