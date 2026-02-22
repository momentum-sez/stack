//! # Inter-Zone Corridor Peer Exchange Routes (P0-CORRIDOR-NET-001)
//!
//! API endpoints for zone-to-zone corridor operations:
//!
//! - `POST /v1/corridors/peers/propose`   — Receive a corridor proposal from a peer zone
//! - `POST /v1/corridors/peers/accept`    — Receive a corridor acceptance from a peer zone
//! - `GET  /v1/corridors/peers`           — List known corridor peers
//! - `GET  /v1/corridors/peers/:zone_id`  — Get a specific peer
//! - `POST /v1/corridors/peers/receipts`  — Receive an inbound receipt from a peer zone
//! - `POST /v1/corridors/peers/attestations` — Receive a watcher attestation from a peer

use std::collections::hash_map::Entry;

use axum::extract::{Path, State};
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use mez_core::{CanonicalBytes, ContentDigest, CorridorId};
use mez_corridor::{
    CorridorAcceptance, CorridorPeer, CorridorProposal, CorridorReceipt, InboundAttestation,
    InboundReceipt, InboundReceiptResult, PeerStatus, ReceiptChain, compute_next_root,
    validate_inbound_receipt, validate_proposal,
};
use mez_crypto::{Ed25519Signature, VerifyingKey};

use crate::error::AppError;
use crate::extractors::extract_json;
use crate::state::AppState;

/// Assemble the peer exchange router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/corridors/peers", get(list_peers))
        .route("/v1/corridors/peers/propose", post(propose_corridor))
        .route("/v1/corridors/peers/accept", post(accept_corridor))
        .route("/v1/corridors/peers/receipts", post(receive_receipt))
        .route(
            "/v1/corridors/peers/attestations",
            post(receive_attestation),
        )
        .route("/v1/corridors/peers/:zone_id", get(get_peer))
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response to a corridor proposal.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ProposalResponse {
    /// Whether the proposal was accepted for processing.
    pub received: bool,
    /// Status of the proposal (e.g., "proposing", "rejected").
    pub status: String,
    /// Message for the proposer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Response to a corridor acceptance.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AcceptanceResponse {
    /// Whether the acceptance was processed.
    pub received: bool,
    /// Corridor ID that is now active.
    pub corridor_id: String,
    /// Status of the peer connection.
    pub peer_status: String,
}

/// Summary of a peer for listing.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PeerSummary {
    pub zone_id: String,
    pub jurisdiction_id: String,
    pub endpoint_url: String,
    pub status: String,
    pub corridor_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<String>,
}

impl From<&CorridorPeer> for PeerSummary {
    fn from(peer: &CorridorPeer) -> Self {
        Self {
            zone_id: peer.zone_id.clone(),
            jurisdiction_id: peer.jurisdiction_id.clone(),
            endpoint_url: peer.endpoint_url.clone(),
            status: peer.status.to_string(),
            corridor_ids: peer.corridor_ids.clone(),
            last_seen: peer.last_seen.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Cryptographic verification helpers
// ---------------------------------------------------------------------------

/// Verify an Ed25519 signature over canonical bytes.
///
/// Parses the verifying key and signature from hex, then verifies.
/// Returns `AppError::Validation` on any failure.
fn verify_ed25519(
    canonical: &CanonicalBytes,
    signature_hex: &str,
    verifying_key_hex: &str,
) -> Result<(), AppError> {
    let vk = VerifyingKey::from_hex(verifying_key_hex)
        .map_err(|e| AppError::Validation(format!("invalid verifying key: {e}")))?;
    let sig = Ed25519Signature::from_hex(signature_hex)
        .map_err(|e| AppError::Validation(format!("invalid signature: {e}")))?;
    vk.verify(canonical, &sig)
        .map_err(|e| AppError::Validation(format!("signature verification failed: {e}")))?;
    Ok(())
}

/// Serialize a value to JSON, remove the `"signature"` key, and canonicalize.
///
/// Used for proposals and acceptances where the signed payload is the
/// canonical JSON of the message with the signature field stripped.
fn canonical_without_signature(obj: &impl Serialize) -> Result<CanonicalBytes, AppError> {
    let mut value = serde_json::to_value(obj)
        .map_err(|e| AppError::Internal(format!("serialization failed: {e}")))?;
    if let Some(map) = value.as_object_mut() {
        map.remove("signature");
    }
    CanonicalBytes::from_value(value)
        .map_err(|e| AppError::Validation(format!("canonicalization failed: {e}")))
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// List all known corridor peers.
#[utoipa::path(
    get,
    path = "/v1/corridors/peers",
    responses(
        (status = 200, description = "List of known peers", body = Vec<PeerSummary>),
    ),
    tag = "corridor-peers"
)]
async fn list_peers(State(state): State<AppState>) -> Json<Vec<PeerSummary>> {
    let registry = state.peer_registry.read();
    // Cap to prevent unbounded response payloads.
    const MAX_LIST: usize = 1000;
    let peers: Vec<PeerSummary> = registry.list_peers().iter().take(MAX_LIST).map(|p| PeerSummary::from(*p)).collect();
    Json(peers)
}

/// Get a specific peer by zone ID.
#[utoipa::path(
    get,
    path = "/v1/corridors/peers/{zone_id}",
    params(
        ("zone_id" = String, Path, description = "Zone ID of the peer")
    ),
    responses(
        (status = 200, description = "Peer details", body = PeerSummary),
        (status = 404, description = "Peer not found"),
    ),
    tag = "corridor-peers"
)]
async fn get_peer(
    State(state): State<AppState>,
    Path(zone_id): Path<String>,
) -> Result<Json<PeerSummary>, AppError> {
    let registry = state.peer_registry.read();
    let peer = registry
        .get_peer(&zone_id)
        .ok_or_else(|| AppError::NotFound(format!("peer not found: {zone_id}")))?;
    Ok(Json(PeerSummary::from(peer)))
}

