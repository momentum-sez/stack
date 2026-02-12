//! # Request/Response Tracing
//!
//! Configures `tower_http::trace::TraceLayer` for structured request
//! logging with tracing spans.

/// Build a `TraceLayer` configured for the SEZ API.
///
/// Each request gets a tracing span with method, URI, and status code.
pub fn layer() -> tower_http::trace::TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
> {
    tower_http::trace::TraceLayer::new_for_http()
}
