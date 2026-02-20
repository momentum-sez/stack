//! # Zone Composition Algebra
//!
//! Defines the type system for composing zones from regulatory primitives.
//! A zone is a deployment of composed regulatory domains — corporate formation
//! law, civic code, digital asset regulation, tax regime, AML/CFT framework,
//! arbitration rules — sourced from any jurisdiction that has codified them.
//!
//! ## Zone Types
//!
//! - **Natural**: All regulatory layers sourced from a single root jurisdiction
//!   (e.g., Pakistan SIFC — all layers from `pk`).
//! - **Synthetic**: Layers sourced from multiple jurisdictions, composed into a
//!   novel regulatory environment (e.g., Delaware corporate law + ADGM digital
//!   assets + Singapore tax + Hong Kong arbitration).
//!
//! ## Invariants
//!
//! - At most one layer per [`RegulatoryDomain`].
//! - [`AmlCft`](RegulatoryDomain::AmlCft) layer is mandatory.
//! - For [`Natural`](ZoneType::Natural) zones, all source jurisdictions must
//!   share the same root prefix (first two characters).
//! - No duplicate domains in a composition.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Regulatory Domain
// ---------------------------------------------------------------------------

/// Exhaustive enumeration of regulatory domains that can be composed into a zone.
///
/// Each domain represents a distinct body of law or regulation that governs
/// one aspect of economic activity within a zone.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum RegulatoryDomain {
    /// Corporate formation and governance law (e.g., DGCL Title 8, Companies Act).
    CorporateFormation,
    /// General civil/commercial code (e.g., UCC, Civil Code).
    CivicCode,
    /// Digital asset and virtual asset regulation (e.g., FSMR 2015, MiCA).
    DigitalAssets,
    /// Arbitration and dispute resolution framework (e.g., UNCITRAL, IAA).
    Arbitration,
    /// Tax regime — income tax, VAT/GST, withholding (e.g., ITA, FDL 8/2017).
    Tax,
    /// Anti-money laundering and counter-terrorism financing framework.
    AmlCft,
    /// Data privacy and protection regulation (e.g., PDPA, GDPR).
    DataPrivacy,
    /// Licensing and authorization regime for regulated activities.
    Licensing,
    /// Payment rails and settlement system regulation.
    PaymentRails,
    /// Securities and capital markets regulation (e.g., SFA, Securities Act).
    Securities,
}

impl std::fmt::Display for RegulatoryDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CorporateFormation => write!(f, "corporate_formation"),
            Self::CivicCode => write!(f, "civic_code"),
            Self::DigitalAssets => write!(f, "digital_assets"),
            Self::Arbitration => write!(f, "arbitration"),
            Self::Tax => write!(f, "tax"),
            Self::AmlCft => write!(f, "aml_cft"),
            Self::DataPrivacy => write!(f, "data_privacy"),
            Self::Licensing => write!(f, "licensing"),
            Self::PaymentRails => write!(f, "payment_rails"),
            Self::Securities => write!(f, "securities"),
        }
    }
}

// ---------------------------------------------------------------------------
// Regulatory Layer
// ---------------------------------------------------------------------------

/// A single regulatory layer: one domain sourced from one jurisdiction.
///
/// For example, a layer might be "corporate formation law from Delaware"
/// or "AML/CFT framework from UAE".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegulatoryLayer {
    /// Which regulatory domain this layer covers.
    pub domain: RegulatoryDomain,
    /// The jurisdiction from which this domain's regulation is sourced
    /// (e.g., "us-de", "ae-abudhabi-adgm", "sg").
    pub source_jurisdiction: String,
    /// Optional reference to the specific profile module that implements
    /// this domain's regulatory rules.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_profile_module: Option<String>,
}

// ---------------------------------------------------------------------------
// Zone Type
// ---------------------------------------------------------------------------