/// Receive a corridor proposal from a peer zone.
///
/// When a remote zone wants to establish a corridor with this zone,
/// it sends a `CorridorProposal`. This endpoint validates the proposal
/// and registers the peer in `Proposing` status.
#[utoipa::path(
    post,
    path = "/v1/corridors/peers/propose",
    request_body = CorridorProposal,
    responses(
        (status = 201, description = "Proposal received", body = ProposalResponse),
        (status = 422, description = "Invalid proposal"),
    ),
    tag = "corridor-peers"
)]
async fn propose_corridor(
    State(state): State<AppState>,
    body: Result<Json<CorridorProposal>, JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    let proposal = extract_json(body)?;

    // Structural validation.
    validate_proposal(&proposal).map_err(|e| AppError::Validation(e.to_string()))?;

    // Verify Ed25519 signature over canonical proposal payload (minus signature field).
    let canonical = canonical_without_signature(&proposal)?;
    verify_ed25519(&canonical, &proposal.signature, &proposal.proposer_verifying_key_hex)?;

    // Register the peer.
    let peer = CorridorPeer {
        zone_id: proposal.proposer_zone_id.clone(),
        jurisdiction_id: proposal.proposer_jurisdiction_id.clone(),
        endpoint_url: String::new(), // Peer should provide endpoint in proposal parameters
        verifying_key_hex: proposal.proposer_verifying_key_hex.clone(),
        did: proposal.proposer_did.clone(),
        last_seen: Some(proposal.proposed_at.clone()),
        corridor_ids: vec![proposal.corridor_id.clone()],
        status: PeerStatus::Proposing,
    };

    let mut registry = state.peer_registry.write();
    registry.register_peer(peer);

    tracing::info!(
        corridor_id = %proposal.corridor_id,
        proposer = %proposal.proposer_zone_id,
        "Corridor proposal received from peer"
    );

    Ok((StatusCode::CREATED, Json(ProposalResponse {
        received: true,
        status: "proposing".to_string(),
        message: Some(format!(
            "Proposal for corridor {} received. Awaiting acceptance.",
            proposal.corridor_id
        )),
    })))
}

