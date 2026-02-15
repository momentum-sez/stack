//! # Authentication Middleware
//!
//! JWT Bearer token middleware implemented as an Axum middleware function.
//!
//! ## Phase 1
//!
//! Accepts a configurable static token or disables auth entirely when
//! no token is configured. Structured so real JWT validation can be added
//! without changing route handlers.

use axum::extract::Request;
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use subtle::ConstantTimeEq;

use crate::error::{ErrorBody, ErrorDetail};

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

/// Extract and validate the Bearer token from the Authorization header.
///
/// When `AuthConfig.token` is `None`, all requests are allowed (auth disabled).
pub async fn auth_middleware(request: Request, next: Next) -> Response {
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
                    if constant_time_token_eq(provided, expected) {
                        next.run(request).await
                    } else {
                        unauthorized_response("invalid bearer token")
                    }
                }
                Some(_) => unauthorized_response("authorization header must use Bearer scheme"),
                None => unauthorized_response("missing authorization header"),
            }
        }
        _ => next.run(request).await,
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
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::middleware::from_fn;
    use axum::routing::get;
    use axum::Router;
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
        assert!(constant_time_token_eq("secret-token-123", "secret-token-123"));
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
}
