//! # Licensepack — License Lifecycle Management
//!
//! Manages the full lifecycle of business licenses, professional certifications,
//! and regulatory authorizations (15+ categories for Pakistan deployment).
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
//! SHA256( b"msez-licensepack-v1\0"
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
//! All canonicalization goes through [`CanonicalBytes`](msez_core::CanonicalBytes)
//! for cross-language digest equality.

pub mod components;
pub mod license;
pub mod pack;
pub mod reference;
pub mod types;

// Re-export all public types at the module root to preserve the existing API.
// External code using `msez_pack::licensepack::Licensepack` continues to work.
pub use components::{LicenseCondition, LicenseHolder, LicensePermission, LicenseRestriction};
pub use license::{License, LicenseTypeDefinition, LicensepackMetadata, LicensepackRegulator};
pub use pack::Licensepack;
pub use reference::{
    canonical_json_bytes, evaluate_license_compliance, resolve_licensepack_refs,
    LicensepackArtifactInfo, LicensepackLock, LicensepackLockInfo, LicensepackRef,
};
pub use types::{LicenseComplianceState, LicenseDomain, LicenseStatus};
