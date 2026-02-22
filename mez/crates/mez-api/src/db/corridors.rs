//! Corridor persistence operations.
//!
//! All functions take a `&PgPool` and operate on the `corridors` table.
//! State machine constraints are enforced at the application layer
//! (via `DynCorridorState::valid_transitions()`), not in SQL.

use chrono::{DateTime, Utc};
use mez_state::{DynCorridorState, TransitionRecord};
use sqlx::PgPool;
use uuid::Uuid;

use crate::state::CorridorRecord;

/// Insert a new corridor record.
pub async fn insert(pool: &PgPool, record: &CorridorRecord) -> Result<(), sqlx::Error> {
    let status = serde_json::to_value(record.state)
        .map_err(|e| sqlx::Error::Protocol(format!("failed to serialize corridor state: {e}")))?
        .as_str()
        .map(String::from)
        .unwrap_or_else(|| format!("{:?}", record.state));
    let transition_log = serde_json::to_value(&record.transition_log)
        .map_err(|e| sqlx::Error::Protocol(format!("failed to serialize corridor transition_log: {e}")))?;

    sqlx::query(
        "INSERT INTO corridors (id, jurisdiction_a, jurisdiction_b, status, transition_log, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)"
    )
    .bind(record.id)
    .bind(&record.jurisdiction_a)
    .bind(&record.jurisdiction_b)
    .bind(&status)
    .bind(&transition_log)
    .bind(record.created_at)
    .bind(record.updated_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Update corridor state and transition log.
pub async fn update_state(
    pool: &PgPool,
    id: Uuid,
    state: &DynCorridorState,
    transition_log: &[TransitionRecord],
    updated_at: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    let status = serde_json::to_value(state)
        .map_err(|e| sqlx::Error::Protocol(format!("failed to serialize corridor state: {e}")))?
        .as_str()
        .map(String::from)
        .unwrap_or_else(|| format!("{state}"));
    let log_json = serde_json::to_value(transition_log)
        .map_err(|e| sqlx::Error::Protocol(format!("failed to serialize corridor transition_log: {e}")))?;

    let result = sqlx::query(
        "UPDATE corridors SET status = $1, transition_log = $2, updated_at = $3 WHERE id = $4",
    )
    .bind(&status)
    .bind(&log_json)
    .bind(updated_at)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Fetch a corridor by ID.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<CorridorRecord>, sqlx::Error> {
    let row = sqlx::query_as::<_, CorridorRow>(
        "SELECT id, jurisdiction_a, jurisdiction_b, status, transition_log, created_at, updated_at
         FROM corridors WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(CorridorRow::into_record))
}

/// List corridors with pagination.
pub async fn list(
    pool: &PgPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<CorridorRecord>, sqlx::Error> {
    let rows = sqlx::query_as::<_, CorridorRow>(
        "SELECT id, jurisdiction_a, jurisdiction_b, status, transition_log, created_at, updated_at
         FROM corridors ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(CorridorRow::into_record).collect())
}

/// Load all corridors from the database into the in-memory store on startup.
pub async fn load_all(pool: &PgPool) -> Result<Vec<CorridorRecord>, sqlx::Error> {
    let rows = sqlx::query_as::<_, CorridorRow>(
        "SELECT id, jurisdiction_a, jurisdiction_b, status, transition_log, created_at, updated_at
         FROM corridors ORDER BY created_at",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(CorridorRow::into_record).collect())
}

/// Internal row type for SQLx mapping.
#[derive(sqlx::FromRow)]
struct CorridorRow {
    id: Uuid,
    jurisdiction_a: String,
    jurisdiction_b: String,
    status: String,
    transition_log: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl CorridorRow {
    fn into_record(self) -> CorridorRecord {
        let state: DynCorridorState = serde_json::from_value(serde_json::Value::String(
            self.status.clone(),
        ))
        .unwrap_or_else(|e| {
            tracing::warn!(
                id = %self.id,
                status = %self.status,
                error = %e,
                "unknown corridor state in database, defaulting to Draft"
            );
            DynCorridorState::Draft
        });

        let transition_log: Vec<TransitionRecord> =
            serde_json::from_value(self.transition_log.clone()).unwrap_or_else(|e| {
                tracing::warn!(
                    id = %self.id,
                    error = %e,
                    "failed to deserialize corridor transition_log, defaulting to empty"
                );
                Vec::new()
            });

        CorridorRecord {
            id: self.id,
            jurisdiction_a: self.jurisdiction_a,
            jurisdiction_b: self.jurisdiction_b,
            state,
            transition_log,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
