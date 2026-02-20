//! # UAE National System Adapters
//!
//! Adapter interfaces for UAE government systems used by Abu Dhabi ADGM
//! (and other UAE free zone) deployments:
//! - **ICA** (Federal Authority for Identity, Citizenship, Customs and Port Security):
//!   Emirates ID verification
//! - **FTA** (Federal Tax Authority): VAT and Economic Substance reporting
//! - **DED** (Department of Economic Development) / **ADGM-RA** (Registration Authority):
//!   Commercial license and trade registry operations
//! - **UAEFTS / IPP** (UAE Funds Transfer System / Instant Payment Platform):
//!   Domestic payment rails operated by the Central Bank of the UAE (CBUAE)

pub mod ded;
pub mod emirates_id;
pub mod fta;
pub mod uaefts;
