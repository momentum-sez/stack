//! Typed client for Mass templating-engine.
//!
//! Base URL: `templating-engine-prod-5edc768c1f80.herokuapp.com`

/// Client for the Mass templating-engine API.
#[derive(Debug, Clone)]
pub struct TemplatingClient {
    _http: reqwest::Client,
    _base_url: url::Url,
}

impl TemplatingClient {
    pub(crate) fn new(http: reqwest::Client, base_url: url::Url) -> Self {
        Self {
            _http: http,
            _base_url: base_url,
        }
    }
}
