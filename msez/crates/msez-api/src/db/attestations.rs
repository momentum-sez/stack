//! Attestation persistence operations.
//!
//! All functions take a `&PgPool` and operate on the `attestations` table.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::state::{AttestationRecord, AttestationStatus};

/// Insert a new attestation record.
pub async fn insert(pool: &PgPool, record: &AttestationRecord) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO attestations (id, entity_id, attestation_type, issuer, status,
         jurisdiction_id, issued_at, expires_at, details)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(record.id)
    .bind(record.entity_id)
    .bind(&record.attestation_type)
    .bind(&record.issuer)
    .bind(record.status.as_str())
    .bind(&record.jurisdiction_id)
    .bind(record.issued_at)
    .bind(record.expires_at)
    .bind(&record.details)
    .execute(pool)
    .await?;

    Ok(())
}

/// Update attestation status.
pub async fn update_status(
    pool: &PgPool,
    id: Uuid,
    status: AttestationStatus,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("UPDATE attestations SET status = $1 WHERE id = $2")
        .bind(status.as_str())
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Fetch an attestation by ID.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<AttestationRecord>, sqlx::Error> {
    let row = sqlx::query_as::<_, AttestationRow>(
        "SELECT id, entity_id, attestation_type, issuer, status,
         jurisdiction_id, issued_at, expires_at, details
         FROM attestations WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(AttestationRow::into_record))
}

/// List attestations by entity ID.
pub async fn list_by_entity(
    pool: &PgPool,
    entity_id: Uuid,
) -> Result<Vec<AttestationRecord>, sqlx::Error> {
    let rows = sqlx::query_as::<_, AttestationRow>(
        "SELECT id, entity_id, attestation_type, issuer, status,
         jurisdiction_id, issued_at, expires_at, details
         FROM attestations WHERE entity_id = $1 ORDER BY issued_at DESC",
    )
    .bind(entity_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(AttestationRow::into_record).collect())
}

/// Load all attestations from the database into the in-memory store on startup.
pub async fn load_all(pool: &PgPool) -> Result<Vec<AttestationRecord>, sqlx::Error> {
    let rows = sqlx::query_as::<_, AttestationRow>(
        "SELECT id, entity_id, attestation_type, issuer, status,
         jurisdiction_id, issued_at, expires_at, details
         FROM attestations ORDER BY issued_at",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(AttestationRow::into_record).collect())
}

fn parse_attestation_status(s: &str) -> AttestationStatus {
    match s {
        "ACTIVE" => AttestationStatus::Active,
        "PENDING" => AttestationStatus::Pending,
        "REVOKED" => AttestationStatus::Revoked,
        "EXPIRED" => AttestationStatus::Expired,
        _ => AttestationStatus::Pending,
    }
}

/// Internal row type for SQLx mapping.
#[derive(sqlx::FromRow)]
struct AttestationRow {
    id: Uuid,
    entity_id: Uuid,
    attestation_type: String,
    issuer: String,
    status: String,
    jurisdiction_id: String,
    issued_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    details: serde_json::Value,
}

impl AttestationRow {
    fn into_record(self) -> AttestationRecord {
        AttestationRecord {
            id: self.id,
            entity_id: self.entity_id,
            attestation_type: self.attestation_type,
            issuer: self.issuer,
            status: parse_attestation_status(&self.status),
            jurisdiction_id: self.jurisdiction_id,
            issued_at: self.issued_at,
            expires_at: self.expires_at,
            details: self.details,
        }
    }
}
