// SPDX-License-Identifier: BUSL-1.1
//! Sovereign Mass routes — serves Mass primitive endpoints directly from
//! Postgres-backed in-memory stores when `SOVEREIGN_MASS=true`.
//!
//! Ported from `mez-mass-stub/src/routes.rs` with identical JSON shapes.
//! The key difference: after each write, data is persisted to Postgres
//! via `db::mass_primitives`.
//!
//! See ADR-007 for rationale.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::state::AppState;

/// Build the complete sovereign Mass router.
///
/// Returns `Router<AppState>` — the caller binds state via `.with_state()`.
/// Mounts all Mass API endpoints and wraps with bearer token auth middleware.
/// Persists writes to Postgres when `db_pool` is available.
pub fn sovereign_mass_router() -> Router<AppState> {
    Router::new()
        // Health
        .route("/health", get(health))
        // ── organization-info (ENTITIES) ───────────────────────────
        .route(
            "/organization-info/api/v1/organization/create",
            post(org_create),
        )
        .route(
            "/organization-info/api/v1/organization/search",
            post(org_search),
        )
        .route(
            "/organization-info/api/v1/organization/supported-jurisdictions",
            get(org_supported_jurisdictions),
        )
        .route(
            "/organization-info/api/v1/organization/:id",
            get(org_get).put(org_update).delete(org_delete),
        )
        .route(
            "/organization-info/api/v1/organization",
            get(org_list),
        )
        // ── organization-info (IDENTITY) ───────────────────────────
        .route(
            "/organization-info/api/v1/membership/:org_id/members",
            get(identity_members),
        )
        .route(
            "/organization-info/api/v1/board/:org_id",
            get(identity_board),
        )
        .route(
            "/organization-info/api/v1/identity/cnic/verify",
            post(identity_verify_cnic),
        )
        .route(
            "/organization-info/api/v1/identity/ntn/verify",
            post(identity_verify_ntn),
        )
        // ── treasury-info (FISCAL) ─────────────────────────────────
        .route(
            "/treasury-info/api/v1/treasury/create",
            post(treasury_create),
        )
        .route(
            "/treasury-info/api/v1/treasury/:id",
            get(treasury_get),
        )
        .route(
            "/treasury-info/api/v1/account/create",
            post(account_create),
        )
        .route(
            "/treasury-info/api/v1/account/:id",
            get(account_get),
        )
        .route(
            "/treasury-info/api/v1/transaction/create/payment",
            post(payment_create),
        )
        .route(
            "/treasury-info/api/v1/transaction/:id",
            get(transaction_get),
        )
        .route(
            "/treasury-info/api/v1/tax-events",
            post(tax_event_create).get(tax_events_list),
        )
        .route(
            "/treasury-info/api/v1/withholding/compute",
            post(withholding_compute),
        )
        // ── consent-info (CONSENT) ─────────────────────────────────
        .route(
            "/consent-info/api/v1/consents",
            post(consent_create),
        )
        .route(
            "/consent-info/api/v1/consents/approve/:id",
            post(consent_approve),
        )
        .route(
            "/consent-info/api/v1/consents/reject/:id",
            post(consent_reject),
        )
        .route(
            "/consent-info/api/v1/consents/organization/:org_id",
            get(consent_list_by_org),
        )
        .route(
            "/consent-info/api/v1/consents/:id",
            get(consent_get).delete(consent_cancel),
        )
        // ── consent-info (OWNERSHIP) ───────────────────────────────
        .route(
            "/consent-info/api/v1/capTables",
            post(cap_table_create),
        )
        .route(
            "/consent-info/api/v1/capTables/organization/:org_id",
            get(cap_table_get_by_org),
        )
        .route(
            "/consent-info/api/v1/capTables/:id",
            get(cap_table_get),
        )
        .route(
            "/consent-info/api/v1/shareClasses/organization/:org_id",
            get(share_classes_get_by_org),
        )
        .route(
            "/consent-info/api/v1/shareholders/organization/:org_id",
            get(shareholders_get_by_org),
        )
        .route(
            "/consent-info/api/v1/identities",
            get(identities_list),
        )
        // ── investment-info (OWNERSHIP) ────────────────────────────
        .route(
            "/investment-info/api/v1/investment",
            post(investment_create),
        )
        .route(
            "/investment-info/api/v1/investment/:id",
            get(investment_get),
        )
        // ── templating-engine ──────────────────────────────────────
        .route(
            "/templating-engine/api/v1/template/sign",
            post(template_sign),
        )
        .route(
            "/templating-engine/api/v1/template/available",
            get(template_available),
        )
        .route(
            "/templating-engine/api/v1/template/:id",
            get(template_get),
        )
        .route(
            "/templating-engine/api/v1/submission/:id",
            get(submission_get),
        )
}

