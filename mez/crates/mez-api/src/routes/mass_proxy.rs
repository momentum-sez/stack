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
//!    Mass API via `mez-mass-client` (the sole authorized gateway).
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

use chrono::{Datelike, Utc};
use uuid::Uuid;

use crate::error::AppError;
use crate::orchestration::{self, OrchestrationEnvelope};
use crate::state::{AppState, TaxEventRecord};

use super::sovereign_ops;

// ── Input Validation ─────────────────────────────────────────────────

/// Trait for request validation at the HTTP boundary.
///
/// Validates field-level constraints before the request reaches the
/// orchestration pipeline. Rejects empty strings, overlong fields,
/// and semantically invalid combinations early.
trait Validate {
    fn validate(&self) -> Result<(), String>;
}

impl Validate for CreateEntityProxyRequest {
    fn validate(&self) -> Result<(), String> {
        if self.entity_type.trim().is_empty() {
            return Err("entity_type must not be empty".into());
        }
        if self.entity_type.len() > 100 {
            return Err("entity_type must not exceed 100 characters".into());
        }
        if self.legal_name.trim().is_empty() {
            return Err("legal_name must not be empty".into());
        }
        if self.legal_name.len() > 1000 {
            return Err("legal_name must not exceed 1000 characters".into());
        }
        if self.jurisdiction_id.trim().is_empty() {
            return Err("jurisdiction_id must not be empty".into());
        }
        if self.jurisdiction_id.len() > 100 {
            return Err("jurisdiction_id must not exceed 100 characters".into());
        }
        if self.beneficial_owners.len() > 100 {
            return Err("beneficial_owners must not exceed 100 entries".into());
        }
        for (i, bo) in self.beneficial_owners.iter().enumerate() {
            if bo.name.trim().is_empty() {
                return Err(format!("beneficial_owners[{i}].name must not be empty"));
            }
            if bo.name.len() > 500 {
                return Err(format!(
                    "beneficial_owners[{i}].name must not exceed 500 characters"
                ));
            }
            if bo.ownership_percentage.trim().is_empty() {
                return Err(format!(
                    "beneficial_owners[{i}].ownership_percentage must not be empty"
                ));
            }
        }
        Ok(())
    }
}

impl Validate for CreateCapTableProxyRequest {
    fn validate(&self) -> Result<(), String> {
        if self.share_classes.is_empty() {
            return Err("share_classes must not be empty".into());
        }
        if self.share_classes.len() > 50 {
            return Err("share_classes must not exceed 50 entries".into());
        }
        for (i, sc) in self.share_classes.iter().enumerate() {
            if sc.name.trim().is_empty() {
                return Err(format!("share_classes[{i}].name must not be empty"));
            }
            if sc.name.len() > 200 {
                return Err(format!(
                    "share_classes[{i}].name must not exceed 200 characters"
                ));
            }
            if sc.issued_shares > sc.authorized_shares {
                return Err(format!(
                    "share_classes[{i}].issued_shares ({}) exceeds authorized_shares ({})",
                    sc.issued_shares, sc.authorized_shares
                ));
            }
        }
        Ok(())
    }
}

impl Validate for CreateAccountProxyRequest {
    fn validate(&self) -> Result<(), String> {
        if self.account_type.trim().is_empty() {
            return Err("account_type must not be empty".into());
        }
        if self.account_type.len() > 100 {
            return Err("account_type must not exceed 100 characters".into());
        }
        if self.currency.trim().is_empty() {
            return Err("currency must not be empty".into());
        }
        if self.currency.len() > 10 {
            return Err("currency must not exceed 10 characters".into());
        }
        Ok(())
    }
}

impl Validate for CreatePaymentProxyRequest {
    fn validate(&self) -> Result<(), String> {
        if self.amount.trim().is_empty() {
            return Err("amount must not be empty".into());
        }
        if self.amount.len() > 50 {
            return Err("amount must not exceed 50 characters".into());
        }
        if self.currency.trim().is_empty() {
            return Err("currency must not be empty".into());
        }
        if self.currency.len() > 10 {
            return Err("currency must not exceed 10 characters".into());
        }
        if self.reference.trim().is_empty() {
            return Err("reference must not be empty".into());
        }
        if self.reference.len() > 500 {
            return Err("reference must not exceed 500 characters".into());
        }
        Ok(())
    }
}

