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

use serde::{Deserialize, Serialize};

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
    pub const COUNT: usize = 20;
}

impl std::fmt::Display for ComplianceDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
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
        };
        write!(f, "{s}")
    }
}
