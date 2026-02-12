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
use crate::extractors::{extract_validated_json, Validate};
use crate::state::{AppState, IdentityAttestation, IdentityRecord, LinkedExternalId};
use axum::extract::rejection::JsonRejection;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractors::Validate;

    // ── VerifyIdentityRequest validation ──────────────────────────

    #[test]
    fn test_verify_identity_request_valid() {
        let req = VerifyIdentityRequest {
            identity_type: "kyc".to_string(),
            details: serde_json::json!({"name": "Ali Khan"}),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_verify_identity_request_empty_identity_type() {
        let req = VerifyIdentityRequest {
            identity_type: "".to_string(),
            details: serde_json::json!({}),
        };
        let err = req.validate().unwrap_err();
        assert!(err.contains("identity_type"), "error should mention identity_type: {err}");
    }

    #[test]
    fn test_verify_identity_request_whitespace_identity_type() {
        let req = VerifyIdentityRequest {
            identity_type: "   ".to_string(),
            details: serde_json::json!({}),
        };
        assert!(req.validate().is_err());
    }

    // ── LinkExternalIdRequest validation ──────────────────────────

    #[test]
    fn test_link_external_id_request_valid() {
        let req = LinkExternalIdRequest {
            id_type: "cnic".to_string(),
            id_value: "12345-6789012-3".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_link_external_id_request_empty_id_type() {
        let req = LinkExternalIdRequest {
            id_type: "".to_string(),
            id_value: "12345-6789012-3".to_string(),
        };
        let err = req.validate().unwrap_err();
        assert!(err.contains("id_type"), "error should mention id_type: {err}");
    }

    #[test]
    fn test_link_external_id_request_empty_id_value() {
        let req = LinkExternalIdRequest {
            id_type: "cnic".to_string(),
            id_value: "".to_string(),
        };
        let err = req.validate().unwrap_err();
        assert!(err.contains("id_value") || err.contains("id_type"), "error should mention fields: {err}");
    }

    #[test]
    fn test_link_external_id_request_both_empty() {
        let req = LinkExternalIdRequest {
            id_type: "  ".to_string(),
            id_value: "  ".to_string(),
        };
        assert!(req.validate().is_err());
    }

    // ── SubmitAttestationRequest validation ───────────────────────

    #[test]
    fn test_submit_attestation_request_valid() {
        let req = SubmitAttestationRequest {
            attestation_type: "identity_verification".to_string(),
            issuer: "NADRA".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_submit_attestation_request_empty_attestation_type() {
        let req = SubmitAttestationRequest {
            attestation_type: "".to_string(),
            issuer: "NADRA".to_string(),
        };
        let err = req.validate().unwrap_err();
        assert!(err.contains("attestation_type"), "error should mention attestation_type: {err}");
    }

    #[test]
    fn test_submit_attestation_request_whitespace_attestation_type() {
        let req = SubmitAttestationRequest {
            attestation_type: "   ".to_string(),
            issuer: "NADRA".to_string(),
        };
        assert!(req.validate().is_err());
    }

    // ── Router construction ───────────────────────────────────────

    #[test]
    fn test_router_builds_successfully() {
        let _router = router();
    }

    // ── Handler integration tests ──────────────────────────────────

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    /// Helper: build the identity router with a fresh AppState.
    fn test_app() -> Router<()> {
        router().with_state(AppState::new())
    }

    /// Helper: read the response body as bytes and deserialize from JSON.
    async fn body_json<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn handler_verify_identity_returns_201() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/identity/verify")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"identity_type":"kyc","details":{"name":"Ali Khan","cnic":"12345-6789012-3"}}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let record: IdentityRecord = body_json(resp).await;
        assert_eq!(record.identity_type, "kyc");
        assert_eq!(record.status, "PENDING");
        assert!(record.linked_ids.is_empty());
        assert!(record.attestations.is_empty());
    }

    #[tokio::test]
    async fn handler_verify_identity_empty_type_returns_422() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/identity/verify")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"identity_type":"","details":{}}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_verify_identity_bad_json_returns_400() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/identity/verify")
            .header("content-type", "application/json")
            .body(Body::from(r#"{bad"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn handler_get_identity_not_found_returns_404() {
        let app = test_app();
        let id = Uuid::new_v4();
        let req = Request::builder()
            .method("GET")
            .uri(&format!("/v1/identity/{id}"))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn handler_verify_then_get_identity_returns_200() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Create an identity via verify.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/identity/verify")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"identity_type":"kyb","details":{"company":"Acme Corp"}}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);

        let created: IdentityRecord = body_json(create_resp).await;

        // Get the identity.
        let get_req = Request::builder()
            .method("GET")
            .uri(&format!("/v1/identity/{}", created.id))
            .body(Body::empty())
            .unwrap();
        let get_resp = app.oneshot(get_req).await.unwrap();
        assert_eq!(get_resp.status(), StatusCode::OK);

        let fetched: IdentityRecord = body_json(get_resp).await;
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.identity_type, "kyb");
    }
}
