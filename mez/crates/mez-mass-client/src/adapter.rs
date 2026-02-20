//! # National System Adapter — Generic Trait Interface
//!
//! Defines the generic `NationalSystemAdapter` trait that abstracts over
//! jurisdiction-specific government system integrations. Each sovereign zone
//! connects to its national identity authority, tax authority, corporate
//! registry, and payment rail through concrete implementations of this trait.
//!
//! ## Architecture
//!
//! The existing Pakistan adapters (FBR, NADRA, SECP, Raast) already implement
//! this pattern via separate per-system traits. `NationalSystemAdapter` provides
//! a higher-level abstraction for zone bootstrapping: given a jurisdiction ID,
//! resolve which adapter implementations to wire, and verify that all required
//! national systems are reachable.
//!
//! ## Adapter Families
//!
//! Each jurisdiction requires adapters in four families:
//! - **Identity**: National ID verification (NADRA/CNIC for PK, ICA/EmiratesID
//!   for AE, MyInfo/NRIC for SG)
//! - **Tax**: Tax authority integration (FBR IRIS for PK, FTA for AE, IRAS for SG)
//! - **Corporate**: Company registry lookup (SECP for PK, DED/ADGM-RA for AE,
//!   ACRA for SG)
//! - **Payments**: Domestic payment rail (SBP Raast for PK, UAEFTS/IPP for AE,
//!   FAST/PayNow for SG)
//!
//! ## Usage
//!
//! Zone configuration specifies which adapter implementations to instantiate.
//! The `NationalSystemAdapter` trait allows the zone bootstrap logic to verify
//! connectivity and capability without knowing the jurisdiction-specific details.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Adapter category for national system integrations.
///
/// Each jurisdiction must provide adapters for these four families.
/// The zone manifest declares which concrete adapter implementations
/// to use for each category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AdapterCategory {
    /// National identity verification system.
    Identity,
    /// Tax authority / revenue service.
    Tax,
    /// Corporate / commercial registry.
    Corporate,
    /// Domestic payment rail.
    Payments,
}

impl fmt::Display for AdapterCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identity => write!(f, "Identity"),
            Self::Tax => write!(f, "Tax"),
            Self::Corporate => write!(f, "Corporate"),
            Self::Payments => write!(f, "Payments"),
        }
    }
}

/// Health status of a national system adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdapterHealth {
    /// Adapter is reachable and operational.
    Healthy,
    /// Adapter is reachable but degraded (e.g. slow responses, partial failures).
    Degraded {
        /// Human-readable reason for the degraded state.
        reason: String,
    },
    /// Adapter is not reachable or not configured.
    Unavailable {
        /// Human-readable reason for unavailability.
        reason: String,
    },
}

impl fmt::Display for AdapterHealth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Healthy => write!(f, "Healthy"),
            Self::Degraded { reason } => write!(f, "Degraded: {reason}"),
            Self::Unavailable { reason } => write!(f, "Unavailable: {reason}"),
        }
    }
}

/// Errors from the generic adapter interface.
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    /// The requested adapter category is not configured for this jurisdiction.
    #[error("adapter not configured: {category} for jurisdiction {jurisdiction}")]
    NotConfigured {
        /// The adapter category that was requested.
        category: AdapterCategory,
        /// The jurisdiction identifier.
        jurisdiction: String,
    },

    /// The adapter is configured but the underlying service is unreachable.
    #[error("adapter unavailable: {category} for jurisdiction {jurisdiction}: {reason}")]
    Unavailable {
        /// The adapter category.
        category: AdapterCategory,
        /// The jurisdiction identifier.
        jurisdiction: String,
        /// Human-readable reason for the failure.
        reason: String,
    },

    /// The operation is not supported by this adapter.
    #[error("operation not supported by {adapter_name}: {operation}")]
    NotSupported {
        /// The adapter implementation name.
        adapter_name: String,
        /// The operation that was attempted.
        operation: String,
    },
}

