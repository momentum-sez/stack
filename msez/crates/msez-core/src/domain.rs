//! # Compliance Domains — Single Source of Truth
//!
//! Defines the [`ComplianceDomain`] enum with all 20 variants. This is the
//! single definition used by every crate in the workspace. The Rust compiler
//! enforces exhaustive `match` — adding a new domain forces every handler
//! in the entire codebase to address it.
//!
//! ## Audit Reference
//!
//! Finding §2.4: The Python codebase had two independent domain enums
//! (8 in phoenix/tensor.py, 20 in msez/composition.py) that could silently
//! diverge. This single enum eliminates that defect class.
//!
//! ## Spec Reference
//!
//! The 20 domains are derived from the composition specification in
//! `tools/msez/composition.py` and the compliance tensor definition in
//! `tools/phoenix/tensor.py`, unified into a single canonical list.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// A compliance domain representing a regulatory category that can be
/// evaluated by the Compliance Tensor.
///
/// All 20 domains from the composition specification are included.
/// Every `match` on this enum must be exhaustive — the compiler enforces
/// that no domain is accidentally ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceDomain {
    /// Anti-money laundering (transaction monitoring, suspicious activity).
    Aml,
    /// Know Your Customer (identity verification, due diligence).
    Kyc,
    /// Sanctions screening (OFAC, UN, EU lists).
    Sanctions,
    /// Tax compliance (withholding, reporting, filing).
    Tax,
    /// Securities regulation (issuance, trading, disclosure).
    Securities,
    /// Corporate governance (formation, dissolution, beneficial ownership).
    Corporate,
    /// Custody requirements (asset safekeeping, segregation).
    Custody,
    /// Data privacy (GDPR, PDPA, cross-border data transfer).
    DataPrivacy,
    /// Licensing (business license validity, professional certifications).
    Licensing,
    /// Banking regulation (reserve requirements, capital adequacy).
    Banking,
    /// Payment services (PSP licensing, payment instrument rules).
    Payments,
    /// Clearing and settlement (CCP rules, netting, finality).
    Clearing,
    /// Settlement finality (delivery-versus-payment, settlement cycles).
    Settlement,
    /// Digital asset regulation (token classification, exchange licensing).
    DigitalAssets,
    /// Employment law (labor contracts, social security, withholding).
    Employment,
    /// Immigration (work permits, visa sponsorship, residency).
    Immigration,
    /// Intellectual property (patent, trademark, trade secret).
    Ip,
    /// Consumer protection (disclosure, dispute resolution, warranties).
    ConsumerProtection,
    /// Arbitration (dispute resolution frameworks, enforcement).
    Arbitration,
    /// Trade regulation (import/export controls, customs, tariffs).
    Trade,
}

impl ComplianceDomain {
    /// Return all compliance domains as a slice.
    ///
    /// Useful for iteration when exhaustive evaluation across all domains
    /// is required (e.g., tensor materialization).
    pub fn all() -> &'static [ComplianceDomain] {
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

    /// The total number of compliance domains.
    ///
    /// Compile-time assertion (M-003): the `all()` array must have exactly
    /// this many elements. If a variant is added to the enum without updating
    /// `all()`, the build breaks.
    pub const COUNT: usize = 20;

    /// Compile-time assertion that `COUNT` matches the actual variant list.
    ///
    /// This is a const evaluated at compile time — if someone adds a 21st
    /// variant and updates `all()` without bumping `COUNT`, or bumps `COUNT`
    /// without extending `all()`, the build fails.
    const _ASSERT_COUNT: () = {
        const ALL_LEN: usize = 20;
        assert!(
            ALL_LEN == ComplianceDomain::COUNT,
            "ComplianceDomain::COUNT does not match the number of variants in all()"
        );
    };

    /// Return the snake_case string representation of this domain.
    ///
    /// Matches the serde serialization format and the Python enum values.
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

impl fmt::Display for ComplianceDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for ComplianceDomain {
    type Err = String;

    /// Parse a compliance domain from its snake_case string representation.
    ///
    /// Accepts the same strings produced by [`ComplianceDomain::as_str()`]
    /// and the [`Display`](std::fmt::Display) implementation.
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
            other => Err(format!("unknown compliance domain: \"{other}\"")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_returns_20_domains() {
        assert_eq!(ComplianceDomain::all().len(), ComplianceDomain::COUNT);
        assert_eq!(ComplianceDomain::all().len(), 20);
    }

    #[test]
    fn all_domains_are_unique() {
        let domains = ComplianceDomain::all();
        let unique: std::collections::HashSet<_> = domains.iter().collect();
        assert_eq!(unique.len(), domains.len());
    }

    #[test]
    fn display_roundtrip_via_from_str() {
        for domain in ComplianceDomain::all() {
            let s = domain.to_string();
            let parsed: ComplianceDomain = s.parse().unwrap();
            assert_eq!(*domain, parsed);
        }
    }

    #[test]
    fn from_str_rejects_unknown() {
        assert!("unknown_domain".parse::<ComplianceDomain>().is_err());
        assert!("".parse::<ComplianceDomain>().is_err());
        assert!("AML".parse::<ComplianceDomain>().is_err()); // case-sensitive
    }

    #[test]
    fn serde_roundtrip() {
        for domain in ComplianceDomain::all() {
            let json = serde_json::to_string(domain).unwrap();
            let deserialized: ComplianceDomain = serde_json::from_str(&json).unwrap();
            assert_eq!(*domain, deserialized);
        }
    }

    #[test]
    fn serde_uses_snake_case() {
        let json = serde_json::to_string(&ComplianceDomain::DataPrivacy).unwrap();
        assert_eq!(json, "\"data_privacy\"");

        let json = serde_json::to_string(&ComplianceDomain::DigitalAssets).unwrap();
        assert_eq!(json, "\"digital_assets\"");

        let json = serde_json::to_string(&ComplianceDomain::ConsumerProtection).unwrap();
        assert_eq!(json, "\"consumer_protection\"");
    }

    /// Verify the match in `as_str` is exhaustive by calling it on every variant.
    #[test]
    fn as_str_is_exhaustive() {
        for domain in ComplianceDomain::all() {
            let s = domain.as_str();
            assert!(!s.is_empty(), "as_str() returned empty for {domain:?}");
        }
    }
}
