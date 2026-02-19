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

use axum::extract::{Path, State};
use axum::extract::rejection::JsonRejection;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use mez_corridor::{
    CorridorAcceptance, CorridorPeer, CorridorProposal, InboundAttestation,
    InboundReceipt, InboundReceiptResult, PeerStatus, validate_inbound_receipt, validate_proposal,
};

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
    let peers: Vec<PeerSummary> = registry.list_peers().iter().map(|p| PeerSummary::from(*p)).collect();
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
        (status = 200, description = "Proposal received", body = ProposalResponse),
        (status = 422, description = "Invalid proposal"),
    ),
    tag = "corridor-peers"
)]
async fn propose_corridor(
    State(state): State<AppState>,
    body: Result<Json<CorridorProposal>, JsonRejection>,
) -> Result<Json<ProposalResponse>, AppError> {
    let proposal = extract_json(body)?;

    // Structural validation.
    validate_proposal(&proposal).map_err(|e| AppError::Validation(e.to_string()))?;

    // TODO: Verify Ed25519 signature over canonical proposal payload.
    // For Phase 1, we accept structurally valid proposals and mark as Proposing.

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

    Ok(Json(ProposalResponse {
        received: true,
        status: "proposing".to_string(),
        message: Some(format!(
            "Proposal for corridor {} received. Awaiting acceptance.",
            proposal.corridor_id
        )),
    }))
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
        (status = 200, description = "Acceptance processed", body = AcceptanceResponse),
        (status = 404, description = "Peer not found"),
        (status = 422, description = "Invalid acceptance"),
    ),
    tag = "corridor-peers"
)]
async fn accept_corridor(
    State(state): State<AppState>,
    body: Result<Json<CorridorAcceptance>, JsonRejection>,
) -> Result<Json<AcceptanceResponse>, AppError> {
    let acceptance = extract_json(body)?;

    if acceptance.corridor_id.is_empty() {
        return Err(AppError::Validation("empty corridor_id".to_string()));
    }
    if acceptance.responder_zone_id.is_empty() {
        return Err(AppError::Validation(
            "empty responder_zone_id".to_string(),
        ));
    }

    // TODO: Verify Ed25519 signature over canonical acceptance payload.

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

    tracing::info!(
        corridor_id = %acceptance.corridor_id,
        responder = %acceptance.responder_zone_id,
        "Corridor acceptance received — peer activated"
    );

    Ok(Json(AcceptanceResponse {
        received: true,
        corridor_id: acceptance.corridor_id,
        peer_status: "active".to_string(),
    }))
}

