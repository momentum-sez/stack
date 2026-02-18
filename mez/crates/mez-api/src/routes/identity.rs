//! # Identity Orchestration Routes (P1-005)
//!
//! Dedicated identity endpoints that compose Mass API identity operations with
//! EZ Stack compliance evaluation, credential issuance, and audit logging.
//!
//! ## Architecture
//!
//! These routes extend the generic identity orchestration in `mass_proxy.rs`
//! with Pakistan GovOS-specific verification flows: CNIC (NADRA) and NTN
//! (FBR IRIS). Each write operation follows the orchestration pipeline
//! defined in [`crate::orchestration`]:
//!
//! 1. Validate the request (format checks)
//! 2. Evaluate compliance tensor across identity-relevant domains
//! 3. Delegate to Mass via `IdentityClient` (aggregation facade)
//! 4. Store attestation via [`crate::orchestration::store_attestation`]
//! 5. Return enriched response with compliance summary
//!
//! ## Integration Points
//!
//! | Operation | Pakistan GovOS Integration |
//! |-----------|--------------------------|
//! | CNIC verify | NADRA (via organization-info or identity-info) |
//! | NTN verify | FBR IRIS (via organization-info or identity-info) |
//! | KYC/KYB | consent-info or identity-info |
//! | Entity identity | Consolidated view across services |

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppError;
use crate::extractors::{extract_validated_json, Validate};
use crate::orchestration;
use crate::state::AppState;

/// Build the identity orchestration router.
///
/// Mounts under `/v1/identity/` — extends the proxy routes with
/// orchestrated CNIC/NTN verification, consolidated views, and
/// entity-level identity aggregation.
pub fn router() -> Router<AppState> {
    Router::new()
        // Orchestrated verification endpoints
        .route("/v1/identity/cnic/verify", post(verify_cnic))
        .route("/v1/identity/ntn/verify", post(verify_ntn))
        // Consolidated entity identity view
        .route("/v1/identity/entity/:entity_id", get(get_entity_identity))
        // Identity status (service health / mode)
        .route("/v1/identity/status", get(identity_service_status))
}

/// Helper: extract the Mass client from AppState or return 503.
fn require_mass_client(state: &AppState) -> Result<&mez_mass_client::MassClient, AppError> {
    state.mass_client.as_ref().ok_or_else(|| {
        AppError::service_unavailable(
            "Mass API client not configured. Set MASS_API_TOKEN environment variable.",
        )
    })
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Request to verify a CNIC number against NADRA records.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CnicVerifyRequest {
    /// CNIC number (13 digits, with or without dashes).
    pub cnic: String,
    /// Full name for cross-reference.
    pub full_name: String,
    /// Date of birth (YYYY-MM-DD) for additional validation.
    #[serde(default)]
    pub date_of_birth: Option<String>,
    /// Entity ID to bind the verified CNIC to.
    #[serde(default)]
    pub entity_id: Option<Uuid>,
    /// Jurisdiction for compliance evaluation (default: "PK").
    #[serde(default)]
    pub jurisdiction_id: Option<String>,
}

/// Orchestrated CNIC verification response.
#[derive(Debug, Serialize, ToSchema)]
pub struct CnicVerifyResponse {
    /// Whether the CNIC was verified.
    pub verified: bool,
    /// CNIC number that was verified.
    pub cnic: String,
    /// Full name returned by NADRA (if verified).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    /// Identity record ID in Mass.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity_id: Option<Uuid>,
    /// Attestation record ID in the EZ Stack.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attestation_id: Option<Uuid>,
    /// Compliance status from 20-domain tensor evaluation.
    pub compliance_status: String,
    /// Timestamp of verification.
    pub verified_at: String,
}

/// Request to verify an NTN against FBR IRIS records.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NtnVerifyRequest {
    /// NTN number (7 digits).
    pub ntn: String,
    /// Entity name for cross-reference.
    pub entity_name: String,
    /// Entity ID to bind the verified NTN to.
    #[serde(default)]
    pub entity_id: Option<Uuid>,
    /// Jurisdiction for compliance evaluation (default: "PK").
    #[serde(default)]
    pub jurisdiction_id: Option<String>,
}

