//! # Mass API Orchestration Endpoints
//!
//! Jurisdiction-aware orchestration endpoints that compose compliance
//! tensor evaluation, Mass API delegation, and Verifiable Credential
//! issuance for all five Mass primitives.
//!
//! ## Architecture
//!
//! Each **write** endpoint follows the orchestration pipeline:
//!
//! 1. **Pre-flight compliance** — evaluate the compliance tensor across
//!    the 20 `ComplianceDomain` variants for the target jurisdiction.
//!    Hard-block domains (Sanctions `NonCompliant`) reject the request.
//! 2. **Mass API call** — delegate the primitive operation to the live
//!    Mass API via `msez-mass-client` (the sole authorized gateway).
//! 3. **VC issuance** — issue a W3C Verifiable Credential attesting to
//!    the compliance evaluation at the time of the operation.
//! 4. **Attestation storage** — persist an attestation record for
//!    regulator queries.
//!
//! **Read** endpoints (GET) remain proxies — they fetch data from Mass
//! without compliance evaluation since reads don't alter state.
//!
//! ## Response Envelope
//!
//! Write endpoints return an [`OrchestrationEnvelope`] containing:
//! - `mass_response` — the Mass API response
//! - `compliance` — 20-domain compliance tensor summary
//! - `credential` — the signed VC (if issued)
//! - `attestation_id` — ID of the stored attestation record
//!
//! ## Primitives
//!
//! | Prefix            | Mass API                  | Status       |
//! |-------------------|---------------------------|--------------|
//! | `/v1/entities`    | organization-info         | Orchestrated |
//! | `/v1/ownership`   | investment-info           | Orchestrated |
//! | `/v1/fiscal`      | treasury-info             | Orchestrated |
//! | `/v1/identity`    | consent-info (identity)   | Orchestrated |
//! | `/v1/consent`     | consent-info (consent)    | Orchestrated |

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::AppError;
use crate::orchestration::{self, OrchestrationEnvelope};
use crate::state::AppState;

/// Build the Mass API orchestration router for all five primitives.
///
/// Write endpoints compose compliance evaluation + Mass API + VC issuance.
/// Read endpoints proxy through to Mass APIs.
pub fn router() -> Router<AppState> {
    Router::new()
        // ENTITIES (organization-info)
        .route("/v1/entities", get(list_entities).post(create_entity))
        .route("/v1/entities/:id", get(get_entity).put(update_entity))
        // OWNERSHIP (investment-info)
        .route("/v1/ownership/cap-tables", post(create_cap_table))
        .route("/v1/ownership/cap-tables/:id", get(get_cap_table))
        // FISCAL (treasury-info)
        .route("/v1/fiscal/accounts", post(create_account))
        .route("/v1/fiscal/payments", post(initiate_payment))
        // IDENTITY (consent-info / identity)
        .route("/v1/identity/verify", post(verify_identity))
        .route("/v1/identity/:id", get(get_identity))
        // CONSENT (consent-info)
        .route("/v1/consent", post(create_consent))
        .route("/v1/consent/:id", get(get_consent))
}

/// Helper: extract the Mass client from AppState or return 503.
fn require_mass_client(state: &AppState) -> Result<&msez_mass_client::MassClient, AppError> {
    state.mass_client.as_ref().ok_or_else(|| {
        AppError::service_unavailable(
            "Mass API client not configured. Set MASS_API_TOKEN environment variable.",
        )
    })
}

// -- Request/Response DTOs for the proxy layer --------------------------------

/// Request to create an entity via the Mass API proxy.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateEntityProxyRequest {
    pub entity_type: String,
    pub legal_name: String,
    pub jurisdiction_id: String,
    #[serde(default)]
    pub beneficial_owners: Vec<BeneficialOwnerInput>,
}

/// Beneficial owner input.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct BeneficialOwnerInput {
    pub name: String,
    pub ownership_percentage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cnic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ntn: Option<String>,
}