impl Validate for VerifyIdentityProxyRequest {
    fn validate(&self) -> Result<(), String> {
        if self.identity_type.trim().is_empty() {
            return Err("identity_type must not be empty".into());
        }
        if self.identity_type.len() > 100 {
            return Err("identity_type must not exceed 100 characters".into());
        }
        if self.linked_ids.len() > 50 {
            return Err("linked_ids must not exceed 50 entries".into());
        }
        for (i, lid) in self.linked_ids.iter().enumerate() {
            if lid.id_type.trim().is_empty() {
                return Err(format!("linked_ids[{i}].id_type must not be empty"));
            }
            if lid.id_value.trim().is_empty() {
                return Err(format!("linked_ids[{i}].id_value must not be empty"));
            }
        }
        Ok(())
    }
}

impl Validate for UpdateEntityProxyRequest {
    fn validate(&self) -> Result<(), String> {
        // At least one field must be provided.
        let has_content = self.legal_name.is_some()
            || self.entity_type.is_some()
            || self.jurisdiction_id.is_some()
            || self.beneficial_owners.is_some()
            || self.address.is_some()
            || self.tags.is_some();
        if !has_content {
            return Err("at least one field must be provided for update".into());
        }
        if let Some(ref name) = self.legal_name {
            if name.trim().is_empty() {
                return Err("legal_name must not be empty when provided".into());
            }
            if name.len() > 1000 {
                return Err("legal_name must not exceed 1000 characters".into());
            }
        }
        if let Some(ref et) = self.entity_type {
            if et.trim().is_empty() {
                return Err("entity_type must not be empty when provided".into());
            }
            if et.len() > 100 {
                return Err("entity_type must not exceed 100 characters".into());
            }
        }
        if let Some(ref jid) = self.jurisdiction_id {
            if jid.trim().is_empty() {
                return Err("jurisdiction_id must not be empty when provided".into());
            }
            if jid.len() > 100 {
                return Err("jurisdiction_id must not exceed 100 characters".into());
            }
        }
        if let Some(ref owners) = self.beneficial_owners {
            if owners.len() > 100 {
                return Err("beneficial_owners must not exceed 100 entries".into());
            }
            for (i, bo) in owners.iter().enumerate() {
                if bo.name.trim().is_empty() {
                    return Err(format!("beneficial_owners[{i}].name must not be empty"));
                }
                if bo.name.len() > 500 {
                    return Err(format!(
                        "beneficial_owners[{i}].name must not exceed 500 characters"
                    ));
                }
            }
        }
        if let Some(ref addr) = self.address {
            if addr.len() > 2000 {
                return Err("address must not exceed 2000 characters".into());
            }
        }
        if let Some(ref tags) = self.tags {
            if tags.len() > 50 {
                return Err("tags must not exceed 50 entries".into());
            }
        }
        Ok(())
    }
}

impl Validate for CreateConsentProxyRequest {
    fn validate(&self) -> Result<(), String> {
        if self.consent_type.trim().is_empty() {
            return Err("consent_type must not be empty".into());
        }
        if self.consent_type.len() > 200 {
            return Err("consent_type must not exceed 200 characters".into());
        }
        if self.description.trim().is_empty() {
            return Err("description must not be empty".into());
        }
        if self.description.len() > 5000 {
            return Err("description must not exceed 5000 characters".into());
        }
        if self.parties.is_empty() {
            return Err("parties must not be empty".into());
        }
        if self.parties.len() > 100 {
            return Err("parties must not exceed 100 entries".into());
        }
        for (i, p) in self.parties.iter().enumerate() {
            if p.role.trim().is_empty() {
                return Err(format!("parties[{i}].role must not be empty"));
            }
        }
        Ok(())
    }
}

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
fn require_mass_client(state: &AppState) -> Result<&mez_mass_client::MassClient, AppError> {
    state.mass_client.as_ref().ok_or_else(|| {
        AppError::service_unavailable(
            "Mass API client not configured. Set MASS_API_TOKEN environment variable.",
        )
    })
}

// -- Request/Response DTOs for the proxy layer --------------------------------

/// Request to create an entity via the Mass API proxy.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateEntityProxyRequest {
    pub entity_type: String,
    pub legal_name: String,
    pub jurisdiction_id: String,
    #[serde(default)]
    pub beneficial_owners: Vec<BeneficialOwnerInput>,
}

/// Request to update an entity via the Mass API proxy.
///
/// Accepts the same fields as creation, all optional. At least one field
/// must be provided (an empty update is semantically meaningless).
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateEntityProxyRequest {
    /// New legal name for the entity.
    #[serde(default)]
    pub legal_name: Option<String>,
    /// New entity type.
    #[serde(default)]
    pub entity_type: Option<String>,
    /// New jurisdiction ID.
    #[serde(default)]
    pub jurisdiction_id: Option<String>,
    /// Updated beneficial owners (replaces the full list).
    #[serde(default)]
    pub beneficial_owners: Option<Vec<BeneficialOwnerInput>>,
    /// New address.
    #[serde(default)]
    pub address: Option<String>,
    /// New tags.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

/// Beneficial owner input.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct CreateCapTableProxyRequest {
    pub entity_id: uuid::Uuid,
    pub share_classes: Vec<ShareClassInput>,
}

