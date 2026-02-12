//! # Middleware Stack
//!
//! Tower middleware for the API layer:
//! - [`tracing_layer`]: request/response tracing with `TraceLayer`.
//! - [`metrics`]: Prometheus-compatible request metrics.
//! - [`rate_limit`]: per-jurisdiction rate limiting.

pub mod metrics;
pub mod rate_limit;
pub mod tracing_layer;
