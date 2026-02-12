//! # Route Modules
//!
//! Each module defines an Axum Router for one API surface area.
//! Routers are assembled in `main.rs` into the application.

pub mod consent;
pub mod corridors;
pub mod entities;
pub mod fiscal;
pub mod identity;
pub mod ownership;
pub mod regulator;
pub mod smart_assets;
