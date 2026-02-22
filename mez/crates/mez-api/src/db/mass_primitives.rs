// SPDX-License-Identifier: BUSL-1.1
//! Sovereign Mass primitive persistence operations.
//!
//! Provides upsert/load functions for all Mass primitive tables when the zone
//! operates in sovereign mode (`SOVEREIGN_MASS=true`). Follows the same
//! pattern as `corridors.rs` and `smart_assets.rs`.
//!
//! Each resource type has:
//! - `save_*` — upsert (INSERT ... ON CONFLICT DO UPDATE)
//! - `load_all_*` — bulk load for hydration on startup
//!
//! For org-keyed resources (members, board, shareholders):
//! - `save_*_by_org` — replace all records for an org
//! - `load_all_*_by_org` — bulk load grouped by org_id

use sqlx::PgPool;
use uuid::Uuid;

// ── Organizations ───────────────────────────────────────────────────

/// Upsert an organization record.
pub async fn save_organization(
    pool: &PgPool,
    id: Uuid,
    value: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    let name = value.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let jurisdiction = value.get("jurisdiction").and_then(|v| v.as_str());
    let status = value.get("status").and_then(|v| v.as_str()).unwrap_or("ACTIVE");
    let tags = value.get("tags").cloned().unwrap_or(serde_json::json!([]));
    let address = value.get("address").cloned();
    let created_at = parse_timestamp(value.get("createdAt"));
    let updated_at = parse_timestamp(value.get("updatedAt"));

    sqlx::query(
        "INSERT INTO mass_organizations (id, name, jurisdiction, status, tags, address, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (id) DO UPDATE SET
            name = EXCLUDED.name,
            jurisdiction = EXCLUDED.jurisdiction,
            status = EXCLUDED.status,
            tags = EXCLUDED.tags,
            address = EXCLUDED.address,
            updated_at = EXCLUDED.updated_at"
    )
    .bind(id)
    .bind(name)
    .bind(jurisdiction)
    .bind(status)
    .bind(&tags)
    .bind(&address)
    .bind(created_at)
    .bind(updated_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Load all organizations for hydration.
pub async fn load_all_organizations(
    pool: &PgPool,
) -> Result<Vec<(Uuid, serde_json::Value)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, GenericRow>(
        "SELECT id, name, jurisdiction, status, tags, address, created_at, updated_at
         FROM mass_organizations ORDER BY created_at"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| {
        let val = serde_json::json!({
            "id": r.id.to_string(),
            "name": r.name,
            "jurisdiction": r.jurisdiction,
            "status": r.status,
            "tags": r.tags,
            "address": r.address,
            "createdAt": r.created_at.to_rfc3339(),
            "updatedAt": r.updated_at.to_rfc3339()
        });
        (r.id, val)
    }).collect())
}

#[derive(sqlx::FromRow)]
struct GenericRow {
    id: Uuid,
    name: String,
    jurisdiction: Option<String>,
    status: String,
    tags: serde_json::Value,
    address: Option<serde_json::Value>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

// ── Treasuries ──────────────────────────────────────────────────────

/// Upsert a treasury record.
pub async fn save_treasury(
    pool: &PgPool,
    id: Uuid,
    value: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    let entity_id = value.get("entityId").and_then(|v| v.as_str()).unwrap_or("");
    let name = value.get("name").and_then(|v| v.as_str());
    let status = value.get("status").and_then(|v| v.as_str()).unwrap_or("ACTIVE");
    let context = value.get("context").and_then(|v| v.as_str());
    let reference_id = value.get("referenceId").and_then(|v| v.as_str());
    let created_at = parse_timestamp(value.get("createdAt"));
    let updated_at = parse_timestamp(value.get("updatedAt"));

    sqlx::query(
        "INSERT INTO mass_treasuries (id, entity_id, name, status, context, reference_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (id) DO UPDATE SET
            entity_id = EXCLUDED.entity_id,
            name = EXCLUDED.name,
            status = EXCLUDED.status,
            context = EXCLUDED.context,
            reference_id = EXCLUDED.reference_id,
            updated_at = EXCLUDED.updated_at"
    )
    .bind(id)
    .bind(entity_id)
    .bind(name)
    .bind(status)
    .bind(context)
    .bind(reference_id)
    .bind(created_at)
    .bind(updated_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Load all treasuries for hydration.
pub async fn load_all_treasuries(
    pool: &PgPool,
) -> Result<Vec<(Uuid, serde_json::Value)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, TreasuryRow>(
        "SELECT id, entity_id, name, status, context, reference_id, created_at, updated_at
         FROM mass_treasuries ORDER BY created_at"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| {
        let val = serde_json::json!({
            "id": r.id.to_string(),
            "entityId": r.entity_id,
            "name": r.name,
            "status": r.status,
            "context": r.context,
            "referenceId": r.reference_id,
            "createdAt": r.created_at.to_rfc3339(),
            "updatedAt": r.updated_at.to_rfc3339()
        });
        (r.id, val)
    }).collect())
}

