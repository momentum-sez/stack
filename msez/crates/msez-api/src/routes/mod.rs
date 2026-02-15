//! # API Route Modules
//!
//! Route modules for the SEZ Stack API surface:
//!
//! - `mass_proxy` — Orchestration endpoints for all five Mass primitives
//!   (entities, ownership, fiscal, identity, consent). Write endpoints compose
//!   compliance tensor evaluation + Mass API delegation + VC issuance. Read
//!   endpoints proxy through to Mass APIs via `msez-mass-client`.
//! - `corridors` — Cross-border corridor lifecycle (SEZ Stack domain).
//! - `smart_assets` — Smart asset lifecycle (SEZ Stack domain).
//! - `credentials` — VC issuance on compliance evaluation, VC verification.
//! - `regulator` — Read-only regulator console (SEZ Stack domain).
//! - `agentic` — Autonomous policy engine: trigger ingestion, policy management,
//!   and reactive corridor transitions via the typestate machine.
//! - `tax` — Tax collection pipeline: event recording, withholding computation,
//!   obligation tracking, and FBR IRIS report generation.

pub mod agentic;
pub mod corridors;
pub mod credentials;
pub mod mass_proxy;
pub mod regulator;
pub mod settlement;
pub mod smart_assets;
pub mod tax;
