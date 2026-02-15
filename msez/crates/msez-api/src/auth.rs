//! # Authentication & Authorization Middleware
//!
//! Bearer token middleware with role-based access control (RBAC).
//!
//! ## Token Format
//!
//! **Phase 1** (current): Bearer tokens encode role and entity identity:
//!
//! ```text
//! Bearer {role}:{entity_id}:{secret}   — new format
//! Bearer {secret}                       — legacy format (treated as ZoneAdmin)
//! ```
//!
//! **Phase 2** (future): JWT claims replace the token format. The
//! `CallerIdentity` extractor stays the same — only parsing changes.
//!
//! ## CallerIdentity
//!
//! Every authenticated request gets a [`CallerIdentity`] injected into the
//! request extensions. Handlers extract it via the `FromRequestParts` impl.

use axum::extract::Request;
use axum::http::request::Parts;
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{AppError, ErrorBody, ErrorDetail};
use crate::state::SmartAssetRecord;

// ── Role ────────────────────────────────────────────────────────────────────

/// Roles in the SEZ Stack, ordered by privilege level.
///
/// The `Ord` derivation respects variant declaration order:
/// `EntityOperator < Regulator < ZoneAdmin`. This enables `>=` comparison
/// for role-based access checks.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Can read/write own entity's resources.
    EntityOperator,
    /// Can read all resources in the jurisdiction. Can query attestations.
    Regulator,
    /// Full access to all resources and endpoints.
    ZoneAdmin,
}

impl Role {
    /// Return the string representation of this role.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EntityOperator => "entity_operator",
            Self::Regulator => "regulator",
            Self::ZoneAdmin => "zone_admin",
        }
    }
}

// ── CallerIdentity ──────────────────────────────────────────────────────────

/// Identity of the authenticated caller, extracted from the auth context
/// and available to all route handlers via Axum's `FromRequestParts`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallerIdentity {
    /// The caller's role in the system.
    pub role: Role,
    /// The caller's entity ID (for entity_operator role).
    /// None for zone_admin and regulator (they operate across entities).
    pub entity_id: Option<Uuid>,
    /// The caller's jurisdiction (for scoped access). Populated from
    /// zone context in future phases.
    pub jurisdiction_id: Option<String>,
}

impl CallerIdentity {
    /// Check if the caller has at least the given minimum role.
    ///
    /// Since `Role` derives `Ord` with `EntityOperator < Regulator < ZoneAdmin`,
    /// this is a single comparison.
    pub fn has_role(&self, minimum: Role) -> bool {
        self.role >= minimum
    }

    /// Check if the caller can access the given smart asset.
    ///
    /// - `ZoneAdmin` and `Regulator` can access any asset.
    /// - `EntityOperator` can only access assets they own.
    pub fn can_access_asset(&self, asset: &SmartAssetRecord) -> bool {
        match self.role {
            Role::ZoneAdmin => true,
            Role::Regulator => true,
            Role::EntityOperator => {
                match (&self.entity_id, &asset.owner_entity_id) {
                    (Some(caller), Some(owner)) => caller == owner,
                    _ => false, // no entity binding or no owner = denied
                }
            }
        }
    }
}

/// Axum `FromRequestParts` implementation for `CallerIdentity`.
///
/// Extracts the identity that the auth middleware injected into extensions.
/// Returns 401 if no identity is present (middleware didn't run or failed).
#[axum::async_trait]
impl<S: Send + Sync> axum::extract::FromRequestParts<S> for CallerIdentity {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<CallerIdentity>()
            .cloned()
            .ok_or_else(|| AppError::Unauthorized("no caller identity in request context".into()))
    }
}

/// Check that the caller has at least the required role.
/// Returns 403 Forbidden if the caller's role is insufficient.
pub fn require_role(caller: &CallerIdentity, minimum: Role) -> Result<(), AppError> {
    if caller.has_role(minimum) {
        Ok(())
    } else {
        Err(AppError::Forbidden(format!(
            "role '{}' required, caller has '{}'",
            minimum.as_str(),
            caller.role.as_str()
        )))
    }
}

// ── Auth Configuration ──────────────────────────────────────────────────────

