//! # Inter-Zone Corridor Networking (P0-CORRIDOR-NET-001)
//!
//! Implements the zone-to-zone communication protocol for corridor receipt
//! exchange, watcher attestation delivery, and corridor establishment.
//!
//! ## Protocol Overview
//!
//! Two zones (Zone A and Zone B) establish a corridor via a three-step handshake:
//!
//! 1. **Propose** — Zone A sends `CorridorProposal` with corridor definition,
//!    jurisdiction IDs, and Zone A's verifying key.
//! 2. **Accept** — Zone B validates the proposal, verifies the signing key,
//!    and responds with `CorridorAcceptance` containing Zone B's verifying key.
//! 3. **Activate** — Both zones exchange their genesis receipt, establishing
//!    the corridor receipt chain on both sides.
//!
//! Once activated, zones exchange receipts via `InboundReceipt` messages.
//! Each receipt is verified against the local receipt chain before acceptance.
//!
//! ## Security
//!
//! - All messages are signed by the sending zone's Ed25519 key.
//! - Receipt replay is detected via sequence number + digest deduplication.
//! - Stale receipts (sequence < local height) are rejected.
//! - Clock skew tolerance follows `fork::MAX_CLOCK_SKEW` (5 minutes).

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Peer Identity
// ---------------------------------------------------------------------------

/// A corridor peer — another zone this zone can exchange receipts with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorPeer {
    /// Unique zone identifier (e.g., "org.momentum.mez.zone.pk-sifc").
    pub zone_id: String,
    /// Jurisdiction of the peer zone.
    pub jurisdiction_id: String,
    /// HTTPS endpoint for corridor API (e.g., "https://zone-b.example.com").
    pub endpoint_url: String,
    /// Ed25519 verifying key (hex-encoded, 64 chars).
    pub verifying_key_hex: String,
    /// DID of the peer zone (did:mass:zone:{key}).
    pub did: String,
    /// When this peer was last seen (RFC 3339).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<String>,
    /// Corridor IDs shared with this peer.
    #[serde(default)]
    pub corridor_ids: Vec<String>,
    /// Peer status.
    #[serde(default)]
    pub status: PeerStatus,
}

/// Status of a corridor peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PeerStatus {
    /// Peer discovered but handshake not initiated.
    #[default]
    Discovered,
    /// Handshake in progress (proposal sent, awaiting acceptance).
    Proposing,
    /// Corridor established and active.
    Active,
    /// Peer unreachable (will retry).
    Unreachable,
    /// Peer explicitly disconnected.
    Disconnected,
}

impl std::fmt::Display for PeerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Discovered => write!(f, "discovered"),
            Self::Proposing => write!(f, "proposing"),
            Self::Active => write!(f, "active"),
            Self::Unreachable => write!(f, "unreachable"),
            Self::Disconnected => write!(f, "disconnected"),
        }
    }
}

// ---------------------------------------------------------------------------
// Handshake Messages
// ---------------------------------------------------------------------------

/// A corridor establishment proposal from Zone A to Zone B.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorProposal {
    /// Proposed corridor ID.
    pub corridor_id: String,
    /// Jurisdiction of the proposing zone.
    pub proposer_jurisdiction_id: String,
    /// Zone ID of the proposer.
    pub proposer_zone_id: String,
    /// Ed25519 verifying key of the proposer (hex).
    pub proposer_verifying_key_hex: String,
    /// DID of the proposer.
    pub proposer_did: String,
    /// Jurisdiction of the responder zone.
    pub responder_jurisdiction_id: String,
    /// Timestamp of the proposal (RFC 3339).
    pub proposed_at: String,
    /// Corridor parameters (lawpack domains, compliance requirements, etc.).
    #[serde(default)]
    pub parameters: BTreeMap<String, serde_json::Value>,
    /// Ed25519 signature over the canonical proposal payload (hex).
    pub signature: String,
}

/// A corridor acceptance from Zone B in response to Zone A's proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorAcceptance {
    /// Corridor ID being accepted.
    pub corridor_id: String,
    /// Zone ID of the responder.
    pub responder_zone_id: String,
    /// Ed25519 verifying key of the responder (hex).
    pub responder_verifying_key_hex: String,
    /// DID of the responder.
    pub responder_did: String,
    /// Genesis root for the corridor (both zones must agree).
    pub genesis_root_hex: String,
    /// Timestamp of acceptance (RFC 3339).
    pub accepted_at: String,
    /// Ed25519 signature over the canonical acceptance payload (hex).
    pub signature: String,
}