/// Receive a corridor acceptance from a peer zone.
///
/// After this zone proposes a corridor to a remote zone, the remote zone
/// responds with a `CorridorAcceptance`. This endpoint activates the peer.
#[utoipa::path(
    post,
    path = "/v1/corridors/peers/accept",
    request_body = CorridorAcceptance,
    responses(
        (status = 201, description = "Acceptance processed", body = AcceptanceResponse),
        (status = 404, description = "Peer not found"),
        (status = 422, description = "Invalid acceptance"),
    ),
    tag = "corridor-peers"
)]
async fn accept_corridor(
    State(state): State<AppState>,
    body: Result<Json<CorridorAcceptance>, JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    let acceptance = extract_json(body)?;

    if acceptance.corridor_id.trim().is_empty() {
        return Err(AppError::Validation("empty corridor_id".to_string()));
    }
    if acceptance.responder_zone_id.trim().is_empty() {
        return Err(AppError::Validation(
            "empty responder_zone_id".to_string(),
        ));
    }

    // Verify Ed25519 signature over canonical acceptance payload (minus signature field).
    let canonical = canonical_without_signature(&acceptance)?;
    verify_ed25519(&canonical, &acceptance.signature, &acceptance.responder_verifying_key_hex)?;

    let mut registry = state.peer_registry.write();

    // Check if we already know this peer (from a prior proposal we sent).
    if let Some(peer) = registry.get_peer_mut(&acceptance.responder_zone_id) {
        peer.status = PeerStatus::Active;
        peer.verifying_key_hex = acceptance.responder_verifying_key_hex.clone();
        peer.did = acceptance.responder_did.clone();
        peer.last_seen = Some(acceptance.accepted_at.clone());
        if !peer.corridor_ids.contains(&acceptance.corridor_id) {
            peer.corridor_ids.push(acceptance.corridor_id.clone());
        }
    } else {
        // New peer — register as Active directly.
        let peer = CorridorPeer {
            zone_id: acceptance.responder_zone_id.clone(),
            jurisdiction_id: String::new(), // Unknown until proposal exchange
            endpoint_url: String::new(),
            verifying_key_hex: acceptance.responder_verifying_key_hex.clone(),
            did: acceptance.responder_did.clone(),
            last_seen: Some(acceptance.accepted_at.clone()),
            corridor_ids: vec![acceptance.corridor_id.clone()],
            status: PeerStatus::Active,
        };
        registry.register_peer(peer);
    }

    // Store the genesis root for receipt chain bootstrapping.
    {
        let mut genesis_roots = state.corridor_genesis_roots.write();
        genesis_roots.insert(
            acceptance.corridor_id.clone(),
            acceptance.genesis_root_hex.clone(),
        );
    }

    tracing::info!(
        corridor_id = %acceptance.corridor_id,
        responder = %acceptance.responder_zone_id,
        "Corridor acceptance received — peer activated"
    );

    Ok((StatusCode::CREATED, Json(AcceptanceResponse {
        received: true,
        corridor_id: acceptance.corridor_id,
        peer_status: "active".to_string(),
    })))
}

