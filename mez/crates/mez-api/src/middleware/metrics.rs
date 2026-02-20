//! # Prometheus Metrics
//!
//! Real Prometheus metrics exporter using `prometheus` crate.
//! Replaces the Phase 1 atomic counters with a full Prometheus registry.
//!
//! HTTP-level metrics (request counts, latency, errors) are recorded in middleware.
//! Domain-level gauges (corridors, assets, attestations, peers, policies) are
//! updated on each `/metrics` scrape (pull model) — see the metrics handler in `lib.rs`.

use std::sync::Arc;
use std::time::Instant;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use prometheus::{
    Encoder, GaugeVec, HistogramVec, IntCounterVec, Opts, Registry, TextEncoder,
    core::Collector,
};

/// Shared metrics state backed by a Prometheus registry.
#[derive(Clone)]
pub struct ApiMetrics {
    inner: Arc<Inner>,
}

struct Inner {
    registry: Registry,

    // -- HTTP middleware metrics (push model) --
    http_requests_total: IntCounterVec,
    http_request_duration_seconds: HistogramVec,
    http_errors_total: IntCounterVec,

    // -- Domain gauges (pull model, updated on /metrics scrape) --
    corridors_total: GaugeVec,
    corridor_receipt_chain_height: GaugeVec,
    receipt_chain_total_receipts: prometheus::Gauge,
    assets_total: GaugeVec,
    attestations_total: prometheus::Gauge,
    peers_total: GaugeVec,
    policies_total: prometheus::Gauge,
    audit_trail_entries_total: prometheus::Gauge,
    zone_key_ephemeral: prometheus::Gauge,
}

impl std::fmt::Debug for ApiMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiMetrics")
            .field("requests", &self.requests())
            .field("errors", &self.errors())
            .finish()
    }
}

impl ApiMetrics {
    /// Create a new metrics instance with a fresh Prometheus registry.
    pub fn new() -> Self {
        let registry = Registry::new();

        let http_requests_total = IntCounterVec::new(
            Opts::new("mez_http_requests_total", "Total HTTP requests"),
            &["method", "path", "status"],
        )
        .expect("metric can be created");

        let http_request_duration_seconds = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "mez_http_request_duration_seconds",
                "HTTP request duration in seconds",
            )
            .buckets(vec![
                0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
            &["method", "path"],
        )
        .expect("metric can be created");

        let http_errors_total = IntCounterVec::new(
            Opts::new("mez_http_errors_total", "Total HTTP errors (4xx and 5xx)"),
            &["method", "path", "status"],
        )
        .expect("metric can be created");

        let corridors_total = GaugeVec::new(
            Opts::new("mez_corridors_total", "Total corridors by state"),
            &["state"],
        )
        .expect("metric can be created");

        let corridor_receipt_chain_height = GaugeVec::new(
            Opts::new(
                "mez_corridor_receipt_chain_height",
                "Receipt chain height per corridor",
            ),
            &["corridor_id"],
        )
        .expect("metric can be created");

        let receipt_chain_total_receipts = prometheus::Gauge::new(
            "mez_receipt_chain_total_receipts",
            "Total receipts across all corridors",
        )
        .expect("metric can be created");

        let assets_total = GaugeVec::new(
            Opts::new("mez_assets_total", "Total assets by compliance status"),
            &["compliance_status"],
        )
        .expect("metric can be created");

        let attestations_total = prometheus::Gauge::new(
            "mez_attestations_total",
            "Total attestation records",
        )
        .expect("metric can be created");

        let peers_total = GaugeVec::new(
            Opts::new("mez_peers_total", "Total peers by status"),
            &["status"],
        )
        .expect("metric can be created");

        let policies_total = prometheus::Gauge::new(
            "mez_policies_total",
            "Total registered policies",
        )
        .expect("metric can be created");

        let audit_trail_entries_total = prometheus::Gauge::new(
            "mez_audit_trail_entries_total",
            "Total audit trail entries",
        )
        .expect("metric can be created");

        let zone_key_ephemeral = prometheus::Gauge::new(
            "mez_zone_key_ephemeral",
            "Whether zone signing key is ephemeral (1=ephemeral, 0=real)",
        )
        .expect("metric can be created");

        // Register all metrics.
        registry
            .register(Box::new(http_requests_total.clone()))
            .expect("metric can be registered");
        registry
            .register(Box::new(http_request_duration_seconds.clone()))
            .expect("metric can be registered");
        registry
            .register(Box::new(http_errors_total.clone()))
            .expect("metric can be registered");
        registry
            .register(Box::new(corridors_total.clone()))
            .expect("metric can be registered");
        registry
            .register(Box::new(corridor_receipt_chain_height.clone()))
            .expect("metric can be registered");
        registry
            .register(Box::new(receipt_chain_total_receipts.clone()))
            .expect("metric can be registered");
        registry
            .register(Box::new(assets_total.clone()))
            .expect("metric can be registered");
        registry
            .register(Box::new(attestations_total.clone()))
            .expect("metric can be registered");
        registry
            .register(Box::new(peers_total.clone()))
            .expect("metric can be registered");
        registry
            .register(Box::new(policies_total.clone()))
            .expect("metric can be registered");
        registry
            .register(Box::new(audit_trail_entries_total.clone()))
            .expect("metric can be registered");
        registry
            .register(Box::new(zone_key_ephemeral.clone()))
            .expect("metric can be registered");

        Self {
            inner: Arc::new(Inner {
                registry,
                http_requests_total,
                http_request_duration_seconds,
                http_errors_total,
                corridors_total,
                corridor_receipt_chain_height,
                receipt_chain_total_receipts,
                assets_total,
                attestations_total,
                peers_total,
                policies_total,
                audit_trail_entries_total,
                zone_key_ephemeral,
            }),
        }
    }