/// Descriptor for a national system adapter, returned by the suite probe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterDescriptor {
    /// The adapter category.
    pub category: AdapterCategory,
    /// Human-readable name of the adapter implementation.
    pub adapter_name: String,
    /// The jurisdiction this adapter serves.
    pub jurisdiction: String,
    /// Current health status.
    pub health: AdapterHealth,
}

/// Generic trait for a national system adapter.
///
/// Each jurisdiction provides concrete implementations for its four
/// adapter families. This trait captures the common lifecycle operations:
/// health probing, capability description, and adapter identification.
///
/// Implementations must be `Send + Sync` so they can be shared across
/// async tasks behind an `Arc`. The trait is object-safe to support
/// runtime adapter selection.
pub trait NationalSystemAdapter: Send + Sync {
    /// Return the adapter category (Identity, Tax, Corporate, Payments).
    fn category(&self) -> AdapterCategory;

    /// Return the jurisdiction identifier this adapter is configured for
    /// (e.g. "pk", "ae-abudhabi-adgm", "sg").
    fn jurisdiction(&self) -> &str;

    /// Probe the health of the underlying national system.
    fn health(&self) -> AdapterHealth;

    /// Return the human-readable name of this adapter implementation
    /// (e.g. "MockIcaAdapter", "FbrIrisLiveApiV1").
    fn adapter_name(&self) -> &str;
}

/// A complete suite of national system adapters for a jurisdiction.
///
/// Zone bootstrap logic uses this to verify that all required adapter
/// families are configured and reachable before the zone becomes active.
pub struct AdapterSuite {
    adapters: Vec<Box<dyn NationalSystemAdapter>>,
}

impl AdapterSuite {
    /// Create a new adapter suite from a list of adapters.
    pub fn new(adapters: Vec<Box<dyn NationalSystemAdapter>>) -> Self {
        Self { adapters }
    }

    /// Probe all adapters and return their descriptors.
    pub fn probe_all(&self) -> Vec<AdapterDescriptor> {
        self.adapters
            .iter()
            .map(|a| AdapterDescriptor {
                category: a.category(),
                adapter_name: a.adapter_name().to_string(),
                jurisdiction: a.jurisdiction().to_string(),
                health: a.health(),
            })
            .collect()
    }

    /// Check whether all required adapter categories are present and healthy.
    pub fn all_healthy(&self) -> bool {
        let required = [
            AdapterCategory::Identity,
            AdapterCategory::Tax,
            AdapterCategory::Corporate,
            AdapterCategory::Payments,
        ];
        required.iter().all(|cat| {
            self.adapters
                .iter()
                .any(|a| a.category() == *cat && matches!(a.health(), AdapterHealth::Healthy))
        })
    }

