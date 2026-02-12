//! # IDENTITY Primitive — Identity Verification API
//!
//! Handles KYC/KYB verification, identity record management,
//! external ID linking (CNIC, NTN, passport), and identity attestations.
//! Supports NADRA CNIC cross-referencing as a verification method.
//!
//! Note: Per audit section 3.3, Identity is the weakest primitive.
//! API surface is implemented with reasonable stubs where the Python
//! codebase has no corresponding implementation.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppError;
use axum::extract::rejection::JsonRejection;
use crate::extractors::{Validate, extract_validated_json};
use crate::state::{AppState, IdentityAttestation, IdentityRecord, LinkedExternalId};

/// KYC/KYB verification request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyIdentityRequest {
    /// Type of identity verification: "kyc", "kyb".
    pub identity_type: String,
    /// Applicant details.
    pub details: serde_json::Value,
}

impl Validate for VerifyIdentityRequest {
    fn validate(&self) -> Result<(), String> {
        if self.identity_type.trim().is_empty() {
            return Err("identity_type must not be empty".to_string());
        }
        Ok(())
    }
}

/// Link external ID request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct LinkExternalIdRequest {
    /// Type of external ID: "cnic", "ntn", "passport".
    pub id_type: String,
    /// The external ID value.
    pub id_value: String,
}

impl Validate for LinkExternalIdRequest {
    fn validate(&self) -> Result<(), String> {
        if self.id_type.trim().is_empty() || self.id_value.trim().is_empty() {
            return Err("id_type and id_value must not be empty".to_string());
        }
        Ok(())
    }
}

/// Submit identity attestation request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SubmitAttestationRequest {
    pub attestation_type: String,
    pub issuer: String,
}

impl Validate for SubmitAttestationRequest {
    fn validate(&self) -> Result<(), String> {
        if self.attestation_type.trim().is_empty() {
            return Err("attestation_type must not be empty".to_string());
        }
        Ok(())
    }
}

/// Build the identity router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/identity/verify", post(verify_identity))
        .route("/v1/identity/:id", get(get_identity))
        .route("/v1/identity/:id/link", post(link_external_id))
        .route("/v1/identity/:id/attestation", post(submit_attestation))
}

/// POST /v1/identity/verify — Submit KYC/KYB verification.
#[utoipa::path(
    post,
    path = "/v1/identity/verify",
    request_body = VerifyIdentityRequest,
    responses(
        (status = 201, description = "Verification submitted", body = IdentityRecord),
    ),
    tag = "identity"
)]
async fn verify_identity(
    State(state): State<AppState>,
    body: Result<Json<VerifyIdentityRequest>, JsonRejection>,
) -> Result<(axum::http::StatusCode, Json<IdentityRecord>), AppError> {
    let req = extract_validated_json(body)?;
    let now = Utc::now();
    let id = Uuid::new_v4();

    let record = IdentityRecord {
        id,
        identity_type: req.identity_type,
        status: "PENDING".to_string(),
        linked_ids: Vec::new(),
        attestations: Vec::new(),
        created_at: now,
        updated_at: now,
    };

    state.identities.insert(id, record.clone());
    Ok((axum::http::StatusCode::CREATED, Json(record)))
}

/// GET /v1/identity/:id — Get identity record.
#[utoipa::path(
    get,
    path = "/v1/identity/{id}",
    params(("id" = Uuid, Path, description = "Identity ID")),
    responses(
        (status = 200, description = "Identity found", body = IdentityRecord),
        (status = 404, description = "Not found", body = crate::error::ErrorBody),
    ),
    tag = "identity"
)]
async fn get_identity(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<IdentityRecord>, AppError> {
    state
        .identities
        .get(&id)
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("identity {id} not found")))
}

/// POST /v1/identity/:id/link — Link an external ID.
#[utoipa::path(
    post,
    path = "/v1/identity/{id}/link",
    params(("id" = Uuid, Path, description = "Identity ID")),
    request_body = LinkExternalIdRequest,
    responses(
        (status = 200, description = "External ID linked", body = IdentityRecord),
        (status = 404, description = "Not found", body = crate::error::ErrorBody),
    ),
    tag = "identity"
)]
async fn link_external_id(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    body: Result<Json<LinkExternalIdRequest>, JsonRejection>,
) -> Result<Json<IdentityRecord>, AppError> {
    let req = extract_validated_json(body)?;
    let linked = LinkedExternalId {
        id_type: req.id_type,
        id_value: req.id_value,
        verified: false,
        linked_at: Utc::now(),
    };

    state
        .identities
        .update(&id, |rec| {
            rec.linked_ids.push(linked);
            rec.updated_at = Utc::now();
        })
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("identity {id} not found")))
}

/// POST /v1/identity/:id/attestation — Submit identity attestation.
#[utoipa::path(
    post,
    path = "/v1/identity/{id}/attestation",
    params(("id" = Uuid, Path, description = "Identity ID")),
    request_body = SubmitAttestationRequest,
    responses(
        (status = 200, description = "Attestation submitted", body = IdentityRecord),
        (status = 404, description = "Not found", body = crate::error::ErrorBody),
    ),
    tag = "identity"
)]
async fn submit_attestation(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    body: Result<Json<SubmitAttestationRequest>, JsonRejection>,
) -> Result<Json<IdentityRecord>, AppError> {
    let req = extract_validated_json(body)?;
    let attestation = IdentityAttestation {
        id: Uuid::new_v4(),
        attestation_type: req.attestation_type,
        issuer: req.issuer,
        status: "PENDING".to_string(),
        issued_at: Utc::now(),
        expires_at: None,
    };

    state
        .identities
        .update(&id, |rec| {
            rec.attestations.push(attestation);
            rec.updated_at = Utc::now();
        })
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("identity {id} not found")))
}
