//! # Prometheus Metrics
//!
//! Lightweight request metrics using atomic counters.
//! Phase 1: in-process counters. Phase 2: Prometheus exporter.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

/// Shared metrics state.
#[derive(Debug, Clone)]
pub struct ApiMetrics {
    pub request_count: Arc<AtomicU64>,
    pub error_count: Arc<AtomicU64>,
}

impl ApiMetrics {
    /// Create a new metrics instance.
    pub fn new() -> Self {
        Self {
            request_count: Arc::new(AtomicU64::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Return current request count.
    pub fn requests(&self) -> u64 {
        self.request_count.load(Ordering::Relaxed)
    }

    /// Return current error count.
    pub fn errors(&self) -> u64 {
        self.error_count.load(Ordering::Relaxed)
    }
}

impl Default for ApiMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Middleware that increments request and error counters.
pub async fn metrics_middleware(request: Request, next: Next) -> Response {
    let metrics = request.extensions().get::<ApiMetrics>().cloned();

    let response = next.run(request).await;

    if let Some(m) = metrics {
        m.request_count.fetch_add(1, Ordering::Relaxed);
        if response.status().is_server_error() || response.status().is_client_error() {
            m.error_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn api_metrics_new_starts_at_zero() {
        let m = ApiMetrics::new();
        assert_eq!(m.requests(), 0);
        assert_eq!(m.errors(), 0);
    }

    #[test]
    fn api_metrics_default_starts_at_zero() {
        let m = ApiMetrics::default();
        assert_eq!(m.requests(), 0);
        assert_eq!(m.errors(), 0);
    }

    #[test]
    fn requests_increments() {
        let m = ApiMetrics::new();
        m.request_count.fetch_add(1, Ordering::Relaxed);
        assert_eq!(m.requests(), 1);
        m.request_count.fetch_add(1, Ordering::Relaxed);
        m.request_count.fetch_add(1, Ordering::Relaxed);
        assert_eq!(m.requests(), 3);
    }

    #[test]
    fn errors_increments() {
        let m = ApiMetrics::new();
        m.error_count.fetch_add(1, Ordering::Relaxed);
        assert_eq!(m.errors(), 1);
        m.error_count.fetch_add(1, Ordering::Relaxed);
        assert_eq!(m.errors(), 2);
    }

    #[test]
    fn request_and_error_counts_independent() {
        let m = ApiMetrics::new();
        m.request_count.fetch_add(5, Ordering::Relaxed);
        m.error_count.fetch_add(2, Ordering::Relaxed);
        assert_eq!(m.requests(), 5);
        assert_eq!(m.errors(), 2);
    }

    #[test]
    fn concurrent_increments_are_safe() {
        let m = ApiMetrics::new();
        let threads: Vec<_> = (0..10)
            .map(|_| {
                let m = m.clone();
                std::thread::spawn(move || {
                    for _ in 0..1000 {
                        m.request_count.fetch_add(1, Ordering::Relaxed);
                        m.error_count.fetch_add(1, Ordering::Relaxed);
                    }
                })
            })
            .collect();

        for t in threads {
            t.join().unwrap();
        }

        assert_eq!(m.requests(), 10_000);
        assert_eq!(m.errors(), 10_000);
    }

    #[test]
    fn clone_shares_underlying_counters() {
        let m = ApiMetrics::new();
        let clone = m.clone();

        m.request_count.fetch_add(1, Ordering::Relaxed);
        assert_eq!(clone.requests(), 1, "clone should see the same counter");

        clone.error_count.fetch_add(1, Ordering::Relaxed);
        assert_eq!(m.errors(), 1, "original should see clone's increment");
    }
}
