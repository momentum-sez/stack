//! # API Route Modules
//!
//! Route modules for the SEZ Stack API surface:
//!
//! - `mass_proxy` — Thin proxy to Mass APIs for primitive operations (entities,
//!   ownership, fiscal, identity, consent) via `msez-mass-client`.
//! - `identity` — Identity orchestration endpoints (CNIC/NTN verification,
//!   consolidated identity views) — P1-005.
//! - `tax` — Tax collection pipeline (withholding computation, FBR IRIS
//!   reporting, tax event recording) — P1-009.
//! - `corridors` — Cross-border corridor lifecycle (SEZ Stack domain).
//! - `smart_assets` — Smart asset lifecycle (SEZ Stack domain).
//! - `credentials` — VC issuance on compliance evaluation, VC verification.
//! - `regulator` — Read-only regulator console (SEZ Stack domain).
//! - `agentic` — Autonomous policy engine: trigger ingestion, policy management,
//!   and reactive corridor transitions via the typestate machine.

pub mod agentic;
pub mod corridors;
pub mod credentials;
pub mod identity;
pub mod mass_proxy;
pub mod regulator;
pub mod settlement;
pub mod smart_assets;
pub mod tax;