/// Share class input for cap table creation.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct CreateAccountProxyRequest {
    pub entity_id: uuid::Uuid,
    pub account_type: String,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ntn: Option<String>,
}

/// Request to initiate a payment via the Mass API proxy.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct VerifyIdentityProxyRequest {
    pub identity_type: String,
    pub linked_ids: Vec<LinkedIdInput>,
}

/// Linked external ID input.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LinkedIdInput {
    pub id_type: String,
    pub id_value: String,
}

/// Request to create a consent request via the Mass API proxy.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateConsentProxyRequest {
    pub consent_type: String,
    pub description: String,
    pub parties: Vec<ConsentPartyInput>,
}

/// Consent party input.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
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
/// 4. Issue a `MezFormationComplianceCredential` VC
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
    req.validate().map_err(AppError::Validation)?;

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

    // Step 3: Mass operation — sovereign or proxy.
    let jurisdiction_id = req.jurisdiction_id.clone();
    let legal_name = req.legal_name.clone();

    let mass_response = if state.sovereign_mass {
        sovereign_ops::create_entity(
            &state,
            &req.legal_name,
            Some(&req.jurisdiction_id),
            Some(&req.entity_type),
            &[],
        )
        .await?
    } else {
        let client = require_mass_client(&state)?;
        let mass_req = mez_mass_client::entities::CreateEntityRequest {
            name: req.legal_name,
            entity_type: Some(req.entity_type),
            jurisdiction: Some(req.jurisdiction_id),
            address: None,
            tags: vec![],
        };
        let entity = client
            .entities()
            .create(&mass_req)
            .await
            .map_err(|e| AppError::upstream(format!("Mass API error: {e}")))?;
        serde_json::to_value(entity)
            .map_err(|e| AppError::Internal(format!("serialization error: {e}")))?
    };

    // Step 4 & 5: Post-operation orchestration (VC issuance + attestation storage).
    let envelope = orchestration::orchestrate_entity_creation(
        &state,
        &jurisdiction_id,
        &legal_name,
        mass_response,
    );

    // Step 6: Audit trail.
    append_audit(
        &state,
        "entity.created",
        "entity",
        &envelope,
        "create",
        serde_json::json!({
            "jurisdiction": &jurisdiction_id,
            "compliance_status": &envelope.compliance.overall_status,
        }),
    )
    .await;

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
    if state.sovereign_mass {
        return match sovereign_ops::get_entity(&state, id)? {
            Some(entity) => Ok(Json(entity)),
            None => Err(AppError::not_found(format!("entity {id} not found"))),
        };
    }

    let client = require_mass_client(&state)?;
    match client.entities().get(id).await {
        Ok(Some(entity)) => serde_json::to_value(entity)
            .map(Json)
            .map_err(|e| AppError::Internal(format!("serialization error: {e}"))),
        Ok(None) => Err(AppError::not_found(format!("entity {id} not found"))),
        Err(e) => Err(AppError::upstream(format!("Mass API error: {e}"))),
    }
}

