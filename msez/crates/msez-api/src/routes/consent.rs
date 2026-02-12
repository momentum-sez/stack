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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractors::Validate;

    // ── CreateConsentRequest validation ───────────────────────────

    #[test]
    fn test_create_consent_request_valid() {
        let req = CreateConsentRequest {
            consent_type: "board_resolution".to_string(),
            description: "Approve annual budget".to_string(),
            parties: vec![ConsentPartyInput {
                entity_id: Uuid::new_v4(),
                role: "director".to_string(),
            }],
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_consent_request_empty_consent_type() {
        let req = CreateConsentRequest {
            consent_type: "".to_string(),
            description: "Approve annual budget".to_string(),
            parties: vec![ConsentPartyInput {
                entity_id: Uuid::new_v4(),
                role: "director".to_string(),
            }],
        };
        let err = req.validate().unwrap_err();
        assert!(err.contains("consent_type"), "error should mention consent_type: {err}");
    }

    #[test]
    fn test_create_consent_request_whitespace_consent_type() {
        let req = CreateConsentRequest {
            consent_type: "   ".to_string(),
            description: "Something".to_string(),
            parties: vec![ConsentPartyInput {
                entity_id: Uuid::new_v4(),
                role: "director".to_string(),
            }],
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_create_consent_request_no_parties() {
        let req = CreateConsentRequest {
            consent_type: "board_resolution".to_string(),
            description: "Approve annual budget".to_string(),
            parties: vec![],
        };
        let err = req.validate().unwrap_err();
        assert!(err.contains("party") || err.contains("parties"), "error should mention parties: {err}");
    }

    #[test]
    fn test_create_consent_request_multiple_parties() {
        let req = CreateConsentRequest {
            consent_type: "shareholder_vote".to_string(),
            description: "Approve merger".to_string(),
            parties: vec![
                ConsentPartyInput {
                    entity_id: Uuid::new_v4(),
                    role: "shareholder".to_string(),
                },
                ConsentPartyInput {
                    entity_id: Uuid::new_v4(),
                    role: "shareholder".to_string(),
                },
            ],
        };
        assert!(req.validate().is_ok());
    }

    // ── SignConsentRequest validation ─────────────────────────────

    #[test]
    fn test_sign_consent_request_approve() {
        let req = SignConsentRequest {
            entity_id: Uuid::new_v4(),
            decision: "approve".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_sign_consent_request_reject() {
        let req = SignConsentRequest {
            entity_id: Uuid::new_v4(),
            decision: "reject".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_sign_consent_request_invalid_decision() {
        let req = SignConsentRequest {
            entity_id: Uuid::new_v4(),
            decision: "abstain".to_string(),
        };
        let err = req.validate().unwrap_err();
        assert!(err.contains("decision"), "error should mention decision: {err}");
    }

    #[test]
    fn test_sign_consent_request_empty_decision() {
        let req = SignConsentRequest {
            entity_id: Uuid::new_v4(),
            decision: "".to_string(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_sign_consent_request_case_sensitive() {
        let req = SignConsentRequest {
            entity_id: Uuid::new_v4(),
            decision: "Approve".to_string(),
        };
        // The validation checks exact match, so uppercase should fail.
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

    /// Helper: build the consent router with a fresh AppState.
    fn test_app() -> Router<()> {
        router().with_state(AppState::new())
    }

    /// Helper: read the response body as bytes and deserialize from JSON.
    async fn body_json<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn handler_create_consent_returns_201() {
        let app = test_app();
        let party_id = Uuid::new_v4();
        let body_str = format!(
            r#"{{"consent_type":"board_resolution","description":"Approve budget","parties":[{{"entity_id":"{}","role":"director"}}]}}"#,
            party_id
        );
        let req = Request::builder()
            .method("POST")
            .uri("/v1/consent/request")
            .header("content-type", "application/json")
            .body(Body::from(body_str))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let record: ConsentRecord = body_json(resp).await;
        assert_eq!(record.consent_type, "board_resolution");
        assert_eq!(record.status, "PENDING");
        assert_eq!(record.parties.len(), 1);
        assert_eq!(record.parties[0].entity_id, party_id);
        assert!(record.parties[0].decision.is_none());
        assert_eq!(record.audit_trail.len(), 1);
        assert_eq!(record.audit_trail[0].action, "CREATED");
    }

    #[tokio::test]
    async fn handler_create_consent_empty_type_returns_422() {
        let app = test_app();
        let party_id = Uuid::new_v4();
        let body_str = format!(
            r#"{{"consent_type":"","description":"Test","parties":[{{"entity_id":"{}","role":"director"}}]}}"#,
            party_id
        );
        let req = Request::builder()
            .method("POST")
            .uri("/v1/consent/request")
            .header("content-type", "application/json")
            .body(Body::from(body_str))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_create_consent_no_parties_returns_422() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/consent/request")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"consent_type":"board_resolution","description":"Test","parties":[]}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_sign_consent_approve_returns_200() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        let party_id = Uuid::new_v4();

        // Create a consent.
        let create_body = format!(
            r#"{{"consent_type":"shareholder_vote","description":"Approve merger","parties":[{{"entity_id":"{}","role":"shareholder"}}]}}"#,
            party_id
        );
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/consent/request")
            .header("content-type", "application/json")
            .body(Body::from(create_body))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);

        let created: ConsentRecord = body_json(create_resp).await;

        // Sign the consent.
        let sign_body = format!(
            r#"{{"entity_id":"{}","decision":"approve"}}"#,
            party_id
        );
        let sign_req = Request::builder()
            .method("POST")
            .uri(&format!("/v1/consent/{}/sign", created.id))
            .header("content-type", "application/json")
            .body(Body::from(sign_body))
            .unwrap();
        let sign_resp = app.oneshot(sign_req).await.unwrap();
        assert_eq!(sign_resp.status(), StatusCode::OK);

        let signed: ConsentRecord = body_json(sign_resp).await;
        assert_eq!(signed.status, "APPROVED");
        assert_eq!(signed.parties[0].decision.as_deref(), Some("approve"));
        // Audit trail should have 2 entries: CREATED + SIGNED:approve.
        assert_eq!(signed.audit_trail.len(), 2);
    }

    #[tokio::test]
    async fn handler_sign_consent_invalid_decision_returns_422() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        let party_id = Uuid::new_v4();

        // Create a consent first.
        let create_body = format!(
            r#"{{"consent_type":"vote","description":"Test","parties":[{{"entity_id":"{}","role":"member"}}]}}"#,
            party_id
        );
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/consent/request")
            .header("content-type", "application/json")
            .body(Body::from(create_body))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        let created: ConsentRecord = body_json(create_resp).await;

        // Sign with invalid decision.
        let sign_body = format!(
            r#"{{"entity_id":"{}","decision":"abstain"}}"#,
            party_id
        );
        let sign_req = Request::builder()
            .method("POST")
            .uri(&format!("/v1/consent/{}/sign", created.id))
            .header("content-type", "application/json")
            .body(Body::from(sign_body))
            .unwrap();
        let sign_resp = app.oneshot(sign_req).await.unwrap();
        assert_eq!(sign_resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_sign_consent_not_found_returns_404() {
        let app = test_app();
        let consent_id = Uuid::new_v4();
        let party_id = Uuid::new_v4();
        let sign_body = format!(
            r#"{{"entity_id":"{}","decision":"approve"}}"#,
            party_id
        );
        let req = Request::builder()
            .method("POST")
            .uri(&format!("/v1/consent/{consent_id}/sign"))
            .header("content-type", "application/json")
            .body(Body::from(sign_body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn handler_get_consent_not_found_returns_404() {
        let app = test_app();
        let id = Uuid::new_v4();
        let req = Request::builder()
            .method("GET")
            .uri(&format!("/v1/consent/{id}"))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
