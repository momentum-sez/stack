//! # Compliance Domain — Single Source of Truth
//!
//! Defines the `ComplianceDomain` enum with all 20 regulatory domains.
//! This is the ONE definition used across the entire stack. Every `match`
//! on `ComplianceDomain` must be exhaustive — adding a new domain forces
//! every consumer to handle it at compile time.
//!
//! ## Security Invariant
//!
//! A single enum prevents the domain mismatch defect (audit §2.4) where
//! the tensor defined 8 domains and the composition module defined 20.
//! Rust's exhaustive match requirement makes silent domain omission impossible.
//!
//! ## Implements
//!
//! Spec §12 — Compliance evaluation domain taxonomy.

use serde::{Deserialize, Serialize};

/// All regulatory compliance domains in the SEZ Stack.
///
/// Each domain represents a distinct regulatory concern that must be
/// independently evaluated for every entity, transaction, and cross-border
/// corridor operation. The compliance tensor materializes a vector over
/// these domains for each jurisdiction.
///
/// Mathematical definition: D = {d_1, d_2, ..., d_20} where each d_i
/// represents an independent compliance axis in the regulatory state space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceDomain {
    /// Anti-money laundering (transaction monitoring, STR filing).
    Aml,
    /// Know Your Customer (identity verification, CDD, EDD).
    Kyc,
    /// Sanctions screening (OFAC, UN, EU, national lists).
    Sanctions,
    /// Tax compliance (withholding, reporting, filing obligations).
    Tax,
    /// Securities regulation (issuance, trading, disclosure).
    Securities,
    /// Corporate governance (board composition, beneficial ownership, filings).
    Corporate,
    /// Custody requirements (asset safekeeping, segregation).
    Custody,
    /// Data privacy (GDPR, national data protection statutes).
    DataPrivacy,
    /// Licensing (business license validity, professional certifications).
    Licensing,
    /// Banking regulation (capital adequacy, reserve requirements).
    Banking,
    /// Payment services (PSP licensing, settlement rules).
    Payments,
    /// Clearing and settlement (CCP rules, netting, finality).
    Clearing,
    /// Settlement finality and delivery-vs-payment rules.
    Settlement,
    /// Digital asset regulation (crypto licensing, token classification).
    DigitalAssets,
    /// Employment law (labor contracts, benefits, termination).
    Employment,
    /// Immigration (work permits, visa sponsorship, residency).
    Immigration,
    /// Intellectual property (patent, trademark, copyright, trade secret).
    Ip,
    /// Consumer protection (disclosure, cooling-off periods, dispute resolution).
    ConsumerProtection,
    /// Arbitration frameworks (institutional rules, enforcement conventions).
    Arbitration,
    /// Trade regulation (import/export controls, tariffs, customs).
    Trade,
}

impl ComplianceDomain {
    /// Returns all 20 compliance domains.
    pub fn all_domains() -> &'static [ComplianceDomain] {
        &[
            Self::Aml,
            Self::Kyc,
            Self::Sanctions,
            Self::Tax,
            Self::Securities,
            Self::Corporate,
            Self::Custody,
            Self::DataPrivacy,
            Self::Licensing,
            Self::Banking,
            Self::Payments,
            Self::Clearing,
            Self::Settlement,
            Self::DigitalAssets,
            Self::Employment,
            Self::Immigration,
            Self::Ip,
            Self::ConsumerProtection,
            Self::Arbitration,
            Self::Trade,
        ]
    }

    /// Returns the string identifier for this domain.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Aml => "aml",
            Self::Kyc => "kyc",
            Self::Sanctions => "sanctions",
            Self::Tax => "tax",
            Self::Securities => "securities",
            Self::Corporate => "corporate",
            Self::Custody => "custody",
            Self::DataPrivacy => "data_privacy",
            Self::Licensing => "licensing",
            Self::Banking => "banking",
            Self::Payments => "payments",
            Self::Clearing => "clearing",
            Self::Settlement => "settlement",
            Self::DigitalAssets => "digital_assets",
            Self::Employment => "employment",
            Self::Immigration => "immigration",
            Self::Ip => "ip",
            Self::ConsumerProtection => "consumer_protection",
            Self::Arbitration => "arbitration",
            Self::Trade => "trade",
        }
    }
}

impl std::fmt::Display for ComplianceDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_domains_count() {
        assert_eq!(ComplianceDomain::all_domains().len(), 20);
    }
}