#[derive(sqlx::FromRow)]
struct TreasuryRow {
    id: Uuid,
    entity_id: String,
    name: Option<String>,
    status: String,
    context: Option<String>,
    reference_id: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

// ── Accounts ────────────────────────────────────────────────────────

/// Upsert an account record.
pub async fn save_account(
    pool: &PgPool,
    id: Uuid,
    value: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    let entity_id = value.get("entityId").and_then(|v| v.as_str());
    let treasury_id = value.get("treasuryId")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<Uuid>().ok());
    let name = value.get("name").and_then(|v| v.as_str()).unwrap_or("Default Account");
    let currency = value.get("currency").and_then(|v| v.as_str()).unwrap_or("PKR");
    let balance = value.get("balance").and_then(|v| v.as_str()).unwrap_or("0.00");
    let available = value.get("available").and_then(|v| v.as_str()).unwrap_or("0.00");
    let status = value.get("status").and_then(|v| v.as_str()).unwrap_or("ACTIVE");
    let funding_details = value.get("fundingDetails").cloned();
    let created_at = parse_timestamp(value.get("createdAt"));
    let updated_at = parse_timestamp(value.get("updatedAt"));

    sqlx::query(
        "INSERT INTO mass_accounts (id, entity_id, treasury_id, name, currency, balance, available, status, funding_details, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
         ON CONFLICT (id) DO UPDATE SET
            entity_id = EXCLUDED.entity_id,
            treasury_id = EXCLUDED.treasury_id,
            name = EXCLUDED.name,
            currency = EXCLUDED.currency,
            balance = EXCLUDED.balance,
            available = EXCLUDED.available,
            status = EXCLUDED.status,
            funding_details = EXCLUDED.funding_details,
            updated_at = EXCLUDED.updated_at"
    )
    .bind(id)
    .bind(entity_id)
    .bind(treasury_id)
    .bind(name)
    .bind(currency)
    .bind(balance)
    .bind(available)
    .bind(status)
    .bind(&funding_details)
    .bind(created_at)
    .bind(updated_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Load all accounts for hydration.
pub async fn load_all_accounts(
    pool: &PgPool,
) -> Result<Vec<(Uuid, serde_json::Value)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, AccountRow>(
        "SELECT id, entity_id, treasury_id, name, currency, balance, available, status, funding_details, created_at, updated_at
         FROM mass_accounts ORDER BY created_at"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| {
        let val = serde_json::json!({
            "id": r.id.to_string(),
            "entityId": r.entity_id,
            "treasuryId": r.treasury_id.map(|u| u.to_string()),
            "name": r.name,
            "currency": r.currency,
            "balance": r.balance,
            "available": r.available,
            "status": r.status,
            "fundingDetails": r.funding_details,
            "createdAt": r.created_at.to_rfc3339(),
            "updatedAt": r.updated_at.to_rfc3339()
        });
        (r.id, val)
    }).collect())
}