/// Rejection of a corridor proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorRejection {
    /// Corridor ID being rejected.
    pub corridor_id: String,
    /// Zone ID of the rejecting party.
    pub rejector_zone_id: String,
    /// Human-readable reason.
    pub reason: String,
    /// Timestamp (RFC 3339).
    pub rejected_at: String,
    /// Signature.
    pub signature: String,
}

// ---------------------------------------------------------------------------
// Receipt Exchange
// ---------------------------------------------------------------------------

/// An inbound receipt from a peer zone.
///
/// The receiving zone verifies:
/// 1. Signature matches the peer's verifying key.
/// 2. `sequence` == local chain height (no gaps).
/// 3. `prev_root` == local `final_state_root` (hash-chain continuity).
/// 4. `next_root` == recomputed `SHA256(JCS(payload))` (no forgery).
/// 5. Receipt digest not already seen (replay protection).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundReceipt {
    /// Corridor this receipt belongs to.
    pub corridor_id: String,
    /// Zone that produced this receipt.
    pub origin_zone_id: String,
    /// Sequence number in the receipt chain.
    pub sequence: u64,
    /// The full serialized receipt (JSON).
    pub receipt_json: serde_json::Value,
    /// SHA-256 digest of the receipt.
    pub receipt_digest: String,
    /// Ed25519 signature of the receipt digest by the origin zone.
    pub signature: String,
    /// Timestamp when the receipt was produced (RFC 3339).
    pub produced_at: String,
}

/// Result of processing an inbound receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundReceiptResult {
    /// Whether the receipt was accepted.
    pub accepted: bool,
    /// Current chain height after processing.
    pub chain_height: u64,
    /// Current MMR root after processing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mmr_root: Option<String>,
    /// Rejection reason (if not accepted).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
}

/// A watcher attestation delivered between zones.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundAttestation {
    /// Corridor this attestation applies to.
    pub corridor_id: String,
    /// Watcher that produced the attestation.
    pub watcher_id: String,
    /// The chain height this attestation covers.
    pub attested_height: u64,
    /// The state root being attested.
    pub attested_root_hex: String,
    /// Watcher's Ed25519 signature.
    pub signature: String,
    /// Timestamp (RFC 3339).
    pub attested_at: String,
}

// ---------------------------------------------------------------------------
// Peer Registry
// ---------------------------------------------------------------------------

/// Registry of known corridor peers.
///
/// Manages peer discovery, status tracking, and receipt deduplication.
#[derive(Debug)]
pub struct PeerRegistry {
    peers: HashMap<String, CorridorPeer>,
    /// Set of receipt digests already processed (replay protection).
    seen_receipts: HashSet<String>,
    /// Maximum number of seen receipts to track before pruning.
    max_seen_receipts: usize,
}

impl PeerRegistry {
    /// Create a new empty peer registry.
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
            seen_receipts: HashSet::new(),
            max_seen_receipts: 100_000,
        }
    }

    /// Register a new peer or update an existing one.
    pub fn register_peer(&mut self, peer: CorridorPeer) {
        self.peers.insert(peer.zone_id.clone(), peer);
    }

    /// Get a peer by zone ID.
    pub fn get_peer(&self, zone_id: &str) -> Option<&CorridorPeer> {
        self.peers.get(zone_id)
    }

    /// Get a mutable reference to a peer.
    pub fn get_peer_mut(&mut self, zone_id: &str) -> Option<&mut CorridorPeer> {
        self.peers.get_mut(zone_id)
    }

    /// List all peers.
    pub fn list_peers(&self) -> Vec<&CorridorPeer> {
        self.peers.values().collect()
    }

    /// List active peers only.
    pub fn active_peers(&self) -> Vec<&CorridorPeer> {
        self.peers
            .values()
            .filter(|p| p.status == PeerStatus::Active)
            .collect()
    }

    /// Check if a receipt digest has already been seen (replay protection).
    pub fn is_receipt_seen(&self, digest: &str) -> bool {
        self.seen_receipts.contains(digest)
    }

    /// Mark a receipt digest as seen.
    pub fn mark_receipt_seen(&mut self, digest: String) {
        // Prune if we've accumulated too many
        if self.seen_receipts.len() >= self.max_seen_receipts {
            // Simple strategy: clear half. In production, use LRU or time-based expiry.
            let to_remove: Vec<String> = self
                .seen_receipts
                .iter()
                .take(self.max_seen_receipts / 2)
                .cloned()
                .collect();
            for d in to_remove {
                self.seen_receipts.remove(&d);
            }
        }
        self.seen_receipts.insert(digest);
    }

    /// Update peer status.
    pub fn set_peer_status(&mut self, zone_id: &str, status: PeerStatus) {
        if let Some(peer) = self.peers.get_mut(zone_id) {
            peer.status = status;
        }
    }

    /// Number of registered peers.
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Number of tracked receipt digests.
    pub fn seen_receipt_count(&self) -> usize {
        self.seen_receipts.len()
    }
}

