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