// Auth for sovereign Mass routes is handled by the mez-api auth middleware
// in lib.rs, which wraps all authenticated API routes.

// ── Persistence helper ──────────────────────────────────────────────

/// Persist to Postgres if db_pool is available. Returns an error to the
/// client if persistence fails — the in-memory record would be lost on
/// restart, causing silent data loss (matches corridors.rs / smart_assets.rs
/// error-propagation pattern).
macro_rules! persist {
    ($state:expr, $save_fn:path, $($args:expr),+) => {
        if let Some(ref pool) = $state.db_pool {
            if let Err(e) = $save_fn(pool, $($args),+).await {
                tracing::error!(error = %e, "sovereign mass: failed to persist to database");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "database persist failed"
                    })),
                ).into_response();
            }
        }
    };
}

// ── Health ──────────────────────────────────────────────────────────

async fn health() -> StatusCode {
    StatusCode::OK
}

// ── Organization-info (ENTITIES) ────────────────────────────────────

async fn org_create(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Response {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    let name = body
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();

    if name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "name is required and must not be empty"})),
        )
            .into_response();
    }
    if name.len() > 1000 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "name must not exceed 1000 characters"})),
        )
            .into_response();
    }

    let entity = json!({
        "id": id.to_string(),
        "name": name,
        "jurisdiction": body.get("jurisdiction"),
        "status": "ACTIVE",
        "tags": body.get("tags").cloned().unwrap_or_else(|| json!([])),
        "address": body.get("address"),
        "createdAt": now,
        "updatedAt": now
    });

    state.mass_organizations.insert(id, entity.clone());
    persist!(state, crate::db::mass_primitives::save_organization, id, &entity);
    (StatusCode::CREATED, Json(entity)).into_response()
}

