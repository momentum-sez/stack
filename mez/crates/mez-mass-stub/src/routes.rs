// SPDX-License-Identifier: BUSL-1.1
//! Route definitions for the Mass API stub.
//!
//! Implements every endpoint that `mez-mass-client` sub-clients call, with
//! responses that deserialize cleanly into the client's types (camelCase
//! or snake_case JSON per each type's serde attributes).

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::store::AppState;

/// Build the complete router with all Mass API stub routes.
pub fn router(state: AppState) -> Router {
    let app = Router::new()
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
        // Fallback: 501 Not Implemented
        .fallback(not_implemented)
        .with_state(state.clone());

    // Wrap with bearer token auth middleware.
    app.layer(middleware::from_fn_with_state(state, auth_middleware))
}

// ── Auth Middleware ──────────────────────────────────────────────────

async fn auth_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if let Some(expected) = state.auth_token() {
        // Skip auth for health endpoint.
        if request.uri().path() != "/health" {
            let authorized = request
                .headers()
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .map(|v| {
                    let expected_header = format!("Bearer {expected}");
                    v == expected_header
                })
                .unwrap_or(false);
            if !authorized {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": "unauthorized"})),
                )
                    .into_response();
            }
        }
    }
    next.run(request).await
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
    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("");

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

    state.organizations().insert(id, entity.clone());
    (StatusCode::CREATED, Json(entity)).into_response()
}

