//! Tax event persistence operations.
//!
//! All functions take a `&PgPool` and operate on the `tax_events` table.
//! Tax events are immutable once created — there are no update operations.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::state::TaxEventRecord;

/// Insert a new tax event record.
pub async fn insert(pool: &PgPool, record: &TaxEventRecord) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO tax_events (id, entity_id, event_type, tax_category,
         jurisdiction_id, gross_amount, withholding_amount, net_amount,
         currency, tax_year, ntn, filer_status, statutory_section,
         withholding_executed, mass_payment_id, rules_applied, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)",
    )
    .bind(record.id)
    .bind(record.entity_id)
    .bind(&record.event_type)
    .bind(&record.tax_category)
    .bind(&record.jurisdiction_id)
    .bind(&record.gross_amount)
    .bind(&record.withholding_amount)
    .bind(&record.net_amount)
    .bind(&record.currency)
    .bind(&record.tax_year)
    .bind(&record.ntn)
    .bind(&record.filer_status)
    .bind(&record.statutory_section)
    .bind(record.withholding_executed)
    .bind(record.mass_payment_id)
    .bind(i32::try_from(record.rules_applied).unwrap_or_else(|_| {
        tracing::warn!(
            rules_applied = record.rules_applied,
            "rules_applied exceeds i32::MAX — clamping to i32::MAX for DB storage"
        );
        i32::MAX
    }))
    .bind(record.created_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Mark a tax event as having had its withholding executed via Mass fiscal API.
pub async fn mark_withholding_executed(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("UPDATE tax_events SET withholding_executed = true WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Fetch a tax event by ID.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<TaxEventRecord>, sqlx::Error> {
    let row = sqlx::query_as::<_, TaxEventRow>(
        "SELECT id, entity_id, event_type, tax_category, jurisdiction_id,
         gross_amount, withholding_amount, net_amount, currency, tax_year,
         ntn, filer_status, statutory_section, withholding_executed,
         mass_payment_id, rules_applied, created_at
         FROM tax_events WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(TaxEventRow::into_record))
}

/// List tax events by entity ID, with optional jurisdiction and tax year filters.
pub async fn list_by_entity(
    pool: &PgPool,
    entity_id: Uuid,
    jurisdiction_id: Option<&str>,
    tax_year: Option<&str>,
) -> Result<Vec<TaxEventRecord>, sqlx::Error> {
    // Build the query dynamically based on provided filters.
    let mut query = String::from(
        "SELECT id, entity_id, event_type, tax_category, jurisdiction_id,
         gross_amount, withholding_amount, net_amount, currency, tax_year,
         ntn, filer_status, statutory_section, withholding_executed,
         mass_payment_id, rules_applied, created_at
         FROM tax_events WHERE entity_id = $1",
    );

    let mut param_idx = 2;

    if jurisdiction_id.is_some() {
        query.push_str(&format!(" AND jurisdiction_id = ${param_idx}"));
        param_idx += 1;
    }

    if tax_year.is_some() {
        query.push_str(&format!(" AND tax_year = ${param_idx}"));
    }

    query.push_str(" ORDER BY created_at DESC");

    // SQLx doesn't have a great dynamic query builder, so we use conditional binding.
    // For simplicity, use the most common case (entity + optional filters) with raw SQL.
    let rows = match (jurisdiction_id, tax_year) {
        (Some(jid), Some(ty)) => {
            sqlx::query_as::<_, TaxEventRow>(&query)
                .bind(entity_id)
                .bind(jid)
                .bind(ty)
                .fetch_all(pool)
                .await?
        }
        (Some(jid), None) => {
            sqlx::query_as::<_, TaxEventRow>(&query)
                .bind(entity_id)
                .bind(jid)
                .fetch_all(pool)
                .await?
        }
        (None, Some(ty)) => {
            // Rebuild query for this case since param index differs.
            let q = "SELECT id, entity_id, event_type, tax_category, jurisdiction_id,
                     gross_amount, withholding_amount, net_amount, currency, tax_year,
                     ntn, filer_status, statutory_section, withholding_executed,
                     mass_payment_id, rules_applied, created_at
                     FROM tax_events WHERE entity_id = $1 AND tax_year = $2
                     ORDER BY created_at DESC";
            sqlx::query_as::<_, TaxEventRow>(q)
                .bind(entity_id)
                .bind(ty)
                .fetch_all(pool)
                .await?
        }
        (None, None) => {
            sqlx::query_as::<_, TaxEventRow>(&query)
                .bind(entity_id)
                .fetch_all(pool)
                .await?
        }
    };

    Ok(rows.into_iter().map(TaxEventRow::into_record).collect())
}

/// Load all tax events from the database into the in-memory store on startup.
pub async fn load_all(pool: &PgPool) -> Result<Vec<TaxEventRecord>, sqlx::Error> {
    let rows = sqlx::query_as::<_, TaxEventRow>(
        "SELECT id, entity_id, event_type, tax_category, jurisdiction_id,
         gross_amount, withholding_amount, net_amount, currency, tax_year,
         ntn, filer_status, statutory_section, withholding_executed,
         mass_payment_id, rules_applied, created_at
         FROM tax_events ORDER BY created_at",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(TaxEventRow::into_record).collect())
}

/// Internal row type for SQLx mapping.
#[derive(sqlx::FromRow)]
struct TaxEventRow {
    id: Uuid,
    entity_id: Uuid,
    event_type: String,
    tax_category: String,
    jurisdiction_id: String,
    gross_amount: String,
    withholding_amount: String,
    net_amount: String,
    currency: String,
    tax_year: String,
    ntn: Option<String>,
    filer_status: String,
    statutory_section: Option<String>,
    withholding_executed: bool,
    mass_payment_id: Option<Uuid>,
    rules_applied: i32,
    created_at: DateTime<Utc>,
}

impl TaxEventRow {
    fn into_record(self) -> TaxEventRecord {
        TaxEventRecord {
            id: self.id,
            entity_id: self.entity_id,
            event_type: self.event_type,
            tax_category: self.tax_category,
            jurisdiction_id: self.jurisdiction_id,
            gross_amount: self.gross_amount,
            withholding_amount: self.withholding_amount,
            net_amount: self.net_amount,
            currency: self.currency,
            tax_year: self.tax_year,
            ntn: self.ntn,
            filer_status: self.filer_status,
            statutory_section: self.statutory_section,
            withholding_executed: self.withholding_executed,
            mass_payment_id: self.mass_payment_id,
            rules_applied: usize::try_from(self.rules_applied).unwrap_or_else(|_| {
                tracing::warn!(
                    rules_applied = self.rules_applied,
                    "rules_applied is negative in database — defaulting to 0"
                );
                0
            }),
            created_at: self.created_at,
        }
    }
}