async fn org_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.mass_organizations.get(&id) {
        Some(entry) => Json(entry).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn org_update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<Value>,
) -> Response {
    let updated = state.mass_organizations.update(&id, |val| {
        if let (Some(existing), Some(updates)) = (val.as_object_mut(), body.as_object()) {
            for (k, v) in updates {
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
            Json(val).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn org_delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.mass_organizations.remove(&id) {
        Some(_) => StatusCode::OK.into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(Deserialize)]
struct OrgListQuery {
    ids: Option<String>,
}

async fn org_list(
    State(state): State<AppState>,
    Query(query): Query<OrgListQuery>,
) -> Json<Value> {
    let all = state.mass_organizations.list();
    let results: Vec<Value> = match query.ids {
        Some(ids_str) => {
            let ids: Vec<Uuid> = ids_str
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            ids.iter()
                .filter_map(|id| state.mass_organizations.get(id))
                .collect()
        }
        None => all,
    };
    Json(json!(results))
}

async fn org_search(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let query = body
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let page = body
        .get("page")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let size = body
        .get("size")
        .and_then(|v| v.as_u64())
        .unwrap_or(10)
        .min(1000) as usize;
    // Ensure size is at least 1 to avoid division by zero in div_ceil below.
    let size = size.max(1);

    let all: Vec<Value> = state
        .mass_organizations
        .list()
        .into_iter()
        .filter(|v| {
            if query.is_empty() {
                return true;
            }
            v.get("name")
                .and_then(|n| n.as_str())
                .map(|n| n.to_lowercase().contains(&query))
                .unwrap_or(false)
        })
        .collect();

    let total = all.len();
    let start = page * size;
    let content: Vec<Value> = all.into_iter().skip(start).take(size).collect();
    let total_pages = total.div_ceil(size);

    Json(json!({
        "content": content,
        "totalElements": total,
        "totalPages": total_pages,
        "number": page,
        "size": size
    }))
}

async fn org_supported_jurisdictions() -> Json<Value> {
    Json(json!([
        {"code": "pk", "name": "Pakistan"},
        {"code": "pk-sifc", "name": "Pakistan SIFC"},
        {"code": "ae-difc", "name": "UAE DIFC"}
    ]))
}

// ── Treasury-info (FISCAL) ──────────────────────────────────────────

async fn treasury_create(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Response {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    let entity_id = body
        .get("entityId")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if entity_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "entityId is required"})),
        )
            .into_response();
    }

    let treasury = json!({
        "id": id.to_string(),
        "referenceId": null,
        "entityId": entity_id,
        "name": body.get("entityName"),
        "status": "ACTIVE",
        "context": body.get("context").cloned().unwrap_or(json!("MASS")),
        "createdAt": now,
        "updatedAt": now
    });

    state.mass_treasuries.insert(id, treasury.clone());
    persist!(state, crate::db::mass_primitives::save_treasury, id, &treasury);
    (StatusCode::CREATED, Json(treasury)).into_response()
}

async fn treasury_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.mass_treasuries.get(&id) {
        Some(entry) => Json(entry).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// ── Fiscal: Accounts ────────────────────────────────────────────────

#[derive(Deserialize)]
struct AccountCreateQuery {
    #[serde(rename = "treasuryId")]
    treasury_id: Uuid,
    #[serde(rename = "idempotencyKey")]
    _idempotency_key: String,
    name: Option<String>,
}

async fn account_create(
    State(state): State<AppState>,
    Query(params): Query<AccountCreateQuery>,
) -> Response {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();

    let entity_id = state
        .mass_treasuries
        .get(&params.treasury_id)
        .and_then(|t| t.get("entityId").and_then(|v| v.as_str()).map(String::from));

    let account = json!({
        "id": id.to_string(),
        "entityId": entity_id,
        "treasuryId": params.treasury_id.to_string(),
        "name": params.name.as_deref().unwrap_or("Default Account"),
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
    (StatusCode::CREATED, Json(account)).into_response()
}

async fn account_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.mass_accounts.get(&id) {
        Some(entry) => Json(entry).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// ── Fiscal: Transactions/Payments ───────────────────────────────────

async fn payment_create(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Response {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();

    let account_id = body.get("sourceAccountId").and_then(|v| v.as_str());
    if account_id.map_or(true, |a| a.is_empty()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "sourceAccountId is required"})),
        )
            .into_response();
    }
    let amount = body.get("amount").and_then(|v| v.as_str());
    if amount.map_or(true, |a| a.is_empty()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "amount is required"})),
        )
            .into_response();
    }
    let entity_id = account_id
        .and_then(|aid| aid.parse::<Uuid>().ok())
        .and_then(|aid| {
            state
                .mass_accounts
                .get(&aid)
                .and_then(|a| a.get("entityId").and_then(|v| v.as_str()).map(String::from))
        });

    let payment = json!({
        "id": id.to_string(),
        "accountId": account_id,
        "entityId": entity_id,
        "transactionType": body.get("paymentType").cloned().unwrap_or(json!("PAYMENT")),
        "status": "PENDING",
        "direction": "OUTBOUND",
        "currency": body.get("currency").cloned().unwrap_or(json!("PKR")),
        "amount": body.get("amount"),
        "reference": body.get("reference"),
        "createdAt": now
    });

    state.mass_transactions.insert(id, payment.clone());
    persist!(state, crate::db::mass_primitives::save_transaction, id, &payment);
    (StatusCode::CREATED, Json(payment)).into_response()
}

