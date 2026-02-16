//! # API Route Modules
//!
//! Route modules for the SEZ Stack API surface:
//!
//! - `mass_proxy` — Orchestration endpoints for all five Mass primitives
//!   (entities, ownership, fiscal, identity, consent). Write endpoints compose
//!   compliance tensor evaluation + Mass API delegation + VC issuance. Read
//!   endpoints proxy through to Mass APIs via `msez-mass-client`.
//! - `identity` — Identity orchestration endpoints (CNIC/NTN verification,
//!   consolidated identity views) — P1-005.
//! - `tax` — Tax collection pipeline (withholding computation, FBR IRIS
//!   reporting, tax event recording) — P1-009.
//! - `corridors` — Cross-border corridor lifecycle (SEZ Stack domain).
//! - `smart_assets` — Smart asset lifecycle (SEZ Stack domain).
//! - `credentials` — VC issuance on compliance evaluation, VC verification.
//! - `regulator` — Read-only regulator console (SEZ Stack domain).
//! - `govos` — GovOS Console dashboards for Pakistan sovereign deployment:
//!   GovOS Console (40+ ministries), Tax & Revenue, Digital Free Zone,
//!   Citizen Tax & Services (M-009).
//! - `agentic` — Autonomous policy engine: trigger ingestion, policy management,
//!   and reactive corridor transitions via the typestate machine.
//! - `tax` — Tax collection pipeline: event recording, withholding computation,
//!   obligation tracking, and FBR IRIS report generation.

pub mod agentic;
pub mod corridors;
pub mod credentials;
#[cfg(feature = "jurisdiction-pk")]
pub mod govos;
pub mod identity;
pub mod mass_proxy;
pub mod regulator;
pub mod settlement;
pub mod smart_assets;
pub mod tax;