/// Orchestrated NTN verification response.
#[derive(Debug, Serialize, ToSchema)]
pub struct NtnVerifyResponse {
    /// Whether the NTN was verified.
    pub verified: bool,
    /// NTN number that was verified.
    pub ntn: String,
    /// Registered entity name from FBR (if verified).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registered_name: Option<String>,
    /// FBR tax status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_status: Option<String>,
    /// Identity record ID in Mass.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity_id: Option<Uuid>,
    /// Attestation record ID in the EZ Stack.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attestation_id: Option<Uuid>,
    /// Compliance status from 20-domain tensor evaluation.
    pub compliance_status: String,
    /// Timestamp of verification.
    pub verified_at: String,
}

/// Consolidated identity view for an entity.
#[derive(Debug, Serialize, ToSchema)]
pub struct EntityIdentityResponse {
    /// The entity this identity belongs to.
    pub entity_id: Uuid,
    /// Identity records from Mass.
    pub identities: Vec<serde_json::Value>,
    /// Attestation records from the EZ Stack.
    pub attestations: Vec<serde_json::Value>,
    /// Whether the identity client is using a dedicated service.
    pub dedicated_service: bool,
    /// Timestamp of this consolidation snapshot.
    pub consolidated_at: String,
}

/// Identity service status response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IdentityServiceStatus {
    /// Whether the identity client is configured.
    pub configured: bool,
    /// Whether a dedicated identity-info service is available.
    pub dedicated_service: bool,
    /// Data sources currently in use.
    pub data_sources: Vec<String>,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

impl Validate for CnicVerifyRequest {
    fn validate(&self) -> Result<(), String> {
        let digits: String = self.cnic.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() != 13 {
            return Err(format!(
                "CNIC must be exactly 13 digits, got {}",
                digits.len()
            ));
        }
        if self.full_name.trim().is_empty() {
            return Err("full_name must not be empty".into());
        }
        if self.full_name.len() > 500 {
            return Err("full_name must not exceed 500 characters".into());
        }
        if let Some(ref dob) = self.date_of_birth {
            if dob.trim().is_empty() {
                return Err("date_of_birth must not be empty when provided".into());
            }
        }
        if let Some(ref jid) = self.jurisdiction_id {
            if jid.trim().is_empty() {
                return Err("jurisdiction_id must not be empty when provided".into());
            }
        }
        Ok(())
    }
}