async fn transaction_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.mass_transactions.get(&id) {
        Some(entry) => Json(entry).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// ── Fiscal: Tax Events ──────────────────────────────────────────────

async fn tax_event_create(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Response {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();

    // RecordTaxEventRequest uses snake_case (no rename_all).
    let entity_id = body
        .get("entity_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if entity_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "entity_id is required"})),
        )
            .into_response();
    }
    let event_type = body
        .get("event_type")
        .and_then(|v| v.as_str())
        .unwrap_or("UNKNOWN");
    let amount = body
        .get("amount")
        .and_then(|v| v.as_str())
        .unwrap_or("0");
    let currency = body
        .get("currency")
        .and_then(|v| v.as_str())
        .unwrap_or("PKR");
    let tax_year = body.get("tax_year").and_then(|v| v.as_str());

    // Response is MassTaxEvent (rename_all = "camelCase").
    let tax_event = json!({
        "id": id.to_string(),
        "entityId": entity_id,
        "eventType": event_type,
        "amount": amount,
        "currency": currency,
        "taxYear": tax_year,
        "details": body.get("details").cloned().unwrap_or(json!({})),
        "createdAt": now
    });

    state.mass_tax_events_sovereign.insert(id, tax_event.clone());
    persist!(state, crate::db::mass_primitives::save_mass_tax_event, id, &tax_event);
    (StatusCode::CREATED, Json(tax_event)).into_response()
}

#[derive(Deserialize)]
struct TaxEventsQuery {
    entity_id: Option<String>,
    tax_year: Option<String>,
}

async fn tax_events_list(
    State(state): State<AppState>,
    Query(params): Query<TaxEventsQuery>,
) -> Json<Value> {
    let results: Vec<Value> = state
        .mass_tax_events_sovereign
        .list()
        .into_iter()
        .filter(|v| {
            if let Some(ref eid) = params.entity_id {
                let matches = v
                    .get("entityId")
                    .and_then(|v| v.as_str())
                    .map(|v| v == eid)
                    .unwrap_or(false);
                if !matches {
                    return false;
                }
            }
            if let Some(ref year) = params.tax_year {
                let matches = v
                    .get("taxYear")
                    .and_then(|v| v.as_str())
                    .map(|v| v == year)
                    .unwrap_or(false);
                if !matches {
                    return false;
                }
            }
            true
        })
        .collect();
    Json(json!(results))
}

// ── Fiscal: Withholding Compute ─────────────────────────────────────

async fn withholding_compute(Json(body): Json<Value>) -> Response {
    let entity_id = body
        .get("entity_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let transaction_amount_str = body
        .get("transaction_amount")
        .and_then(|v| v.as_str())
        .unwrap_or("0");
    let currency = body
        .get("currency")
        .and_then(|v| v.as_str())
        .unwrap_or("PKR");
    let transaction_type = body
        .get("transaction_type")
        .and_then(|v| v.as_str())
        .unwrap_or("payment_for_goods");
    let ntn = body.get("ntn").and_then(|v| v.as_str());

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

    let gross: f64 = match transaction_amount_str.parse() {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("invalid transaction_amount: {transaction_amount_str:?}")})),
            ).into_response();
        }
    };
    let withholding = gross * rate_percent / 100.0;
    let net = gross - withholding;

    let now = Utc::now().to_rfc3339();

    let result = json!({
        "entity_id": entity_id,
        "gross_amount": format!("{gross:.2}"),
        "withholding_amount": format!("{withholding:.2}"),
        "withholding_rate": format!("{rate_percent}"),
        "net_amount": format!("{net:.2}"),
        "currency": currency,
        "withholding_type": transaction_type,
        "ntn_status": ntn_status_str,
        "computed_at": now
    });

    Json(result).into_response()
}

// ── Consent-info (CONSENT) ──────────────────────────────────────────