/// PUT /v1/entities/{id} — Update an entity with compliance evaluation and VC issuance.
///
/// Orchestration pipeline:
/// 1. Fetch existing entity from Mass (to determine jurisdiction)
/// 2. Evaluate compliance tensor for that jurisdiction
/// 3. Reject if sanctions domain is `NonCompliant` (hard block)
/// 4. Update entity via Mass organization-info API
/// 5. Issue a `MezFormationComplianceCredential` VC
/// 6. Store attestation record for regulator queries
#[utoipa::path(
    put,
    path = "/v1/entities/:id",
    params(("id" = uuid::Uuid, Path, description = "Entity UUID")),
    request_body = UpdateEntityProxyRequest,
    responses(
        (status = 200, description = "Entity updated with compliance evaluation and VC"),
        (status = 403, description = "Blocked by compliance hard-block (sanctions)"),
        (status = 404, description = "Entity not found"),
        (status = 422, description = "Validation error"),
        (status = 502, description = "Mass API error"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "entities"
)]
async fn update_entity(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<UpdateEntityProxyRequest>,
) -> Result<Json<OrchestrationEnvelope>, AppError> {
    req.validate().map_err(AppError::Validation)?;

    // Determine jurisdiction from existing entity or request.
    let jurisdiction_id = if state.sovereign_mass {
        let existing = sovereign_ops::get_entity(&state, id)?
            .ok_or_else(|| AppError::not_found(format!("entity {id} not found")))?;
        req.jurisdiction_id
            .as_deref()
            .or_else(|| existing.get("jurisdiction").and_then(|v| v.as_str()))
            .filter(|j| !j.is_empty())
            .unwrap_or("GLOBAL")
            .to_string()
    } else {
        let client = require_mass_client(&state)?;
        let existing = client
            .entities()
            .get(id)
            .await
            .map_err(|e| AppError::upstream(format!("Mass API error: {e}")))?
            .ok_or_else(|| AppError::not_found(format!("entity {id} not found")))?;
        req.jurisdiction_id
            .as_deref()
            .or(existing.jurisdiction.as_deref())
            .filter(|j| !j.is_empty())
            .unwrap_or("GLOBAL")
            .to_string()
    };

    // Pre-flight compliance evaluation.
    let (_tensor, pre_summary) = orchestration::evaluate_compliance(
        &jurisdiction_id,
        &id.to_string(),
        orchestration::entity_domains(),
    );

    // Hard-block check (sanctions).
    if let Some(reason) = orchestration::check_hard_blocks(&pre_summary) {
        return Err(AppError::Forbidden(reason));
    }

    // Mass operation — sovereign or proxy.
    let mass_response = if state.sovereign_mass {
        let body = serde_json::to_value(&req)
            .map_err(|e| AppError::Internal(format!("serialization error: {e}")))?;
        sovereign_ops::update_entity(&state, id, &body).await?
    } else {
        let client = require_mass_client(&state)?;
        let body = serde_json::to_value(&req)
            .map_err(|e| AppError::Internal(format!("serialization error: {e}")))?;
        let entity = client
            .entities()
            .update(id, &body)
            .await
            .map_err(|e| AppError::upstream(format!("Mass API error: {e}")))?;
        serde_json::to_value(entity)
            .map_err(|e| AppError::Internal(format!("serialization error: {e}")))?
    };

    // Post-operation orchestration (VC issuance + attestation storage).
    let envelope =
        orchestration::orchestrate_entity_update(&state, id, &jurisdiction_id, mass_response);

    // Audit trail.
    append_audit(
        &state,
        "entity.updated",
        "entity",
        &envelope,
        "update",
        serde_json::json!({
            "jurisdiction": &jurisdiction_id,
            "compliance_status": &envelope.compliance.overall_status,
        }),
    )
    .await;

    Ok(Json(envelope))
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
    if state.sovereign_mass {
        let entities = sovereign_ops::list_entities(&state, None)?;
        return Ok(Json(serde_json::to_value(entities)
            .map_err(|e| AppError::Internal(format!("serialization error: {e}")))?));
    }

    let client = require_mass_client(&state)?;
    let entities = client
        .entities()
        .list(None)
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
/// 4. Issue a `MezOwnershipComplianceCredential` VC
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
    req.validate().map_err(AppError::Validation)?;

    // Pre-flight compliance evaluation.
    let entity_id = req.entity_id;
    let (_tensor, pre_summary) = orchestration::evaluate_compliance(
        "GLOBAL",
        &entity_id.to_string(),
        orchestration::ownership_domains(),
    );

    // Hard-block check.
    if let Some(reason) = orchestration::check_hard_blocks(&pre_summary) {
        return Err(AppError::Forbidden(reason));
    }

    // Mass operation — sovereign or proxy.
    let mass_response = if state.sovereign_mass {
        let authorized = req.share_classes.first().map(|sc| sc.authorized_shares).unwrap_or(0);
        let par_value = req.share_classes.first().and_then(|sc| sc.par_value.as_deref());
        sovereign_ops::create_cap_table(
            &state,
            &entity_id.to_string(),
            authorized,
            par_value,
            None,
        )
        .await?
    } else {
        let client = require_mass_client(&state)?;
        let mass_req = mez_mass_client::ownership::CreateCapTableRequest {
            organization_id: req.entity_id.to_string(),
            authorized_shares: req.share_classes.first().map(|sc| sc.authorized_shares),
            options_pool: None,
            par_value: req
                .share_classes
                .first()
                .and_then(|sc| sc.par_value.clone()),
            shareholders: vec![],
        };
        let cap_table = client
            .ownership()
            .create_cap_table(&mass_req)
            .await
            .map_err(|e| AppError::upstream(format!("Mass API error: {e}")))?;
        serde_json::to_value(cap_table)
            .map_err(|e| AppError::Internal(format!("serialization error: {e}")))?
    };

    // Post-operation orchestration.
    let envelope = orchestration::orchestrate_cap_table_creation(&state, entity_id, mass_response);

    // Audit trail.
    append_audit(
        &state,
        "ownership.cap_table_created",
        "cap_table",
        &envelope,
        "create",
        serde_json::json!({
            "entity_id": entity_id.to_string(),
            "compliance_status": &envelope.compliance.overall_status,
        }),
    )
    .await;

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
    if state.sovereign_mass {
        // In sovereign mode, the path param is the entity UUID — look up by org ID.
        return match sovereign_ops::get_cap_table_by_org(&state, &id.to_string())? {
            Some(ct) => Ok(Json(ct)),
            None => Err(AppError::not_found(format!(
                "cap table for entity {id} not found"
            ))),
        };
    }

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
/// 4. Issue a `MezFiscalComplianceCredential` VC
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
    req.validate().map_err(AppError::Validation)?;

    // Pre-flight compliance evaluation.
    let entity_id = req.entity_id;
    let currency = req.currency.clone();
    let inferred_jurisdiction = orchestration::infer_jurisdiction(&currency);
    let (_tensor, pre_summary) = orchestration::evaluate_compliance(
        inferred_jurisdiction,
        &entity_id.to_string(),
        orchestration::fiscal_account_domains(),
    );

    // Hard-block check.
    if let Some(reason) = orchestration::check_hard_blocks(&pre_summary) {
        return Err(AppError::Forbidden(reason));
    }

    // Mass operation — sovereign or proxy.
    let mass_response = if state.sovereign_mass {
        // In sovereign mode, find or create a treasury for this entity.
        let treasury_id = find_or_create_treasury(&state, &entity_id.to_string()).await?;
        sovereign_ops::create_account(&state, treasury_id, Some(&req.account_type)).await?
    } else {
        let client = require_mass_client(&state)?;
        let idempotency_key = format!("{}-{}", req.entity_id, req.account_type);
        let account = client
            .fiscal()
            .create_account(req.entity_id, &idempotency_key, Some(&req.account_type))
            .await
            .map_err(|e| AppError::upstream(format!("Mass API error: {e}")))?;
        serde_json::to_value(account)
            .map_err(|e| AppError::Internal(format!("serialization error: {e}")))?
    };

    // Post-operation orchestration.
    let envelope =
        orchestration::orchestrate_account_creation(&state, entity_id, &currency, mass_response);

    // Audit trail.
    append_audit(
        &state,
        "fiscal.account_created",
        "account",
        &envelope,
        "create",
        serde_json::json!({
            "entity_id": entity_id.to_string(),
            "currency": &currency,
            "compliance_status": &envelope.compliance.overall_status,
        }),
    )
    .await;

    Ok((axum::http::StatusCode::CREATED, Json(envelope)))
}