#[derive(sqlx::FromRow)]
struct AccountRow {
    id: Uuid,
    entity_id: Option<String>,
    treasury_id: Option<Uuid>,
    name: String,
    currency: String,
    balance: String,
    available: String,
    status: String,
    funding_details: Option<serde_json::Value>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

// ── Transactions ────────────────────────────────────────────────────

/// Upsert a transaction record.
pub async fn save_transaction(
    pool: &PgPool,
    id: Uuid,
    value: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    let account_id = value.get("accountId").and_then(|v| v.as_str());
    let entity_id = value.get("entityId").and_then(|v| v.as_str());
    let transaction_type = value.get("transactionType").and_then(|v| v.as_str()).unwrap_or("PAYMENT");
    let status = value.get("status").and_then(|v| v.as_str()).unwrap_or("PENDING");
    let direction = value.get("direction").and_then(|v| v.as_str()).unwrap_or("OUTBOUND");
    let currency = value.get("currency").and_then(|v| v.as_str()).unwrap_or("PKR");
    let amount = value.get("amount").and_then(|v| v.as_str());
    let reference = value.get("reference").and_then(|v| v.as_str());
    let created_at = parse_timestamp(value.get("createdAt"));

    sqlx::query(
        "INSERT INTO mass_transactions (id, account_id, entity_id, transaction_type, status, direction, currency, amount, reference, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         ON CONFLICT (id) DO UPDATE SET
            account_id = EXCLUDED.account_id,
            entity_id = EXCLUDED.entity_id,
            transaction_type = EXCLUDED.transaction_type,
            status = EXCLUDED.status,
            direction = EXCLUDED.direction,
            currency = EXCLUDED.currency,
            amount = EXCLUDED.amount,
            reference = EXCLUDED.reference"
    )
    .bind(id)
    .bind(account_id)
    .bind(entity_id)
    .bind(transaction_type)
    .bind(status)
    .bind(direction)
    .bind(currency)
    .bind(amount)
    .bind(reference)
    .bind(created_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Load all transactions for hydration.
pub async fn load_all_transactions(
    pool: &PgPool,
) -> Result<Vec<(Uuid, serde_json::Value)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, TransactionRow>(
        "SELECT id, account_id, entity_id, transaction_type, status, direction, currency, amount, reference, created_at
         FROM mass_transactions ORDER BY created_at"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| {
        let val = serde_json::json!({
            "id": r.id.to_string(),
            "accountId": r.account_id,
            "entityId": r.entity_id,
            "transactionType": r.transaction_type,
            "status": r.status,
            "direction": r.direction,
            "currency": r.currency,
            "amount": r.amount,
            "reference": r.reference,
            "createdAt": r.created_at.to_rfc3339()
        });
        (r.id, val)
    }).collect())
}