async fn consent_create(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Response {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    let org_id = body
        .get("organizationId")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if org_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "organizationId is required"})),
        )
            .into_response();
    }

    let consent = json!({
        "id": id.to_string(),
        "organizationId": org_id,
        "operationId": body.get("operationId"),
        "operationType": body.get("operationType"),
        "status": "PENDING",
        "votes": [],
        "numVotesRequired": body.get("numBoardMemberApprovalsRequired"),
        "approvalCount": 0,
        "rejectionCount": 0,
        "requestedBy": body.get("requestedBy"),
        "createdAt": now,
        "updatedAt": now
    });

    state.mass_consents.insert(id, consent.clone());
    persist!(state, crate::db::mass_primitives::save_consent, id, &consent);
    (StatusCode::CREATED, Json(consent)).into_response()
}

async fn consent_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.mass_consents.get(&id) {
        Some(entry) => Json(entry).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn consent_approve(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    let result = state.mass_consents.update(&id, |consent| {
        if let Some(obj) = consent.as_object_mut() {
            let count = obj
                .get("approvalCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                + 1;
            obj.insert("approvalCount".to_string(), json!(count));

            let required = obj
                .get("numVotesRequired")
                .and_then(|v| v.as_u64())
                .unwrap_or(1);
            if count >= required {
                obj.insert("status".to_string(), json!("APPROVED"));
            }
            obj.insert("updatedAt".to_string(), json!(Utc::now().to_rfc3339()));
        }
    });

    match result {
        Some(consent) => {
            let org_id = consent
                .get("organizationId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let operation_id = consent.get("operationId").cloned();
            let operation_type = consent.get("operationType").cloned();
            let status = consent
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("PENDING");
            let majority_reached = status == "APPROVED";

            persist!(state, crate::db::mass_primitives::save_consent, id, &consent);

            let vote_response = json!({
                "consentId": id.to_string(),
                "operationId": operation_id,
                "organizationId": org_id,
                "vote": "APPROVED",
                "votedBy": "system",
                "operationType": operation_type,
                "majorityReached": majority_reached,
                "createdAt": Utc::now().to_rfc3339()
            });

            Json(vote_response).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn consent_reject(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    let result = state.mass_consents.update(&id, |consent| {
        if let Some(obj) = consent.as_object_mut() {
            let count = obj
                .get("rejectionCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                + 1;
            obj.insert("rejectionCount".to_string(), json!(count));

            let required = obj
                .get("numVotesRequired")
                .and_then(|v| v.as_u64())
                .unwrap_or(1);
            if count >= required {
                obj.insert("status".to_string(), json!("REJECTED"));
            }
            obj.insert("updatedAt".to_string(), json!(Utc::now().to_rfc3339()));
        }
    });

    match result {
        Some(consent) => {
            let org_id = consent
                .get("organizationId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let operation_id = consent.get("operationId").cloned();
            let operation_type = consent.get("operationType").cloned();
            let status = consent
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("PENDING");
            let majority_reached = status == "REJECTED";

            persist!(state, crate::db::mass_primitives::save_consent, id, &consent);

            let vote_response = json!({
                "consentId": id.to_string(),
                "operationId": operation_id,
                "organizationId": org_id,
                "vote": "REJECTED",
                "votedBy": "system",
                "operationType": operation_type,
                "majorityReached": majority_reached,
                "createdAt": Utc::now().to_rfc3339()
            });

            Json(vote_response).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn consent_list_by_org(
    State(state): State<AppState>,
    Path(org_id): Path<String>,
) -> Json<Value> {
    let results: Vec<Value> = state
        .mass_consents
        .list()
        .into_iter()
        .filter(|v| {
            v.get("organizationId")
                .and_then(|v| v.as_str())
                .map(|v| v == org_id)
                .unwrap_or(false)
        })
        .collect();
    Json(json!(results))
}

async fn consent_cancel(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    let result = state.mass_consents.update(&id, |consent| {
        if let Some(obj) = consent.as_object_mut() {
            obj.insert("status".to_string(), json!("CANCELED"));
            obj.insert("updatedAt".to_string(), json!(Utc::now().to_rfc3339()));
        }
    });

    match result {
        Some(consent) => {
            persist!(state, crate::db::mass_primitives::save_consent, id, &consent);
            StatusCode::OK.into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// ── Consent-info (OWNERSHIP) ────────────────────────────────────────

async fn cap_table_create(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Response {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    let org_id = body
        .get("organizationId")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if org_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "organizationId is required"})),
        )
            .into_response();
    }
    let authorized_shares = body
        .get("authorizedShares")
        .and_then(|v| v.as_u64())
        .unwrap_or(10_000_000);

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
            "parValue": body.get("parValue").and_then(|v| v.as_str()).unwrap_or("1.00"),
            "votingRights": true,
            "restricted": false,
            "classType": "COMMON"
        }],
        "shareholders": body.get("shareholders").cloned().unwrap_or(json!([])),
        "optionsPools": [],
        "createdAt": now,
        "updatedAt": now
    });

    state.mass_cap_tables.insert(id, cap_table.clone());
    persist!(state, crate::db::mass_primitives::save_cap_table, id, &cap_table);
    (StatusCode::CREATED, Json(cap_table)).into_response()
}

async fn cap_table_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.mass_cap_tables.get(&id) {
        Some(entry) => Json(entry).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn cap_table_get_by_org(
    State(state): State<AppState>,
    Path(org_id): Path<String>,
) -> Response {
    let found = state.mass_cap_tables.list().into_iter().find(|v| {
        v.get("organizationId")
            .and_then(|v| v.as_str())
            .map(|v| v == org_id)
            .unwrap_or(false)
    });
    match found {
        Some(ct) => Json(ct).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn share_classes_get_by_org(
    State(state): State<AppState>,
    Path(org_id): Path<String>,
) -> Json<Value> {
    let classes: Vec<Value> = state
        .mass_cap_tables
        .list()
        .into_iter()
        .filter(|v| {
            v.get("organizationId")
                .and_then(|v| v.as_str())
                .map(|v| v == org_id)
                .unwrap_or(false)
        })
        .flat_map(|v| {
            v.get("shareClasses")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
        })
        .collect();
    Json(json!(classes))
}

// ── Consent-info (IDENTITY: shareholders) ───────────────────────────

async fn shareholders_get_by_org(
    State(state): State<AppState>,
    Path(org_id): Path<String>,
) -> Json<Value> {
    let shareholders = state
        .mass_shareholders_by_org
        .read()
        .get(&org_id)
        .cloned()
        .unwrap_or_default();
    Json(json!(shareholders))
}

#[derive(Deserialize)]
struct IdentitiesListQuery {
    entity_id: Option<String>,
}

async fn identities_list(
    Query(params): Query<IdentitiesListQuery>,
) -> Json<Value> {
    // Identity service stub — returns empty list, filtered by entity_id if provided.
    let _ = params.entity_id;
    Json(json!([]))
}

// ── Investment-info (OWNERSHIP) ─────────────────────────────────────

async fn investment_create(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Response {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();

    let mut investment = body.clone();
    if let Some(obj) = investment.as_object_mut() {
        obj.insert("id".to_string(), json!(id.to_string()));
        obj.insert("createdAt".to_string(), json!(now.clone()));
        obj.insert("updatedAt".to_string(), json!(now));
    }

    state.mass_investments.insert(id, investment.clone());
    persist!(state, crate::db::mass_primitives::save_investment, id, &investment);
    (StatusCode::CREATED, Json(investment)).into_response()
}

async fn investment_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.mass_investments.get(&id) {
        Some(entry) => Json(entry).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// ── Organization-info (IDENTITY) ────────────────────────────────────

async fn identity_members(
    State(state): State<AppState>,
    Path(org_id): Path<String>,
) -> Json<Value> {
    let members = state
        .mass_members_by_org
        .read()
        .get(&org_id)
        .cloned()
        .unwrap_or_default();
    Json(json!(members))
}

async fn identity_board(
    State(state): State<AppState>,
    Path(org_id): Path<String>,
) -> Json<Value> {
    let board = state
        .mass_board_by_org
        .read()
        .get(&org_id)
        .cloned()
        .unwrap_or_default();
    Json(json!(board))
}

async fn identity_verify_cnic(Json(body): Json<Value>) -> Response {
    let cnic_raw = body
        .get("cnic")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let full_name = body
        .get("full_name")
        .and_then(|v| v.as_str())
        .map(String::from);

    let digits: String = cnic_raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 13 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("CNIC must be exactly 13 digits, got {}", digits.len())
            })),
        )
            .into_response();
    }

    let now = Utc::now().to_rfc3339();
    let response = json!({
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
    });

    Json(response).into_response()
}

async fn identity_verify_ntn(Json(body): Json<Value>) -> Response {
    let ntn = body.get("ntn").and_then(|v| v.as_str()).unwrap_or("");
    let entity_name = body
        .get("entity_name")
        .and_then(|v| v.as_str())
        .map(String::from);

    let is_valid = ntn.len() == 7 && ntn.chars().all(|c| c.is_ascii_digit());
    if !is_valid {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("NTN must be exactly 7 digits, got '{ntn}'")
            })),
        )
            .into_response();
    }

    let tax_status = match ntn.as_bytes().first() {
        Some(b'0') => "NonFiler",
        Some(b'9') => "LateFiler",
        _ => "Filer",
    };

    let now = Utc::now().to_rfc3339();
    let response = json!({
        "ntn": ntn,
        "verified": true,
        "registered_name": entity_name.unwrap_or_else(|| format!("Mock Entity {ntn}")),
        "tax_status": tax_status,
        "identity_id": Uuid::new_v4().to_string(),
        "verification_timestamp": now,
        "details": {
            "filer_status": tax_status,
            "source": "MockFbrIrisAdapter"
        }
    });

    Json(response).into_response()
}

// ── Templating-engine ───────────────────────────────────────────────

async fn template_sign(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Response {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let submission = json!({
        "id": id,
        "entityId": body.get("entityId"),
        "context": "SOVEREIGN_GOVOS",
        "status": "PENDING",
        "signingOrder": body.get("signingOrder").cloned().unwrap_or(json!("RANDOM")),
        "signers": body.get("signers").cloned().unwrap_or(json!([])),
        "documentUri": null,
        "createdAt": now,
        "updatedAt": now
    });

    state.mass_submissions.write().insert(id.clone(), submission.clone());
    if let Some(ref pool) = state.db_pool {
        if let Err(e) = crate::db::mass_primitives::save_submission(pool, &id, &submission).await {
            tracing::error!(error = %e, "sovereign mass: failed to persist submission");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "database persist failed"})),
            ).into_response();
        }
    }
    (StatusCode::CREATED, Json(submission)).into_response()
}

