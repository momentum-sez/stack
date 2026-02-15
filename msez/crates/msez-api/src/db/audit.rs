//! Audit event persistence — immutable hash chain.
//!
//! Every state mutation (corridor transition, asset creation, attestation
//! update) appends an audit event with a SHA-256 hash chaining to the
//! previous event. This forms a tamper-evident log.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// An audit event to be persisted.
pub struct AuditEvent {
    pub event_type: String,
    pub actor_did: Option<String>,
    pub resource_type: String,
    pub resource_id: Uuid,
    pub action: String,
    pub metadata: serde_json::Value,
}

/// Append an audit event to the immutable log.
///
/// Computes the event hash by chaining with the previous event's hash.
/// If no previous event exists, the chain starts with a zero hash.
pub async fn append(pool: &PgPool, event: AuditEvent) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();

    // Fetch the most recent event hash for chain integrity.
    let previous_hash: Option<String> =
        sqlx::query_scalar("SELECT event_hash FROM audit_events ORDER BY created_at DESC LIMIT 1")
            .fetch_optional(pool)
            .await?;

    let prev = previous_hash
        .as_deref()
        .unwrap_or("0000000000000000000000000000000000000000000000000000000000000000");

    // Compute event hash via CanonicalBytes — deterministic JSON canonicalization
    // with sorted keys, not fragile string concatenation. Keys are alphabetically
    // sorted in the json!() literal to match JCS output for readability.
    let hash_object = serde_json::json!({
        "action": &event.action,
        "event_type": &event.event_type,
        "previous_hash": prev,
        "resource_id": event.resource_id.to_string(),
        "resource_type": &event.resource_type,
    });
    let canonical = msez_core::CanonicalBytes::new(&hash_object)
        .map_err(|e| sqlx::Error::Protocol(format!("canonical bytes error: {e}")))?;
    let event_hash = msez_core::sha256_digest(&canonical).to_hex();

    sqlx::query(
        "INSERT INTO audit_events (id, event_type, actor_did, resource_type, resource_id,
         action, metadata, previous_hash, event_hash, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())",
    )
    .bind(id)
    .bind(&event.event_type)
    .bind(&event.actor_did)
    .bind(&event.resource_type)
    .bind(event.resource_id)
    .bind(&event.action)
    .bind(&event.metadata)
    .bind(prev)
    .bind(&event_hash)
    .execute(pool)
    .await?;

    Ok(id)
}

/// Query audit events for a specific resource.
pub async fn events_for_resource(
    pool: &PgPool,
    resource_type: &str,
    resource_id: Uuid,
) -> Result<Vec<AuditEventRow>, sqlx::Error> {
    sqlx::query_as::<_, AuditEventRow>(
        "SELECT id, event_type, actor_did, resource_type, resource_id,
         action, metadata, previous_hash, event_hash, created_at
         FROM audit_events
         WHERE resource_type = $1 AND resource_id = $2
         ORDER BY created_at ASC",
    )
    .bind(resource_type)
    .bind(resource_id)
    .fetch_all(pool)
    .await
}

/// Verify audit chain integrity by checking hash continuity.
pub async fn verify_chain_integrity(
    pool: &PgPool,
    limit: i64,
) -> Result<ChainIntegrityResult, sqlx::Error> {
    let events = sqlx::query_as::<_, AuditEventRow>(
        "SELECT id, event_type, actor_did, resource_type, resource_id,
         action, metadata, previous_hash, event_hash, created_at
         FROM audit_events ORDER BY created_at ASC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let total = events.len();
    let mut broken_links = 0;
    let mut last_hash: Option<&str> = None;

    for event in &events {
        if let Some(expected_prev) = last_hash {
            if event.previous_hash.as_deref() != Some(expected_prev) {
                broken_links += 1;
            }
        }
        last_hash = Some(&event.event_hash);
    }

    Ok(ChainIntegrityResult {
        total_events: total,
        broken_links,
        chain_valid: broken_links == 0,
    })
}

/// Result of chain integrity verification.
pub struct ChainIntegrityResult {
    pub total_events: usize,
    pub broken_links: usize,
    pub chain_valid: bool,
}

/// Database row for audit events.
#[derive(sqlx::FromRow)]
pub struct AuditEventRow {
    pub id: Uuid,
    pub event_type: String,
    pub actor_did: Option<String>,
    pub resource_type: String,
    pub resource_id: Uuid,
    pub action: String,
    pub metadata: serde_json::Value,
    pub previous_hash: Option<String>,
    pub event_hash: String,
    pub created_at: DateTime<Utc>,
}

