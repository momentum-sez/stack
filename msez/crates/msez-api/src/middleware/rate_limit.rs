//! # Per-Jurisdiction Rate Limiting
//!
//! Simple token-bucket rate limiter keyed by jurisdiction ID.
//! Phase 1: in-memory. Phase 2: Redis-backed distributed rate limiting.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::error::{ErrorBody, ErrorDetail};

/// Rate limiter configuration.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window.
    pub max_requests: u64,
    /// Window duration in seconds.
    pub window_secs: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 1000,
            window_secs: 60,
        }
    }
}

/// Per-key rate limit state.
#[derive(Debug, Clone)]
struct BucketState {
    count: u64,
    window_start: Instant,
}

/// Shared rate limiter state.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    config: RateLimitConfig,
    buckets: Arc<RwLock<HashMap<String, BucketState>>>,
}

impl RateLimiter {
    /// Create a new rate limiter with the given config.
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            buckets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a request from the given key should be allowed.
    fn check(&self, key: &str) -> bool {
        let mut buckets = self.buckets.write().expect("rate limit lock poisoned");
        let now = Instant::now();

        let bucket = buckets.entry(key.to_string()).or_insert(BucketState {
            count: 0,
            window_start: now,
        });

        if now.duration_since(bucket.window_start).as_secs() >= self.config.window_secs {
            bucket.count = 0;
            bucket.window_start = now;
        }

        if bucket.count >= self.config.max_requests {
            false
        } else {
            bucket.count += 1;
            true
        }
    }
}

/// Middleware that enforces per-client rate limits.
///
/// The rate limit key is extracted from the `X-Jurisdiction-Id` header.
/// If no header is present, the key defaults to `"anonymous"`.
pub async fn rate_limit_middleware(request: Request, next: Next) -> Response {
    let limiter = request.extensions().get::<RateLimiter>().cloned();

    if let Some(limiter) = limiter {
        let key = request
            .headers()
            .get("x-jurisdiction-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("anonymous")
            .to_string();

        if !limiter.check(&key) {
            let body = ErrorBody {
                error: ErrorDetail {
                    code: "RATE_LIMITED".to_string(),
                    message: "rate limit exceeded".to_string(),
                    details: None,
                },
            };
            return (StatusCode::TOO_MANY_REQUESTS, Json(body)).into_response();
        }
    }

    next.run(request).await
}
