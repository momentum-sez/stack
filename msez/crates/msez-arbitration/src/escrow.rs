//! # Escrow Operations
//!
//! Manages escrow accounts for disputed amounts, including conditional
//! release and clawback.

use serde::{Deserialize, Serialize};

/// An escrow account holding disputed funds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscrowAccount {
    /// Unique escrow account identifier.
    pub id: String,
    /// Amount held in escrow (integer, smallest currency unit).
    pub amount: i64,
    /// Currency code (ISO 4217).
    pub currency: String,
    /// Current escrow status.
    pub status: EscrowStatus,
}

/// The status of an escrow account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EscrowStatus {
    /// Funds are held pending dispute resolution.
    Held,
    /// Funds have been released to the prevailing party.
    Released,
    /// Funds have been clawed back.
    ClawedBack,
}
