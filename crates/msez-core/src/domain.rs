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
use std::str::FromStr;

use crate::error::MsezError;

/// All regulatory compliance domains in the SEZ Stack.
///
/// Each domain represents a distinct regulatory concern that must be
/// independently evaluated for every entity, transaction, and cross-border
/// corridor operation. The compliance tensor materializes a vector over
/// these domains for each jurisdiction.
///
/// Mathematical definition: D = {d_1, d_2, ..., d_20} where each d_i
/// represents an independent compliance axis in the regulatory state space.
///
/// # Domains
///
/// | # | Domain | Description |
/// |---|--------|-------------|
/// |  1 | AML | Anti-money laundering |
/// |  2 | KYC | Know Your Customer |
/// |  3 | Sanctions | OFAC, UN, EU screening |
/// |  4 | Tax | Withholding, reporting |
/// |  5 | Securities | Issuance, trading, disclosure |
/// |  6 | Corporate | Governance, beneficial ownership |
/// |  7 | Custody | Asset safekeeping, segregation |
/// |  8 | DataPrivacy | GDPR, national data protection |
/// |  9 | Licensing | Business license, certifications |
/// | 10 | Banking | Capital adequacy, reserves |
/// | 11 | Payments | PSP licensing, settlement |
/// | 12 | Clearing | CCP rules, netting, finality |
/// | 13 | Settlement | DVP rules |
/// | 14 | DigitalAssets | Crypto licensing, tokens |
/// | 15 | Employment | Labor contracts, benefits |
/// | 16 | Immigration | Work permits, visa sponsorship |
/// | 17 | Ip | Patents, trademarks, copyrights |
/// | 18 | ConsumerProtection | Disclosure, dispute resolution |
/// | 19 | Arbitration | Institutional rules, enforcement |
/// | 20 | Trade | Import/export, tariffs, customs |
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

/// Total number of compliance domains. Used for compile-time assertions.
pub const COMPLIANCE_DOMAIN_COUNT: usize = 20;

impl ComplianceDomain {
    /// Returns all 20 compliance domains in canonical order.
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

    /// Returns the snake_case string identifier for this domain.
    ///
    /// This must match the serde serialization format and the Python
    /// `ComplianceDomain` enum values in `tools/phoenix/tensor.py`.
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

impl FromStr for ComplianceDomain {
    type Err = MsezError;

    /// Parse a compliance domain from its snake_case string identifier.
    ///
    /// Accepts the same identifiers produced by [`ComplianceDomain::as_str()`].
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "aml" => Ok(Self::Aml),
            "kyc" => Ok(Self::Kyc),
            "sanctions" => Ok(Self::Sanctions),
            "tax" => Ok(Self::Tax),
            "securities" => Ok(Self::Securities),
            "corporate" => Ok(Self::Corporate),
            "custody" => Ok(Self::Custody),
            "data_privacy" => Ok(Self::DataPrivacy),
            "licensing" => Ok(Self::Licensing),
            "banking" => Ok(Self::Banking),
            "payments" => Ok(Self::Payments),
            "clearing" => Ok(Self::Clearing),
            "settlement" => Ok(Self::Settlement),
            "digital_assets" => Ok(Self::DigitalAssets),
            "employment" => Ok(Self::Employment),
            "immigration" => Ok(Self::Immigration),
            "ip" => Ok(Self::Ip),
            "consumer_protection" => Ok(Self::ConsumerProtection),
            "arbitration" => Ok(Self::Arbitration),
            "trade" => Ok(Self::Trade),
            other => Err(MsezError::SchemaValidation(format!(
                "unknown compliance domain: {other:?}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_domains_count() {
        assert_eq!(ComplianceDomain::all_domains().len(), COMPLIANCE_DOMAIN_COUNT);
        assert_eq!(ComplianceDomain::all_domains().len(), 20);
    }

    #[test]
    fn test_all_domains_unique() {
        let domains = ComplianceDomain::all_domains();
        let mut seen = std::collections::HashSet::new();
        for d in domains {
            assert!(seen.insert(d), "Duplicate domain: {d}");
        }
    }

    #[test]
    fn test_as_str_roundtrip() {
        for domain in ComplianceDomain::all_domains() {
            let s = domain.as_str();
            let parsed: ComplianceDomain = s.parse().unwrap_or_else(|e| {
                panic!("Failed to parse {s:?}: {e}")
            });
            assert_eq!(*domain, parsed);
        }
    }

    #[test]
    fn test_from_str_invalid() {
        assert!("nonexistent".parse::<ComplianceDomain>().is_err());
        assert!("AML".parse::<ComplianceDomain>().is_err()); // case-sensitive
        assert!("".parse::<ComplianceDomain>().is_err());
    }

    #[test]
    fn test_serde_roundtrip() {
        for domain in ComplianceDomain::all_domains() {
            let json = serde_json::to_string(domain).unwrap();
            let parsed: ComplianceDomain = serde_json::from_str(&json).unwrap();
            assert_eq!(*domain, parsed);
        }
    }

    #[test]
    fn test_serde_format_matches_as_str() {
        for domain in ComplianceDomain::all_domains() {
            let json = serde_json::to_string(domain).unwrap();
            let expected = format!("\"{}\"", domain.as_str());
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn test_display_matches_as_str() {
        for domain in ComplianceDomain::all_domains() {
            assert_eq!(domain.to_string(), domain.as_str());
        }
    }

    #[test]
    fn test_exhaustive_match_compiles() {
        // This test ensures that adding a new domain variant causes a
        // compile error here, forcing the developer to update all match arms.
        fn domain_description(d: &ComplianceDomain) -> &'static str {
            match d {
                ComplianceDomain::Aml => "Anti-money laundering",
                ComplianceDomain::Kyc => "Know your customer",
                ComplianceDomain::Sanctions => "Sanctions screening",
                ComplianceDomain::Tax => "Tax compliance",
                ComplianceDomain::Securities => "Securities regulation",
                ComplianceDomain::Corporate => "Corporate governance",
                ComplianceDomain::Custody => "Custody requirements",
                ComplianceDomain::DataPrivacy => "Data privacy",
                ComplianceDomain::Licensing => "Licensing",
                ComplianceDomain::Banking => "Banking regulation",
                ComplianceDomain::Payments => "Payment services",
                ComplianceDomain::Clearing => "Clearing rules",
                ComplianceDomain::Settlement => "Settlement finality",
                ComplianceDomain::DigitalAssets => "Digital assets",
                ComplianceDomain::Employment => "Employment law",
                ComplianceDomain::Immigration => "Immigration",
                ComplianceDomain::Ip => "Intellectual property",
                ComplianceDomain::ConsumerProtection => "Consumer protection",
                ComplianceDomain::Arbitration => "Arbitration",
                ComplianceDomain::Trade => "Trade regulation",
            }
        }
        for d in ComplianceDomain::all_domains() {
            assert!(!domain_description(d).is_empty());
        }
    }
}
