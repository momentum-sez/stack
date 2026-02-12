//! # API Route Modules
//!
//! Each module defines an Axum `Router` for one of the five programmable
//! primitives or an auxiliary service.

pub mod consent;
pub mod corridors;
pub mod entities;
pub mod fiscal;
pub mod identity;
pub mod ownership;
pub mod regulator;
pub mod smart_assets;