/// Classification of a zone based on how its regulatory layers are composed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoneType {
    /// All layers sourced from a single root jurisdiction.
    Natural,
    /// Layers sourced from multiple jurisdictions, composed into a novel
    /// regulatory environment.
    Synthetic,
}

impl Default for ZoneType {
    fn default() -> Self {
        Self::Natural
    }
}

impl std::fmt::Display for ZoneType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Natural => write!(f, "natural"),
            Self::Synthetic => write!(f, "synthetic"),
        }
    }
}

// ---------------------------------------------------------------------------
// Zone Composition
// ---------------------------------------------------------------------------

/// A complete zone composition: the set of regulatory layers that define a zone.
///
/// For natural zones, all layers share the same root jurisdiction prefix.
/// For synthetic zones, layers may reference different jurisdictions; the
/// `primary_jurisdiction` determines the country code for corridor classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ZoneComposition {
    /// Unique zone identifier (e.g., "org.momentum.mez.zone.synthetic.atlantic-fintech").
    pub zone_id: String,
    /// Human-readable zone name (e.g., "Atlantic Fintech Hub").
    pub zone_name: String,
    /// Whether this zone is natural or synthetic.
    pub zone_type: ZoneType,
    /// The regulatory layers composing this zone.
    pub layers: Vec<RegulatoryLayer>,
    /// The primary jurisdiction for corridor classification purposes.
    /// For natural zones, this is the root jurisdiction.
    /// For synthetic zones, this determines `country_code` (first 2 chars).
    pub primary_jurisdiction: String,
    /// Jurisdiction identifier used in corridor registry and zone manifests.
    pub jurisdiction_id: String,
}

impl ZoneComposition {
    /// Extract the country code from `primary_jurisdiction` (first 2 characters).
    pub fn country_code(&self) -> &str {
        if self.primary_jurisdiction.len() >= 2 {
            &self.primary_jurisdiction[..2]
        } else {
            &self.primary_jurisdiction
        }
    }

    /// Return the set of unique source jurisdictions across all layers.
    pub fn source_jurisdictions(&self) -> BTreeSet<&str> {
        self.layers
            .iter()
            .map(|l| l.source_jurisdiction.as_str())
            .collect()
    }

    /// Return the set of domains covered by this composition.
    pub fn domains(&self) -> BTreeSet<RegulatoryDomain> {
        self.layers.iter().map(|l| l.domain).collect()
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Errors that can occur when validating a zone composition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompositionError {
    /// A regulatory domain appears more than once.
    DuplicateDomain(RegulatoryDomain),
    /// The mandatory AML/CFT layer is missing.
    MissingAmlCft,
    /// For a natural zone, a layer's source jurisdiction does not share
    /// the same root prefix as the primary jurisdiction.
    NaturalJurisdictionMismatch {
        domain: RegulatoryDomain,
        source: String,
        expected_prefix: String,
    },
    /// The composition has no layers.
    EmptyComposition,
}

impl std::fmt::Display for CompositionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateDomain(d) => {
                write!(f, "duplicate regulatory domain: {d}")
            }
            Self::MissingAmlCft => {
                write!(f, "AML/CFT layer is mandatory but not present")
            }
            Self::NaturalJurisdictionMismatch {
                domain,
                source,
                expected_prefix,
            } => {
                write!(
                    f,
                    "natural zone: layer {domain} sources from '{source}' \
                     but expected prefix '{expected_prefix}'"
                )
            }
            Self::EmptyComposition => write!(f, "composition has no layers"),
        }
    }
}

impl std::error::Error for CompositionError {}

