//! Smart asset persistence operations.
//!
//! All functions take a `&PgPool` and operate on the `smart_assets` table.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::state::{AssetComplianceStatus, AssetStatus, SmartAssetRecord, SmartAssetType};

/// Insert a new smart asset record.
pub async fn insert(pool: &PgPool, record: &SmartAssetRecord) -> Result<(), sqlx::Error> {
    let status_str = record.status.as_str();
    let compliance_str = compliance_status_to_str(record.compliance_status);

    sqlx::query(
        "INSERT INTO smart_assets (id, asset_type, jurisdiction_id, status, genesis_digest,
         compliance_status, metadata, owner_entity_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(record.id)
    .bind(record.asset_type.as_str())
    .bind(&record.jurisdiction_id)
    .bind(status_str)
    .bind(&record.genesis_digest)
    .bind(compliance_str)
    .bind(&record.metadata)
    .bind(record.owner_entity_id)
    .bind(record.created_at)
    .bind(record.updated_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Update smart asset status and compliance status.
pub async fn update_status(
    pool: &PgPool,
    id: Uuid,
    status: AssetStatus,
    compliance_status: AssetComplianceStatus,
    updated_at: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE smart_assets SET status = $1, compliance_status = $2, updated_at = $3 WHERE id = $4",
    )
    .bind(status.as_str())
    .bind(compliance_status_to_str(compliance_status))
    .bind(updated_at)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Update smart asset genesis digest.
pub async fn update_genesis_digest(
    pool: &PgPool,
    id: Uuid,
    genesis_digest: &str,
    updated_at: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    let result =
        sqlx::query("UPDATE smart_assets SET genesis_digest = $1, updated_at = $2 WHERE id = $3")
            .bind(genesis_digest)
            .bind(updated_at)
            .bind(id)
            .execute(pool)
            .await?;

    Ok(result.rows_affected() > 0)
}

/// Fetch a smart asset by ID.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<SmartAssetRecord>, sqlx::Error> {
    let row = sqlx::query_as::<_, SmartAssetRow>(
        "SELECT id, asset_type, jurisdiction_id, status, genesis_digest,
         compliance_status, metadata, owner_entity_id, created_at, updated_at
         FROM smart_assets WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.and_then(|r| r.into_record()))
}

/// List smart assets with pagination.
pub async fn list(
    pool: &PgPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<SmartAssetRecord>, sqlx::Error> {
    let rows = sqlx::query_as::<_, SmartAssetRow>(
        "SELECT id, asset_type, jurisdiction_id, status, genesis_digest,
         compliance_status, metadata, owner_entity_id, created_at, updated_at
         FROM smart_assets ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let mut records = Vec::with_capacity(rows.len());
    for row in rows {
        match row.into_record() {
            Some(record) => records.push(record),
            None => {
                // into_record() already logs a warning; escalate to error for visibility
                tracing::error!("skipping smart asset row with invalid asset_type during list query");
            }
        }
    }
    Ok(records)
}

/// Load all smart assets from the database into the in-memory store on startup.
pub async fn load_all(pool: &PgPool) -> Result<Vec<SmartAssetRecord>, sqlx::Error> {
    let rows = sqlx::query_as::<_, SmartAssetRow>(
        "SELECT id, asset_type, jurisdiction_id, status, genesis_digest,
         compliance_status, metadata, owner_entity_id, created_at, updated_at
         FROM smart_assets ORDER BY created_at",
    )
    .fetch_all(pool)
    .await?;

    let mut records = Vec::with_capacity(rows.len());
    for row in rows {
        match row.into_record() {
            Some(record) => records.push(record),
            None => {
                tracing::error!("skipping smart asset row with invalid asset_type during load_all");
            }
        }
    }
    Ok(records)
}

fn compliance_status_to_str(status: AssetComplianceStatus) -> &'static str {
    match status {
        AssetComplianceStatus::Compliant => "compliant",
        AssetComplianceStatus::Pending => "pending",
        AssetComplianceStatus::NonCompliant => "non_compliant",
        AssetComplianceStatus::PartiallyCompliant => "partially_compliant",
        AssetComplianceStatus::Unevaluated => "unevaluated",
    }
}

fn parse_compliance_status(s: &str) -> AssetComplianceStatus {
    match s {
        "compliant" => AssetComplianceStatus::Compliant,
        "pending" => AssetComplianceStatus::Pending,
        "non_compliant" => AssetComplianceStatus::NonCompliant,
        "partially_compliant" => AssetComplianceStatus::PartiallyCompliant,
        other => {
            tracing::warn!(
                status = other,
                "unknown compliance status in database, defaulting to Unevaluated"
            );
            AssetComplianceStatus::Unevaluated
        }
    }
}

fn parse_asset_status(s: &str) -> AssetStatus {
    match s {
        "GENESIS" => AssetStatus::Genesis,
        "REGISTERED" => AssetStatus::Registered,
        "ACTIVE" => AssetStatus::Active,
        "PENDING" => AssetStatus::Pending,
        "SUSPENDED" => AssetStatus::Suspended,
        "RETIRED" => AssetStatus::Retired,
        other => {
            tracing::warn!(
                status = other,
                "unknown asset status in database, defaulting to Genesis"
            );
            AssetStatus::Genesis
        }
    }
}

/// Internal row type for SQLx mapping.
#[derive(sqlx::FromRow)]
struct SmartAssetRow {
    id: Uuid,
    asset_type: String,
    jurisdiction_id: String,
    status: String,
    genesis_digest: Option<String>,
    compliance_status: String,
    metadata: serde_json::Value,
    owner_entity_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl SmartAssetRow {
    fn into_record(self) -> Option<SmartAssetRecord> {
        let asset_type = match SmartAssetType::new(self.asset_type.clone()) {
            Ok(t) => t,
            Err(_) => {
                tracing::warn!(
                    id = %self.id,
                    asset_type = %self.asset_type,
                    "skipping smart asset row with invalid asset_type"
                );
                return None;
            }
        };
        Some(SmartAssetRecord {
            id: self.id,
            asset_type,
            jurisdiction_id: self.jurisdiction_id,
            status: parse_asset_status(&self.status),
            genesis_digest: self.genesis_digest,
            compliance_status: parse_compliance_status(&self.compliance_status),
            metadata: self.metadata,
            owner_entity_id: self.owner_entity_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}