async fn org_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.organizations().get(&id) {
        Some(entry) => Json(entry.value().clone()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn org_update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<Value>,
) -> Response {
    match state.organizations().get_mut(&id) {
        Some(mut entry) => {
            let val = entry.value_mut();
            if let (Some(existing), Some(updates)) = (val.as_object_mut(), body.as_object()) {
                for (k, v) in updates {
                    existing.insert(k.clone(), v.clone());
                }
                existing.insert(
                    "updatedAt".to_string(),
                    json!(Utc::now().to_rfc3339()),
                );
            }
            Json(val.clone()).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn org_delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.organizations().remove(&id) {
        Some(_) => StatusCode::NO_CONTENT.into_response(),
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
) -> Result<Json<Value>, (StatusCode, String)> {
    const MAX_LIST: usize = 1000;
    let results: Vec<Value> = match query.ids {
        Some(ids_str) => {
            let ids: Vec<Uuid> = ids_str
                .split(',')
                .take(MAX_LIST)
                .map(|s| {
                    let trimmed = s.trim();
                    trimmed.parse::<Uuid>().map_err(|_| {
                        (StatusCode::UNPROCESSABLE_ENTITY, format!("invalid UUID in ids parameter: {trimmed:?}"))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            ids.iter()
                .filter_map(|id| state.organizations().get(id).map(|e| e.value().clone()))
                .collect()
        }
        None => state
            .organizations()
            .iter()
            .take(MAX_LIST)
            .map(|e| e.value().clone())
            .collect(),
    };
    Ok(Json(json!(results)))
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
        .min(100) as usize;

    let all: Vec<Value> = state
        .organizations()
        .iter()
        .map(|e| e.value().clone())
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

    state.treasuries().insert(id, treasury.clone());
    (StatusCode::CREATED, Json(treasury)).into_response()
}

async fn treasury_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.treasuries().get(&id) {
        Some(entry) => Json(entry.value().clone()).into_response(),
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

/// POST /treasury-info/api/v1/account/create?treasuryId=...&idempotencyKey=...&name=...
/// FiscalClient.create_account sends empty POST with query params.
/// Returns MassFiscalAccount-shaped JSON (rename_all = "camelCase").
async fn account_create(
    State(state): State<AppState>,
    Query(params): Query<AccountCreateQuery>,
) -> Response {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();

    // Look up the treasury to get entity_id.
    let entity_id = state
        .treasuries()
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

    state.accounts().insert(id, account.clone());
    (StatusCode::CREATED, Json(account)).into_response()
}

/// GET /treasury-info/api/v1/account/{id}
/// Returns MassFiscalAccount-shaped JSON.
async fn account_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.accounts().get(&id) {
        Some(entry) => Json(entry.value().clone()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// ── Fiscal: Transactions/Payments ───────────────────────────────────

/// POST /treasury-info/api/v1/transaction/create/payment
/// Body is CreatePaymentRequest-shaped (rename_all = "camelCase").
/// Returns MassPayment-shaped JSON (rename_all = "camelCase").
async fn payment_create(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Response {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();

    let account_id = body.get("sourceAccountId").and_then(|v| v.as_str());
    // Look up entity from account.
    let entity_id = account_id
        .and_then(|aid| aid.parse::<Uuid>().ok())
        .and_then(|aid| {
            state
                .accounts()
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

    state.transactions().insert(id, payment.clone());
    (StatusCode::CREATED, Json(payment)).into_response()
}

/// GET /treasury-info/api/v1/transaction/{id}
/// Returns MassPayment-shaped JSON.
async fn transaction_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.transactions().get(&id) {
        Some(entry) => Json(entry.value().clone()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// ── Fiscal: Tax Events ──────────────────────────────────────────────

/// POST /treasury-info/api/v1/tax-events
/// Body is RecordTaxEventRequest-shaped (NO rename_all — snake_case keys).
/// Returns MassTaxEvent-shaped JSON (rename_all = "camelCase").
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

    state.tax_events().insert(id, tax_event.clone());
    (StatusCode::CREATED, Json(tax_event)).into_response()
}

#[derive(Deserialize)]
struct TaxEventsQuery {
    entity_id: Option<String>,
    tax_year: Option<String>,
}

/// GET /treasury-info/api/v1/tax-events?entity_id=...&tax_year=...
/// Returns Vec<MassTaxEvent> (camelCase).
async fn tax_events_list(
    State(state): State<AppState>,
    Query(params): Query<TaxEventsQuery>,
) -> Json<Value> {
    const MAX_LIST: usize = 1000;
    let results: Vec<Value> = state
        .tax_events()
        .iter()
        .map(|e| e.value().clone())
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
        .take(MAX_LIST)
        .collect();
    Json(json!(results))
}

// ── Fiscal: Withholding Compute ─────────────────────────────────────

/// POST /treasury-info/api/v1/withholding/compute
/// Body is WithholdingComputeRequest (NO rename_all — snake_case keys).
/// Returns WithholdingResult (NO rename_all — snake_case keys).
///
/// Deterministic: derives filer status from NTN prefix per MockFbrIrisAdapter
/// convention. Applies ITO 2001 rates.
async fn withholding_compute(Json(body): Json<Value>) -> Response {
    // Parse snake_case input fields.
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

    // Determine filer status from NTN prefix (mirror MockFbrIrisAdapter).
    let (filer_status, ntn_status_str) = match ntn {
        Some(n) if !n.is_empty() => match n.as_bytes().first() {
            Some(b'0') => ("NonFiler", "NonFiler"),
            Some(b'9') => ("LateFiler", "LateFiler"),
            _ => ("Filer", "Filer"),
        },
        _ => ("NonFiler", "NonFiler"),
    };

    // Look up rate based on transaction type and filer status (ITO 2001).
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
        Ok(v) if f64::is_finite(v) => v,
        _ => {
            return (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({
                "error": "invalid transaction_amount: must be a finite number"
            }))).into_response();
        }
    };
    let withholding = gross * rate_percent / 100.0;
    let net = gross - withholding;

    let now = Utc::now().to_rfc3339();

    // Response uses snake_case (WithholdingResult has no rename_all).
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

    state.consents().insert(id, consent.clone());
    (StatusCode::CREATED, Json(consent)).into_response()
}

async fn consent_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.consents().get(&id) {
        Some(entry) => Json(entry.value().clone()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// POST /consent-info/api/v1/consents/approve/{id}
/// Returns MassConsentVoteResponse-shaped JSON (camelCase).
async fn consent_approve(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.consents().get_mut(&id) {
        Some(mut entry) => {
            let consent = entry.value_mut();
            if let Some(obj) = consent.as_object_mut() {
                obj.insert("status".to_string(), json!("APPROVED"));
                let count = obj
                    .get("approvalCount")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                obj.insert("approvalCount".to_string(), json!(count + 1));
                obj.insert("updatedAt".to_string(), json!(Utc::now().to_rfc3339()));
            }

            let org_id = consent
                .get("organizationId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let operation_id = consent.get("operationId").cloned();
            let operation_type = consent.get("operationType").cloned();

            let vote_response = json!({
                "consentId": id.to_string(),
                "operationId": operation_id,
                "organizationId": org_id,
                "vote": "APPROVED",
                "votedBy": "system",
                "operationType": operation_type,
                "majorityReached": true,
                "createdAt": Utc::now().to_rfc3339()
            });

            Json(vote_response).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// POST /consent-info/api/v1/consents/reject/{id}
/// Returns MassConsentVoteResponse-shaped JSON (camelCase).
async fn consent_reject(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.consents().get_mut(&id) {
        Some(mut entry) => {
            let consent = entry.value_mut();
            if let Some(obj) = consent.as_object_mut() {
                obj.insert("status".to_string(), json!("REJECTED"));
                let count = obj
                    .get("rejectionCount")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                obj.insert("rejectionCount".to_string(), json!(count + 1));
                obj.insert("updatedAt".to_string(), json!(Utc::now().to_rfc3339()));
            }

            let org_id = consent
                .get("organizationId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let operation_id = consent.get("operationId").cloned();
            let operation_type = consent.get("operationType").cloned();

            let vote_response = json!({
                "consentId": id.to_string(),
                "operationId": operation_id,
                "organizationId": org_id,
                "vote": "REJECTED",
                "votedBy": "system",
                "operationType": operation_type,
                "majorityReached": false,
                "createdAt": Utc::now().to_rfc3339()
            });

            Json(vote_response).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// GET /consent-info/api/v1/consents/organization/{org_id}
/// Returns Vec<MassConsent> matching the organization.
async fn consent_list_by_org(
    State(state): State<AppState>,
    Path(org_id): Path<String>,
) -> Json<Value> {
    const MAX_LIST: usize = 1000;
    let results: Vec<Value> = state
        .consents()
        .iter()
        .map(|e| e.value().clone())
        .filter(|v| {
            v.get("organizationId")
                .and_then(|v| v.as_str())
                .map(|v| v == org_id)
                .unwrap_or(false)
        })
        .take(MAX_LIST)
        .collect();
    Json(json!(results))
}

/// DELETE /consent-info/api/v1/consents/{id}
/// Sets status to CANCELED and returns 200.
async fn consent_cancel(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.consents().get_mut(&id) {
        Some(mut entry) => {
            let consent = entry.value_mut();
            if let Some(obj) = consent.as_object_mut() {
                obj.insert("status".to_string(), json!("CANCELED"));
                obj.insert("updatedAt".to_string(), json!(Utc::now().to_rfc3339()));
            }
            StatusCode::OK.into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// ── Consent-info (OWNERSHIP) ────────────────────────────────────────

/// POST /consent-info/api/v1/capTables
/// Body is CreateCapTableRequest-shaped (camelCase).
/// Returns MassCapTable-shaped JSON (camelCase).
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

    state.cap_tables().insert(id, cap_table.clone());
    (StatusCode::CREATED, Json(cap_table)).into_response()
}

/// GET /consent-info/api/v1/capTables/{id}
/// Returns MassCapTable-shaped JSON.
async fn cap_table_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.cap_tables().get(&id) {
        Some(entry) => Json(entry.value().clone()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// GET /consent-info/api/v1/capTables/organization/{org_id}
/// Returns MassCapTable or 404.
async fn cap_table_get_by_org(
    State(state): State<AppState>,
    Path(org_id): Path<String>,
) -> Response {
    let found = state.cap_tables().iter().find_map(|e| {
        let v = e.value();
        if v.get("organizationId")
            .and_then(|v| v.as_str())
            .map(|v| v == org_id)
            .unwrap_or(false)
        {
            Some(v.clone())
        } else {
            None
        }
    });
    match found {
        Some(ct) => Json(ct).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// GET /consent-info/api/v1/shareClasses/organization/{org_id}
/// Returns Vec<MassShareClass> from the cap table.
async fn share_classes_get_by_org(
    State(state): State<AppState>,
    Path(org_id): Path<String>,
) -> Json<Value> {
    let classes: Vec<Value> = state
        .cap_tables()
        .iter()
        .filter(|e| {
            e.value()
                .get("organizationId")
                .and_then(|v| v.as_str())
                .map(|v| v == org_id)
                .unwrap_or(false)
        })
        .flat_map(|e| {
            e.value()
                .get("shareClasses")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
        })
        .collect();
    Json(json!(classes))
}

// ── Consent-info (IDENTITY: shareholders) ───────────────────────────

/// GET /consent-info/api/v1/shareholders/organization/{org_id}
/// Returns Vec<MassShareholder> (camelCase).
async fn shareholders_get_by_org(
    State(state): State<AppState>,
    Path(org_id): Path<String>,
) -> Json<Value> {
    let shareholders = state
        .shareholders_by_org()
        .get(&org_id)
        .map(|v| v.value().clone())
        .unwrap_or_default();
    Json(json!(shareholders))
}

/// GET /consent-info/api/v1/identities?entity_id=...
/// Returns Vec<MassIdentity> (NO rename_all — snake_case).
async fn identities_list(
    Query(params): Query<IdentitiesListQuery>,
) -> Json<Value> {
    // In stub mode, return an empty list. Real identity data is assembled
    // by the IdentityClient facade from members + board + shareholders.
    let _ = params.entity_id;
    Json(json!([]))
}

#[derive(Deserialize)]
struct IdentitiesListQuery {
    entity_id: Option<String>,
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

    state.investments().insert(id, investment.clone());
    (StatusCode::CREATED, Json(investment)).into_response()
}

async fn investment_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.investments().get(&id) {
        Some(entry) => Json(entry.value().clone()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// ── Organization-info (IDENTITY) ────────────────────────────────────

/// GET /organization-info/api/v1/membership/{orgId}/members
/// Returns Vec<MassMember> (camelCase).
async fn identity_members(
    State(state): State<AppState>,
    Path(org_id): Path<String>,
) -> Json<Value> {
    let members = state
        .members_by_org()
        .get(&org_id)
        .map(|v| v.value().clone())
        .unwrap_or_default();
    Json(json!(members))
}

/// GET /organization-info/api/v1/board/{orgId}
/// Returns Vec<MassDirector> (camelCase).
async fn identity_board(
    State(state): State<AppState>,
    Path(org_id): Path<String>,
) -> Json<Value> {
    let board = state
        .board_by_org()
        .get(&org_id)
        .map(|v| v.value().clone())
        .unwrap_or_default();
    Json(json!(board))
}

/// POST /organization-info/api/v1/identity/cnic/verify
/// Body is CnicVerificationRequest (NO rename_all — snake_case keys).
/// Returns CnicVerificationResponse (NO rename_all — snake_case keys).
/// Mirrors MockNadraAdapter: strip dashes, validate 13 digits, verified=true, match_score=0.95.
async fn identity_verify_cnic(Json(body): Json<Value>) -> Response {
    let cnic_raw = body
        .get("cnic")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let full_name = body
        .get("full_name")
        .and_then(|v| v.as_str())
        .map(String::from);

    // Strip dashes and validate 13 digits.
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

/// POST /organization-info/api/v1/identity/ntn/verify
/// Body is NtnVerificationRequest (NO rename_all — snake_case keys).
/// Returns NtnVerificationResponse (NO rename_all — snake_case keys).
/// Mirrors MockFbrIrisAdapter: prefix '0' = NonFiler, '9' = LateFiler, else Filer.
async fn identity_verify_ntn(Json(body): Json<Value>) -> Response {
    let ntn = body.get("ntn").and_then(|v| v.as_str()).unwrap_or("");
    let entity_name = body
        .get("entity_name")
        .and_then(|v| v.as_str())
        .map(String::from);

    // Validate 7 digits.
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

    // Determine filer status from NTN prefix.
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

/// POST /templating-engine/api/v1/template/sign
/// Body is SignTemplateRequest (camelCase).
/// Returns SubmissionResponse-shaped JSON (camelCase).
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

    state.submissions().insert(id.clone(), submission.clone());
    (StatusCode::CREATED, Json(submission)).into_response()
}

/// GET /templating-engine/api/v1/template/available
/// Returns Vec<TemplateOption> (camelCase).
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

/// GET /templating-engine/api/v1/template/{id}
/// Returns Template-shaped JSON (camelCase) or 404.
async fn template_get(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    match state.templates().get(&id) {
        Some(entry) => Json(entry.value().clone()).into_response(),
        None => {
            // Return a default template for any known template ID prefix.
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

/// GET /templating-engine/api/v1/submission/{id}
/// Returns SubmissionResponse-shaped JSON (camelCase) or 404.
async fn submission_get(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    match state.submissions().get(&id) {
        Some(entry) => Json(entry.value().clone()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// ── Fallback ────────────────────────────────────────────────────────

async fn not_implemented() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_app() -> Router {
        router(AppState::new())
    }

    async fn body_json(resp: Response) -> Value {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn health_returns_200() {
        let app = test_app();
        let req = axum::http::Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn org_crud_lifecycle() {
        let state = AppState::new();
        let app = router(state);

        // Create
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/organization-info/api/v1/organization/create")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "name": "Test Corp",
                    "jurisdiction": "pk-sifc",
                    "tags": ["ez"]
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let created = body_json(resp).await;
        let id = created["id"].as_str().unwrap();
        assert_eq!(created["name"], "Test Corp");
        assert_eq!(created["status"], "ACTIVE");

        // Get
        let get_uri = format!("/organization-info/api/v1/organization/{id}");
        let req = axum::http::Request::builder()
            .uri(&get_uri)
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let fetched = body_json(resp).await;
        assert_eq!(fetched["name"], "Test Corp");

        // Update
        let req = axum::http::Request::builder()
            .method("PUT")
            .uri(&get_uri)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({"name": "Updated Corp"})).unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let updated = body_json(resp).await;
        assert_eq!(updated["name"], "Updated Corp");

        // Delete
        let req = axum::http::Request::builder()
            .method("DELETE")
            .uri(&get_uri)
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // Get after delete → 404
        let req = axum::http::Request::builder()
            .uri(&get_uri)
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn org_search_filters_by_name() {
        let state = AppState::new();
        let app = router(state.clone());

        for name in &["Alpha Corp", "Beta Inc"] {
            let req = axum::http::Request::builder()
                .method("POST")
                .uri("/organization-info/api/v1/organization/create")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&json!({"name": name, "tags": []})).unwrap(),
                ))
                .unwrap();
            app.clone().oneshot(req).await.unwrap();
        }

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/organization-info/api/v1/organization/search")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({"query": "alpha", "page": 0, "size": 10})).unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let result = body_json(resp).await;
        assert_eq!(result["totalElements"], 1);
        assert_eq!(result["content"][0]["name"], "Alpha Corp");
    }

    #[tokio::test]
    async fn treasury_create_and_get() {
        let state = AppState::new();
        let app = router(state);

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/treasury-info/api/v1/treasury/create")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "entityId": "some-entity",
                    "entityName": "Test Treasury"
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let created = body_json(resp).await;
        let id = created["id"].as_str().unwrap();

        let req = axum::http::Request::builder()
            .uri(format!("/treasury-info/api/v1/treasury/{id}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn account_create_and_get() {
        let state = AppState::new();
        let app = router(state.clone());

        // Create treasury first.
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/treasury-info/api/v1/treasury/create")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({"entityId": "ent-1"})).unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let treasury = body_json(resp).await;
        let treasury_id = treasury["id"].as_str().unwrap();

        // Create account via query params.
        let uri = format!(
            "/treasury-info/api/v1/account/create?treasuryId={}&idempotencyKey=key1&name=MyAcct",
            treasury_id
        );
        let req = axum::http::Request::builder()
            .method("POST")
            .uri(&uri)
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let acct = body_json(resp).await;
        assert_eq!(acct["name"], "MyAcct");
        assert!(acct["id"].as_str().is_some());

        // GET the account.
        let acct_id = acct["id"].as_str().unwrap();
        let req = axum::http::Request::builder()
            .uri(format!("/treasury-info/api/v1/account/{acct_id}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn payment_create_and_get() {
        let state = AppState::new();
        let app = router(state);

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/treasury-info/api/v1/transaction/create/payment")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "sourceAccountId": Uuid::new_v4().to_string(),
                    "amount": "50000.00",
                    "currency": "PKR"
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let payment = body_json(resp).await;
        assert_eq!(payment["status"], "PENDING");
        assert_eq!(payment["amount"], "50000.00");

        let pid = payment["id"].as_str().unwrap();
        let req = axum::http::Request::builder()
            .uri(format!("/treasury-info/api/v1/transaction/{pid}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn tax_events_create_and_list() {
        let state = AppState::new();
        let app = router(state);

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/treasury-info/api/v1/tax-events")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "entity_id": "ent-1",
                    "event_type": "WITHHOLDING_AT_SOURCE",
                    "amount": "10000",
                    "currency": "PKR",
                    "tax_year": "2025-2026",
                    "details": {}
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let event = body_json(resp).await;
        assert_eq!(event["entityId"], "ent-1");

        // List by entity.
        let req = axum::http::Request::builder()
            .uri("/treasury-info/api/v1/tax-events?entity_id=ent-1")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let list = body_json(resp).await;
        assert_eq!(list.as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn withholding_compute_deterministic() {
        let app = test_app();

        // Filer (NTN starting with '1') + goods → 4.5%
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/treasury-info/api/v1/withholding/compute")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "entity_id": "ent-1",
                    "transaction_amount": "100000.00",
                    "currency": "PKR",
                    "transaction_type": "payment_for_goods",
                    "ntn": "1234567",
                    "jurisdiction_id": "PK"
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let result = body_json(resp).await;
        assert_eq!(result["withholding_rate"], "4.5");
        assert_eq!(result["ntn_status"], "Filer");
        assert_eq!(result["withholding_amount"], "4500.00");
        assert_eq!(result["net_amount"], "95500.00");

        // Non-filer (NTN starting with '0') + goods → 9.0%
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/treasury-info/api/v1/withholding/compute")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "entity_id": "ent-2",
                    "transaction_amount": "100000.00",
                    "currency": "PKR",
                    "transaction_type": "payment_for_goods",
                    "ntn": "0123456",
                    "jurisdiction_id": "PK"
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let result = body_json(resp).await;
        assert_eq!(result["withholding_rate"], "9");
        assert_eq!(result["ntn_status"], "NonFiler");
    }

    #[tokio::test]
    async fn consent_approve_reject_cancel() {
        let state = AppState::new();
        let app = router(state);

        // Create consent.
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/consent-info/api/v1/consents")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "organizationId": "org-1",
                    "operationType": "EQUITY_OFFER"
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let consent = body_json(resp).await;
        let id = consent["id"].as_str().unwrap();

        // Approve.
        let req = axum::http::Request::builder()
            .method("POST")
            .uri(format!("/consent-info/api/v1/consents/approve/{id}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let vote = body_json(resp).await;
        assert_eq!(vote["vote"], "APPROVED");
        assert_eq!(vote["organizationId"], "org-1");

        // Create another for reject test.
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/consent-info/api/v1/consents")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "organizationId": "org-1",
                    "operationType": "ISSUE_NEW_SHARES"
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let consent2 = body_json(resp).await;
        let id2 = consent2["id"].as_str().unwrap();

        // Reject.
        let req = axum::http::Request::builder()
            .method("POST")
            .uri(format!("/consent-info/api/v1/consents/reject/{id2}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let vote = body_json(resp).await;
        assert_eq!(vote["vote"], "REJECTED");

        // Cancel (DELETE) the first consent.
        let req = axum::http::Request::builder()
            .method("DELETE")
            .uri(format!("/consent-info/api/v1/consents/{id}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // List by org.
        let req = axum::http::Request::builder()
            .uri("/consent-info/api/v1/consents/organization/org-1")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let list = body_json(resp).await;
        assert_eq!(list.as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn cap_table_lifecycle() {
        let state = AppState::new();
        let app = router(state);

        // Create cap table.
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/consent-info/api/v1/capTables")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "organizationId": "org-1",
                    "authorizedShares": 1000000
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let ct = body_json(resp).await;
        assert_eq!(ct["organizationId"], "org-1");
        assert_eq!(ct["authorizedShares"], 1000000);
        let ct_id = ct["id"].as_str().unwrap();

        // Get by ID.
        let req = axum::http::Request::builder()
            .uri(format!("/consent-info/api/v1/capTables/{ct_id}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Get by org.
        let req = axum::http::Request::builder()
            .uri("/consent-info/api/v1/capTables/organization/org-1")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Share classes by org.
        let req = axum::http::Request::builder()
            .uri("/consent-info/api/v1/shareClasses/organization/org-1")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let classes = body_json(resp).await;
        assert!(!classes.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn identity_endpoints() {
        let app = test_app();

        // Members (empty for unknown org).
        let req = axum::http::Request::builder()
            .uri("/organization-info/api/v1/membership/org-1/members")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let members = body_json(resp).await;
        assert!(members.as_array().unwrap().is_empty());

        // Board.
        let req = axum::http::Request::builder()
            .uri("/organization-info/api/v1/board/org-1")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Verify CNIC.
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/organization-info/api/v1/identity/cnic/verify")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "cnic": "12345-1234567-1",
                    "full_name": "Ali Khan"
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let cnic_resp = body_json(resp).await;
        assert_eq!(cnic_resp["verified"], true);
        assert_eq!(cnic_resp["cnic"], "1234512345671");

        // Verify NTN.
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/organization-info/api/v1/identity/ntn/verify")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "ntn": "1234567",
                    "entity_name": "Test Corp"
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let ntn_resp = body_json(resp).await;
        assert_eq!(ntn_resp["verified"], true);
        assert_eq!(ntn_resp["tax_status"], "Filer");

        // Identities list.
        let req = axum::http::Request::builder()
            .uri("/consent-info/api/v1/identities?entity_id=ent-1")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn templating_endpoints() {
        let state = AppState::new();
        let app = router(state);

        // Sign template.
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/templating-engine/api/v1/template/sign")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "entityId": "ent-1",
                    "templateTypes": ["CERTIFICATE_OF_INCORPORATION"],
                    "signers": []
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let sub = body_json(resp).await;
        let sub_id = sub["id"].as_str().unwrap();
        assert_eq!(sub["entityId"], "ent-1");

        // Get submission.
        let req = axum::http::Request::builder()
            .uri(format!("/templating-engine/api/v1/submission/{sub_id}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Available templates.
        let req = axum::http::Request::builder()
            .uri("/templating-engine/api/v1/template/available")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let templates = body_json(resp).await;
        assert!(!templates.as_array().unwrap().is_empty());

        // Get template by ID.
        let req = axum::http::Request::builder()
            .uri("/templating-engine/api/v1/template/tpl-cert-incorp-001")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn unknown_path_returns_501() {
        let app = test_app();
        let req = axum::http::Request::builder()
            .uri("/some/unknown/path")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn supported_jurisdictions_returns_list() {
        let app = test_app();
        let req = axum::http::Request::builder()
            .uri("/organization-info/api/v1/organization/supported-jurisdictions")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert!(body.as_array().unwrap().len() >= 2);
    }

    #[tokio::test]
    async fn org_list_returns_all_when_no_ids() {
        let state = AppState::new();
        let app = router(state.clone());

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/organization-info/api/v1/organization/create")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({"name": "Listed Corp", "tags": []})).unwrap(),
            ))
            .unwrap();
        app.clone().oneshot(req).await.unwrap();

        let req = axum::http::Request::builder()
            .uri("/organization-info/api/v1/organization")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let body = body_json(resp).await;
        assert_eq!(body.as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn consent_create_and_get() {
        let state = AppState::new();
        let app = router(state);

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/consent-info/api/v1/consents")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "organizationId": "org-1",
                    "operationType": "EQUITY_OFFER"
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let created = body_json(resp).await;
        let id = created["id"].as_str().unwrap();

        let req = axum::http::Request::builder()
            .uri(format!("/consent-info/api/v1/consents/{id}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn investment_create_and_get() {
        let state = AppState::new();
        let app = router(state);

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/investment-info/api/v1/investment")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({"amount": "10000", "currency": "PKR"})).unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let created = body_json(resp).await;
        let id = created["id"].as_str().unwrap();

        let req = axum::http::Request::builder()
            .uri(format!("/investment-info/api/v1/investment/{id}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn shareholders_returns_empty_for_unknown_org() {
        let app = test_app();
        let req = axum::http::Request::builder()
            .uri("/consent-info/api/v1/shareholders/organization/unknown-org")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert!(body.as_array().unwrap().is_empty());
    }
}