#[derive(sqlx::FromRow)]
struct TransactionRow {
    id: Uuid,
    account_id: Option<String>,
    entity_id: Option<String>,
    transaction_type: String,
    status: String,
    direction: String,
    currency: String,
    amount: Option<String>,
    reference: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

// ── Tax Events (Sovereign) ──────────────────────────────────────────

/// Upsert a sovereign Mass tax event record.
pub async fn save_mass_tax_event(
    pool: &PgPool,
    id: Uuid,
    value: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    let entity_id = value.get("entityId").and_then(|v| v.as_str()).unwrap_or("");
    let event_type = value.get("eventType").and_then(|v| v.as_str()).unwrap_or("UNKNOWN");
    let amount = value.get("amount").and_then(|v| v.as_str()).unwrap_or("0");
    let currency = value.get("currency").and_then(|v| v.as_str()).unwrap_or("PKR");
    let tax_year = value.get("taxYear").and_then(|v| v.as_str());
    let details = value.get("details").cloned().unwrap_or(serde_json::json!({}));
    let created_at = parse_timestamp(value.get("createdAt"));

    sqlx::query(
        "INSERT INTO mass_tax_events_sovereign (id, entity_id, event_type, amount, currency, tax_year, details, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (id) DO UPDATE SET
            entity_id = EXCLUDED.entity_id,
            event_type = EXCLUDED.event_type,
            amount = EXCLUDED.amount,
            currency = EXCLUDED.currency,
            tax_year = EXCLUDED.tax_year,
            details = EXCLUDED.details"
    )
    .bind(id)
    .bind(entity_id)
    .bind(event_type)
    .bind(amount)
    .bind(currency)
    .bind(tax_year)
    .bind(&details)
    .bind(created_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Load all sovereign Mass tax events for hydration.
pub async fn load_all_mass_tax_events(
    pool: &PgPool,
) -> Result<Vec<(Uuid, serde_json::Value)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, MassTaxEventRow>(
        "SELECT id, entity_id, event_type, amount, currency, tax_year, details, created_at
         FROM mass_tax_events_sovereign ORDER BY created_at"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| {
        let val = serde_json::json!({
            "id": r.id.to_string(),
            "entityId": r.entity_id,
            "eventType": r.event_type,
            "amount": r.amount,
            "currency": r.currency,
            "taxYear": r.tax_year,
            "details": r.details,
            "createdAt": r.created_at.to_rfc3339()
        });
        (r.id, val)
    }).collect())
}

#[derive(sqlx::FromRow)]
struct MassTaxEventRow {
    id: Uuid,
    entity_id: String,
    event_type: String,
    amount: String,
    currency: String,
    tax_year: Option<String>,
    details: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
}

// ── Consents ────────────────────────────────────────────────────────

/// Upsert a consent record.
pub async fn save_consent(
    pool: &PgPool,
    id: Uuid,
    value: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    let organization_id = value.get("organizationId").and_then(|v| v.as_str()).unwrap_or("");
    let operation_id = value.get("operationId").and_then(|v| v.as_str());
    let operation_type = value.get("operationType").and_then(|v| v.as_str());
    let status = value.get("status").and_then(|v| v.as_str()).unwrap_or("PENDING");
    let votes = value.get("votes").cloned().unwrap_or(serde_json::json!([]));
    let num_votes_required = value.get("numVotesRequired").and_then(|v| v.as_i64()).map(|v| v.clamp(i32::MIN as i64, i32::MAX as i64) as i32);
    let approval_count = value.get("approvalCount").and_then(|v| v.as_i64()).unwrap_or(0).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    let rejection_count = value.get("rejectionCount").and_then(|v| v.as_i64()).unwrap_or(0).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    let requested_by = value.get("requestedBy").and_then(|v| v.as_str());
    let created_at = parse_timestamp(value.get("createdAt"));
    let updated_at = parse_timestamp(value.get("updatedAt"));

    sqlx::query(
        "INSERT INTO mass_consents (id, organization_id, operation_id, operation_type, status, votes, num_votes_required, approval_count, rejection_count, requested_by, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
         ON CONFLICT (id) DO UPDATE SET
            organization_id = EXCLUDED.organization_id,
            operation_id = EXCLUDED.operation_id,
            operation_type = EXCLUDED.operation_type,
            status = EXCLUDED.status,
            votes = EXCLUDED.votes,
            num_votes_required = EXCLUDED.num_votes_required,
            approval_count = EXCLUDED.approval_count,
            rejection_count = EXCLUDED.rejection_count,
            requested_by = EXCLUDED.requested_by,
            updated_at = EXCLUDED.updated_at"
    )
    .bind(id)
    .bind(organization_id)
    .bind(operation_id)
    .bind(operation_type)
    .bind(status)
    .bind(&votes)
    .bind(num_votes_required)
    .bind(approval_count)
    .bind(rejection_count)
    .bind(requested_by)
    .bind(created_at)
    .bind(updated_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Load all consents for hydration.
pub async fn load_all_consents(
    pool: &PgPool,
) -> Result<Vec<(Uuid, serde_json::Value)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ConsentRow>(
        "SELECT id, organization_id, operation_id, operation_type, status, votes, num_votes_required, approval_count, rejection_count, requested_by, created_at, updated_at
         FROM mass_consents ORDER BY created_at"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| {
        let val = serde_json::json!({
            "id": r.id.to_string(),
            "organizationId": r.organization_id,
            "operationId": r.operation_id,
            "operationType": r.operation_type,
            "status": r.status,
            "votes": r.votes,
            "numVotesRequired": r.num_votes_required,
            "approvalCount": r.approval_count,
            "rejectionCount": r.rejection_count,
            "requestedBy": r.requested_by,
            "createdAt": r.created_at.to_rfc3339(),
            "updatedAt": r.updated_at.to_rfc3339()
        });
        (r.id, val)
    }).collect())
}