/// POST /v1/fiscal/payments — Initiate a payment with compliance evaluation.
///
/// Orchestration pipeline:
/// 1. Evaluate compliance tensor for AML/sanctions/payment domains
/// 2. Reject if sanctions hard-block
/// 3. Initiate payment via Mass treasury-info API
/// 4. Issue a `MezPaymentComplianceCredential` VC
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
    req.validate().map_err(AppError::Validation)?;

    // Pre-flight compliance evaluation.
    let from_account_id = req.from_account_id;
    let currency = req.currency.clone();
    let amount = req.amount.clone();
    let reference = req.reference.clone();
    let inferred_jurisdiction = orchestration::infer_jurisdiction(&currency);
    let (_tensor, pre_summary) = orchestration::evaluate_compliance(
        inferred_jurisdiction,
        &from_account_id.to_string(),
        orchestration::payment_domains(),
    );

    // Hard-block check.
    if let Some(reason) = orchestration::check_hard_blocks(&pre_summary) {
        return Err(AppError::Forbidden(reason));
    }

    // Mass operation — sovereign or proxy.
    let mass_response = if state.sovereign_mass {
        sovereign_ops::create_payment(
            &state,
            &from_account_id.to_string(),
            &req.amount,
            &req.currency,
            Some(&req.reference),
        )
        .await?
    } else {
        let client = require_mass_client(&state)?;
        let mass_req = mez_mass_client::fiscal::CreatePaymentRequest {
            source_account_id: req.from_account_id,
            amount: req.amount,
            currency: Some(req.currency),
            reference: Some(req.reference),
            payment_type: None,
            description: None,
            payment_entity: None,
            idempotency_key: None,
        };
        let payment = client
            .fiscal()
            .create_payment(&mass_req)
            .await
            .map_err(|e| AppError::upstream(format!("Mass API error: {e}")))?;
        serde_json::to_value(payment)
            .map_err(|e| AppError::Internal(format!("serialization error: {e}")))?
    };

    // Post-operation orchestration.
    let envelope =
        orchestration::orchestrate_payment(&state, from_account_id, &currency, mass_response);

    // Auto-generate tax event for the payment.
    generate_payment_tax_event(
        &state,
        from_account_id,
        &amount,
        &currency,
        &envelope.compliance.jurisdiction_id,
        &reference,
    )
    .await;

    // Audit trail.
    append_audit(
        &state,
        "fiscal.payment_initiated",
        "payment",
        &envelope,
        "create",
        serde_json::json!({
            "from_account_id": from_account_id.to_string(),
            "currency": &currency,
            "amount": &amount,
            "compliance_status": &envelope.compliance.overall_status,
        }),
    )
    .await;

    Ok((axum::http::StatusCode::CREATED, Json(envelope)))
}