/// Auth configuration injected into request extensions.
///
/// Custom `Debug` redacts the token value to prevent credential leakage in logs.
#[derive(Clone)]
pub struct AuthConfig {
    pub token: Option<String>,
}

impl std::fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthConfig")
            .field("token", &self.token.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}

// ── Token Validation ────────────────────────────────────────────────────────

/// Constant-time comparison of bearer tokens.
///
/// Prevents timing side-channels that could reveal token length or prefix.
/// When lengths differ, performs a dummy comparison to avoid leaking length
/// information through timing variance.
fn constant_time_token_eq(provided: &str, expected: &str) -> bool {
    let provided = provided.as_bytes();
    let expected = expected.as_bytes();
    if provided.len() != expected.len() {
        // Dummy comparison to keep timing constant regardless of length match.
        let _ = expected.ct_eq(expected);
        return false;
    }
    provided.ct_eq(expected).into()
}

/// Parse the bearer token in format `{role}:{entity_id}:{secret}` or `{secret}` (legacy).
///
/// Legacy tokens (without role prefix) are treated as `ZoneAdmin` for backward
/// compatibility with existing deployments.
pub fn parse_bearer_token(provided: &str, expected_secret: &str) -> Result<CallerIdentity, String> {
    let parts: Vec<&str> = provided.splitn(3, ':').collect();

    match parts.len() {
        // Legacy format: just the secret. Treated as zone_admin for backward compat.
        1 => {
            if constant_time_token_eq(provided, expected_secret) {
                Ok(CallerIdentity {
                    role: Role::ZoneAdmin,
                    entity_id: None,
                    jurisdiction_id: None,
                })
            } else {
                Err("invalid bearer token".into())
            }
        }
        // New format: role:entity_id:secret (entity_id may be empty)
        3 => {
            let role_str = parts[0];
            let entity_str = parts[1];
            let secret = parts[2];

            if !constant_time_token_eq(secret, expected_secret) {
                return Err("invalid bearer token".into());
            }

            let role = match role_str {
                "zone_admin" => Role::ZoneAdmin,
                "regulator" => Role::Regulator,
                "entity_operator" => Role::EntityOperator,
                other => return Err(format!("unknown role: {other}")),
            };

            let entity_id = if entity_str.is_empty() {
                None
            } else {
                Some(
                    entity_str
                        .parse::<Uuid>()
                        .map_err(|e| format!("invalid entity_id: {e}"))?,
                )
            };

            Ok(CallerIdentity {
                role,
                entity_id,
                jurisdiction_id: None,
            })
        }
        _ => Err("invalid token format — expected {role}:{entity_id}:{secret} or {secret}".into()),
    }
}

// ── Middleware ───────────────────────────────────────────────────────────────

/// Extract and validate the Bearer token from the Authorization header.
///
/// Parses the token to extract `CallerIdentity` (role + entity binding) and
/// injects it into request extensions for downstream handlers.
///
/// When `AuthConfig.token` is `None`, all requests are allowed with `ZoneAdmin`
/// identity (auth disabled / development mode).
pub async fn auth_middleware(mut request: Request, next: Next) -> Response {
    let expected_token = request.extensions().get::<AuthConfig>().cloned();

    match expected_token {
        Some(AuthConfig {
            token: Some(ref expected),
        }) => {
            let auth_header = request
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok());

            match auth_header {
                Some(header_value) if header_value.starts_with("Bearer ") => {
                    let provided = &header_value[7..];
                    match parse_bearer_token(provided, expected) {
                        Ok(identity) => {
                            request.extensions_mut().insert(identity);
                            next.run(request).await
                        }
                        Err(msg) => {
                            tracing::warn!(reason = %msg, "authentication failed: invalid bearer token");
                            unauthorized_response(&msg)
                        }
                    }
                }
                Some(_) => {
                    tracing::warn!("authentication failed: non-Bearer authorization scheme");
                    unauthorized_response("authorization header must use Bearer scheme")
                }
                None => {
                    tracing::warn!("authentication failed: missing authorization header");
                    unauthorized_response("missing authorization header")
                }
            }
        }
        _ => {
            // Auth disabled — inject ZoneAdmin identity for full access.
            request.extensions_mut().insert(CallerIdentity {
                role: Role::ZoneAdmin,
                entity_id: None,
                jurisdiction_id: None,
            });
            next.run(request).await
        }
    }
}