#[derive(sqlx::FromRow)]
struct ConsentRow {
    id: Uuid,
    organization_id: String,
    operation_id: Option<String>,
    operation_type: Option<String>,
    status: String,
    votes: serde_json::Value,
    num_votes_required: Option<i32>,
    approval_count: i32,
    rejection_count: i32,
    requested_by: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

// ── Cap Tables ──────────────────────────────────────────────────────

/// Upsert a cap table record.
pub async fn save_cap_table(
    pool: &PgPool,
    id: Uuid,
    value: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    let organization_id = value.get("organizationId").and_then(|v| v.as_str()).unwrap_or("");
    let authorized_shares = value.get("authorizedShares").and_then(|v| v.as_i64()).unwrap_or(0);
    let outstanding_shares = value.get("outstandingShares").and_then(|v| v.as_i64()).unwrap_or(0);
    let fully_diluted_shares = value.get("fullyDilutedShares").and_then(|v| v.as_i64()).unwrap_or(0);
    let reserved_shares = value.get("reservedShares").and_then(|v| v.as_i64()).unwrap_or(0);
    let unreserved_shares = value.get("unreservedShares").and_then(|v| v.as_i64()).unwrap_or(0);
    let share_classes = value.get("shareClasses").cloned().unwrap_or(serde_json::json!([]));
    let shareholders = value.get("shareholders").cloned().unwrap_or(serde_json::json!([]));
    let options_pools = value.get("optionsPools").cloned().unwrap_or(serde_json::json!([]));
    let par_value = value.get("parValue").and_then(|v| v.as_str())
        .or_else(|| value.get("shareClasses")
            .and_then(|sc| sc.as_array())
            .and_then(|arr| arr.first())
            .and_then(|sc| sc.get("parValue"))
            .and_then(|v| v.as_str()));
    let created_at = parse_timestamp(value.get("createdAt"));
    let updated_at = parse_timestamp(value.get("updatedAt"));

    sqlx::query(
        "INSERT INTO mass_cap_tables (id, organization_id, authorized_shares, outstanding_shares, fully_diluted_shares, reserved_shares, unreserved_shares, share_classes, shareholders, options_pools, par_value, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
         ON CONFLICT (id) DO UPDATE SET
            organization_id = EXCLUDED.organization_id,
            authorized_shares = EXCLUDED.authorized_shares,
            outstanding_shares = EXCLUDED.outstanding_shares,
            fully_diluted_shares = EXCLUDED.fully_diluted_shares,
            reserved_shares = EXCLUDED.reserved_shares,
            unreserved_shares = EXCLUDED.unreserved_shares,
            share_classes = EXCLUDED.share_classes,
            shareholders = EXCLUDED.shareholders,
            options_pools = EXCLUDED.options_pools,
            par_value = EXCLUDED.par_value,
            updated_at = EXCLUDED.updated_at"
    )
    .bind(id)
    .bind(organization_id)
    .bind(authorized_shares)
    .bind(outstanding_shares)
    .bind(fully_diluted_shares)
    .bind(reserved_shares)
    .bind(unreserved_shares)
    .bind(&share_classes)
    .bind(&shareholders)
    .bind(&options_pools)
    .bind(par_value)
    .bind(created_at)
    .bind(updated_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Load all cap tables for hydration.
pub async fn load_all_cap_tables(
    pool: &PgPool,
) -> Result<Vec<(Uuid, serde_json::Value)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, CapTableRow>(
        "SELECT id, organization_id, authorized_shares, outstanding_shares, fully_diluted_shares, reserved_shares, unreserved_shares, share_classes, shareholders, options_pools, par_value, created_at, updated_at
         FROM mass_cap_tables ORDER BY created_at"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| {
        let val = serde_json::json!({
            "id": r.id.to_string(),
            "organizationId": r.organization_id,
            "authorizedShares": r.authorized_shares,
            "outstandingShares": r.outstanding_shares,
            "fullyDilutedShares": r.fully_diluted_shares,
            "reservedShares": r.reserved_shares,
            "unreservedShares": r.unreserved_shares,
            "shareClasses": r.share_classes,
            "shareholders": r.shareholders,
            "optionsPools": r.options_pools,
            "parValue": r.par_value,
            "createdAt": r.created_at.to_rfc3339(),
            "updatedAt": r.updated_at.to_rfc3339()
        });
        (r.id, val)
    }).collect())
}