impl Validate for NtnVerifyRequest {
    fn validate(&self) -> Result<(), String> {
        let digits: String = self.ntn.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() != 7 {
            return Err(format!(
                "NTN must be exactly 7 digits, got {}",
                digits.len()
            ));
        }
        if self.entity_name.trim().is_empty() {
            return Err("entity_name must not be empty".into());
        }
        if self.entity_name.len() > 1000 {
            return Err("entity_name must not exceed 1000 characters".into());
        }
        if let Some(ref jid) = self.jurisdiction_id {
            if jid.trim().is_empty() {
                return Err("jurisdiction_id must not be empty when provided".into());
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /v1/identity/cnic/verify — Verify a CNIC number with KYC compliance.
///
/// Orchestration flow:
/// 1. Validate CNIC format (13 digits)
/// 2. Evaluate compliance tensor across identity-relevant domains
/// 3. Delegate NADRA verification to Mass via `IdentityClient`
/// 4. Store attestation via orchestration module
/// 5. Return enriched response with compliance status
#[utoipa::path(
    post,
    path = "/v1/identity/cnic/verify",
    request_body = CnicVerifyRequest,
    responses(
        (status = 200, description = "CNIC verification result"),
        (status = 400, description = "Invalid CNIC format"),
        (status = 502, description = "Mass API error"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "identity"
)]
async fn verify_cnic(
    State(state): State<AppState>,
    body: Result<Json<CnicVerifyRequest>, JsonRejection>,
) -> Result<Json<CnicVerifyResponse>, AppError> {
    let req = extract_validated_json(body)?;
    let client = require_mass_client(&state)?;

    let jurisdiction = req.jurisdiction_id.as_deref().unwrap_or("PK");
    let entity_id = req.entity_id.unwrap_or_else(Uuid::new_v4);
    let entity_ref = entity_id.to_string();

    // Evaluate compliance tensor across identity-relevant domains.
    let (_tensor, summary) = orchestration::evaluate_compliance(
        jurisdiction,
        &entity_ref,
        orchestration::identity_domains(),
    );

    // Delegate to Mass identity client for NADRA verification.
    let mass_req = mez_mass_client::identity::CnicVerificationRequest {
        cnic: req.cnic.clone(),
        full_name: req.full_name.clone(),
        date_of_birth: req.date_of_birth.clone(),
        entity_id: req.entity_id,
    };

    let result = client
        .identity()
        .verify_cnic(&mass_req)
        .await
        .map_err(|e| AppError::upstream(format!("NADRA verification error: {e}")))?;

    // Store attestation via the orchestration module for regulator queries.
    let attestation_id = if result.verified {
        Some(orchestration::store_attestation(
            &state,
            entity_id,
            "CNIC_VERIFICATION",
            jurisdiction,
            serde_json::json!({
                "cnic": req.cnic,
                "verified_name": result.full_name,
                "issuer": "NADRA",
                "overall_compliance": summary.overall_status,
            }),
        ))
    } else {
        None
    };

    Ok(Json(CnicVerifyResponse {
        verified: result.verified,
        cnic: result.cnic,
        full_name: result.full_name,
        identity_id: result.identity_id,
        attestation_id,
        compliance_status: summary.overall_status,
        verified_at: result.verification_timestamp.to_rfc3339(),
    }))
}

/// POST /v1/identity/ntn/verify — Verify an NTN number with tax compliance.
///
/// Orchestration flow:
/// 1. Validate NTN format (7 digits)
/// 2. Evaluate compliance tensor across identity-relevant domains
/// 3. Delegate FBR IRIS verification to Mass via `IdentityClient`
/// 4. Store attestation via orchestration module
/// 5. Return enriched response with compliance status
#[utoipa::path(
    post,
    path = "/v1/identity/ntn/verify",
    request_body = NtnVerifyRequest,
    responses(
        (status = 200, description = "NTN verification result"),
        (status = 400, description = "Invalid NTN format"),
        (status = 502, description = "Mass API error"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "identity"
)]
async fn verify_ntn(
    State(state): State<AppState>,
    body: Result<Json<NtnVerifyRequest>, JsonRejection>,
) -> Result<Json<NtnVerifyResponse>, AppError> {
    let req = extract_validated_json(body)?;
    let client = require_mass_client(&state)?;

    let jurisdiction = req.jurisdiction_id.as_deref().unwrap_or("PK");
    let entity_id = req.entity_id.unwrap_or_else(Uuid::new_v4);
    let entity_ref = entity_id.to_string();

    // Evaluate compliance tensor across identity-relevant domains.
    let (_tensor, summary) = orchestration::evaluate_compliance(
        jurisdiction,
        &entity_ref,
        orchestration::identity_domains(),
    );

    // Delegate to Mass identity client for FBR IRIS verification.
    let mass_req = mez_mass_client::identity::NtnVerificationRequest {
        ntn: req.ntn.clone(),
        entity_name: req.entity_name.clone(),
        entity_id: req.entity_id,
    };

    let result = client
        .identity()
        .verify_ntn(&mass_req)
        .await
        .map_err(|e| AppError::upstream(format!("FBR IRIS verification error: {e}")))?;

    // Store attestation via the orchestration module for regulator queries.
    let attestation_id = if result.verified {
        Some(orchestration::store_attestation(
            &state,
            entity_id,
            "NTN_VERIFICATION",
            jurisdiction,
            serde_json::json!({
                "ntn": req.ntn,
                "registered_name": result.registered_name,
                "tax_status": result.tax_status,
                "issuer": "FBR_IRIS",
                "overall_compliance": summary.overall_status,
            }),
        ))
    } else {
        None
    };

    Ok(Json(NtnVerifyResponse {
        verified: result.verified,
        ntn: result.ntn,
        registered_name: result.registered_name,
        tax_status: result.tax_status,
        identity_id: result.identity_id,
        attestation_id,
        compliance_status: summary.overall_status,
        verified_at: result.verification_timestamp.to_rfc3339(),
    }))
}

/// GET /v1/identity/entity/{entity_id} — Consolidated identity view.
///
/// Aggregates identity data from consent-info and organization-info (or the
/// dedicated identity-info service) into a single response. Includes EZ Stack
/// attestation records for the entity.
#[utoipa::path(
    get,
    path = "/v1/identity/entity/:entity_id",
    params(("entity_id" = Uuid, Path, description = "Entity UUID")),
    responses(
        (status = 200, description = "Consolidated identity view"),
        (status = 502, description = "Mass API error"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "identity"
)]
async fn get_entity_identity(
    State(state): State<AppState>,
    Path(entity_id): Path<Uuid>,
) -> Result<Json<EntityIdentityResponse>, AppError> {
    let client = require_mass_client(&state)?;

    // Fetch identities from Mass via the aggregation facade.
    let identities = client
        .identity()
        .list_by_entity(entity_id)
        .await
        .map_err(|e| AppError::upstream(format!("Mass identity list error: {e}")))?;

    let identity_values: Vec<serde_json::Value> = identities
        .into_iter()
        .filter_map(|id| serde_json::to_value(id).ok())
        .collect();

    // Fetch attestations from EZ Stack for this entity.
    let attestations: Vec<serde_json::Value> = state
        .attestations
        .list()
        .into_iter()
        .filter(|a| a.entity_id == entity_id)
        .filter_map(|a| serde_json::to_value(a).ok())
        .collect();

    let dedicated = client.identity().has_dedicated_service();

    Ok(Json(EntityIdentityResponse {
        entity_id,
        identities: identity_values,
        attestations,
        dedicated_service: dedicated,
        consolidated_at: Utc::now().to_rfc3339(),
    }))
}

/// GET /v1/identity/status — Identity service health and mode.
///
/// Reports whether the identity client is configured, whether a dedicated
/// identity-info service is available, and which data sources are in use.
#[utoipa::path(
    get,
    path = "/v1/identity/status",
    responses(
        (status = 200, description = "Identity service status"),
    ),
    tag = "identity"
)]
async fn identity_service_status(State(state): State<AppState>) -> Json<IdentityServiceStatus> {
    let (configured, dedicated, sources) = match &state.mass_client {
        Some(client) => {
            let dedicated = client.identity().has_dedicated_service();
            let sources = if dedicated {
                vec!["identity-info.api.mass.inc".to_string()]
            } else {
                vec![
                    "consent.api.mass.inc".to_string(),
                    "organization-info.api.mass.inc".to_string(),
                ]
            };
            (true, dedicated, sources)
        }
        None => (false, false, vec![]),
    };

    Json(IdentityServiceStatus {
        configured,
        dedicated_service: dedicated,
        data_sources: sources,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_builds_successfully() {
        let _router = router();
    }

    #[test]
    fn cnic_verify_request_deserializes() {
        let json = r#"{
            "cnic": "12345-1234567-1",
            "full_name": "Muhammad Ali",
            "entity_id": "550e8400-e29b-41d4-a716-446655440000"
        }"#;
        let req: CnicVerifyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.cnic, "12345-1234567-1");
        assert_eq!(req.full_name, "Muhammad Ali");
        assert!(req.entity_id.is_some());
    }

    #[test]
    fn ntn_verify_request_deserializes() {
        let json = r#"{
            "ntn": "1234567",
            "entity_name": "Momentum Technologies Pvt Ltd"
        }"#;
        let req: NtnVerifyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.ntn, "1234567");
        assert_eq!(req.entity_name, "Momentum Technologies Pvt Ltd");
        assert!(req.entity_id.is_none());
    }

    // ── 503 tests (no Mass client configured) ────────────────────

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn verify_cnic_returns_503_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("POST")
            .uri("/v1/identity/cnic/verify")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"cnic":"12345-1234567-1","full_name":"Test"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["error"]["code"], "SERVICE_UNAVAILABLE");
    }

    #[tokio::test]
    async fn verify_ntn_returns_503_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("POST")
            .uri("/v1/identity/ntn/verify")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"ntn":"1234567","entity_name":"Test Corp"}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn get_entity_identity_returns_503_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("GET")
            .uri("/v1/identity/entity/550e8400-e29b-41d4-a716-446655440000")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn identity_status_returns_unconfigured_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("GET")
            .uri("/v1/identity/status")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let body: IdentityServiceStatus = serde_json::from_slice(&bytes).unwrap();
        assert!(!body.configured);
        assert!(!body.dedicated_service);
        assert!(body.data_sources.is_empty());
    }
}
