//! Mass API client error types.

/// Errors from Mass API calls.
#[derive(Debug, thiserror::Error)]
pub enum MassApiError {
    /// HTTP transport error.
    #[error("HTTP error calling {endpoint}: {source}")]
    Http {
        endpoint: String,
        source: reqwest::Error,
    },
    /// Mass API returned a non-2xx status.
    #[error("Mass API {endpoint} returned {status}: {body}")]
    ApiError {
        endpoint: String,
        status: u16,
        body: String,
    },
    /// Response deserialization failed.
    #[error("failed to deserialize response from {endpoint}: {source}")]
    Deserialization {
        endpoint: String,
        source: reqwest::Error,
    },
    /// Configuration error.
    #[error("configuration error: {0}")]
    Config(#[from] super::config::ConfigError),
}
