// SPDX-License-Identifier: BUSL-1.1
//! Sovereign Mass CRUD operations — shared backend for orchestrated and
//! direct sovereign routes.
//!
//! Each function takes `&AppState` + typed inputs, writes to the in-memory
//! store, persists to Postgres when available, and returns `Result<Value, AppError>`.
//!
//! These functions produce JSON with camelCase field names matching what
//! `mez-mass-client` types serialize to — the orchestration layer expects
//! identical shapes regardless of backend (proxy vs sovereign).

use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::AppError;
use crate::state::AppState;

/// Persist to Postgres if db_pool is available. Returns an error
/// if persistence fails — the in-memory and database stores must stay in sync.
macro_rules! persist {
    ($state:expr, $save_fn:path, $($args:expr),+) => {
        if let Some(ref pool) = $state.db_pool {
            if let Err(e) = $save_fn(pool, $($args),+).await {
                tracing::error!(error = %e, "sovereign_ops: failed to persist to database");
                return Err(AppError::Internal(format!("database persist failed: {e}")));
            }
        }
    };
}

// ── Entities (organization-info) ──────────────────────────────────────

/// Create an entity in the sovereign store.
///
/// Returns JSON matching the shape of `mez_mass_client::entities::Entity`
/// (camelCase, `id` as string UUID).
pub async fn create_entity(
    state: &AppState,
    name: &str,
    jurisdiction: Option<&str>,
    entity_type: Option<&str>,
    tags: &[String],
) -> Result<Value, AppError> {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();

    let entity = json!({
        "id": id.to_string(),
        "name": name,
        "jurisdiction": jurisdiction,
        "entityType": entity_type,
        "status": "ACTIVE",
        "tags": tags,
        "address": null,
        "createdAt": now,
        "updatedAt": now
    });

    state.mass_organizations.insert(id, entity.clone());
    persist!(state, crate::db::mass_primitives::save_organization, id, &entity);
    Ok(entity)
}

/// Update an entity in the sovereign store.
///
/// Returns the updated entity JSON, or 404 if not found.
pub async fn update_entity(
    state: &AppState,
    id: Uuid,
    updates: &Value,
) -> Result<Value, AppError> {
    let updated = state.mass_organizations.update(&id, |val| {
        if let (Some(existing), Some(update_obj)) = (val.as_object_mut(), updates.as_object()) {
            for (k, v) in update_obj {
                existing.insert(k.clone(), v.clone());
            }
            existing.insert(
                "updatedAt".to_string(),
                json!(Utc::now().to_rfc3339()),
            );
        }
    });

    match updated {
        Some(val) => {
            persist!(state, crate::db::mass_primitives::save_organization, id, &val);
            Ok(val)
        }
        None => Err(AppError::not_found(format!("entity {id} not found"))),
    }
}

/// Get an entity from the sovereign store.
pub fn get_entity(state: &AppState, id: Uuid) -> Result<Option<Value>, AppError> {
    Ok(state.mass_organizations.get(&id))
}

/// List entities from the sovereign store, with optional ID filter.
///
/// When listing all entities (no ID filter), results are capped at 1000
/// to prevent unbounded response payloads. Callers needing full pagination
/// should use the ID-filtered variant.
pub fn list_entities(state: &AppState, ids: Option<&[Uuid]>) -> Result<Vec<Value>, AppError> {
    const MAX_LIST: usize = 1000;
    match ids {
        Some(id_list) => {
            let results: Vec<Value> = id_list
                .iter()
                .take(MAX_LIST)
                .filter_map(|id| state.mass_organizations.get(id))
                .collect();
            Ok(results)
        }
        None => {
            let all = state.mass_organizations.list();
            Ok(all.into_iter().take(MAX_LIST).collect())
        }
    }
}

// ── Ownership (cap tables) ────────────────────────────────────────────

/// Create a cap table in the sovereign store.
///
/// Returns JSON matching the shape of `mez_mass_client::ownership::CapTable`.
pub async fn create_cap_table(
    state: &AppState,
    org_id: &str,
    authorized_shares: u64,
    par_value: Option<&str>,
    shareholders: Option<&Value>,
) -> Result<Value, AppError> {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();

    let cap_table = json!({
        "id": id.to_string(),
        "organizationId": org_id,
        "authorizedShares": authorized_shares,
        "outstandingShares": 0,
        "fullyDilutedShares": 0,
        "reservedShares": 0,
        "unreservedShares": authorized_shares,
        "shareClasses": [{
            "id": Uuid::new_v4().to_string(),
            "name": "Common",
            "authorizedShares": authorized_shares,
            "outstandingShares": 0,
            "parValue": par_value.unwrap_or("1.00"),
            "votingRights": true,
            "restricted": false,
            "classType": "COMMON"
        }],
        "shareholders": shareholders.cloned().unwrap_or(json!([])),
        "optionsPools": [],
        "createdAt": now,
        "updatedAt": now
    });

    state.mass_cap_tables.insert(id, cap_table.clone());
    persist!(state, crate::db::mass_primitives::save_cap_table, id, &cap_table);
    Ok(cap_table)
}

