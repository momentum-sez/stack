//! # CONSENT Primitive — Consent Info API
//!
//! Handles multi-party consent requests, consent signing,
//! and full audit trail for consent lifecycle.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppError;
use crate::extractors::{extract_validated_json, Validate};
use crate::state::{AppState, ConsentAuditEntry, ConsentParty, ConsentRecord};
use axum::extract::rejection::JsonRejection;

/// Request to create a multi-party consent.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateConsentRequest {
    pub consent_type: String,
    pub description: String,
    pub parties: Vec<ConsentPartyInput>,
}

/// Party input for consent creation.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ConsentPartyInput {
    pub entity_id: Uuid,
    pub role: String,
}

impl Validate for CreateConsentRequest {
    fn validate(&self) -> Result<(), String> {
        if self.consent_type.trim().is_empty() {
            return Err("consent_type must not be empty".to_string());
        }
        if self.parties.is_empty() {
            return Err("at least one party is required".to_string());
        }
        Ok(())
    }
}

/// Request to sign a consent.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SignConsentRequest {
    pub entity_id: Uuid,
    pub decision: String,
}

impl Validate for SignConsentRequest {
    fn validate(&self) -> Result<(), String> {
        if !["approve", "reject"].contains(&self.decision.as_str()) {
            return Err("decision must be 'approve' or 'reject'".to_string());
        }
        Ok(())
    }
}

/// Build the consent router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/consent/request", post(create_consent))
        .route("/v1/consent/:id", get(get_consent))
        .route("/v1/consent/:id/sign", post(sign_consent))
        .route("/v1/consent/:id/audit-trail", get(get_audit_trail))
}

/// POST /v1/consent/request — Create a multi-party consent request.
#[utoipa::path(
    post,
    path = "/v1/consent/request",
    request_body = CreateConsentRequest,
    responses(
        (status = 201, description = "Consent created", body = ConsentRecord),
    ),
    tag = "consent"
)]
async fn create_consent(
    State(state): State<AppState>,
    body: Result<Json<CreateConsentRequest>, JsonRejection>,
) -> Result<(axum::http::StatusCode, Json<ConsentRecord>), AppError> {
    let req = extract_validated_json(body)?;
    let now = Utc::now();
    let id = Uuid::new_v4();

    let parties: Vec<ConsentParty> = req
        .parties
        .into_iter()
        .map(|p| ConsentParty {
            entity_id: p.entity_id,
            role: p.role,
            decision: None,
            decided_at: None,
        })
        .collect();

    let audit_entry = ConsentAuditEntry {
        action: "CREATED".to_string(),
        actor_id: Uuid::nil(),
        timestamp: now,
        details: Some(format!("Consent '{}' created", req.consent_type)),
    };

    let record = ConsentRecord {
        id,
        consent_type: req.consent_type,
        description: req.description,
        parties,
        status: "PENDING".to_string(),
        audit_trail: vec![audit_entry],
        created_at: now,
        updated_at: now,
    };

    state.consents.insert(id, record.clone());
    Ok((axum::http::StatusCode::CREATED, Json(record)))
}

/// GET /v1/consent/:id — Get consent status.
#[utoipa::path(
    get,
    path = "/v1/consent/{id}",
    params(("id" = Uuid, Path, description = "Consent ID")),
    responses(
        (status = 200, description = "Consent found", body = ConsentRecord),
        (status = 404, description = "Not found", body = crate::error::ErrorBody),
    ),
    tag = "consent"
)]
async fn get_consent(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ConsentRecord>, AppError> {
    state
        .consents
        .get(&id)
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("consent {id} not found")))
}

/// POST /v1/consent/:id/sign — Sign a consent.
#[utoipa::path(
    post,
    path = "/v1/consent/{id}/sign",
    params(("id" = Uuid, Path, description = "Consent ID")),
    request_body = SignConsentRequest,
    responses(
        (status = 200, description = "Consent signed", body = ConsentRecord),
        (status = 404, description = "Not found", body = crate::error::ErrorBody),
    ),
    tag = "consent"
)]
async fn sign_consent(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    body: Result<Json<SignConsentRequest>, JsonRejection>,
) -> Result<Json<ConsentRecord>, AppError> {
    let req = extract_validated_json(body)?;
    let now = Utc::now();
    let entity_id = req.entity_id;
    let decision = req.decision.clone();

    state
        .consents
        .update(&id, |consent| {
            for party in &mut consent.parties {
                if party.entity_id == entity_id {
                    party.decision = Some(decision.clone());
                    party.decided_at = Some(now);
                }
            }

            consent.audit_trail.push(ConsentAuditEntry {
                action: format!("SIGNED:{}", decision),
                actor_id: entity_id,
                timestamp: now,
                details: None,
            });

            // Check if all parties have decided.
            let all_decided = consent.parties.iter().all(|p| p.decision.is_some());
            if all_decided {
                let all_approved = consent
                    .parties
                    .iter()
                    .all(|p| p.decision.as_deref() == Some("approve"));
                consent.status = if all_approved {
                    "APPROVED".to_string()
                } else {
                    "REJECTED".to_string()
                };
            }

            consent.updated_at = now;
        })
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("consent {id} not found")))
}

/// GET /v1/consent/:id/audit-trail — Get consent audit trail.
#[utoipa::path(
    get,
    path = "/v1/consent/{id}/audit-trail",
    params(("id" = Uuid, Path, description = "Consent ID")),
    responses(
        (status = 200, description = "Audit trail", body = Vec<ConsentAuditEntry>),
        (status = 404, description = "Not found", body = crate::error::ErrorBody),
    ),
    tag = "consent"
)]
async fn get_audit_trail(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ConsentAuditEntry>>, AppError> {
    state
        .consents
        .get(&id)
        .map(|c| Json(c.audit_trail))
        .ok_or_else(|| AppError::NotFound(format!("consent {id} not found")))
}
