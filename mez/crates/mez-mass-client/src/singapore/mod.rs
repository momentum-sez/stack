//! # Singapore National System Adapters
//!
//! Adapter interfaces for Singapore government systems:
//! - **MyInfo / Singpass** (via GovTech): National identity verification (NRIC-based)
//! - **IRAS** (Inland Revenue Authority of Singapore): GST and corporate tax
//! - **ACRA** (Accounting and Corporate Regulatory Authority): BizFile+ corporate registry

pub mod acra;
pub mod iras;
pub mod myinfo;