    /// Return current total request count (sum across all labels).
    ///
    /// Backward-compatible accessor used by the regulator dashboard handler.
    pub fn requests(&self) -> u64 {
        let mut total = 0u64;
        let families = self.inner.http_requests_total.collect();
        for mf in &families {
            for m in mf.get_metric() {
                total += m.get_counter().get_value() as u64;
            }
        }
        total
    }

    /// Return current total error count (sum across all labels).
    ///
    /// Backward-compatible accessor used by the regulator dashboard handler.
    pub fn errors(&self) -> u64 {
        let mut total = 0u64;
        let families = self.inner.http_errors_total.collect();
        for mf in &families {
            for m in mf.get_metric() {
                total += m.get_counter().get_value() as u64;
            }
        }
        total
    }

    /// Record an HTTP request (called by the middleware).
    fn record_request(&self, method: &str, path: &str, status: u16, duration_secs: f64) {
        let status_str = status.to_string();
        self.inner
            .http_requests_total
            .with_label_values(&[method, path, &status_str])
            .inc();

        self.inner
            .http_request_duration_seconds
            .with_label_values(&[method, path])
            .observe(duration_secs);

        if status >= 400 {
            self.inner
                .http_errors_total
                .with_label_values(&[method, path, &status_str])
                .inc();
        }
    }

    // -- Domain gauge accessors (used by the /metrics handler) --

    /// Access the corridors gauge for updating.
    pub fn corridors_total(&self) -> &GaugeVec {
        &self.inner.corridors_total
    }

    /// Access the corridor receipt chain height gauge for updating.
    pub fn corridor_receipt_chain_height(&self) -> &GaugeVec {
        &self.inner.corridor_receipt_chain_height
    }

    /// Access the total receipts gauge for updating.
    pub fn receipt_chain_total_receipts(&self) -> &prometheus::Gauge {
        &self.inner.receipt_chain_total_receipts
    }

    /// Access the assets gauge for updating.
    pub fn assets_total(&self) -> &GaugeVec {
        &self.inner.assets_total
    }

    /// Access the attestations gauge for updating.
    pub fn attestations_total(&self) -> &prometheus::Gauge {
        &self.inner.attestations_total
    }

    /// Access the peers gauge for updating.
    pub fn peers_total(&self) -> &GaugeVec {
        &self.inner.peers_total
    }

    /// Access the policies gauge for updating.
    pub fn policies_total(&self) -> &prometheus::Gauge {
        &self.inner.policies_total
    }

    /// Access the audit trail entries gauge for updating.
    pub fn audit_trail_entries_total(&self) -> &prometheus::Gauge {
        &self.inner.audit_trail_entries_total
    }

    /// Access the zone key ephemeral gauge for updating.
    pub fn zone_key_ephemeral(&self) -> &prometheus::Gauge {
        &self.inner.zone_key_ephemeral
    }

    /// Gather all metrics and encode to Prometheus text format.
    pub fn gather_and_encode(&self) -> Result<String, String> {
        let encoder = TextEncoder::new();
        let metric_families = self.inner.registry.gather();
        let mut buffer = Vec::new();
        encoder
            .encode(&metric_families, &mut buffer)
            .map_err(|e| format!("failed to encode metrics: {e}"))?;
        String::from_utf8(buffer).map_err(|e| format!("metrics encoding produced invalid UTF-8: {e}"))
    }
}