/// Get a cap table by organization ID from the sovereign store.
pub fn get_cap_table_by_org(state: &AppState, org_id: &str) -> Result<Option<Value>, AppError> {
    let found = state.mass_cap_tables.list().into_iter().find(|v| {
        v.get("organizationId")
            .and_then(|v| v.as_str())
            .map(|v| v == org_id)
            .unwrap_or(false)
    });
    Ok(found)
}

// ── Fiscal (treasury, accounts, payments) ─────────────────────────────

/// Create a treasury in the sovereign store.
///
/// Returns JSON matching the shape of `mez_mass_client::fiscal::Treasury`.
pub async fn create_treasury(
    state: &AppState,
    entity_id: &str,
    entity_name: Option<&str>,
    context: Option<&str>,
) -> Result<Value, AppError> {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();

    let treasury = json!({
        "id": id.to_string(),
        "referenceId": null,
        "entityId": entity_id,
        "name": entity_name,
        "status": "ACTIVE",
        "context": context.unwrap_or("MASS"),
        "createdAt": now,
        "updatedAt": now
    });

    state.mass_treasuries.insert(id, treasury.clone());
    persist!(state, crate::db::mass_primitives::save_treasury, id, &treasury);
    Ok(treasury)
}

/// Create an account in the sovereign store.
///
/// Returns JSON matching the shape of `mez_mass_client::fiscal::Account`.
pub async fn create_account(
    state: &AppState,
    treasury_id: Uuid,
    name: Option<&str>,
) -> Result<Value, AppError> {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();

    let entity_id = state
        .mass_treasuries
        .get(&treasury_id)
        .and_then(|t| t.get("entityId").and_then(|v| v.as_str()).map(String::from));

    let account = json!({
        "id": id.to_string(),
        "entityId": entity_id,
        "treasuryId": treasury_id.to_string(),
        "name": name.unwrap_or("Default Account"),
        "currency": "PKR",
        "balance": "0.00",
        "available": "0.00",
        "status": "ACTIVE",
        "fundingDetails": null,
        "createdAt": now,
        "updatedAt": now
    });

    state.mass_accounts.insert(id, account.clone());
    persist!(state, crate::db::mass_primitives::save_account, id, &account);
    Ok(account)
}

/// Create a payment in the sovereign store.
///
/// Returns JSON matching the shape of `mez_mass_client::fiscal::Payment`.
pub async fn create_payment(
    state: &AppState,
    source_account_id: &str,
    amount: &str,
    currency: &str,
    reference: Option<&str>,
) -> Result<Value, AppError> {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();

    let entity_id = source_account_id
        .parse::<Uuid>()
        .ok()
        .and_then(|aid| {
            state
                .mass_accounts
                .get(&aid)
                .and_then(|a| a.get("entityId").and_then(|v| v.as_str()).map(String::from))
        });

    let payment = json!({
        "id": id.to_string(),
        "accountId": source_account_id,
        "entityId": entity_id,
        "transactionType": "PAYMENT",
        "status": "PENDING",
        "direction": "OUTBOUND",
        "currency": currency,
        "amount": amount,
        "reference": reference,
        "createdAt": now
    });

    state.mass_transactions.insert(id, payment.clone());
    persist!(state, crate::db::mass_primitives::save_transaction, id, &payment);
    Ok(payment)
}

// ── Identity (CNIC/NTN verification) ──────────────────────────────────

/// Verify a CNIC number. Returns mock verification result.
pub fn verify_cnic(cnic: &str, full_name: Option<&str>) -> Result<Value, AppError> {
    let digits: String = cnic.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 13 {
        return Err(AppError::Validation(format!(
            "CNIC must be exactly 13 digits, got {}",
            digits.len()
        )));
    }

    let now = Utc::now().to_rfc3339();
    Ok(json!({
        "id": Uuid::new_v4().to_string(),
        "cnic": digits,
        "verified": true,
        "full_name": full_name,
        "identity_id": Uuid::new_v4().to_string(),
        "verification_timestamp": now,
        "details": {
            "match_score": 0.95,
            "cnic_status": "Active",
            "source": "MockNadraAdapter"
        }
    }))
}