fn unauthorized_response(message: &str) -> Response {
    let body = ErrorBody {
        error: ErrorDetail {
            code: "UNAUTHORIZED".to_string(),
            message: message.to_string(),
            details: None,
        },
    };
    (StatusCode::UNAUTHORIZED, Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{SmartAssetRecord, SmartAssetType};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::middleware::from_fn;
    use axum::routing::get;
    use axum::Router;
    use chrono::Utc;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    /// Build a minimal router with the auth middleware and a simple handler.
    fn test_app(token: Option<String>) -> Router {
        let auth_config = AuthConfig { token };
        Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(from_fn(auth_middleware))
            .layer(axum::Extension(auth_config))
    }

    // ── Existing auth middleware tests ────────────────────────────

    #[tokio::test]
    async fn valid_bearer_token_accepted() {
        let app = test_app(Some("my-secret".to_string()));

        let request = Request::builder()
            .uri("/test")
            .header("Authorization", "Bearer my-secret")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"ok");
    }

    #[tokio::test]
    async fn missing_authorization_header_rejected() {
        let app = test_app(Some("my-secret".to_string()));

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let err: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(err["error"]["code"], "UNAUTHORIZED");
        assert!(err["error"]["message"]
            .as_str()
            .unwrap()
            .contains("missing"));
    }

    #[tokio::test]
    async fn invalid_token_rejected() {
        let app = test_app(Some("my-secret".to_string()));

        let request = Request::builder()
            .uri("/test")
            .header("Authorization", "Bearer wrong-token")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let err: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(err["error"]["code"], "UNAUTHORIZED");
        assert!(err["error"]["message"]
            .as_str()
            .unwrap()
            .contains("invalid"));
    }

    #[tokio::test]
    async fn non_bearer_scheme_rejected() {
        let app = test_app(Some("my-secret".to_string()));

        let request = Request::builder()
            .uri("/test")
            .header("Authorization", "Basic dXNlcjpwYXNz")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let err: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(err["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Bearer scheme"));
    }

    #[tokio::test]
    async fn auth_disabled_allows_all_requests() {
        let app = test_app(None);

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"ok");
    }

    #[tokio::test]
    async fn auth_disabled_ignores_provided_token() {
        let app = test_app(None);

        let request = Request::builder()
            .uri("/test")
            .header("Authorization", "Bearer anything")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn constant_time_eq_identical_tokens() {
        assert!(constant_time_token_eq(
            "secret-token-123",
            "secret-token-123"
        ));
    }

    #[test]
    fn constant_time_eq_rejects_wrong_token() {
        assert!(!constant_time_token_eq("wrong-token", "secret-token-123"));
    }

    #[test]
    fn constant_time_eq_rejects_prefix() {
        assert!(!constant_time_token_eq("secret", "secret-token-123"));
    }

    #[test]
    fn constant_time_eq_rejects_empty() {
        assert!(!constant_time_token_eq("", "secret-token-123"));
    }

    // ── Role tests ───────────────────────────────────────────────

    #[test]
    fn role_ordering_is_correct() {
        assert!(Role::EntityOperator < Role::Regulator);
        assert!(Role::Regulator < Role::ZoneAdmin);
    }

    #[test]
    fn role_as_str() {
        assert_eq!(Role::EntityOperator.as_str(), "entity_operator");
        assert_eq!(Role::Regulator.as_str(), "regulator");
        assert_eq!(Role::ZoneAdmin.as_str(), "zone_admin");
    }

    // ── CallerIdentity tests ─────────────────────────────────────

    #[test]
    fn has_role_zone_admin_has_everything() {
        let admin = CallerIdentity {
            role: Role::ZoneAdmin,
            entity_id: None,
            jurisdiction_id: None,
        };
        assert!(admin.has_role(Role::EntityOperator));
        assert!(admin.has_role(Role::Regulator));
        assert!(admin.has_role(Role::ZoneAdmin));
    }

    #[test]
    fn has_role_regulator_has_own_and_below() {
        let regulator = CallerIdentity {
            role: Role::Regulator,
            entity_id: None,
            jurisdiction_id: None,
        };
        assert!(regulator.has_role(Role::EntityOperator));
        assert!(regulator.has_role(Role::Regulator));
        assert!(!regulator.has_role(Role::ZoneAdmin));
    }

    #[test]
    fn has_role_entity_operator_only_has_own_level() {
        let entity = CallerIdentity {
            role: Role::EntityOperator,
            entity_id: Some(Uuid::new_v4()),
            jurisdiction_id: None,
        };
        assert!(entity.has_role(Role::EntityOperator));
        assert!(!entity.has_role(Role::Regulator));
        assert!(!entity.has_role(Role::ZoneAdmin));
    }

    #[test]
    fn can_access_asset_zone_admin_any_asset() {
        let caller = CallerIdentity {
            role: Role::ZoneAdmin,
            entity_id: None,
            jurisdiction_id: None,
        };
        let asset = SmartAssetRecord {
            id: Uuid::new_v4(),
            asset_type: SmartAssetType::new("bond").expect("valid"),
            jurisdiction_id: "PK-PSEZ".to_string(),
            status: crate::state::AssetStatus::Genesis,
            genesis_digest: None,
            compliance_status: crate::state::AssetComplianceStatus::Unevaluated,
            metadata: serde_json::json!({}),
            owner_entity_id: Some(Uuid::new_v4()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(caller.can_access_asset(&asset));
    }

    #[test]
    fn can_access_asset_regulator_any_asset() {
        let caller = CallerIdentity {
            role: Role::Regulator,
            entity_id: None,
            jurisdiction_id: None,
        };
        let asset = SmartAssetRecord {
            id: Uuid::new_v4(),
            asset_type: SmartAssetType::new("equity").expect("valid"),
            jurisdiction_id: "AE-DIFC".to_string(),
            status: crate::state::AssetStatus::Genesis,
            genesis_digest: None,
            compliance_status: crate::state::AssetComplianceStatus::Unevaluated,
            metadata: serde_json::json!({}),
            owner_entity_id: Some(Uuid::new_v4()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(caller.can_access_asset(&asset));
    }

    #[test]
    fn can_access_asset_entity_operator_own_asset() {
        let entity_id = Uuid::new_v4();
        let caller = CallerIdentity {
            role: Role::EntityOperator,
            entity_id: Some(entity_id),
            jurisdiction_id: None,
        };
        let asset = SmartAssetRecord {
            id: Uuid::new_v4(),
            asset_type: SmartAssetType::new("bond").expect("valid"),
            jurisdiction_id: "PK-PSEZ".to_string(),
            status: crate::state::AssetStatus::Genesis,
            genesis_digest: None,
            compliance_status: crate::state::AssetComplianceStatus::Unevaluated,
            metadata: serde_json::json!({}),
            owner_entity_id: Some(entity_id),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(caller.can_access_asset(&asset));
    }

    #[test]
    fn can_access_asset_entity_operator_other_asset() {
        let caller = CallerIdentity {
            role: Role::EntityOperator,
            entity_id: Some(Uuid::new_v4()),
            jurisdiction_id: None,
        };
        let asset = SmartAssetRecord {
            id: Uuid::new_v4(),
            asset_type: SmartAssetType::new("bond").expect("valid"),
            jurisdiction_id: "PK-PSEZ".to_string(),
            status: crate::state::AssetStatus::Genesis,
            genesis_digest: None,
            compliance_status: crate::state::AssetComplianceStatus::Unevaluated,
            metadata: serde_json::json!({}),
            owner_entity_id: Some(Uuid::new_v4()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(!caller.can_access_asset(&asset));
    }

    #[test]
    fn can_access_asset_entity_operator_no_entity_id_denied() {
        let caller = CallerIdentity {
            role: Role::EntityOperator,
            entity_id: None,
            jurisdiction_id: None,
        };
        let asset = SmartAssetRecord {
            id: Uuid::new_v4(),
            asset_type: SmartAssetType::new("bond").expect("valid"),
            jurisdiction_id: "PK-PSEZ".to_string(),
            status: crate::state::AssetStatus::Genesis,
            genesis_digest: None,
            compliance_status: crate::state::AssetComplianceStatus::Unevaluated,
            metadata: serde_json::json!({}),
            owner_entity_id: Some(Uuid::new_v4()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(!caller.can_access_asset(&asset));
    }

    // ── require_role tests ───────────────────────────────────────

    #[test]
    fn require_role_passes_for_sufficient_role() {
        let caller = CallerIdentity {
            role: Role::ZoneAdmin,
            entity_id: None,
            jurisdiction_id: None,
        };
        assert!(require_role(&caller, Role::Regulator).is_ok());
    }

    #[test]
    fn require_role_fails_for_insufficient_role() {
        let caller = CallerIdentity {
            role: Role::EntityOperator,
            entity_id: Some(Uuid::new_v4()),
            jurisdiction_id: None,
        };
        assert!(require_role(&caller, Role::Regulator).is_err());
    }

    // ── parse_bearer_token tests ─────────────────────────────────

    #[test]
    fn parse_bearer_token_legacy_format() {
        let identity = parse_bearer_token("my-secret", "my-secret").unwrap();
        assert_eq!(identity.role, Role::ZoneAdmin);
        assert!(identity.entity_id.is_none());
    }

    #[test]
    fn parse_bearer_token_new_format_zone_admin() {
        let identity = parse_bearer_token("zone_admin::my-secret", "my-secret").unwrap();
        assert_eq!(identity.role, Role::ZoneAdmin);
        assert!(identity.entity_id.is_none());
    }

    #[test]
    fn parse_bearer_token_new_format_regulator() {
        let identity = parse_bearer_token("regulator::my-secret", "my-secret").unwrap();
        assert_eq!(identity.role, Role::Regulator);
        assert!(identity.entity_id.is_none());
    }

    #[test]
    fn parse_bearer_token_new_format_entity_operator() {
        let identity = parse_bearer_token(
            "entity_operator:550e8400-e29b-41d4-a716-446655440000:my-secret",
            "my-secret",
        )
        .unwrap();
        assert_eq!(identity.role, Role::EntityOperator);
        assert_eq!(
            identity.entity_id.unwrap().to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn parse_bearer_token_wrong_secret() {
        let result = parse_bearer_token("zone_admin::wrong", "my-secret");
        assert!(result.is_err());
    }

    #[test]
    fn parse_bearer_token_unknown_role() {
        let result = parse_bearer_token("superadmin::my-secret", "my-secret");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown role"));
    }

    #[test]
    fn parse_bearer_token_invalid_uuid() {
        let result = parse_bearer_token("entity_operator:not-a-uuid:my-secret", "my-secret");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid entity_id"));
    }

    #[test]
    fn parse_bearer_token_two_parts_rejected() {
        // "role:secret" has exactly 2 parts — treated as legacy (1 part with colon).
        // Actually splitn(3, ':') with "a:b" gives ["a", "b"] = 2 parts, which
        // should be rejected.
        let result = parse_bearer_token("role:secret", "secret");
        assert!(result.is_err());
    }

    // ── Middleware with new token format ──────────────────────────

    #[tokio::test]
    async fn middleware_new_format_zone_admin_accepted() {
        let app = test_app(Some("my-secret".to_string()));

        let request = Request::builder()
            .uri("/test")
            .header("Authorization", "Bearer zone_admin::my-secret")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn middleware_new_format_entity_operator_accepted() {
        let app = test_app(Some("my-secret".to_string()));

        let request = Request::builder()
            .uri("/test")
            .header(
                "Authorization",
                "Bearer entity_operator:550e8400-e29b-41d4-a716-446655440000:my-secret",
            )
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn middleware_unknown_role_rejected() {
        let app = test_app(Some("my-secret".to_string()));

        let request = Request::builder()
            .uri("/test")
            .header("Authorization", "Bearer superadmin::my-secret")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn middleware_invalid_uuid_rejected() {
        let app = test_app(Some("my-secret".to_string()));

        let request = Request::builder()
            .uri("/test")
            .header(
                "Authorization",
                "Bearer entity_operator:not-a-uuid:my-secret",
            )
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
