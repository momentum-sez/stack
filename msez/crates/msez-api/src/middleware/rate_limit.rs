//! # Per-Jurisdiction Rate Limiting
//!
//! Simple token-bucket rate limiter keyed by jurisdiction ID.
//! Phase 1: in-memory. Phase 2: Redis-backed distributed rate limiting.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
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

    /// Maximum number of unique keys before triggering a prune sweep.
    /// Prevents unbounded memory growth from unique X-Jurisdiction-Id headers.
    const MAX_BUCKETS: usize = 10_000;

    /// Check if a request from the given key should be allowed.
    fn check(&self, key: &str) -> bool {
        let mut buckets = self.buckets.write();
        let now = Instant::now();

        // Prune stale entries when the map exceeds the size threshold
        // to prevent unbounded memory growth (DoS vector).
        if buckets.len() >= Self::MAX_BUCKETS {
            let window = self.config.window_secs.max(1);
            buckets.retain(|_, bucket| now.duration_since(bucket.window_start).as_secs() < window);
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limiter_new_creates_limiter() {
        let config = RateLimitConfig {
            max_requests: 10,
            window_secs: 60,
        };
        let limiter = RateLimiter::new(config);
        // Freshly created limiter should allow requests.
        assert!(limiter.check("test-key"));
    }

    #[test]
    fn check_under_limit_returns_true() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 5,
            window_secs: 60,
        });

        for i in 0..5 {
            assert!(
                limiter.check("client-a"),
                "request {i} should be allowed (under limit)"
            );
        }
    }

    #[test]
    fn check_over_limit_returns_false() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 3,
            window_secs: 60,
        });

        // Exhaust the budget.
        assert!(limiter.check("client-a"));
        assert!(limiter.check("client-a"));
        assert!(limiter.check("client-a"));

        // Next request should be rejected.
        assert!(
            !limiter.check("client-a"),
            "request beyond limit should be rejected"
        );
        assert!(
            !limiter.check("client-a"),
            "subsequent requests should also be rejected"
        );
    }

    #[test]
    fn different_keys_have_independent_buckets() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 2,
            window_secs: 60,
        });

        assert!(limiter.check("client-a"));
        assert!(limiter.check("client-a"));
        assert!(!limiter.check("client-a"), "client-a should be exhausted");

        // client-b should still have its full budget.
        assert!(limiter.check("client-b"));
        assert!(limiter.check("client-b"));
        assert!(!limiter.check("client-b"), "client-b should be exhausted");
    }

    #[test]
    fn window_reset_allows_new_requests() {
        // Use a 0-second window so it resets immediately on the next check.
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 1,
            window_secs: 0,
        });

        assert!(limiter.check("client-a"));
        // With window_secs=0, the elapsed time (even if 0ns) will NOT be >= 0 secs
        // unless real time passes. But Instant math: duration_since returns Duration,
        // and as_secs() truncates sub-second. A 0-second window means that once any
        // full second elapses the bucket resets. We can force this by sleeping briefly.
        // However, the implementation checks `>= window_secs` where window_secs is 0.
        // Since duration_since for the same Instant is 0, and 0 >= 0 is true, the
        // bucket WILL reset on every call.
        assert!(
            limiter.check("client-a"),
            "zero-second window should reset on every check"
        );
    }

    #[test]
    fn default_config_values() {
        let config = RateLimitConfig::default();
        assert_eq!(config.max_requests, 1000);
        assert_eq!(config.window_secs, 60);
    }

    #[test]
    fn clone_shares_underlying_state() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 2,
            window_secs: 60,
        });
        let clone = limiter.clone();

        // Consume one request through the original.
        assert!(limiter.check("client-a"));
        // The clone sees the same bucket state.
        assert!(clone.check("client-a"));
        // Both should now be exhausted.
        assert!(!limiter.check("client-a"));
        assert!(!clone.check("client-a"));
    }

    // ── Additional coverage for rate_limit_middleware ─────────────

    #[test]
    fn check_exactly_at_limit_rejects() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 1,
            window_secs: 60,
        });

        assert!(limiter.check("k")); // count becomes 1, which equals max_requests
        assert!(!limiter.check("k")); // count 1 >= 1, rejected
    }

    #[test]
    fn multiple_keys_interleaved() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 2,
            window_secs: 60,
        });

        assert!(limiter.check("a"));
        assert!(limiter.check("b"));
        assert!(limiter.check("a")); // a: 2 out of 2
        assert!(limiter.check("b")); // b: 2 out of 2
        assert!(!limiter.check("a")); // a exhausted
        assert!(!limiter.check("b")); // b exhausted
        assert!(limiter.check("c")); // c still fresh
    }

    #[test]
    fn empty_key_is_valid() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 1,
            window_secs: 60,
        });

        assert!(limiter.check(""));
        assert!(!limiter.check(""));
    }

    #[test]
    fn rate_limit_config_custom_values() {
        let config = RateLimitConfig {
            max_requests: 500,
            window_secs: 120,
        };
        assert_eq!(config.max_requests, 500);
        assert_eq!(config.window_secs, 120);
    }

    #[test]
    fn rate_limiter_new_with_default_config() {
        let limiter = RateLimiter::new(RateLimitConfig::default());
        // Default allows 1000 requests; should easily accept a few.
        for _ in 0..10 {
            assert!(limiter.check("test"));
        }
    }
}