async fn template_available() -> Json<Value> {
    Json(json!([
        {
            "templateType": "CERTIFICATE_OF_INCORPORATION",
            "templateName": "Certificate of Incorporation",
            "entityId": null,
            "templateId": "tpl-cert-incorp-001",
            "documentUri": null
        },
        {
            "templateType": "SHAREHOLDER_AGREEMENT",
            "templateName": "Shareholder Agreement",
            "entityId": null,
            "templateId": "tpl-sha-001",
            "documentUri": null
        },
        {
            "templateType": "BOARD_RESOLUTION",
            "templateName": "Board Resolution",
            "entityId": null,
            "templateId": "tpl-board-res-001",
            "documentUri": null
        }
    ]))
}

async fn template_get(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let guard = state.mass_templates.read();
    match guard.get(&id) {
        Some(entry) => Json(entry.clone()).into_response(),
        None => {
            drop(guard);
            if id.starts_with("tpl-") {
                let template = json!({
                    "id": id,
                    "name": format!("Template {id}"),
                    "context": "SOVEREIGN_GOVOS",
                    "entityId": null,
                    "version": "1.0",
                    "type": "DOCUMENT",
                    "grouping": "CORPORATE",
                    "status": "ACTIVE"
                });
                Json(template).into_response()
            } else {
                StatusCode::NOT_FOUND.into_response()
            }
        }
    }
}

async fn submission_get(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let guard = state.mass_submissions.read();
    match guard.get(&id) {
        Some(entry) => Json(entry.clone()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