/// Receive an inbound receipt from a peer zone.
///
/// Validates the receipt structure, checks for replay, and records it.
/// Full Ed25519 signature verification is a Phase 2 feature.
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

    // Replay protection.
    if registry.is_receipt_seen(&receipt.receipt_digest) {
        return Err(AppError::Conflict(format!(
            "receipt replay detected: digest {}",
            receipt.receipt_digest
        )));
    }

    // TODO: Verify Ed25519 signature against peer's verifying key.
    // TODO: Verify receipt.next_root == SHA256(JCS(payload)).
    // TODO: Append to local receipt chain mirror.

    // Mark receipt as seen.
    registry.mark_receipt_seen(receipt.receipt_digest.clone());

    tracing::info!(
        corridor_id = %receipt.corridor_id,
        origin = %receipt.origin_zone_id,
        sequence = receipt.sequence,
        "Inbound receipt accepted from peer"
    );

    Ok(Json(InboundReceiptResult {
        accepted: true,
        chain_height: receipt.sequence + 1,
        mmr_root: None, // TODO: Return actual MMR root after chain append
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

    if attestation.corridor_id.is_empty() {
        return Err(AppError::Validation(
            "empty corridor_id".to_string(),
        ));
    }
    if attestation.watcher_id.is_empty() {
        return Err(AppError::Validation(
            "empty watcher_id".to_string(),
        ));
    }
    if attestation.signature.is_empty() {
        return Err(AppError::Validation(
            "empty signature".to_string(),
        ));
    }

    // Verify the peer is known.
    {
        let registry = state.peer_registry.read();
        // Attestation can come from any known peer or watcher.
        // For now, just log it. Full watcher registry verification is Phase 2.
        let _ = registry.peer_count(); // Ensure registry is accessible
    }

    // TODO: Verify watcher signature against registered watcher key.
    // TODO: Store attestation for fork resolution input.

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
    use tower::ServiceExt;

    fn test_app() -> Router {
        router().with_state(AppState::new())
    }

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

    #[tokio::test]
    async fn propose_corridor_valid() {
        let app = test_app();
        let proposal = serde_json::json!({
            "corridor_id": "corridor-pk-ae-001",
            "proposer_jurisdiction_id": "pk",
            "proposer_zone_id": "org.momentum.mez.zone.pk-sifc",
            "proposer_verifying_key_hex": "a".repeat(64),
            "proposer_did": "did:mass:zone:test",
            "responder_jurisdiction_id": "ae",
            "proposed_at": "2026-01-01T00:00:00Z",
            "parameters": {},
            "signature": "sig123"
        });

        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/peers/propose")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&proposal).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: ProposalResponse = serde_json::from_slice(&body).unwrap();
        assert!(result.received);
        assert_eq!(result.status, "proposing");
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

    #[tokio::test]
    async fn accept_corridor_creates_active_peer() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        let acceptance = serde_json::json!({
            "corridor_id": "corridor-pk-ae-001",
            "responder_zone_id": "org.momentum.mez.zone.ae-difc",
            "responder_verifying_key_hex": "b".repeat(64),
            "responder_did": "did:mass:zone:responder",
            "genesis_root_hex": "c".repeat(64),
            "accepted_at": "2026-01-01T00:00:00Z",
            "signature": "sig456"
        });

        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/peers/accept")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&acceptance).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Verify peer was registered.
        let registry = state.peer_registry.read();
        let peer = registry
            .get_peer("org.momentum.mez.zone.ae-difc")
            .unwrap();
        assert_eq!(peer.status, PeerStatus::Active);
    }

    #[tokio::test]
    async fn receive_receipt_from_unknown_peer_rejected() {
        let app = test_app();

        let receipt = serde_json::json!({
            "corridor_id": "corridor-1",
            "origin_zone_id": "unknown-zone",
            "sequence": 0,
            "receipt_json": {"test": true},
            "receipt_digest": "a".repeat(64),
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
    async fn receive_receipt_replay_rejected() {
        let state = AppState::new();

        // Register an active peer first.
        {
            let mut registry = state.peer_registry.write();
            registry.register_peer(CorridorPeer {
                zone_id: "zone-a".to_string(),
                jurisdiction_id: "pk".to_string(),
                endpoint_url: "https://zone-a.example.com".to_string(),
                verifying_key_hex: "a".repeat(64),
                did: "did:mass:zone:a".to_string(),
                last_seen: None,
                corridor_ids: vec!["corridor-1".to_string()],
                status: PeerStatus::Active,
            });
        }

        let receipt = serde_json::json!({
            "corridor_id": "corridor-1",
            "origin_zone_id": "zone-a",
            "sequence": 0,
            "receipt_json": {"test": true},
            "receipt_digest": "a".repeat(64),
            "signature": "sig",
            "produced_at": "2026-01-01T00:00:00Z"
        });

        // First request — accepted.
        let app = router().with_state(state.clone());
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/peers/receipts")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&receipt).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Second request with same digest — replay rejected.
        let app = router().with_state(state.clone());
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/peers/receipts")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&receipt).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn receive_attestation_valid() {
        let app = test_app();

        let attestation = serde_json::json!({
            "corridor_id": "corridor-1",
            "watcher_id": "watcher-001",
            "attested_height": 42,
            "attested_root_hex": "d".repeat(64),
            "signature": "sig789",
            "attested_at": "2026-01-01T00:00:00Z"
        });

        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/peers/attestations")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&attestation).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