/// Receive an inbound receipt from a peer zone.
///
/// Validates the receipt structure, checks for replay, verifies the Ed25519
/// signature against the peer's verifying key, recomputes and verifies
/// `next_root` (I-RECEIPT-COMMIT), and appends to the local receipt chain
/// mirror (I-RECEIPT-LINK).
#[utoipa::path(
    post,
    path = "/v1/corridors/peers/receipts",
    request_body = InboundReceipt,
    responses(
        (status = 200, description = "Receipt processed", body = InboundReceiptResult),
        (status = 409, description = "Replay detected"),
        (status = 422, description = "Invalid receipt"),
    ),
    tag = "corridor-peers"
)]
async fn receive_receipt(
    State(state): State<AppState>,
    body: Result<Json<InboundReceipt>, JsonRejection>,
) -> Result<Json<InboundReceiptResult>, AppError> {
    let receipt = extract_json(body)?;

    // Structural validation.
    validate_inbound_receipt(&receipt).map_err(|e| AppError::Validation(e.to_string()))?;

    let mut registry = state.peer_registry.write();

    // Check the origin zone is a known active peer.
    let peer = registry
        .get_peer(&receipt.origin_zone_id)
        .ok_or_else(|| {
            AppError::Validation(format!("unknown peer zone: {}", receipt.origin_zone_id))
        })?;

    if peer.status != PeerStatus::Active {
        return Err(AppError::Validation(format!(
            "peer {} is not active (status: {})",
            receipt.origin_zone_id, peer.status
        )));
    }

    // Clone the peer's verifying key before releasing the borrow on registry.
    let peer_vk_hex = peer.verifying_key_hex.clone();

    // Replay protection.
    if registry.is_receipt_seen(&receipt.receipt_digest) {
        return Err(AppError::Conflict(format!(
            "receipt replay detected: digest {}",
            receipt.receipt_digest
        )));
    }

    // Verify Ed25519 signature over the receipt digest.
    let digest_canonical = CanonicalBytes::new(&receipt.receipt_digest)
        .map_err(|e| AppError::Validation(format!("canonicalization failed: {e}")))?;
    verify_ed25519(&digest_canonical, &receipt.signature, &peer_vk_hex)?;

    // Deserialize the receipt JSON into a CorridorReceipt.
    let corridor_receipt: CorridorReceipt =
        serde_json::from_value(receipt.receipt_json.clone())
            .map_err(|e| AppError::Validation(format!("invalid receipt_json: {e}")))?;

    // Verify next_root: recompute and compare (I-RECEIPT-COMMIT).
    let computed_next_root = compute_next_root(&corridor_receipt)
        .map_err(|e| AppError::Validation(format!("next_root computation failed: {e}")))?;
    if computed_next_root.to_hex() != corridor_receipt.next_root {
        return Err(AppError::Validation(format!(
            "next_root mismatch: computed {} but receipt declares {}",
            computed_next_root.to_hex(),
            corridor_receipt.next_root
        )));
    }

    // Append to local receipt chain mirror.
    let corridor_uuid = Uuid::parse_str(&receipt.corridor_id)
        .map_err(|e| AppError::Validation(format!("invalid corridor_id UUID: {e}")))?;

    let (chain_height, mmr_root) = {
        let mut chains = state.receipt_chains.write();
        let chain = match chains.entry(corridor_uuid) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => {
                // Bootstrap a new chain. Genesis root from the acceptance record,
                // or from the first receipt's prev_root (which IS the genesis root
                // for sequence 0).
                let genesis_hex = {
                    let genesis_roots = state.corridor_genesis_roots.read();
                    genesis_roots.get(&receipt.corridor_id).cloned()
                }
                .unwrap_or_else(|| corridor_receipt.prev_root.clone());

                let genesis_root = ContentDigest::from_hex(&genesis_hex).map_err(|e| {
                    AppError::Validation(format!("invalid genesis root hex: {e}"))
                })?;

                e.insert(ReceiptChain::new(
                    CorridorId::from_uuid(corridor_uuid),
                    genesis_root,
                ))
            }
        };

        chain
            .append(corridor_receipt)
            .map_err(|e| AppError::Validation(format!("receipt chain append failed: {e}")))?;

        let height = chain.height();
        let mmr_root = chain.mmr_root().ok();
        (height, mmr_root)
    };

    // Mark receipt as seen (still under the registry write lock).
    registry.mark_receipt_seen(receipt.receipt_digest.clone());

    tracing::info!(
        corridor_id = %receipt.corridor_id,
        origin = %receipt.origin_zone_id,
        sequence = receipt.sequence,
        chain_height,
        "Inbound receipt accepted from peer"
    );

    Ok(Json(InboundReceiptResult {
        accepted: true,
        chain_height,
        mmr_root,
        rejection_reason: None,
    }))
}