#[derive(sqlx::FromRow)]
struct CapTableRow {
    id: Uuid,
    organization_id: String,
    authorized_shares: i64,
    outstanding_shares: i64,
    fully_diluted_shares: i64,
    reserved_shares: i64,
    unreserved_shares: i64,
    share_classes: serde_json::Value,
    shareholders: serde_json::Value,
    options_pools: serde_json::Value,
    par_value: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

// ── Investments ─────────────────────────────────────────────────────

/// Upsert an investment record.
pub async fn save_investment(
    pool: &PgPool,
    id: Uuid,
    value: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    let created_at = parse_timestamp(value.get("createdAt"));
    let updated_at = parse_timestamp(value.get("updatedAt"));

    sqlx::query(
        "INSERT INTO mass_investments (id, payload, created_at, updated_at)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (id) DO UPDATE SET
            payload = EXCLUDED.payload,
            updated_at = EXCLUDED.updated_at"
    )
    .bind(id)
    .bind(value)
    .bind(created_at)
    .bind(updated_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Load all investments for hydration.
pub async fn load_all_investments(
    pool: &PgPool,
) -> Result<Vec<(Uuid, serde_json::Value)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, InvestmentRow>(
        "SELECT id, payload FROM mass_investments ORDER BY created_at"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| (r.id, r.payload)).collect())
}

#[derive(sqlx::FromRow)]
struct InvestmentRow {
    id: Uuid,
    payload: serde_json::Value,
}

// ── Templates (string-keyed) ────────────────────────────────────────

/// Upsert a template record.
pub async fn save_template(
    pool: &PgPool,
    id: &str,
    value: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    let name = value.get("name").and_then(|v| v.as_str());
    let context = value.get("context").and_then(|v| v.as_str());
    let entity_id = value.get("entityId").and_then(|v| v.as_str());
    let version = value.get("version").and_then(|v| v.as_str());
    let type_field = value.get("type").and_then(|v| v.as_str());
    let grouping = value.get("grouping").and_then(|v| v.as_str());
    let status = value.get("status").and_then(|v| v.as_str()).unwrap_or("ACTIVE");

    sqlx::query(
        "INSERT INTO mass_templates (id, name, context, entity_id, version, type_field, grouping, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (id) DO UPDATE SET
            name = EXCLUDED.name,
            context = EXCLUDED.context,
            entity_id = EXCLUDED.entity_id,
            version = EXCLUDED.version,
            type_field = EXCLUDED.type_field,
            grouping = EXCLUDED.grouping,
            status = EXCLUDED.status"
    )
    .bind(id)
    .bind(name)
    .bind(context)
    .bind(entity_id)
    .bind(version)
    .bind(type_field)
    .bind(grouping)
    .bind(status)
    .execute(pool)
    .await?;

    Ok(())
}

/// Load all templates for hydration.
pub async fn load_all_templates(
    pool: &PgPool,
) -> Result<Vec<(String, serde_json::Value)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, TemplateRow>(
        "SELECT id, name, context, entity_id, version, type_field, grouping, status FROM mass_templates"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| {
        let val = serde_json::json!({
            "id": r.id,
            "name": r.name,
            "context": r.context,
            "entityId": r.entity_id,
            "version": r.version,
            "type": r.type_field,
            "grouping": r.grouping,
            "status": r.status
        });
        (r.id, val)
    }).collect())
}