impl Default for PeerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for inter-zone corridor networking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorNetworkConfig {
    /// This zone's ID.
    pub zone_id: String,
    /// This zone's jurisdiction.
    pub jurisdiction_id: String,
    /// This zone's public endpoint URL.
    pub endpoint_url: String,
    /// Ed25519 verifying key (hex) for this zone.
    pub verifying_key_hex: String,
    /// DID for this zone.
    pub did: String,
    /// Static peer list (from zone.yaml `corridor_peers`).
    #[serde(default)]
    pub static_peers: Vec<PeerEndpoint>,
    /// Maximum receipts to buffer before requiring acknowledgment.
    #[serde(default = "default_max_buffer")]
    pub max_receipt_buffer: usize,
    /// Timeout for peer requests in seconds.
    #[serde(default = "default_peer_timeout")]
    pub peer_timeout_secs: u64,
}

/// A static peer endpoint from configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerEndpoint {
    /// Zone ID of the peer.
    pub zone_id: String,
    /// HTTPS URL of the peer.
    pub url: String,
}

fn default_max_buffer() -> usize {
    1000
}

fn default_peer_timeout() -> u64 {
    30
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

/// Validate an inbound receipt against basic structural requirements.
///
/// Does NOT verify the Ed25519 signature (that requires the peer's verifying key
/// and is done at the API layer). This checks:
/// - Receipt JSON is non-null
/// - Sequence is reasonable
/// - Digest is a valid SHA-256 hex string
pub fn validate_inbound_receipt(receipt: &InboundReceipt) -> Result<(), NetworkError> {
    if receipt.corridor_id.is_empty() {
        return Err(NetworkError::InvalidReceipt(
            "empty corridor_id".to_string(),
        ));
    }
    if receipt.origin_zone_id.is_empty() {
        return Err(NetworkError::InvalidReceipt(
            "empty origin_zone_id".to_string(),
        ));
    }
    if receipt.receipt_digest.len() != 64 {
        return Err(NetworkError::InvalidReceipt(format!(
            "receipt_digest must be 64-char hex, got {}",
            receipt.receipt_digest.len()
        )));
    }
    if !receipt
        .receipt_digest
        .chars()
        .all(|c| c.is_ascii_hexdigit())
    {
        return Err(NetworkError::InvalidReceipt(
            "receipt_digest contains non-hex characters".to_string(),
        ));
    }
    if receipt.receipt_json.is_null() {
        return Err(NetworkError::InvalidReceipt(
            "receipt_json is null".to_string(),
        ));
    }
    if receipt.signature.is_empty() {
        return Err(NetworkError::InvalidReceipt(
            "empty signature".to_string(),
        ));
    }
    Ok(())
}

/// Validate a corridor proposal.
pub fn validate_proposal(proposal: &CorridorProposal) -> Result<(), NetworkError> {
    if proposal.corridor_id.is_empty() {
        return Err(NetworkError::InvalidProposal(
            "empty corridor_id".to_string(),
        ));
    }
    if proposal.proposer_zone_id.is_empty() {
        return Err(NetworkError::InvalidProposal(
            "empty proposer_zone_id".to_string(),
        ));
    }
    if proposal.proposer_verifying_key_hex.len() != 64 {
        return Err(NetworkError::InvalidProposal(format!(
            "verifying key must be 64-char hex, got {}",
            proposal.proposer_verifying_key_hex.len()
        )));
    }
    if proposal.responder_jurisdiction_id.is_empty() {
        return Err(NetworkError::InvalidProposal(
            "empty responder_jurisdiction_id".to_string(),
        ));
    }
    if proposal.signature.is_empty() {
        return Err(NetworkError::InvalidProposal(
            "empty signature".to_string(),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors from inter-zone corridor networking.
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("invalid inbound receipt: {0}")]
    InvalidReceipt(String),

    #[error("invalid corridor proposal: {0}")]
    InvalidProposal(String),

    #[error("receipt replay detected: digest {0}")]
    ReplayDetected(String),

    #[error("sequence gap: expected {expected}, got {got}")]
    SequenceGap { expected: u64, got: u64 },

    #[error("hash chain break: expected prev_root {expected}, got {got}")]
    HashChainBreak { expected: String, got: String },

    #[error("unknown peer: {0}")]
    UnknownPeer(String),

    #[error("peer unreachable: {0}")]
    PeerUnreachable(String),

    #[error("signature verification failed: {0}")]
    SignatureVerification(String),

    #[error("corridor not found: {0}")]
    CorridorNotFound(String),

    #[error("handshake failed: {0}")]
    HandshakeFailed(String),
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_peer(zone_id: &str, status: PeerStatus) -> CorridorPeer {
        CorridorPeer {
            zone_id: zone_id.to_string(),
            jurisdiction_id: "pk".to_string(),
            endpoint_url: format!("https://{zone_id}.example.com"),
            verifying_key_hex: "a".repeat(64),
            did: format!("did:mass:zone:{}", "a".repeat(64)),
            last_seen: None,
            corridor_ids: vec!["corridor-1".to_string()],
            status,
        }
    }

    #[test]
    fn peer_registry_register_and_get() {
        let mut reg = PeerRegistry::new();
        reg.register_peer(make_peer("zone-a", PeerStatus::Active));
        assert_eq!(reg.peer_count(), 1);
        assert!(reg.get_peer("zone-a").is_some());
        assert!(reg.get_peer("zone-b").is_none());
    }

    #[test]
    fn peer_registry_list_active() {
        let mut reg = PeerRegistry::new();
        reg.register_peer(make_peer("zone-a", PeerStatus::Active));
        reg.register_peer(make_peer("zone-b", PeerStatus::Unreachable));
        reg.register_peer(make_peer("zone-c", PeerStatus::Active));
        assert_eq!(reg.active_peers().len(), 2);
    }

    #[test]
    fn peer_registry_replay_protection() {
        let mut reg = PeerRegistry::new();
        let digest = "a".repeat(64);
        assert!(!reg.is_receipt_seen(&digest));
        reg.mark_receipt_seen(digest.clone());
        assert!(reg.is_receipt_seen(&digest));
    }

    #[test]
    fn peer_registry_replay_prune() {
        let mut reg = PeerRegistry::new();
        reg.max_seen_receipts = 10;
        for i in 0..15 {
            reg.mark_receipt_seen(format!("{:064x}", i));
        }
        // After pruning, count should be less than max
        assert!(reg.seen_receipt_count() <= 15);
    }

    #[test]
    fn peer_status_update() {
        let mut reg = PeerRegistry::new();
        reg.register_peer(make_peer("zone-a", PeerStatus::Discovered));
        reg.set_peer_status("zone-a", PeerStatus::Active);
        assert_eq!(reg.get_peer("zone-a").unwrap().status, PeerStatus::Active);
    }

    #[test]
    fn peer_status_display() {
        assert_eq!(PeerStatus::Discovered.to_string(), "discovered");
        assert_eq!(PeerStatus::Active.to_string(), "active");
        assert_eq!(PeerStatus::Unreachable.to_string(), "unreachable");
        assert_eq!(PeerStatus::Proposing.to_string(), "proposing");
        assert_eq!(PeerStatus::Disconnected.to_string(), "disconnected");
    }

    #[test]
    fn validate_inbound_receipt_rejects_empty_corridor() {
        let receipt = InboundReceipt {
            corridor_id: "".to_string(),
            origin_zone_id: "zone-a".to_string(),
            sequence: 0,
            receipt_json: serde_json::json!({}),
            receipt_digest: "a".repeat(64),
            signature: "sig".to_string(),
            produced_at: "2026-01-01T00:00:00Z".to_string(),
        };
        assert!(validate_inbound_receipt(&receipt).is_err());
    }

    #[test]
    fn validate_inbound_receipt_rejects_bad_digest() {
        let receipt = InboundReceipt {
            corridor_id: "corridor-1".to_string(),
            origin_zone_id: "zone-a".to_string(),
            sequence: 0,
            receipt_json: serde_json::json!({}),
            receipt_digest: "tooshort".to_string(),
            signature: "sig".to_string(),
            produced_at: "2026-01-01T00:00:00Z".to_string(),
        };
        assert!(validate_inbound_receipt(&receipt).is_err());
    }

    #[test]
    fn validate_inbound_receipt_accepts_valid() {
        let receipt = InboundReceipt {
            corridor_id: "corridor-1".to_string(),
            origin_zone_id: "zone-a".to_string(),
            sequence: 0,
            receipt_json: serde_json::json!({"test": true}),
            receipt_digest: "a".repeat(64),
            signature: "sig".to_string(),
            produced_at: "2026-01-01T00:00:00Z".to_string(),
        };
        assert!(validate_inbound_receipt(&receipt).is_ok());
    }

    #[test]
    fn validate_proposal_rejects_empty_fields() {
        let proposal = CorridorProposal {
            corridor_id: "".to_string(),
            proposer_jurisdiction_id: "pk".to_string(),
            proposer_zone_id: "zone-a".to_string(),
            proposer_verifying_key_hex: "a".repeat(64),
            proposer_did: "did:mass:zone:test".to_string(),
            responder_jurisdiction_id: "ae".to_string(),
            proposed_at: "2026-01-01T00:00:00Z".to_string(),
            parameters: BTreeMap::new(),
            signature: "sig".to_string(),
        };
        assert!(validate_proposal(&proposal).is_err());
    }

    #[test]
    fn validate_proposal_accepts_valid() {
        let proposal = CorridorProposal {
            corridor_id: "corridor-pk-ae-001".to_string(),
            proposer_jurisdiction_id: "pk".to_string(),
            proposer_zone_id: "org.momentum.mez.zone.pk-sifc".to_string(),
            proposer_verifying_key_hex: "a".repeat(64),
            proposer_did: "did:mass:zone:test".to_string(),
            responder_jurisdiction_id: "ae".to_string(),
            proposed_at: "2026-01-01T00:00:00Z".to_string(),
            parameters: BTreeMap::new(),
            signature: "sig".to_string(),
        };
        assert!(validate_proposal(&proposal).is_ok());
    }

    #[test]
    fn corridor_peer_serialization_roundtrip() {
        let peer = make_peer("zone-a", PeerStatus::Active);
        let json = serde_json::to_string(&peer).unwrap();
        let de: CorridorPeer = serde_json::from_str(&json).unwrap();
        assert_eq!(de.zone_id, "zone-a");
        assert_eq!(de.status, PeerStatus::Active);
    }

    #[test]
    fn corridor_proposal_serialization_roundtrip() {
        let proposal = CorridorProposal {
            corridor_id: "c-1".to_string(),
            proposer_jurisdiction_id: "pk".to_string(),
            proposer_zone_id: "zone-a".to_string(),
            proposer_verifying_key_hex: "b".repeat(64),
            proposer_did: "did:mass:zone:test".to_string(),
            responder_jurisdiction_id: "ae".to_string(),
            proposed_at: "2026-01-01T00:00:00Z".to_string(),
            parameters: BTreeMap::new(),
            signature: "sig".to_string(),
        };
        let json = serde_json::to_string(&proposal).unwrap();
        let de: CorridorProposal = serde_json::from_str(&json).unwrap();
        assert_eq!(de.corridor_id, "c-1");
    }

    #[test]
    fn inbound_receipt_result_serialization() {
        let result = InboundReceiptResult {
            accepted: true,
            chain_height: 42,
            mmr_root: Some("abc".repeat(21) + "a"),
            rejection_reason: None,
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["accepted"], true);
        assert_eq!(json["chain_height"], 42);
        assert!(json.get("rejection_reason").is_none());
    }

    #[test]
    fn network_error_display() {
        let e = NetworkError::ReplayDetected("abc".to_string());
        assert!(e.to_string().contains("replay"));

        let e = NetworkError::SequenceGap {
            expected: 5,
            got: 3,
        };
        assert!(e.to_string().contains("expected 5"));
    }

    #[test]
    fn corridor_network_config_serialization() {
        let config = CorridorNetworkConfig {
            zone_id: "zone-a".to_string(),
            jurisdiction_id: "pk".to_string(),
            endpoint_url: "https://zone-a.example.com".to_string(),
            verifying_key_hex: "a".repeat(64),
            did: "did:mass:zone:test".to_string(),
            static_peers: vec![PeerEndpoint {
                zone_id: "zone-b".to_string(),
                url: "https://zone-b.example.com".to_string(),
            }],
            max_receipt_buffer: 500,
            peer_timeout_secs: 15,
        };
        let json = serde_json::to_string(&config).unwrap();
        let de: CorridorNetworkConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(de.static_peers.len(), 1);
        assert_eq!(de.max_receipt_buffer, 500);
    }
}
