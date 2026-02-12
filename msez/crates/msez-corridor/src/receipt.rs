//! # Corridor Receipt Chain
//!
//! Append-only corridor receipts backed by MMR for efficient inclusion proofs.

use msez_core::{ContentDigest, CorridorId, Timestamp};
use serde::{Deserialize, Serialize};

/// A corridor receipt recording a cross-border transaction event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorReceipt {
    /// The corridor this receipt belongs to.
    pub corridor_id: CorridorId,
    /// Sequence number within the corridor.
    pub sequence: u64,
    /// Content digest of the receipt payload.
    pub payload_digest: ContentDigest,
    /// When the receipt was created.
    pub timestamp: Timestamp,
}