/// Validate a zone composition against the composition algebra rules.
///
/// # Rules
///
/// 1. At most one layer per domain (no duplicates).
/// 2. [`AmlCft`](RegulatoryDomain::AmlCft) layer is mandatory.
/// 3. For [`Natural`](ZoneType::Natural) zones, all source jurisdictions must
///    share the same root prefix (first 2 characters) as `primary_jurisdiction`.
/// 4. The composition must have at least one layer.
///
/// # Returns
///
/// `Ok(())` if valid, or a vector of all validation errors found.
pub fn validate_composition(
    composition: &ZoneComposition,
) -> Result<(), Vec<CompositionError>> {
    let mut errors = Vec::new();

    // Rule 4: non-empty.
    if composition.layers.is_empty() {
        errors.push(CompositionError::EmptyComposition);
        return Err(errors);
    }

    // Rule 1: no duplicate domains.
    let mut seen = BTreeSet::new();
    for layer in &composition.layers {
        if !seen.insert(layer.domain) {
            errors.push(CompositionError::DuplicateDomain(layer.domain));
        }
    }

    // Rule 2: AML/CFT mandatory.
    if !seen.contains(&RegulatoryDomain::AmlCft) {
        errors.push(CompositionError::MissingAmlCft);
    }

    // Rule 3: natural zone jurisdiction prefix consistency.
    if composition.zone_type == ZoneType::Natural {
        let prefix = if composition.primary_jurisdiction.len() >= 2 {
            &composition.primary_jurisdiction[..2]
        } else {
            &composition.primary_jurisdiction
        };

        for layer in &composition.layers {
            let layer_prefix = if layer.source_jurisdiction.len() >= 2 {
                &layer.source_jurisdiction[..2]
            } else {
                &layer.source_jurisdiction
            };
            if layer_prefix != prefix {
                errors.push(CompositionError::NaturalJurisdictionMismatch {
                    domain: layer.domain,
                    source: layer.source_jurisdiction.clone(),
                    expected_prefix: prefix.to_string(),
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(domain: RegulatoryDomain, source: &str) -> RegulatoryLayer {
        RegulatoryLayer {
            domain,
            source_jurisdiction: source.to_string(),
            source_profile_module: None,
        }
    }

    fn natural_pk() -> ZoneComposition {
        ZoneComposition {
            zone_id: "org.momentum.mez.zone.pk-sifc".into(),
            zone_name: "Pakistan SIFC".into(),
            zone_type: ZoneType::Natural,
            layers: vec![
                layer(RegulatoryDomain::CorporateFormation, "pk"),
                layer(RegulatoryDomain::Tax, "pk"),
                layer(RegulatoryDomain::AmlCft, "pk"),
                layer(RegulatoryDomain::Securities, "pk"),
            ],
            primary_jurisdiction: "pk".into(),
            jurisdiction_id: "pk-sifc".into(),
        }
    }

    fn synthetic_atlantic() -> ZoneComposition {
        ZoneComposition {
            zone_id: "org.momentum.mez.zone.synthetic.atlantic-fintech".into(),
            zone_name: "Atlantic Fintech Hub".into(),
            zone_type: ZoneType::Synthetic,
            layers: vec![
                layer(RegulatoryDomain::CorporateFormation, "us-de"),
                layer(RegulatoryDomain::CivicCode, "us-ny"),
                layer(RegulatoryDomain::DigitalAssets, "ae-abudhabi-adgm"),
                layer(RegulatoryDomain::Arbitration, "hk"),
                layer(RegulatoryDomain::Tax, "sg"),
                layer(RegulatoryDomain::AmlCft, "ae"),
            ],
            primary_jurisdiction: "us".into(),
            jurisdiction_id: "synth-atlantic-fintech".into(),
        }
    }

    // -- Validation tests --

    #[test]
    fn valid_natural_zone() {
        let comp = natural_pk();
        assert!(validate_composition(&comp).is_ok());
    }

    #[test]
    fn valid_synthetic_zone() {
        let comp = synthetic_atlantic();
        assert!(validate_composition(&comp).is_ok());
    }

    #[test]
    fn empty_composition_fails() {
        let comp = ZoneComposition {
            zone_id: "empty".into(),
            zone_name: "Empty".into(),
            zone_type: ZoneType::Natural,
            layers: vec![],
            primary_jurisdiction: "pk".into(),
            jurisdiction_id: "empty".into(),
        };
        let errs = validate_composition(&comp).unwrap_err();
        assert!(errs.iter().any(|e| matches!(e, CompositionError::EmptyComposition)));
    }

    #[test]
    fn missing_aml_cft_fails() {
        let comp = ZoneComposition {
            zone_id: "no-aml".into(),
            zone_name: "No AML".into(),
            zone_type: ZoneType::Synthetic,
            layers: vec![
                layer(RegulatoryDomain::CorporateFormation, "us-de"),
                layer(RegulatoryDomain::Tax, "sg"),
            ],
            primary_jurisdiction: "us".into(),
            jurisdiction_id: "no-aml".into(),
        };
        let errs = validate_composition(&comp).unwrap_err();
        assert!(errs.iter().any(|e| matches!(e, CompositionError::MissingAmlCft)));
    }

    #[test]
    fn duplicate_domain_fails() {
        let comp = ZoneComposition {
            zone_id: "dup".into(),
            zone_name: "Duplicate".into(),
            zone_type: ZoneType::Synthetic,
            layers: vec![
                layer(RegulatoryDomain::Tax, "sg"),
                layer(RegulatoryDomain::Tax, "hk"),
                layer(RegulatoryDomain::AmlCft, "ae"),
            ],
            primary_jurisdiction: "sg".into(),
            jurisdiction_id: "dup".into(),
        };
        let errs = validate_composition(&comp).unwrap_err();
        assert!(errs.iter().any(|e| matches!(
            e,
            CompositionError::DuplicateDomain(RegulatoryDomain::Tax)
        )));
    }

    #[test]
    fn natural_zone_cross_jurisdiction_fails() {
        let comp = ZoneComposition {
            zone_id: "bad-natural".into(),
            zone_name: "Bad Natural".into(),
            zone_type: ZoneType::Natural,
            layers: vec![
                layer(RegulatoryDomain::CorporateFormation, "pk"),
                layer(RegulatoryDomain::Tax, "sg"),
                layer(RegulatoryDomain::AmlCft, "pk"),
            ],
            primary_jurisdiction: "pk".into(),
            jurisdiction_id: "bad-natural".into(),
        };
        let errs = validate_composition(&comp).unwrap_err();
        assert!(errs.iter().any(|e| matches!(
            e,
            CompositionError::NaturalJurisdictionMismatch { .. }
        )));
    }

    #[test]
    fn natural_zone_sub_jurisdiction_allowed() {
        // pk-sifc layers can source from pk-sifc or pk — same root prefix.
        let comp = ZoneComposition {
            zone_id: "pk-sub".into(),
            zone_name: "PK Sub".into(),
            zone_type: ZoneType::Natural,
            layers: vec![
                layer(RegulatoryDomain::CorporateFormation, "pk-sifc"),
                layer(RegulatoryDomain::Tax, "pk"),
                layer(RegulatoryDomain::AmlCft, "pk"),
            ],
            primary_jurisdiction: "pk".into(),
            jurisdiction_id: "pk-sub".into(),
        };
        assert!(validate_composition(&comp).is_ok());
    }

    // -- Accessor tests --

    #[test]
    fn country_code_extraction() {
        let comp = synthetic_atlantic();
        assert_eq!(comp.country_code(), "us");
    }

    #[test]
    fn source_jurisdictions_deduplication() {
        let comp = synthetic_atlantic();
        let sources = comp.source_jurisdictions();
        assert_eq!(sources.len(), 6);
        assert!(sources.contains("us-de"));
        assert!(sources.contains("ae-abudhabi-adgm"));
        assert!(sources.contains("hk"));
        assert!(sources.contains("sg"));
    }

    #[test]
    fn domains_returns_correct_set() {
        let comp = synthetic_atlantic();
        let domains = comp.domains();
        assert_eq!(domains.len(), 6);
        assert!(domains.contains(&RegulatoryDomain::CorporateFormation));
        assert!(domains.contains(&RegulatoryDomain::AmlCft));
    }

    // -- Serialization tests --

    #[test]
    fn zone_type_serialization() {
        let natural = serde_json::to_string(&ZoneType::Natural).unwrap();
        assert_eq!(natural, "\"natural\"");
        let synthetic = serde_json::to_string(&ZoneType::Synthetic).unwrap();
        assert_eq!(synthetic, "\"synthetic\"");

        let de: ZoneType = serde_json::from_str(&natural).unwrap();
        assert_eq!(de, ZoneType::Natural);
    }

    #[test]
    fn regulatory_domain_serialization() {
        let domain = RegulatoryDomain::DigitalAssets;
        let json = serde_json::to_string(&domain).unwrap();
        assert_eq!(json, "\"digital_assets\"");

        let de: RegulatoryDomain = serde_json::from_str(&json).unwrap();
        assert_eq!(de, RegulatoryDomain::DigitalAssets);
    }

    #[test]
    fn zone_composition_roundtrip() {
        let comp = synthetic_atlantic();
        let json = serde_json::to_string_pretty(&comp).unwrap();
        let de: ZoneComposition = serde_json::from_str(&json).unwrap();
        assert_eq!(de, comp);
    }

    #[test]
    fn regulatory_layer_roundtrip() {
        let l = RegulatoryLayer {
            domain: RegulatoryDomain::Tax,
            source_jurisdiction: "sg".into(),
            source_profile_module: Some("org.momentum.mez.tax.sg-gst".into()),
        };
        let json = serde_json::to_string(&l).unwrap();
        let de: RegulatoryLayer = serde_json::from_str(&json).unwrap();
        assert_eq!(de, l);
    }

    // -- Display tests --

    #[test]
    fn zone_type_display() {
        assert_eq!(ZoneType::Natural.to_string(), "natural");
        assert_eq!(ZoneType::Synthetic.to_string(), "synthetic");
    }

    #[test]
    fn regulatory_domain_display() {
        assert_eq!(
            RegulatoryDomain::CorporateFormation.to_string(),
            "corporate_formation"
        );
        assert_eq!(RegulatoryDomain::AmlCft.to_string(), "aml_cft");
        assert_eq!(
            RegulatoryDomain::PaymentRails.to_string(),
            "payment_rails"
        );
    }

    #[test]
    fn composition_error_display() {
        let e = CompositionError::DuplicateDomain(RegulatoryDomain::Tax);
        assert_eq!(e.to_string(), "duplicate regulatory domain: tax");

        let e = CompositionError::MissingAmlCft;
        assert_eq!(e.to_string(), "AML/CFT layer is mandatory but not present");

        let e = CompositionError::NaturalJurisdictionMismatch {
            domain: RegulatoryDomain::Tax,
            source: "sg".into(),
            expected_prefix: "pk".into(),
        };
        assert!(e.to_string().contains("natural zone"));
    }

    #[test]
    fn zone_type_default_is_natural() {
        assert_eq!(ZoneType::default(), ZoneType::Natural);
    }

    #[test]
    fn multiple_errors_collected() {
        let comp = ZoneComposition {
            zone_id: "multi-err".into(),
            zone_name: "Multi Error".into(),
            zone_type: ZoneType::Natural,
            layers: vec![
                layer(RegulatoryDomain::Tax, "pk"),
                layer(RegulatoryDomain::Tax, "sg"), // duplicate + wrong jurisdiction
            ],
            primary_jurisdiction: "pk".into(),
            jurisdiction_id: "multi-err".into(),
        };
        let errs = validate_composition(&comp).unwrap_err();
        // Should have: DuplicateDomain(Tax), MissingAmlCft, NaturalJurisdictionMismatch
        assert!(errs.len() >= 2);
    }
}