// ── TAX EVENT BRIDGE ────────────────────────────────────────────────

/// Generate a tax event record for a payment that flowed through orchestration.
///
/// This is the bridge between Mass fiscal operations and the EZ tax pipeline.
/// The generated event runs through the withholding engine to compute applicable
/// withholdings, then is stored in both the in-memory store and the database.
///
/// Failures are logged but never block the payment — tax event generation is
/// a post-operation side effect, not a gate.
/// Compute the current Pakistan fiscal year string (Jul-Jun).
///
/// Pakistan's fiscal year runs July 1 to June 30. If the current month
/// is July or later, the fiscal year is `YYYY-(YYYY+1)`. Otherwise it is
/// `(YYYY-1)-YYYY`.
fn current_pk_fiscal_year() -> String {
    let now = Utc::now();
    let year = now.year();
    let month = now.month();
    if month >= 7 {
        format!("{}-{}", year, year + 1)
    } else {
        format!("{}-{}", year - 1, year)
    }
}

async fn generate_payment_tax_event(
    state: &AppState,
    from_account_id: uuid::Uuid,
    amount: &str,
    currency: &str,
    jurisdiction_id: &str,
    reference: &str,
) {
    use mez_agentic::tax::{format_amount, parse_amount, FilerStatus, TaxEvent, TaxEventType};

    let fiscal_year = current_pk_fiscal_year();
    let event = TaxEvent::new(
        from_account_id,
        TaxEventType::PaymentForGoods,
        jurisdiction_id,
        amount,
        currency,
        &fiscal_year,
    );

    let withholdings = {
        let pipeline = state.tax_pipeline.lock();
        pipeline.process_event(&event)
    };

    let gross_cents = parse_amount(amount).unwrap_or_else(|| {
        tracing::warn!(
            amount = amount,
            "failed to parse payment amount for tax event — recording as 0 cents",
        );
        0
    });
    let mut total_wht_cents: i64 = 0;
    for w in &withholdings {
        total_wht_cents = total_wht_cents.saturating_add(
            parse_amount(&w.withholding_amount).unwrap_or_else(|| {
                tracing::warn!(
                    withholding_amount = %w.withholding_amount,
                    "failed to parse withholding amount — recording as 0 cents",
                );
                0
            }),
        );
    }
    let net_cents = gross_cents.saturating_sub(total_wht_cents);

    let record = TaxEventRecord {
        id: Uuid::new_v4(),
        entity_id: from_account_id,
        event_type: "payment_for_goods".to_string(),
        tax_category: withholdings
            .first()
            .map(|w| w.tax_category.to_string())
            .unwrap_or_else(|| "income_tax".to_string()),
        jurisdiction_id: jurisdiction_id.to_string(),
        gross_amount: amount.to_string(),
        withholding_amount: format_amount(total_wht_cents),
        net_amount: format_amount(net_cents),
        currency: currency.to_string(),
        tax_year: fiscal_year,
        ntn: None,
        filer_status: FilerStatus::NonFiler.to_string(),
        statutory_section: withholdings.first().map(|w| w.statutory_section.clone()),
        withholding_executed: false,
        mass_payment_id: None,
        rules_applied: withholdings.len(),
        created_at: Utc::now(),
    };

    tracing::info!(
        tax_event_id = %record.id,
        entity_id = %from_account_id,
        jurisdiction = %jurisdiction_id,
        gross = %amount,
        withholding = %record.withholding_amount,
        reference = %reference,
        "auto-generated tax event from payment orchestration"
    );

    state.tax_events.insert(record.id, record.clone());

    // Persist to database (write-through).
    if let Some(pool) = &state.db_pool {
        if let Err(e) = crate::db::tax_events::insert(pool, &record).await {
            tracing::error!(tax_event_id = %record.id, error = %e, "failed to persist auto-generated tax event");
        }
    }
}