/// Verify an NTN number. Returns mock verification result.
pub fn verify_ntn(ntn: &str, entity_name: Option<&str>) -> Result<Value, AppError> {
    let is_valid = ntn.len() == 7 && ntn.chars().all(|c| c.is_ascii_digit());
    if !is_valid {
        return Err(AppError::Validation(format!(
            "NTN must be exactly 7 digits, got '{ntn}'"
        )));
    }

    let tax_status = match ntn.as_bytes().first() {
        Some(b'0') => "NonFiler",
        Some(b'9') => "LateFiler",
        _ => "Filer",
    };

    let now = Utc::now().to_rfc3339();
    let name = entity_name
        .map(String::from)
        .unwrap_or_else(|| format!("Mock Entity {ntn}"));

    Ok(json!({
        "id": Uuid::new_v4().to_string(),
        "ntn": ntn,
        "verified": true,
        "registered_name": name,
        "tax_status": tax_status,
        "identity_id": Uuid::new_v4().to_string(),
        "verification_timestamp": now,
        "details": {
            "filer_status": tax_status,
            "source": "MockFbrIrisAdapter"
        }
    }))
}

// ── Consent ───────────────────────────────────────────────────────────

/// Create a consent request in the sovereign store.
///
/// Returns JSON matching the shape of `mez_mass_client::consent::ConsentRequest`.
pub async fn create_consent(
    state: &AppState,
    org_id: &str,
    operation_id: Option<&Value>,
    operation_type: Option<&Value>,
    num_approvals: Option<u64>,
    requested_by: Option<&Value>,
) -> Result<Value, AppError> {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();

    let consent = json!({
        "id": id.to_string(),
        "organizationId": org_id,
        "operationId": operation_id,
        "operationType": operation_type,
        "status": "PENDING",
        "votes": [],
        "numVotesRequired": num_approvals,
        "approvalCount": 0,
        "rejectionCount": 0,
        "requestedBy": requested_by,
        "createdAt": now,
        "updatedAt": now
    });

    state.mass_consents.insert(id, consent.clone());
    persist!(state, crate::db::mass_primitives::save_consent, id, &consent);
    Ok(consent)
}

/// Get a consent request from the sovereign store.
pub fn get_consent(state: &AppState, id: Uuid) -> Result<Option<Value>, AppError> {
    Ok(state.mass_consents.get(&id))
}

// ── Withholding computation (pure) ────────────────────────────────────

/// Compute withholding for a transaction. Pure computation, no state needed.
pub fn compute_withholding(
    entity_id: &str,
    transaction_amount: &str,
    currency: &str,
    transaction_type: &str,
    ntn: Option<&str>,
) -> Result<Value, AppError> {
    let (filer_status, ntn_status_str) = match ntn {
        Some(n) if !n.is_empty() => match n.as_bytes().first() {
            Some(b'0') => ("NonFiler", "NonFiler"),
            Some(b'9') => ("LateFiler", "LateFiler"),
            _ => ("Filer", "Filer"),
        },
        _ => ("NonFiler", "NonFiler"),
    };

    let rate_percent: f64 = match (transaction_type, filer_status) {
        ("payment_for_goods", "Filer") => 4.5,
        ("payment_for_goods", "LateFiler") => 6.5,
        ("payment_for_goods", _) => 9.0,
        ("payment_for_services", "Filer") => 8.0,
        ("payment_for_services", "LateFiler") => 12.0,
        ("payment_for_services", _) => 16.0,
        ("salary_payment", "Filer") => 12.5,
        ("salary_payment", "LateFiler") => 15.0,
        ("salary_payment", _) => 20.0,
        (_, "Filer") => 4.5,
        (_, "LateFiler") => 6.5,
        (_, _) => 9.0,
    };

    let gross: f64 = transaction_amount.parse().map_err(|_| {
        AppError::Validation(format!("invalid transaction_amount: {transaction_amount:?}"))
    })?;
    let withholding = gross * rate_percent / 100.0;
    let net = gross - withholding;

    let now = Utc::now().to_rfc3339();

    Ok(json!({
        "entity_id": entity_id,
        "gross_amount": format!("{gross:.2}"),
        "withholding_amount": format!("{withholding:.2}"),
        "withholding_rate": format!("{rate_percent}"),
        "net_amount": format!("{net:.2}"),
        "currency": currency,
        "withholding_type": transaction_type,
        "ntn_status": ntn_status_str,
        "computed_at": now
    }))
}