#[derive(sqlx::FromRow)]
struct TemplateRow {
    id: String,
    name: Option<String>,
    context: Option<String>,
    entity_id: Option<String>,
    version: Option<String>,
    type_field: Option<String>,
    grouping: Option<String>,
    status: String,
}

// ── Submissions (string-keyed) ──────────────────────────────────────

/// Upsert a submission record.
pub async fn save_submission(
    pool: &PgPool,
    id: &str,
    value: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    let entity_id = value.get("entityId").and_then(|v| v.as_str());
    let context = value.get("context").and_then(|v| v.as_str());
    let status = value.get("status").and_then(|v| v.as_str()).unwrap_or("PENDING");
    let signing_order = value.get("signingOrder").and_then(|v| v.as_str());
    let signers = value.get("signers").cloned().unwrap_or(serde_json::json!([]));
    let document_uri = value.get("documentUri").and_then(|v| v.as_str());
    let created_at = parse_timestamp(value.get("createdAt"));
    let updated_at = parse_timestamp(value.get("updatedAt"));

    sqlx::query(
        "INSERT INTO mass_submissions (id, entity_id, context, status, signing_order, signers, document_uri, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         ON CONFLICT (id) DO UPDATE SET
            entity_id = EXCLUDED.entity_id,
            context = EXCLUDED.context,
            status = EXCLUDED.status,
            signing_order = EXCLUDED.signing_order,
            signers = EXCLUDED.signers,
            document_uri = EXCLUDED.document_uri,
            updated_at = EXCLUDED.updated_at"
    )
    .bind(id)
    .bind(entity_id)
    .bind(context)
    .bind(status)
    .bind(signing_order)
    .bind(&signers)
    .bind(document_uri)
    .bind(created_at)
    .bind(updated_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Load all submissions for hydration.
pub async fn load_all_submissions(
    pool: &PgPool,
) -> Result<Vec<(String, serde_json::Value)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, SubmissionRow>(
        "SELECT id, entity_id, context, status, signing_order, signers, document_uri, created_at, updated_at
         FROM mass_submissions ORDER BY created_at"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| {
        let val = serde_json::json!({
            "id": r.id,
            "entityId": r.entity_id,
            "context": r.context,
            "status": r.status,
            "signingOrder": r.signing_order,
            "signers": r.signers,
            "documentUri": r.document_uri,
            "createdAt": r.created_at.to_rfc3339(),
            "updatedAt": r.updated_at.to_rfc3339()
        });
        (r.id, val)
    }).collect())
}