    /// Return the adapter for a given category, if present.
    pub fn get(&self, category: AdapterCategory) -> Option<&dyn NationalSystemAdapter> {
        self.adapters
            .iter()
            .find(|a| a.category() == category)
            .map(|a| a.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal test adapter for verifying the trait and suite machinery.
    #[derive(Debug)]
    struct TestAdapter {
        cat: AdapterCategory,
        jurisdiction: String,
        name: String,
        healthy: bool,
    }

    impl NationalSystemAdapter for TestAdapter {
        fn category(&self) -> AdapterCategory {
            self.cat
        }
        fn jurisdiction(&self) -> &str {
            &self.jurisdiction
        }
        fn health(&self) -> AdapterHealth {
            if self.healthy {
                AdapterHealth::Healthy
            } else {
                AdapterHealth::Unavailable {
                    reason: "test".to_string(),
                }
            }
        }
        fn adapter_name(&self) -> &str {
            &self.name
        }
    }

    fn make_adapter(cat: AdapterCategory, healthy: bool) -> Box<dyn NationalSystemAdapter> {
        Box::new(TestAdapter {
            cat,
            jurisdiction: "test".to_string(),
            name: format!("Test{cat}Adapter"),
            healthy,
        })
    }

    #[test]
    fn adapter_category_display() {
        assert_eq!(format!("{}", AdapterCategory::Identity), "Identity");
        assert_eq!(format!("{}", AdapterCategory::Tax), "Tax");
        assert_eq!(format!("{}", AdapterCategory::Corporate), "Corporate");
        assert_eq!(format!("{}", AdapterCategory::Payments), "Payments");
    }

    #[test]
    fn adapter_health_display() {
        assert_eq!(format!("{}", AdapterHealth::Healthy), "Healthy");
        assert_eq!(
            format!(
                "{}",
                AdapterHealth::Degraded {
                    reason: "slow".to_string()
                }
            ),
            "Degraded: slow"
        );
        assert_eq!(
            format!(
                "{}",
                AdapterHealth::Unavailable {
                    reason: "down".to_string()
                }
            ),
            "Unavailable: down"
        );
    }

    #[test]
    fn adapter_error_display() {
        let err = AdapterError::NotConfigured {
            category: AdapterCategory::Identity,
            jurisdiction: "pk".to_string(),
        };
        assert!(format!("{err}").contains("Identity"));
        assert!(format!("{err}").contains("pk"));
    }

    #[test]
    fn suite_all_healthy_when_complete() {
        let suite = AdapterSuite::new(vec![
            make_adapter(AdapterCategory::Identity, true),
            make_adapter(AdapterCategory::Tax, true),
            make_adapter(AdapterCategory::Corporate, true),
            make_adapter(AdapterCategory::Payments, true),
        ]);
        assert!(suite.all_healthy());
    }

    #[test]
    fn suite_not_healthy_when_missing_category() {
        let suite = AdapterSuite::new(vec![
            make_adapter(AdapterCategory::Identity, true),
            make_adapter(AdapterCategory::Tax, true),
            // Missing Corporate and Payments.
        ]);
        assert!(!suite.all_healthy());
    }

    #[test]
    fn suite_not_healthy_when_one_unhealthy() {
        let suite = AdapterSuite::new(vec![
            make_adapter(AdapterCategory::Identity, true),
            make_adapter(AdapterCategory::Tax, false), // Unhealthy
            make_adapter(AdapterCategory::Corporate, true),
            make_adapter(AdapterCategory::Payments, true),
        ]);
        assert!(!suite.all_healthy());
    }

    #[test]
    fn suite_probe_all_returns_all_descriptors() {
        let suite = AdapterSuite::new(vec![
            make_adapter(AdapterCategory::Identity, true),
            make_adapter(AdapterCategory::Tax, true),
        ]);
        let descriptors = suite.probe_all();
        assert_eq!(descriptors.len(), 2);
        assert_eq!(descriptors[0].category, AdapterCategory::Identity);
        assert_eq!(descriptors[1].category, AdapterCategory::Tax);
    }

    #[test]
    fn suite_get_returns_correct_adapter() {
        let suite = AdapterSuite::new(vec![
            make_adapter(AdapterCategory::Identity, true),
            make_adapter(AdapterCategory::Tax, true),
        ]);
        let identity = suite.get(AdapterCategory::Identity);
        assert!(identity.is_some());
        assert_eq!(identity.unwrap().category(), AdapterCategory::Identity);

        let corporate = suite.get(AdapterCategory::Corporate);
        assert!(corporate.is_none());
    }

    #[test]
    fn adapter_descriptor_serde_roundtrip() {
        let desc = AdapterDescriptor {
            category: AdapterCategory::Identity,
            adapter_name: "MockIcaAdapter".to_string(),
            jurisdiction: "ae-abudhabi-adgm".to_string(),
            health: AdapterHealth::Healthy,
        };
        let json = serde_json::to_string(&desc).expect("serialize");
        let back: AdapterDescriptor = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.category, AdapterCategory::Identity);
        assert_eq!(back.adapter_name, "MockIcaAdapter");
    }

    #[test]
    fn trait_is_object_safe() {
        // Verify the trait can be used as a trait object.
        let _: Box<dyn NationalSystemAdapter> = make_adapter(AdapterCategory::Identity, true);
    }

    #[test]
    fn trait_is_arc_safe() {
        use std::sync::Arc;
        let adapter = make_adapter(AdapterCategory::Tax, true);
        let _: Arc<dyn NationalSystemAdapter> = Arc::from(adapter);
    }
}