// ── IDENTITY HANDLERS ───────────────────────────────────────────────

/// POST /v1/identity/verify — Verify identity with compliance evaluation.
///
/// Orchestration pipeline:
/// 1. Evaluate compliance tensor for KYC/sanctions/data-privacy domains
/// 2. Reject if sanctions hard-block
/// 3. Submit verification via Mass identity API
/// 4. Issue a `MezIdentityComplianceCredential` VC
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
    req.validate().map_err(AppError::Validation)?;

    // Pre-flight compliance evaluation.
    let identity_type_str = req.identity_type.clone();
    let (_tensor, pre_summary) = orchestration::evaluate_compliance(
        "GLOBAL",
        "pre-flight",
        orchestration::identity_domains(),
    );

    // Hard-block check.
    if let Some(reason) = orchestration::check_hard_blocks(&pre_summary) {
        return Err(AppError::Forbidden(reason));
    }

    // Mass operation — sovereign or proxy.
    let mass_response = if state.sovereign_mass {
        // In sovereign mode, use identity verification from sovereign_ops.
        let first_id = req.linked_ids.first();
        let id_type = first_id.map(|lid| lid.id_type.as_str()).unwrap_or("");
        let id_value = first_id.map(|lid| lid.id_value.as_str()).unwrap_or("");

        match id_type.to_uppercase().as_str() {
            "CNIC" => sovereign_ops::verify_cnic(id_value, None)?,
            "NTN" => sovereign_ops::verify_ntn(id_value, None)?,
            other => {
                return Err(AppError::Validation(format!(
                    "unsupported identity type: {other:?} (supported: CNIC, NTN)"
                )));
            }
        }
    } else {
        let client = require_mass_client(&state)?;
        let org_id = req
            .linked_ids
            .first()
            .map(|lid| lid.id_value.clone())
            .unwrap_or_else(|| req.identity_type.clone());

        let identity = client
            .identity()
            .get_composite_identity(&org_id)
            .await
            .map_err(|e| AppError::upstream(format!("Mass API error: {e}")))?;
        serde_json::to_value(identity)
            .map_err(|e| AppError::Internal(format!("serialization error: {e}")))?
    };

    // Post-operation orchestration.
    let envelope =
        orchestration::orchestrate_identity_verification(&state, &identity_type_str, mass_response);

    // Audit trail.
    append_audit(
        &state,
        "identity.verified",
        "identity",
        &envelope,
        "verify",
        serde_json::json!({
            "identity_type": &identity_type_str,
            "compliance_status": &envelope.compliance.overall_status,
        }),
    )
    .await;

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
    if state.sovereign_mass {
        // In sovereign mode, return the entity data as identity info.
        return match sovereign_ops::get_entity(&state, id)? {
            Some(entity) => Ok(Json(entity)),
            None => Err(AppError::not_found(format!("identity {id} not found"))),
        };
    }

    let client = require_mass_client(&state)?;
    match client
        .identity()
        .get_composite_identity(&id.to_string())
        .await
    {
        Ok(identity) => serde_json::to_value(identity)
            .map(Json)
            .map_err(|e| AppError::Internal(format!("serialization error: {e}"))),
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
/// 4. Issue a `MezConsentComplianceCredential` VC
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
    req.validate().map_err(AppError::Validation)?;

    // Pre-flight compliance evaluation.
    let consent_type_str = req.consent_type.clone();
    let (_tensor, pre_summary) = orchestration::evaluate_compliance(
        "GLOBAL",
        "pre-flight",
        orchestration::consent_domains(),
    );

    // Hard-block check.
    if let Some(reason) = orchestration::check_hard_blocks(&pre_summary) {
        return Err(AppError::Forbidden(reason));
    }

    // Mass operation — sovereign or proxy.
    let mass_response = if state.sovereign_mass {
        let org_id = req
            .parties
            .first()
            .map(|p| p.entity_id.to_string())
            .unwrap_or_else(|| req.description.clone());
        let op_type = serde_json::Value::String(req.consent_type.clone());
        sovereign_ops::create_consent(
            &state,
            &org_id,
            None,
            Some(&op_type),
            None,
            None,
        )
        .await?
    } else {
        let client = require_mass_client(&state)?;
        let operation_type: mez_mass_client::consent::MassConsentOperationType =
            serde_json::from_value(serde_json::Value::String(req.consent_type.clone())).map_err(
                |e| {
                    AppError::Validation(format!(
                        "unknown consent type '{}': {e}",
                        req.consent_type,
                    ))
                },
            )?;

        let organization_id = req
            .parties
            .first()
            .map(|p| p.entity_id.to_string())
            .unwrap_or_else(|| req.description.clone());

        let mass_req = mez_mass_client::consent::CreateConsentRequest {
            organization_id,
            operation_type,
            operation_id: None,
            num_board_member_approvals_required: None,
            requested_by: None,
            signatory: None,
            expiry_date: None,
            details: Some(serde_json::json!({
                "description": req.description,
                "parties": req.parties.iter().map(|p| {
                    serde_json::json!({ "entity_id": p.entity_id, "role": p.role })
                }).collect::<Vec<_>>(),
            })),
        };

        let consent = client
            .consent()
            .create(&mass_req)
            .await
            .map_err(|e| AppError::upstream(format!("Mass API error: {e}")))?;
        serde_json::to_value(consent)
            .map_err(|e| AppError::Internal(format!("serialization error: {e}")))?
    };

    // Post-operation orchestration.
    let envelope =
        orchestration::orchestrate_consent_creation(&state, &consent_type_str, mass_response);

    // Audit trail.
    append_audit(
        &state,
        "consent.created",
        "consent",
        &envelope,
        "create",
        serde_json::json!({
            "consent_type": &consent_type_str,
            "compliance_status": &envelope.compliance.overall_status,
        }),
    )
    .await;

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
    if state.sovereign_mass {
        return match sovereign_ops::get_consent(&state, id)? {
            Some(consent) => Ok(Json(consent)),
            None => Err(AppError::not_found(format!(
                "consent request {id} not found"
            ))),
        };
    }

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

// ── SOVEREIGN HELPERS ──────────────────────────────────────────────────

/// Append an audit event after a successful orchestration write.
///
/// Logs but never fails the request — audit is a post-operation side effect.
async fn append_audit(
    state: &AppState,
    event_type: &str,
    resource_type: &str,
    envelope: &OrchestrationEnvelope,
    action: &str,
    metadata: serde_json::Value,
) {
    if let Some(ref pool) = state.db_pool {
        let resource_id = match envelope
            .mass_response
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<uuid::Uuid>().ok())
        {
            Some(id) => id,
            None => {
                let fallback = uuid::Uuid::new_v4();
                tracing::warn!(
                    event_type,
                    fallback_id = %fallback,
                    "Mass API response missing parseable 'id' for audit — using generated fallback"
                );
                fallback
            }
        };

        let event = crate::db::audit::AuditEvent {
            event_type: event_type.to_string(),
            actor_did: Some(state.zone_did.clone()),
            resource_type: resource_type.to_string(),
            resource_id,
            action: action.to_string(),
            metadata,
        };

        if let Err(e) = crate::db::audit::append(pool, event).await {
            tracing::warn!(error = %e, event_type, "Failed to append audit event");
        }
    }
}

