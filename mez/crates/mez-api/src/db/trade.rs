// SPDX-License-Identifier: BUSL-1.1
//! Trade flow persistence operations.
//!
//! Provides save/load functions for trade flows and their transitions.
//! Follows the same pattern as `mass_primitives.rs`.

use chrono::{DateTime, Utc};
use mez_corridor::{
    TradeFlowRecord, TradeFlowState, TradeFlowType, TradeTransitionRecord,
};
use sqlx::PgPool;
use uuid::Uuid;

/// Save a trade flow record to the database (upsert).
pub async fn save_trade_flow(pool: &PgPool, record: &TradeFlowRecord) -> Result<(), sqlx::Error> {
    let seller_json = serde_json::to_value(&record.seller)
        .map_err(|e| sqlx::Error::Protocol(format!("failed to serialize seller: {e}")))?;
    let buyer_json = serde_json::to_value(&record.buyer)
        .map_err(|e| sqlx::Error::Protocol(format!("failed to serialize buyer: {e}")))?;

    sqlx::query(
        "INSERT INTO trade_flows (flow_id, corridor_id, flow_type, state, seller, buyer, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (flow_id) DO UPDATE SET
            corridor_id = EXCLUDED.corridor_id,
            state = EXCLUDED.state,
            seller = EXCLUDED.seller,
            buyer = EXCLUDED.buyer,
            updated_at = EXCLUDED.updated_at",
    )
    .bind(record.flow_id)
    .bind(record.corridor_id)
    .bind(format!("{}", record.flow_type))
    .bind(format!("{}", record.state))
    .bind(&seller_json)
    .bind(&buyer_json)
    .bind(record.created_at)
    .bind(record.updated_at)
    .execute(pool)
    .await?;

    // Save transitions (upsert each).
    for t in &record.transitions {
        save_trade_transition(pool, record.flow_id, t).await?;
    }

    Ok(())
}

/// Save a single trade transition record.
async fn save_trade_transition(
    pool: &PgPool,
    flow_id: Uuid,
    t: &TradeTransitionRecord,
) -> Result<(), sqlx::Error> {
    let digests_json = serde_json::to_value(&t.document_digests)
        .map_err(|e| sqlx::Error::Protocol(format!("failed to serialize document_digests: {e}")))?;

    sqlx::query(
        "INSERT INTO trade_transitions (transition_id, flow_id, kind, from_state, to_state, payload, document_digests, receipt_digest, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         ON CONFLICT (transition_id) DO NOTHING",
    )
    .bind(t.transition_id)
    .bind(flow_id)
    .bind(&t.kind)
    .bind(format!("{}", t.from_state))
    .bind(format!("{}", t.to_state))
    .bind(&t.payload)
    .bind(&digests_json)
    .bind(&t.receipt_digest)
    .bind(t.created_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Load all trade flows from the database for hydration.
pub async fn load_all_trade_flows(
    pool: &PgPool,
) -> Result<Vec<TradeFlowRecord>, sqlx::Error> {
    let flow_rows = sqlx::query_as::<_, TradeFlowRow>(
        "SELECT flow_id, corridor_id, flow_type, state, seller, buyer, created_at, updated_at
         FROM trade_flows ORDER BY created_at",
    )
    .fetch_all(pool)
    .await?;

    let mut records = Vec::with_capacity(flow_rows.len());
    for row in flow_rows {
        let transitions = load_transitions_for_flow(pool, row.flow_id).await?;
        let record = TradeFlowRecord {
            flow_id: row.flow_id,
            corridor_id: row.corridor_id,
            flow_type: parse_flow_type(&row.flow_type),
            state: parse_flow_state(&row.state),
            seller: serde_json::from_value(row.seller).map_err(|e| {
                sqlx::Error::Protocol(format!(
                    "corrupt seller data in trade flow {}: {e}",
                    row.flow_id
                ))
            })?,
            buyer: serde_json::from_value(row.buyer).map_err(|e| {
                sqlx::Error::Protocol(format!(
                    "corrupt buyer data in trade flow {}: {e}",
                    row.flow_id
                ))
            })?,
            transitions,
            created_at: row.created_at,
            updated_at: row.updated_at,
        };
        records.push(record);
    }

    Ok(records)
}

/// Load transitions for a specific trade flow.
pub async fn load_transitions_for_flow(
    pool: &PgPool,
    flow_id: Uuid,
) -> Result<Vec<TradeTransitionRecord>, sqlx::Error> {
    let rows = sqlx::query_as::<_, TradeTransitionRow>(
        "SELECT transition_id, flow_id, kind, from_state, to_state, payload, document_digests, receipt_digest, created_at
         FROM trade_transitions WHERE flow_id = $1 ORDER BY created_at",
    )
    .bind(flow_id)
    .fetch_all(pool)
    .await?;

    let mut records = Vec::with_capacity(rows.len());
    for r in rows {
        let document_digests = serde_json::from_value(r.document_digests).map_err(|e| {
            sqlx::Error::Protocol(format!(
                "corrupt document_digests in transition {}: {e}",
                r.transition_id
            ))
        })?;
        records.push(TradeTransitionRecord {
            transition_id: r.transition_id,
            kind: r.kind,
            from_state: parse_flow_state(&r.from_state),
            to_state: parse_flow_state(&r.to_state),
            payload: r.payload,
            document_digests,
            receipt_digest: r.receipt_digest,
            created_at: r.created_at,
        });
    }
    Ok(records)
}

// ---------------------------------------------------------------------------
// Row types
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct TradeFlowRow {
    flow_id: Uuid,
    corridor_id: Option<Uuid>,
    flow_type: String,
    state: String,
    seller: serde_json::Value,
    buyer: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct TradeTransitionRow {
    transition_id: Uuid,
    #[sqlx(rename = "flow_id")]
    _flow_id: Uuid,
    kind: String,
    from_state: String,
    to_state: String,
    payload: serde_json::Value,
    document_digests: serde_json::Value,
    receipt_digest: Option<String>,
    created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

fn parse_flow_type(s: &str) -> TradeFlowType {
    match s {
        "Export" => TradeFlowType::Export,
        "Import" => TradeFlowType::Import,
        "LetterOfCredit" => TradeFlowType::LetterOfCredit,
        "OpenAccount" => TradeFlowType::OpenAccount,
        other => {
            tracing::warn!(value = other, "unrecognized trade flow type in database, defaulting to Export");
            TradeFlowType::Export
        }
    }
}

fn parse_flow_state(s: &str) -> TradeFlowState {
    match s {
        "Created" => TradeFlowState::Created,
        "InvoiceIssued" => TradeFlowState::InvoiceIssued,
        "InvoiceAccepted" => TradeFlowState::InvoiceAccepted,
        "GoodsShipped" => TradeFlowState::GoodsShipped,
        "BolEndorsed" => TradeFlowState::BolEndorsed,
        "GoodsReleased" => TradeFlowState::GoodsReleased,
        "LcIssued" => TradeFlowState::LcIssued,
        "LcAmended" => TradeFlowState::LcAmended,
        "DocumentsPresented" => TradeFlowState::DocumentsPresented,
        "LcHonored" => TradeFlowState::LcHonored,
        "SettlementInitiated" => TradeFlowState::SettlementInitiated,
        "Settled" => TradeFlowState::Settled,
        other => {
            tracing::warn!(value = other, "unrecognized trade flow state in database, defaulting to Created");
            TradeFlowState::Created
        }
    }
}