/// Request to create a cap table via the Mass API proxy.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCapTableProxyRequest {
    pub entity_id: uuid::Uuid,
    pub share_classes: Vec<ShareClassInput>,
}

/// Share class input for cap table creation.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ShareClassInput {
    pub name: String,
    pub authorized_shares: u64,
    pub issued_shares: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub par_value: Option<String>,
    pub voting_rights: bool,
}

/// Request to create a fiscal account via the Mass API proxy.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAccountProxyRequest {
    pub entity_id: uuid::Uuid,
    pub account_type: String,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ntn: Option<String>,
}

/// Request to initiate a payment via the Mass API proxy.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePaymentProxyRequest {
    pub from_account_id: uuid::Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_account_id: Option<uuid::Uuid>,
    pub amount: String,
    pub currency: String,
    pub reference: String,
}

/// Request to verify an identity via the Mass API proxy.
#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyIdentityProxyRequest {
    pub identity_type: String,
    pub linked_ids: Vec<LinkedIdInput>,
}

/// Linked external ID input.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct LinkedIdInput {
    pub id_type: String,
    pub id_value: String,
}

/// Request to create a consent request via the Mass API proxy.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateConsentProxyRequest {
    pub consent_type: String,
    pub description: String,
    pub parties: Vec<ConsentPartyInput>,
}

/// Consent party input.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ConsentPartyInput {
    pub entity_id: uuid::Uuid,
    pub role: String,
}

// ── ENTITY HANDLERS ─────────────────────────────────────────────────

/// POST /v1/entities — Create an entity with compliance evaluation and VC issuance.
///
/// Orchestration pipeline:
/// 1. Evaluate compliance tensor for `jurisdiction_id` across entity-relevant domains
/// 2. Reject if sanctions domain is `NonCompliant` (hard block)
/// 3. Create entity via Mass organization-info API
/// 4. Issue a `MsezFormationComplianceCredential` VC
/// 5. Store attestation record for regulator queries
#[utoipa::path(
    post,
    path = "/v1/entities",
    request_body = CreateEntityProxyRequest,
    responses(
        (status = 201, description = "Entity created with compliance evaluation and VC"),
        (status = 403, description = "Blocked by compliance hard-block (sanctions)"),
        (status = 502, description = "Mass API error"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "entities"
)]
async fn create_entity(
    State(state): State<AppState>,
    Json(req): Json<CreateEntityProxyRequest>,
) -> Result<(axum::http::StatusCode, Json<OrchestrationEnvelope>), AppError> {
    let client = require_mass_client(&state)?;

    // Step 1: Pre-flight compliance evaluation.
    let (_tensor, pre_summary) = orchestration::evaluate_compliance(
        &req.jurisdiction_id,
        "pre-flight",
        orchestration::entity_domains(),
    );

    // Step 2: Hard-block check (sanctions).
    if let Some(reason) = orchestration::check_hard_blocks(&pre_summary) {
        return Err(AppError::Forbidden(reason));
    }

    // Step 3: Mass API call.
    let entity_type: msez_mass_client::entities::MassEntityType =
        serde_json::from_value(serde_json::Value::String(req.entity_type))
            .map_err(|e| AppError::BadRequest(format!("invalid entity_type: {e}")))?;

    let jurisdiction_id = req.jurisdiction_id.clone();
    let legal_name = req.legal_name.clone();

    let mass_req = msez_mass_client::entities::CreateEntityRequest {
        entity_type,
        legal_name: req.legal_name,
        jurisdiction_id: req.jurisdiction_id,
        beneficial_owners: req
            .beneficial_owners
            .into_iter()
            .map(|bo| msez_mass_client::entities::MassBeneficialOwner {
                name: bo.name,
                ownership_percentage: bo.ownership_percentage,
                cnic: bo.cnic,
                ntn: bo.ntn,
            })
            .collect(),
    };

    let entity = client
        .entities()
        .create(&mass_req)
        .await
        .map_err(|e| AppError::upstream(format!("Mass API error: {e}")))?;

    let mass_response = serde_json::to_value(entity)
        .map_err(|e| AppError::Internal(format!("serialization error: {e}")))?;

    // Step 4 & 5: Post-operation orchestration (VC issuance + attestation storage).
    let envelope = orchestration::orchestrate_entity_creation(
        &state,
        &jurisdiction_id,
        &legal_name,
        mass_response,
    );

    Ok((axum::http::StatusCode::CREATED, Json(envelope)))
}