#[derive(sqlx::FromRow)]
struct SubmissionRow {
    id: String,
    entity_id: Option<String>,
    context: Option<String>,
    status: String,
    signing_order: Option<String>,
    signers: serde_json::Value,
    document_uri: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

// ── Org-keyed Identity Tables ───────────────────────────────────────

/// Save members for an organization (delete + re-insert).
///
/// Uses a transaction so the delete + insert is atomic — a partial failure
/// won't leave the org with zero members.
pub async fn save_members_by_org(
    pool: &PgPool,
    org_id: &str,
    members: &[serde_json::Value],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM mass_members WHERE org_id = $1")
        .bind(org_id)
        .execute(&mut *tx)
        .await?;

    for member in members {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO mass_members (id, org_id, payload) VALUES ($1, $2, $3)"
        )
        .bind(id)
        .bind(org_id)
        .bind(member)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Load all members grouped by org_id for hydration.
pub async fn load_all_members_by_org(
    pool: &PgPool,
) -> Result<Vec<(String, Vec<serde_json::Value>)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OrgPayloadRow>(
        "SELECT org_id, payload FROM mass_members ORDER BY org_id, created_at"
    )
    .fetch_all(pool)
    .await?;

    Ok(group_by_org(rows))
}

/// Save board members for an organization (delete + re-insert).
///
/// Uses a transaction so the delete + insert is atomic.
pub async fn save_board_by_org(
    pool: &PgPool,
    org_id: &str,
    members: &[serde_json::Value],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM mass_board_members WHERE org_id = $1")
        .bind(org_id)
        .execute(&mut *tx)
        .await?;

    for member in members {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO mass_board_members (id, org_id, payload) VALUES ($1, $2, $3)"
        )
        .bind(id)
        .bind(org_id)
        .bind(member)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Load all board members grouped by org_id for hydration.
pub async fn load_all_board_by_org(
    pool: &PgPool,
) -> Result<Vec<(String, Vec<serde_json::Value>)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OrgPayloadRow>(
        "SELECT org_id, payload FROM mass_board_members ORDER BY org_id, created_at"
    )
    .fetch_all(pool)
    .await?;

    Ok(group_by_org(rows))
}

/// Save shareholders for an organization (delete + re-insert).
///
/// Uses a transaction so the delete + insert is atomic.
pub async fn save_shareholders_by_org(
    pool: &PgPool,
    org_id: &str,
    shareholders: &[serde_json::Value],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM mass_shareholders WHERE org_id = $1")
        .bind(org_id)
        .execute(&mut *tx)
        .await?;

    for shareholder in shareholders {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO mass_shareholders (id, org_id, payload) VALUES ($1, $2, $3)"
        )
        .bind(id)
        .bind(org_id)
        .bind(shareholder)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Load all shareholders grouped by org_id for hydration.
pub async fn load_all_shareholders_by_org(
    pool: &PgPool,
) -> Result<Vec<(String, Vec<serde_json::Value>)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OrgPayloadRow>(
        "SELECT org_id, payload FROM mass_shareholders ORDER BY org_id, created_at"
    )
    .fetch_all(pool)
    .await?;

    Ok(group_by_org(rows))
}

#[derive(sqlx::FromRow)]
struct OrgPayloadRow {
    org_id: String,
    payload: serde_json::Value,
}

/// Group org-keyed rows by org_id.
fn group_by_org(rows: Vec<OrgPayloadRow>) -> Vec<(String, Vec<serde_json::Value>)> {
    use std::collections::HashMap;
    let mut map: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
    for row in rows {
        map.entry(row.org_id).or_default().push(row.payload);
    }
    map.into_iter().collect()
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Parse an RFC 3339 timestamp from a JSON value, defaulting to now.
///
/// Logs a warning if a timestamp string is present but malformed, rather
/// than silently defaulting to `Utc::now()` which would corrupt audit trails.
fn parse_timestamp(val: Option<&serde_json::Value>) -> chrono::DateTime<chrono::Utc> {
    match val.and_then(|v| v.as_str()) {
        Some(s) => match chrono::DateTime::parse_from_rfc3339(s) {
            Ok(dt) => dt.with_timezone(&chrono::Utc),
            Err(e) => {
                tracing::warn!(
                    timestamp = %s,
                    error = %e,
                    "malformed RFC3339 timestamp in database record — defaulting to now"
                );
                chrono::Utc::now()
            }
        },
        None => chrono::Utc::now(),
    }
}
