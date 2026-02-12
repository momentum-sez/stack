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

use crate::error::{ErrorBody, ErrorDetail};

/// Auth configuration injected into request extensions.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub token: Option<String>,
}

/// Extract and validate the Bearer token from the Authorization header.
///
/// When `AuthConfig.token` is `None`, all requests are allowed (auth disabled).
pub async fn auth_middleware(request: Request, next: Next) -> Response {
    let expected_token = request.extensions().get::<AuthConfig>().cloned();

    match expected_token {
        Some(AuthConfig { token: Some(ref expected) }) => {
            let auth_header = request
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok());

            match auth_header {
                Some(header_value) if header_value.starts_with("Bearer ") => {
                    let provided = &header_value[7..];
                    if provided == expected.as_str() {
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