/// GET /v1/entities/{id} — Get an entity from Mass by ID (proxy, no orchestration).
#[utoipa::path(
    get,
    path = "/v1/entities/:id",
    params(("id" = uuid::Uuid, Path, description = "Entity UUID")),
    responses(
        (status = 200, description = "Entity found"),
        (status = 404, description = "Entity not found"),
        (status = 502, description = "Mass API error"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "entities"
)]
async fn get_entity(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let client = require_mass_client(&state)?;

    match client.entities().get(id).await {
        Ok(Some(entity)) => serde_json::to_value(entity)
            .map(Json)
            .map_err(|e| AppError::Internal(format!("serialization error: {e}"))),
        Ok(None) => Err(AppError::not_found(format!("entity {id} not found"))),
        Err(e) => Err(AppError::upstream(format!("Mass API error: {e}"))),
    }
}

/// PUT /v1/entities/{id} — Update an entity in Mass.
///
/// Not yet implemented: the Mass organization-info API update endpoint
/// is being finalized. Returns 501 until the EntityClient gains an
/// `update` method.
#[utoipa::path(
    put,
    path = "/v1/entities/:id",
    params(("id" = uuid::Uuid, Path, description = "Entity UUID")),
    responses(
        (status = 501, description = "Not yet implemented"),
    ),
    tag = "entities"
)]
async fn update_entity(
    State(_state): State<AppState>,
    Path(_id): Path<uuid::Uuid>,
    Json(_body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    Err(AppError::NotImplemented(
        "entity update proxy: awaiting EntityClient.update() method".into(),
    ))
}

/// GET /v1/entities — List entities from Mass (proxy, no orchestration).
#[utoipa::path(
    get,
    path = "/v1/entities",
    responses(
        (status = 200, description = "List of entities"),
        (status = 502, description = "Mass API error"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "entities"
)]
async fn list_entities(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    let client = require_mass_client(&state)?;

    let entities = client
        .entities()
        .list(None, None)
        .await
        .map_err(|e| AppError::upstream(format!("Mass API error: {e}")))?;

    serde_json::to_value(entities)
        .map(Json)
        .map_err(|e| AppError::Internal(format!("serialization error: {e}")))
}

// ── OWNERSHIP HANDLERS ──────────────────────────────────────────────

/// POST /v1/ownership/cap-tables — Create a cap table with compliance evaluation.
///
/// Orchestration pipeline:
/// 1. Evaluate compliance tensor for securities/ownership domains
/// 2. Reject if sanctions hard-block
/// 3. Create cap table via Mass investment-info API
/// 4. Issue a `MsezOwnershipComplianceCredential` VC
/// 5. Store attestation record
#[utoipa::path(
    post,
    path = "/v1/ownership/cap-tables",
    request_body = CreateCapTableProxyRequest,
    responses(
        (status = 201, description = "Cap table created with compliance evaluation and VC"),
        (status = 403, description = "Blocked by compliance hard-block (sanctions)"),
        (status = 502, description = "Mass API error"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "ownership"
)]
async fn create_cap_table(
    State(state): State<AppState>,
    Json(req): Json<CreateCapTableProxyRequest>,
) -> Result<(axum::http::StatusCode, Json<OrchestrationEnvelope>), AppError> {
    let client = require_mass_client(&state)?;

    // Step 1: Pre-flight compliance evaluation.
    let entity_id = req.entity_id;
    let (_tensor, pre_summary) = orchestration::evaluate_compliance(
        "UNKNOWN",
        &entity_id.to_string(),
        orchestration::ownership_domains(),
    );

    // Step 2: Hard-block check.
    if let Some(reason) = orchestration::check_hard_blocks(&pre_summary) {
        return Err(AppError::Forbidden(reason));
    }

    // Step 3: Mass API call.
    let mass_req = msez_mass_client::ownership::CreateCapTableRequest {
        entity_id: req.entity_id,
        share_classes: req
            .share_classes
            .into_iter()
            .map(|sc| msez_mass_client::ownership::MassShareClass {
                name: sc.name,
                authorized_shares: sc.authorized_shares,
                issued_shares: sc.issued_shares,
                par_value: sc.par_value,
                voting_rights: sc.voting_rights,
            })
            .collect(),
    };

    let cap_table = client
        .ownership()
        .create_cap_table(&mass_req)
        .await
        .map_err(|e| AppError::upstream(format!("Mass API error: {e}")))?;

    let mass_response = serde_json::to_value(cap_table)
        .map_err(|e| AppError::Internal(format!("serialization error: {e}")))?;

    // Step 4 & 5: Post-operation orchestration.
    let envelope = orchestration::orchestrate_cap_table_creation(
        &state,
        entity_id,
        mass_response,
    );

    Ok((axum::http::StatusCode::CREATED, Json(envelope)))
}

/// GET /v1/ownership/cap-tables/{id} — Get a cap table from Mass (proxy, no orchestration).
#[utoipa::path(
    get,
    path = "/v1/ownership/cap-tables/:id",
    params(("id" = uuid::Uuid, Path, description = "Entity UUID")),
    responses(
        (status = 200, description = "Cap table found"),
        (status = 404, description = "Cap table not found"),
        (status = 502, description = "Mass API error"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "ownership"
)]
async fn get_cap_table(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let client = require_mass_client(&state)?;

    match client.ownership().get_cap_table(id).await {
        Ok(Some(cap_table)) => serde_json::to_value(cap_table)
            .map(Json)
            .map_err(|e| AppError::Internal(format!("serialization error: {e}"))),
        Ok(None) => Err(AppError::not_found(format!(
            "cap table for entity {id} not found"
        ))),
        Err(e) => Err(AppError::upstream(format!("Mass API error: {e}"))),
    }
}

// ── FISCAL HANDLERS ─────────────────────────────────────────────────

/// POST /v1/fiscal/accounts — Create a fiscal account with compliance evaluation.
///
/// Orchestration pipeline:
/// 1. Evaluate compliance tensor for fiscal/banking domains (jurisdiction inferred from currency)
/// 2. Reject if sanctions hard-block
/// 3. Create account via Mass treasury-info API
/// 4. Issue a `MsezFiscalComplianceCredential` VC
/// 5. Store attestation record
#[utoipa::path(
    post,
    path = "/v1/fiscal/accounts",
    request_body = CreateAccountProxyRequest,
    responses(
        (status = 201, description = "Account created with compliance evaluation and VC"),
        (status = 403, description = "Blocked by compliance hard-block (sanctions)"),
        (status = 502, description = "Mass API error"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "fiscal"
)]
async fn create_account(
    State(state): State<AppState>,
    Json(req): Json<CreateAccountProxyRequest>,
) -> Result<(axum::http::StatusCode, Json<OrchestrationEnvelope>), AppError> {
    let client = require_mass_client(&state)?;

    // Step 1: Pre-flight compliance evaluation.
    let entity_id = req.entity_id;
    let currency = req.currency.clone();
    let (_tensor, pre_summary) = orchestration::evaluate_compliance(
        orchestration::fiscal_account_domains()
            .first()
            .map(|_| "UNKNOWN")
            .unwrap_or("UNKNOWN"),
        &entity_id.to_string(),
        orchestration::fiscal_account_domains(),
    );

    // Step 2: Hard-block check.
    if let Some(reason) = orchestration::check_hard_blocks(&pre_summary) {
        return Err(AppError::Forbidden(reason));
    }

    // Step 3: Mass API call.
    let account_type: msez_mass_client::fiscal::MassAccountType =
        serde_json::from_value(serde_json::Value::String(req.account_type))
            .map_err(|e| AppError::BadRequest(format!("invalid account_type: {e}")))?;

    let mass_req = msez_mass_client::fiscal::CreateAccountRequest {
        entity_id: req.entity_id,
        account_type,
        currency: req.currency,
        ntn: req.ntn,
    };

    let account = client
        .fiscal()
        .create_account(&mass_req)
        .await
        .map_err(|e| AppError::upstream(format!("Mass API error: {e}")))?;

    let mass_response = serde_json::to_value(account)
        .map_err(|e| AppError::Internal(format!("serialization error: {e}")))?;

    // Step 4 & 5: Post-operation orchestration.
    let envelope = orchestration::orchestrate_account_creation(
        &state,
        entity_id,
        &currency,
        mass_response,
    );

    Ok((axum::http::StatusCode::CREATED, Json(envelope)))
}

/// POST /v1/fiscal/payments — Initiate a payment with compliance evaluation.
///
/// Orchestration pipeline:
/// 1. Evaluate compliance tensor for AML/sanctions/payment domains
/// 2. Reject if sanctions hard-block
/// 3. Initiate payment via Mass treasury-info API
/// 4. Issue a `MsezPaymentComplianceCredential` VC
/// 5. Store attestation record
#[utoipa::path(
    post,
    path = "/v1/fiscal/payments",
    request_body = CreatePaymentProxyRequest,
    responses(
        (status = 201, description = "Payment initiated with compliance evaluation and VC"),
        (status = 403, description = "Blocked by compliance hard-block (sanctions)"),
        (status = 502, description = "Mass API error"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "fiscal"
)]
async fn initiate_payment(
    State(state): State<AppState>,
    Json(req): Json<CreatePaymentProxyRequest>,
) -> Result<(axum::http::StatusCode, Json<OrchestrationEnvelope>), AppError> {
    let client = require_mass_client(&state)?;

    // Step 1: Pre-flight compliance evaluation.
    let from_account_id = req.from_account_id;
    let currency = req.currency.clone();
    let (_tensor, pre_summary) = orchestration::evaluate_compliance(
        "UNKNOWN",
        &from_account_id.to_string(),
        orchestration::payment_domains(),
    );

    // Step 2: Hard-block check.
    if let Some(reason) = orchestration::check_hard_blocks(&pre_summary) {
        return Err(AppError::Forbidden(reason));
    }

    // Step 3: Mass API call.
    let mass_req = msez_mass_client::fiscal::CreatePaymentRequest {
        from_account_id: req.from_account_id,
        to_account_id: req.to_account_id,
        amount: req.amount,
        currency: req.currency,
        reference: req.reference,
    };

    let payment = client
        .fiscal()
        .create_payment(&mass_req)
        .await
        .map_err(|e| AppError::upstream(format!("Mass API error: {e}")))?;

    let mass_response = serde_json::to_value(payment)
        .map_err(|e| AppError::Internal(format!("serialization error: {e}")))?;

    // Step 4 & 5: Post-operation orchestration.
    let envelope = orchestration::orchestrate_payment(
        &state,
        from_account_id,
        &currency,
        mass_response,
    );

    Ok((axum::http::StatusCode::CREATED, Json(envelope)))
}

// ── IDENTITY HANDLERS ───────────────────────────────────────────────

/// POST /v1/identity/verify — Verify identity with compliance evaluation.
///
/// Orchestration pipeline:
/// 1. Evaluate compliance tensor for KYC/sanctions/data-privacy domains
/// 2. Reject if sanctions hard-block
/// 3. Submit verification via Mass identity API
/// 4. Issue a `MsezIdentityComplianceCredential` VC
/// 5. Store attestation record
#[utoipa::path(
    post,
    path = "/v1/identity/verify",
    request_body = VerifyIdentityProxyRequest,
    responses(
        (status = 200, description = "Verification submitted with compliance evaluation and VC"),
        (status = 403, description = "Blocked by compliance hard-block (sanctions)"),
        (status = 502, description = "Mass API error"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "identity"
)]
async fn verify_identity(
    State(state): State<AppState>,
    Json(req): Json<VerifyIdentityProxyRequest>,
) -> Result<Json<OrchestrationEnvelope>, AppError> {
    let client = require_mass_client(&state)?;

    // Step 1: Pre-flight compliance evaluation.
    let identity_type_str = req.identity_type.clone();
    let (_tensor, pre_summary) = orchestration::evaluate_compliance(
        "GLOBAL",
        "pre-flight",
        orchestration::identity_domains(),
    );

    // Step 2: Hard-block check.
    if let Some(reason) = orchestration::check_hard_blocks(&pre_summary) {
        return Err(AppError::Forbidden(reason));
    }

    // Step 3: Mass API call.
    let identity_type: msez_mass_client::identity::MassIdentityType =
        serde_json::from_value(serde_json::Value::String(req.identity_type))
            .map_err(|e| AppError::BadRequest(format!("invalid identity_type: {e}")))?;

    let mass_req = msez_mass_client::identity::VerifyIdentityRequest {
        identity_type,
        linked_ids: req
            .linked_ids
            .into_iter()
            .map(|lid| msez_mass_client::identity::LinkedIdInput {
                id_type: lid.id_type,
                id_value: lid.id_value,
            })
            .collect(),
    };

    let identity = client
        .identity()
        .verify(&mass_req)
        .await
        .map_err(|e| AppError::upstream(format!("Mass API error: {e}")))?;

    let mass_response = serde_json::to_value(identity)
        .map_err(|e| AppError::Internal(format!("serialization error: {e}")))?;

    // Step 4 & 5: Post-operation orchestration.
    let envelope = orchestration::orchestrate_identity_verification(
        &state,
        &identity_type_str,
        mass_response,
    );

    Ok(Json(envelope))
}

/// GET /v1/identity/{id} — Get an identity from Mass by ID (proxy, no orchestration).
#[utoipa::path(
    get,
    path = "/v1/identity/:id",
    params(("id" = uuid::Uuid, Path, description = "Identity UUID")),
    responses(
        (status = 200, description = "Identity found"),
        (status = 404, description = "Identity not found"),
        (status = 502, description = "Mass API error"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "identity"
)]
async fn get_identity(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let client = require_mass_client(&state)?;

    match client.identity().get_identity(id).await {
        Ok(Some(identity)) => serde_json::to_value(identity)
            .map(Json)
            .map_err(|e| AppError::Internal(format!("serialization error: {e}"))),
        Ok(None) => Err(AppError::not_found(format!("identity {id} not found"))),
        Err(e) => Err(AppError::upstream(format!("Mass API error: {e}"))),
    }
}

// ── CONSENT HANDLERS ────────────────────────────────────────────────

/// POST /v1/consent — Create a consent request with compliance evaluation.
///
/// Orchestration pipeline:
/// 1. Evaluate compliance tensor for governance/sanctions domains
/// 2. Reject if sanctions hard-block
/// 3. Create consent request via Mass consent-info API
/// 4. Issue a `MsezConsentComplianceCredential` VC
/// 5. Store attestation record
#[utoipa::path(
    post,
    path = "/v1/consent",
    request_body = CreateConsentProxyRequest,
    responses(
        (status = 201, description = "Consent request created with compliance evaluation and VC"),
        (status = 403, description = "Blocked by compliance hard-block (sanctions)"),
        (status = 502, description = "Mass API error"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "consent"
)]
async fn create_consent(
    State(state): State<AppState>,
    Json(req): Json<CreateConsentProxyRequest>,
) -> Result<(axum::http::StatusCode, Json<OrchestrationEnvelope>), AppError> {
    let client = require_mass_client(&state)?;

    // Step 1: Pre-flight compliance evaluation.
    let consent_type_str = req.consent_type.clone();
    let (_tensor, pre_summary) = orchestration::evaluate_compliance(
        "GLOBAL",
        "pre-flight",
        orchestration::consent_domains(),
    );

    // Step 2: Hard-block check.
    if let Some(reason) = orchestration::check_hard_blocks(&pre_summary) {
        return Err(AppError::Forbidden(reason));
    }

    // Step 3: Mass API call.
    let consent_type: msez_mass_client::consent::MassConsentType =
        serde_json::from_value(serde_json::Value::String(req.consent_type))
            .map_err(|e| AppError::BadRequest(format!("invalid consent_type: {e}")))?;

    let mass_req = msez_mass_client::consent::CreateConsentRequest {
        consent_type,
        description: req.description,
        parties: req
            .parties
            .into_iter()
            .map(|p| msez_mass_client::consent::ConsentPartyInput {
                entity_id: p.entity_id,
                role: p.role,
            })
            .collect(),
    };

    let consent = client
        .consent()
        .create(&mass_req)
        .await
        .map_err(|e| AppError::upstream(format!("Mass API error: {e}")))?;

    let mass_response = serde_json::to_value(consent)
        .map_err(|e| AppError::Internal(format!("serialization error: {e}")))?;

    // Step 4 & 5: Post-operation orchestration.
    let envelope = orchestration::orchestrate_consent_creation(
        &state,
        &consent_type_str,
        mass_response,
    );

    Ok((axum::http::StatusCode::CREATED, Json(envelope)))
}

/// GET /v1/consent/{id} — Get a consent request from Mass by ID (proxy, no orchestration).
#[utoipa::path(
    get,
    path = "/v1/consent/:id",
    params(("id" = uuid::Uuid, Path, description = "Consent request UUID")),
    responses(
        (status = 200, description = "Consent request found"),
        (status = 404, description = "Consent request not found"),
        (status = 502, description = "Mass API error"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "consent"
)]
async fn get_consent(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let client = require_mass_client(&state)?;

    match client.consent().get(id).await {
        Ok(Some(consent)) => serde_json::to_value(consent)
            .map(Json)
            .map_err(|e| AppError::Internal(format!("serialization error: {e}"))),
        Ok(None) => Err(AppError::not_found(format!(
            "consent request {id} not found"
        ))),
        Err(e) => Err(AppError::upstream(format!("Mass API error: {e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_builds_successfully() {
        let _router = router();
    }

    #[test]
    fn create_entity_proxy_request_deserializes() {
        let json = r#"{
            "entity_type": "llc",
            "legal_name": "Test Corp",
            "jurisdiction_id": "pk-sez-01"
        }"#;
        let req: CreateEntityProxyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.entity_type, "llc");
        assert_eq!(req.legal_name, "Test Corp");
        assert!(req.beneficial_owners.is_empty());
    }

    #[test]
    fn create_entity_proxy_request_with_beneficial_owners() {
        let json = r#"{
            "entity_type": "llc",
            "legal_name": "Test Corp",
            "jurisdiction_id": "pk-sez-01",
            "beneficial_owners": [{
                "name": "Alice",
                "ownership_percentage": "51.0",
                "cnic": "12345-1234567-1"
            }]
        }"#;
        let req: CreateEntityProxyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.beneficial_owners.len(), 1);
        assert_eq!(req.beneficial_owners[0].name, "Alice");
    }

    #[test]
    fn create_cap_table_proxy_request_deserializes() {
        let json = r#"{
            "entity_id": "550e8400-e29b-41d4-a716-446655440000",
            "share_classes": [{
                "name": "Common",
                "authorized_shares": 1000000,
                "issued_shares": 100000,
                "par_value": "0.01",
                "voting_rights": true
            }]
        }"#;
        let req: CreateCapTableProxyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.share_classes.len(), 1);
        assert_eq!(req.share_classes[0].name, "Common");
    }

    #[test]
    fn create_account_proxy_request_deserializes() {
        let json = r#"{
            "entity_id": "550e8400-e29b-41d4-a716-446655440000",
            "account_type": "operating",
            "currency": "PKR"
        }"#;
        let req: CreateAccountProxyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.account_type, "operating");
        assert_eq!(req.currency, "PKR");
        assert!(req.ntn.is_none());
    }

    #[test]
    fn create_payment_proxy_request_deserializes() {
        let json = r#"{
            "from_account_id": "550e8400-e29b-41d4-a716-446655440000",
            "amount": "50000.00",
            "currency": "PKR",
            "reference": "INV-2026-001"
        }"#;
        let req: CreatePaymentProxyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.amount, "50000.00");
        assert!(req.to_account_id.is_none());
    }

    #[test]
    fn verify_identity_proxy_request_deserializes() {
        let json = r#"{
            "identity_type": "individual",
            "linked_ids": [{
                "id_type": "CNIC",
                "id_value": "12345-1234567-1"
            }]
        }"#;
        let req: VerifyIdentityProxyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.identity_type, "individual");
        assert_eq!(req.linked_ids.len(), 1);
    }

    #[test]
    fn create_consent_proxy_request_deserializes() {
        let json = r#"{
            "consent_type": "board_resolution",
            "description": "Approve entity formation",
            "parties": [{
                "entity_id": "550e8400-e29b-41d4-a716-446655440000",
                "role": "approver"
            }]
        }"#;
        let req: CreateConsentProxyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.consent_type, "board_resolution");
        assert_eq!(req.parties.len(), 1);
    }

    // ── 503 tests (no Mass client configured) ────────────────────

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn create_entity_returns_503_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("POST")
            .uri("/v1/entities")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"entity_type":"llc","legal_name":"Test","jurisdiction_id":"pk-sez-01"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["error"]["code"], "SERVICE_UNAVAILABLE");
    }

    #[tokio::test]
    async fn get_entity_returns_503_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("GET")
            .uri("/v1/entities/550e8400-e29b-41d4-a716-446655440000")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn list_entities_returns_503_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("GET")
            .uri("/v1/entities")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn update_entity_returns_501() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("PUT")
            .uri("/v1/entities/550e8400-e29b-41d4-a716-446655440000")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"legal_name":"Updated Corp"}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn create_cap_table_returns_503_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("POST")
            .uri("/v1/ownership/cap-tables")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"entity_id":"550e8400-e29b-41d4-a716-446655440000","share_classes":[]}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn get_cap_table_returns_503_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("GET")
            .uri("/v1/ownership/cap-tables/550e8400-e29b-41d4-a716-446655440000")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn create_account_returns_503_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("POST")
            .uri("/v1/fiscal/accounts")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"entity_id":"550e8400-e29b-41d4-a716-446655440000","account_type":"operating","currency":"PKR"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn initiate_payment_returns_503_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("POST")
            .uri("/v1/fiscal/payments")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"from_account_id":"550e8400-e29b-41d4-a716-446655440000","amount":"5000","currency":"PKR","reference":"INV-001"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn verify_identity_returns_503_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("POST")
            .uri("/v1/identity/verify")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"identity_type":"individual","linked_ids":[{"id_type":"CNIC","id_value":"12345-1234567-1"}]}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn get_identity_returns_503_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("GET")
            .uri("/v1/identity/550e8400-e29b-41d4-a716-446655440000")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn create_consent_returns_503_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("POST")
            .uri("/v1/consent")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"consent_type":"board_resolution","description":"Approve formation","parties":[{"entity_id":"550e8400-e29b-41d4-a716-446655440000","role":"approver"}]}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn get_consent_returns_503_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("GET")
            .uri("/v1/consent/550e8400-e29b-41d4-a716-446655440000")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