/// Receive a watcher attestation from a peer zone.
///
/// Watcher attestations are cryptographic confirmations of receipt chain state
/// from independent watchers. They are used in fork resolution.
#[utoipa::path(
    post,
    path = "/v1/corridors/peers/attestations",
    request_body = InboundAttestation,
    responses(
        (status = 200, description = "Attestation received"),
        (status = 422, description = "Invalid attestation"),
    ),
    tag = "corridor-peers"
)]
async fn receive_attestation(
    State(state): State<AppState>,
    body: Result<Json<InboundAttestation>, JsonRejection>,
) -> Result<Json<serde_json::Value>, AppError> {
    let attestation = extract_json(body)?;

    if attestation.corridor_id.trim().is_empty() {
        return Err(AppError::Validation(
            "empty corridor_id".to_string(),
        ));
    }
    if attestation.watcher_id.trim().is_empty() {
        return Err(AppError::Validation(
            "empty watcher_id".to_string(),
        ));
    }
    if attestation.signature.trim().is_empty() {
        return Err(AppError::Validation(
            "empty signature".to_string(),
        ));
    }

    // Look up the watcher's verifying key from the peer registry.
    let watcher_vk_hex = {
        let registry = state.peer_registry.read();
        let watcher_peer = registry.get_peer(&attestation.watcher_id).ok_or_else(|| {
            AppError::Validation(format!("unknown watcher: {}", attestation.watcher_id))
        })?;
        watcher_peer.verifying_key_hex.clone()
    };

    // Verify watcher signature over canonical attestation message.
    let canonical_msg = format!(
        "{}:{}:{}",
        attestation.corridor_id, attestation.attested_height, attestation.attested_root_hex,
    );
    let canonical = CanonicalBytes::new(&canonical_msg)
        .map_err(|e| AppError::Validation(format!("canonicalization failed: {e}")))?;
    verify_ed25519(&canonical, &attestation.signature, &watcher_vk_hex)?;

    // Store attestation for fork resolution input.
    {
        let mut log = state.attestation_log.write();
        log.entry(attestation.corridor_id.clone())
            .or_default()
            .push(attestation.clone());
    }

    tracing::info!(
        corridor_id = %attestation.corridor_id,
        watcher_id = %attestation.watcher_id,
        height = attestation.attested_height,
        "Watcher attestation received"
    );

    Ok(Json(serde_json::json!({
        "received": true,
        "corridor_id": attestation.corridor_id,
        "watcher_id": attestation.watcher_id,
        "attested_height": attestation.attested_height,
    })))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use mez_core::{sha256_digest, Timestamp};
    use mez_corridor::DigestEntry;
    use mez_crypto::SigningKey;
    use tower::ServiceExt;

    fn test_app() -> Router {
        router().with_state(AppState::new())
    }

    /// Generate an Ed25519 keypair and return (signing_key, verifying_key_hex).
    fn generate_keypair() -> (SigningKey, String) {
        let sk = SigningKey::generate(&mut rand_core::OsRng);
        let vk_hex = sk.verifying_key().to_hex();
        (sk, vk_hex)
    }

    /// Sign a JSON payload: strip `"signature"` key, canonicalize, sign, set signature.
    fn sign_json_payload(sk: &SigningKey, payload: &mut serde_json::Value) {
        let mut for_signing = payload.clone();
        if let Some(map) = for_signing.as_object_mut() {
            map.remove("signature");
        }
        let canonical = CanonicalBytes::from_value(for_signing).unwrap();
        let sig = sk.sign(&canonical);
        payload["signature"] = serde_json::Value::String(sig.to_hex());
    }

    /// Build a valid CorridorReceipt with correct next_root for testing.
    fn build_test_receipt(
        corridor_uuid: Uuid,
        genesis_root_hex: &str,
    ) -> CorridorReceipt {
        let mut receipt = CorridorReceipt {
            receipt_type: "MEZCorridorStateReceipt".to_string(),
            corridor_id: CorridorId::from_uuid(corridor_uuid),
            sequence: 0,
            timestamp: Timestamp::now(),
            prev_root: genesis_root_hex.to_string(),
            next_root: String::new(),
            lawpack_digest_set: vec![DigestEntry::from(
                "aa".repeat(32), // 64 hex chars
            )],
            ruleset_digest_set: vec![DigestEntry::from(
                "bb".repeat(32),
            )],
            proof: None,
            transition: None,
            transition_type_registry_digest_sha256: None,
            zk: None,
            anchor: None,
        };
        // Seal the next_root to the correct computed value.
        receipt.seal_next_root().unwrap();
        receipt
    }

    /// Build a complete signed InboundReceipt from a CorridorReceipt.
    fn build_signed_inbound_receipt(
        corridor_receipt: &CorridorReceipt,
        corridor_id_str: &str,
        origin_zone_id: &str,
        sk: &SigningKey,
    ) -> serde_json::Value {
        let receipt_json = serde_json::to_value(corridor_receipt).unwrap();
        let receipt_digest =
            sha256_digest(&CanonicalBytes::new(corridor_receipt).unwrap()).to_hex();
        let sig_canonical = CanonicalBytes::new(&receipt_digest).unwrap();
        let signature = sk.sign(&sig_canonical).to_hex();

        serde_json::json!({
            "corridor_id": corridor_id_str,
            "origin_zone_id": origin_zone_id,
            "sequence": corridor_receipt.sequence,
            "receipt_json": receipt_json,
            "receipt_digest": receipt_digest,
            "signature": signature,
            "produced_at": "2026-01-01T00:00:00Z"
        })
    }

    // -- GET endpoints (unchanged behavior) -----------------------------------

    #[tokio::test]
    async fn list_peers_empty() {
        let app = test_app();
        let req = Request::builder()
            .uri("/v1/corridors/peers")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let peers: Vec<PeerSummary> = serde_json::from_slice(&body).unwrap();
        assert!(peers.is_empty());
    }

    #[tokio::test]
    async fn get_peer_not_found() {
        let app = test_app();
        let req = Request::builder()
            .uri("/v1/corridors/peers/nonexistent-zone")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // -- Proposal tests -------------------------------------------------------

    #[tokio::test]
    async fn propose_corridor_valid_signature() {
        let app = test_app();
        let (sk, vk_hex) = generate_keypair();

        let mut proposal = serde_json::json!({
            "corridor_id": "corridor-pk-ae-001",
            "proposer_jurisdiction_id": "pk",
            "proposer_zone_id": "org.momentum.mez.zone.pk-sifc",
            "proposer_verifying_key_hex": vk_hex,
            "proposer_did": "did:mass:zone:test",
            "responder_jurisdiction_id": "ae",
            "proposed_at": "2026-01-01T00:00:00Z",
            "parameters": {},
            "signature": ""
        });
        sign_json_payload(&sk, &mut proposal);

        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/peers/propose")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&proposal).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: ProposalResponse = serde_json::from_slice(&body).unwrap();
        assert!(result.received);
        assert_eq!(result.status, "proposing");
    }

    #[tokio::test]
    async fn propose_corridor_invalid_signature() {
        let app = test_app();
        let (sk, vk_hex) = generate_keypair();

        let mut proposal = serde_json::json!({
            "corridor_id": "corridor-pk-ae-001",
            "proposer_jurisdiction_id": "pk",
            "proposer_zone_id": "org.momentum.mez.zone.pk-sifc",
            "proposer_verifying_key_hex": vk_hex,
            "proposer_did": "did:mass:zone:test",
            "responder_jurisdiction_id": "ae",
            "proposed_at": "2026-01-01T00:00:00Z",
            "parameters": {},
            "signature": ""
        });
        sign_json_payload(&sk, &mut proposal);

        // Tamper with the proposal after signing — signature becomes invalid.
        proposal["proposer_zone_id"] = serde_json::Value::String("tampered-zone".to_string());

        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/peers/propose")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&proposal).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn propose_corridor_invalid_key_length() {
        let app = test_app();
        let proposal = serde_json::json!({
            "corridor_id": "corridor-1",
            "proposer_jurisdiction_id": "pk",
            "proposer_zone_id": "zone-a",
            "proposer_verifying_key_hex": "tooshort",
            "proposer_did": "did:mass:zone:test",
            "responder_jurisdiction_id": "ae",
            "proposed_at": "2026-01-01T00:00:00Z",
            "parameters": {},
            "signature": "sig"
        });

        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/peers/propose")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&proposal).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    // -- Acceptance tests -----------------------------------------------------

    #[tokio::test]
    async fn accept_corridor_creates_active_peer() {
        let state = AppState::new();
        let app = router().with_state(state.clone());
        let (sk, vk_hex) = generate_keypair();

        let mut acceptance = serde_json::json!({
            "corridor_id": "corridor-pk-ae-001",
            "responder_zone_id": "org.momentum.mez.zone.ae-difc",
            "responder_verifying_key_hex": vk_hex,
            "responder_did": "did:mass:zone:responder",
            "genesis_root_hex": "cc".repeat(32),
            "accepted_at": "2026-01-01T00:00:00Z",
            "signature": ""
        });
        sign_json_payload(&sk, &mut acceptance);

        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/peers/accept")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&acceptance).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        // Verify peer was registered.
        let registry = state.peer_registry.read();
        let peer = registry
            .get_peer("org.momentum.mez.zone.ae-difc")
            .unwrap();
        assert_eq!(peer.status, PeerStatus::Active);

        // Verify genesis root was stored.
        let genesis_roots = state.corridor_genesis_roots.read();
        assert_eq!(
            genesis_roots.get("corridor-pk-ae-001"),
            Some(&"cc".repeat(32))
        );
    }

    // -- Receipt tests --------------------------------------------------------

    #[tokio::test]
    async fn receive_receipt_from_unknown_peer_rejected() {
        let app = test_app();

        let receipt = serde_json::json!({
            "corridor_id": "corridor-1",
            "origin_zone_id": "unknown-zone",
            "sequence": 0,
            "receipt_json": {"test": true},
            "receipt_digest": "aa".repeat(32),
            "signature": "sig",
            "produced_at": "2026-01-01T00:00:00Z"
        });

        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/peers/receipts")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&receipt).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn receive_receipt_forged_next_root_rejected() {
        let state = AppState::new();
        let (sk, vk_hex) = generate_keypair();
        let corridor_uuid = Uuid::new_v4();
        let corridor_id = corridor_uuid.to_string();
        let genesis_root_hex = "aa".repeat(32);

        // Register active peer.
        {
            let mut registry = state.peer_registry.write();
            registry.register_peer(CorridorPeer {
                zone_id: "zone-a".to_string(),
                jurisdiction_id: "pk".to_string(),
                endpoint_url: "https://zone-a.example.com".to_string(),
                verifying_key_hex: vk_hex.clone(),
                did: "did:mass:zone:a".to_string(),
                last_seen: None,
                corridor_ids: vec![corridor_id.clone()],
                status: PeerStatus::Active,
            });
        }

        // Build receipt with a forged next_root.
        let mut corridor_receipt = build_test_receipt(corridor_uuid, &genesis_root_hex);
        corridor_receipt.next_root = "ff".repeat(32); // Forged!

        let receipt_json = serde_json::to_value(&corridor_receipt).unwrap();
        let receipt_digest =
            sha256_digest(&CanonicalBytes::new(&corridor_receipt).unwrap()).to_hex();
        let sig_canonical = CanonicalBytes::new(&receipt_digest).unwrap();
        let signature = sk.sign(&sig_canonical).to_hex();

        let inbound = serde_json::json!({
            "corridor_id": corridor_id,
            "origin_zone_id": "zone-a",
            "sequence": 0,
            "receipt_json": receipt_json,
            "receipt_digest": receipt_digest,
            "signature": signature,
            "produced_at": "2026-01-01T00:00:00Z"
        });

        let app = router().with_state(state);
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/peers/receipts")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&inbound).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8_lossy(&body);
        assert!(
            body_str.contains("next_root mismatch"),
            "expected next_root mismatch error, got: {body_str}"
        );
    }

    #[tokio::test]
    async fn receive_receipt_appended_to_chain_returns_real_mmr_root() {
        let state = AppState::new();
        let (sk, vk_hex) = generate_keypair();
        let corridor_uuid = Uuid::new_v4();
        let corridor_id = corridor_uuid.to_string();
        let genesis_root_hex = "aa".repeat(32);

        // Store genesis root.
        {
            let mut genesis_roots = state.corridor_genesis_roots.write();
            genesis_roots.insert(corridor_id.clone(), genesis_root_hex.clone());
        }

        // Register active peer.
        {
            let mut registry = state.peer_registry.write();
            registry.register_peer(CorridorPeer {
                zone_id: "zone-a".to_string(),
                jurisdiction_id: "pk".to_string(),
                endpoint_url: "https://zone-a.example.com".to_string(),
                verifying_key_hex: vk_hex.clone(),
                did: "did:mass:zone:a".to_string(),
                last_seen: None,
                corridor_ids: vec![corridor_id.clone()],
                status: PeerStatus::Active,
            });
        }

        // Build a valid receipt.
        let corridor_receipt = build_test_receipt(corridor_uuid, &genesis_root_hex);
        let inbound = build_signed_inbound_receipt(
            &corridor_receipt,
            &corridor_id,
            "zone-a",
            &sk,
        );

        let app = router().with_state(state.clone());
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/peers/receipts")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&inbound).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: InboundReceiptResult = serde_json::from_slice(&body).unwrap();
        assert!(result.accepted);
        assert_eq!(result.chain_height, 1);
        assert!(
            result.mmr_root.is_some(),
            "expected a real MMR root, got None"
        );
        assert!(
            result.mmr_root.as_ref().unwrap().len() == 64,
            "MMR root should be 64-char hex"
        );

        // Verify the receipt chain was actually created in state.
        let chains = state.receipt_chains.read();
        let chain = chains.get(&corridor_uuid).expect("chain should exist");
        assert_eq!(chain.height(), 1);
    }

    #[tokio::test]
    async fn receive_receipt_replay_rejected() {
        let state = AppState::new();
        let (sk, vk_hex) = generate_keypair();
        let corridor_uuid = Uuid::new_v4();
        let corridor_id = corridor_uuid.to_string();
        let genesis_root_hex = "aa".repeat(32);

        // Store genesis root.
        {
            let mut genesis_roots = state.corridor_genesis_roots.write();
            genesis_roots.insert(corridor_id.clone(), genesis_root_hex.clone());
        }

        // Register active peer.
        {
            let mut registry = state.peer_registry.write();
            registry.register_peer(CorridorPeer {
                zone_id: "zone-a".to_string(),
                jurisdiction_id: "pk".to_string(),
                endpoint_url: "https://zone-a.example.com".to_string(),
                verifying_key_hex: vk_hex.clone(),
                did: "did:mass:zone:a".to_string(),
                last_seen: None,
                corridor_ids: vec![corridor_id.clone()],
                status: PeerStatus::Active,
            });
        }

        // Build a valid signed receipt.
        let corridor_receipt = build_test_receipt(corridor_uuid, &genesis_root_hex);
        let inbound = build_signed_inbound_receipt(
            &corridor_receipt,
            &corridor_id,
            "zone-a",
            &sk,
        );

        // First request — accepted.
        let app = router().with_state(state.clone());
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/peers/receipts")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&inbound).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Second request with same digest — replay rejected.
        let app = router().with_state(state.clone());
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/peers/receipts")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&inbound).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    // -- Attestation tests ----------------------------------------------------

    #[tokio::test]
    async fn receive_attestation_valid_signature_and_stored() {
        let state = AppState::new();
        let (sk, vk_hex) = generate_keypair();

        // Register the watcher as a known peer.
        {
            let mut registry = state.peer_registry.write();
            registry.register_peer(CorridorPeer {
                zone_id: "watcher-001".to_string(),
                jurisdiction_id: "pk".to_string(),
                endpoint_url: "https://watcher.example.com".to_string(),
                verifying_key_hex: vk_hex.clone(),
                did: "did:mass:watcher:001".to_string(),
                last_seen: None,
                corridor_ids: vec!["corridor-1".to_string()],
                status: PeerStatus::Active,
            });
        }

        let corridor_id = "corridor-1";
        let attested_height: u64 = 42;
        let attested_root_hex = "dd".repeat(32);

        // Sign the canonical attestation message.
        let canonical_msg =
            format!("{corridor_id}:{attested_height}:{attested_root_hex}");
        let canonical = CanonicalBytes::new(&canonical_msg).unwrap();
        let signature = sk.sign(&canonical).to_hex();

        let attestation = serde_json::json!({
            "corridor_id": corridor_id,
            "watcher_id": "watcher-001",
            "attested_height": attested_height,
            "attested_root_hex": attested_root_hex,
            "signature": signature,
            "attested_at": "2026-01-01T00:00:00Z"
        });

        let app = router().with_state(state.clone());
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/peers/attestations")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&attestation).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Verify attestation was stored.
        let log = state.attestation_log.read();
        let entries = log.get("corridor-1").expect("attestations should be stored");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].watcher_id, "watcher-001");
        assert_eq!(entries[0].attested_height, 42);
    }

    #[tokio::test]
    async fn receive_attestation_unknown_watcher_rejected() {
        let app = test_app();

        let attestation = serde_json::json!({
            "corridor_id": "corridor-1",
            "watcher_id": "unknown-watcher",
            "attested_height": 42,
            "attested_root_hex": "dd".repeat(32),
            "signature": "aa".repeat(64),
            "attested_at": "2026-01-01T00:00:00Z"
        });

        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/peers/attestations")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&attestation).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn receive_attestation_invalid_signature_rejected() {
        let state = AppState::new();
        let (_sk, vk_hex) = generate_keypair();

        // Register the watcher.
        {
            let mut registry = state.peer_registry.write();
            registry.register_peer(CorridorPeer {
                zone_id: "watcher-001".to_string(),
                jurisdiction_id: "pk".to_string(),
                endpoint_url: "https://watcher.example.com".to_string(),
                verifying_key_hex: vk_hex,
                did: "did:mass:watcher:001".to_string(),
                last_seen: None,
                corridor_ids: vec!["corridor-1".to_string()],
                status: PeerStatus::Active,
            });
        }

        // Use a different key to sign — signature won't match the registered key.
        let (wrong_sk, _) = generate_keypair();
        let canonical_msg = "corridor-1:42:".to_string() + &"dd".repeat(32);
        let canonical = CanonicalBytes::new(&canonical_msg).unwrap();
        let bad_signature = wrong_sk.sign(&canonical).to_hex();

        let attestation = serde_json::json!({
            "corridor_id": "corridor-1",
            "watcher_id": "watcher-001",
            "attested_height": 42,
            "attested_root_hex": "dd".repeat(32),
            "signature": bad_signature,
            "attested_at": "2026-01-01T00:00:00Z"
        });

        let app = router().with_state(state);
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/peers/attestations")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&attestation).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