impl Default for ApiMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Normalize a request path by replacing UUID segments with `{id}`.
///
/// Prevents cardinality explosion in Prometheus labels. UUIDs are detected
/// as 32-hex-char strings with optional hyphens (standard UUID format).
fn normalize_path(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            // Match standard UUID: 8-4-4-4-12 hex chars with hyphens
            if segment.len() == 36
                && segment.chars().enumerate().all(|(i, c)| {
                    if i == 8 || i == 13 || i == 18 || i == 23 {
                        c == '-'
                    } else {
                        c.is_ascii_hexdigit()
                    }
                })
            {
                "{id}"
            } else if segment.len() == 32 && segment.chars().all(|c| c.is_ascii_hexdigit()) {
                // UUID without hyphens
                "{id}"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Middleware that records HTTP request metrics via Prometheus.
pub async fn metrics_middleware(request: Request, next: Next) -> Response {
    let metrics = request.extensions().get::<ApiMetrics>().cloned();
    let method = request.method().to_string();
    let path = normalize_path(request.uri().path());
    let start = Instant::now();

    let response = next.run(request).await;

    if let Some(m) = metrics {
        let duration = start.elapsed().as_secs_f64();
        let status = response.status().as_u16();
        m.record_request(&method, &path, status, duration);
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

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
        m.record_request("GET", "/test", 200, 0.01);
        assert_eq!(m.requests(), 1);
        m.record_request("POST", "/test", 201, 0.02);
        m.record_request("GET", "/other", 200, 0.005);
        assert_eq!(m.requests(), 3);
    }

    #[test]
    fn errors_increments() {
        let m = ApiMetrics::new();
        m.record_request("GET", "/test", 500, 0.1);
        assert_eq!(m.errors(), 1);
        m.record_request("GET", "/test", 404, 0.05);
        assert_eq!(m.errors(), 2);
    }

    #[test]
    fn request_and_error_counts_independent() {
        let m = ApiMetrics::new();
        // 5 successful requests
        for _ in 0..5 {
            m.record_request("GET", "/ok", 200, 0.01);
        }
        // 2 error requests
        m.record_request("GET", "/fail", 500, 0.1);
        m.record_request("POST", "/fail", 400, 0.05);
        assert_eq!(m.requests(), 7); // 5 + 2
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
                        m.record_request("GET", "/test", 200, 0.001);
                        m.record_request("GET", "/err", 500, 0.001);
                    }
                })
            })
            .collect();

        for t in threads {
            t.join().unwrap();
        }

        assert_eq!(m.requests(), 20_000); // 10 threads * 2000 requests each
        assert_eq!(m.errors(), 10_000); // 10 threads * 1000 errors each
    }

    #[test]
    fn clone_shares_underlying_counters() {
        let m = ApiMetrics::new();
        let clone = m.clone();

        m.record_request("GET", "/test", 200, 0.01);
        assert_eq!(clone.requests(), 1, "clone should see the same counter");

        clone.record_request("GET", "/err", 500, 0.01);
        assert_eq!(m.errors(), 1, "original should see clone's increment");
    }

    #[test]
    fn gather_and_encode_produces_text() {
        let m = ApiMetrics::new();
        m.record_request("GET", "/test", 200, 0.01);
        let output = m.gather_and_encode().unwrap();
        assert!(output.contains("mez_http_requests_total"));
        assert!(output.contains("mez_http_request_duration_seconds"));
    }

    #[test]
    fn normalize_path_replaces_uuid_with_hyphens() {
        let path = "/v1/corridors/550e8400-e29b-41d4-a716-446655440000/receipts";
        assert_eq!(normalize_path(path), "/v1/corridors/{id}/receipts");
    }

    #[test]
    fn normalize_path_replaces_uuid_without_hyphens() {
        let path = "/v1/assets/550e8400e29b41d4a716446655440000";
        assert_eq!(normalize_path(path), "/v1/assets/{id}");
    }

    #[test]
    fn normalize_path_preserves_non_uuid_segments() {
        let path = "/v1/regulator/summary";
        assert_eq!(normalize_path(path), "/v1/regulator/summary");
    }

    #[test]
    fn normalize_path_multiple_uuids() {
        let path = "/v1/assets/550e8400-e29b-41d4-a716-446655440000/credentials/660e8400-e29b-41d4-a716-446655440001";
        assert_eq!(
            normalize_path(path),
            "/v1/assets/{id}/credentials/{id}"
        );
    }

    #[test]
    fn domain_gauges_update() {
        let m = ApiMetrics::new();
        m.corridors_total().with_label_values(&["ACTIVE"]).set(3.0);
        m.corridors_total().with_label_values(&["HALTED"]).set(1.0);
        m.attestations_total().set(42.0);
        m.zone_key_ephemeral().set(1.0);

        let output = m.gather_and_encode().unwrap();
        assert!(output.contains("mez_corridors_total"));
        assert!(output.contains("mez_attestations_total"));
        assert!(output.contains("mez_zone_key_ephemeral"));
    }
}
