//! # Licensepack — License Lifecycle Management
//!
//! Manages the full lifecycle of business licenses, professional certifications,
//! and regulatory authorizations across 70+ jurisdictions including:
//!
//! - **United States**: All 50 states + DC + 5 territories (PR, GU, VI, AS, MP)
//! - **UAE**: Federal + Abu Dhabi/ADGM + Dubai/DIFC/DMCC/JAFZA + 10 free zones
//! - **Caribbean**: British Virgin Islands, Cayman Islands
//! - **Asia-Pacific**: Hong Kong, Singapore, China (Hainan, Shenzhen, Shanghai, Beijing, Hangzhou)
//! - **Middle East**: Qatar/QFC, Kazakhstan/AIFC
//! - **Americas**: Brazil, Honduras/Prospera ZEDE
//! - **Africa**: Kenya, Seychelles, South Africa, Egypt, Tanzania/Zanzibar
//! - **Europe**: Portugal, Ireland
//! - **Pakistan**: SECP, SBP, PTA, PEMRA, DRAP
//!
//! ## Module Structure
//!
//! - [`types`]: Core enums (`LicenseStatus`, `LicenseDomain`, `LicenseComplianceState`)
//! - [`components`]: Sub-record types (`LicenseCondition`, `LicensePermission`,
//!   `LicenseRestriction`, `LicenseHolder`)
//! - [`license`]: License records (`License`, `LicenseTypeDefinition`,
//!   `LicensepackRegulator`, `LicensepackMetadata`)
//! - [`pack`]: Top-level container (`Licensepack`) with digest and delta computation
//! - [`reference`]: Zone references and lock files (`LicensepackRef`,
//!   `LicensepackLock`, utility functions)
//!
//! ## Digest Computation
//!
//! Licensepack digests follow the same content-addressed pattern as lawpack/regpack:
//!
//! ```text
//! SHA256( b"mez-licensepack-v1\0"
//!       + canonical(metadata) + b"\0"
//!       + for each license_type in sorted(license_types.keys()):
//!           "license-types/{type_id}\0" + canonical(type_data) + b"\0"
//!       + for each license in sorted(licenses.keys()):
//!           "licenses/{license_id}\0" + canonical(license_data) + b"\0"
//!           + conditions... + permissions... + restrictions...
//!       + for each holder in sorted(holders.keys()):
//!           "holders/{holder_id}\0" + canonical(holder_data) + b"\0" )
//! ```
//!
//! All canonicalization goes through [`CanonicalBytes`](mez_core::CanonicalBytes)
//! for cross-language digest equality.

pub mod additional;
pub mod brazil;
pub mod bvi;
pub mod cayman;
pub mod china;
pub mod components;
pub mod hong_kong;
pub mod kazakhstan;

pub mod kenya;
pub mod license;
pub mod pack;
pub mod pakistan;
pub mod prospera;
pub mod qatar;
pub mod reference;
pub mod seychelles;
pub mod singapore;
pub mod types;
pub mod uae;
pub mod united_states;

// Re-export all public types at the module root to preserve the existing API.
// External code using `mez_pack::licensepack::Licensepack` continues to work.
pub use components::{LicenseCondition, LicenseHolder, LicensePermission, LicenseRestriction};
pub use license::{License, LicenseTypeDefinition, LicensepackMetadata, LicensepackRegulator};
pub use pack::Licensepack;
pub use additional::{additional_license_types, additional_regulators};
pub use brazil::{brazil_license_types, brazil_regulators};
pub use bvi::{bvi_license_types, bvi_regulators};
pub use cayman::{cayman_license_types, cayman_regulators};
pub use china::{china_license_types, china_regulators};
pub use hong_kong::{hong_kong_license_types, hong_kong_regulators};
pub use kazakhstan::{kazakhstan_license_types, kazakhstan_regulators};
pub use kenya::{kenya_license_types, kenya_regulators};
pub use pakistan::{pakistan_license_types, pakistan_regulators};
pub use prospera::{prospera_license_types, prospera_regulators};
pub use qatar::{qatar_license_types, qatar_regulators};
pub use seychelles::{seychelles_license_types, seychelles_regulators};
pub use singapore::{singapore_license_types, singapore_regulators};
pub use uae::{uae_license_types, uae_regulators};
pub use united_states::{us_license_types, us_regulators};
pub use reference::{
    canonical_json_bytes, evaluate_license_compliance, resolve_licensepack_refs,
    LicensepackArtifactInfo, LicensepackLock, LicensepackLockInfo, LicensepackRef,
};
pub use types::{LicenseComplianceState, LicenseDomain, LicenseStatus};