/// Find an existing treasury for an entity, or create one.
///
/// In sovereign mode, `create_account` needs a treasury UUID but the
/// orchestrated endpoint only has `entity_id`. This helper searches
/// existing treasuries or creates one on the fly.
async fn find_or_create_treasury(
    state: &AppState,
    entity_id: &str,
) -> Result<uuid::Uuid, AppError> {
    // Check existing treasuries for one belonging to this entity.
    for treasury in state.mass_treasuries.list() {
        if treasury
            .get("entityId")
            .and_then(|v| v.as_str())
            .map(|v| v == entity_id)
            .unwrap_or(false)
        {
            if let Some(id_str) = treasury.get("id").and_then(|v| v.as_str()) {
                if let Ok(id) = id_str.parse::<uuid::Uuid>() {
                    return Ok(id);
                }
            }
        }
    }

    // None found — create one.
    let treasury = sovereign_ops::create_treasury(state, entity_id, None, None).await?;
    treasury
        .get("id")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<uuid::Uuid>().ok())
        .ok_or_else(|| AppError::Internal("failed to parse created treasury id".to_string()))
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
            "jurisdiction_id": "pk-ez-01"
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
            "jurisdiction_id": "pk-ez-01",
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
                r#"{"entity_type":"llc","legal_name":"Test","jurisdiction_id":"pk-ez-01"}"#,
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
    async fn update_entity_returns_503_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("PUT")
            .uri("/v1/entities/550e8400-e29b-41d4-a716-446655440000")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"legal_name":"Updated Corp"}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn create_cap_table_returns_503_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("POST")
            .uri("/v1/ownership/cap-tables")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"entity_id":"550e8400-e29b-41d4-a716-446655440000","share_classes":[{"name":"Common","authorized_shares":1000000,"issued_shares":100000,"par_value":"0.01","voting_rights":true}]}"#,
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
