//! Mass API client configuration.
//!
//! Configures base URLs for each Mass API service. Defaults point to
//! production endpoints. Override via environment variables or explicit
//! construction for staging/testing.

use url::Url;

/// Configuration for connecting to Mass API services.
///
/// Custom `Debug` implementation redacts the `api_token` field
/// to prevent credential leakage in log output.
#[derive(Clone)]
pub struct MassApiConfig {
    /// Base URL for organization-info (ENTITIES primitive).
    /// Default: <https://organization-info.api.mass.inc>
    pub organization_info_url: Url,
    /// Base URL for investment-info (OWNERSHIP primitive).
    pub investment_info_url: Url,
    /// Base URL for treasury-info (FISCAL primitive).
    pub treasury_info_url: Url,
    /// Base URL for consent-info (CONSENT primitive).
    pub consent_info_url: Url,
    /// Base URL for templating-engine.
    pub templating_engine_url: Url,
    /// Bearer token for API authentication.
    pub api_token: String,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
}

impl std::fmt::Debug for MassApiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MassApiConfig")
            .field("organization_info_url", &self.organization_info_url)
            .field("investment_info_url", &self.investment_info_url)
            .field("treasury_info_url", &self.treasury_info_url)
            .field("consent_info_url", &self.consent_info_url)
            .field("templating_engine_url", &self.templating_engine_url)
            .field("api_token", &"[REDACTED]")
            .field("timeout_secs", &self.timeout_secs)
            .finish()
    }
}

impl MassApiConfig {
    /// Load configuration from environment variables.
    ///
    /// Variables:
    /// - `MASS_ORG_INFO_URL` (default: `https://organization-info.api.mass.inc`)
    /// - `MASS_INVESTMENT_INFO_URL` (default: `https://investment-info-production-4f3779c81425.herokuapp.com`)
    /// - `MASS_TREASURY_INFO_URL` (default: `https://treasury-info.api.mass.inc`)
    /// - `MASS_CONSENT_INFO_URL` (default: `https://consent.api.mass.inc`)
    /// - `MASS_TEMPLATING_URL` (default: `https://templating-engine-prod-5edc768c1f80.herokuapp.com`)
    /// - `MASS_API_TOKEN` (required)
    /// - `MASS_TIMEOUT_SECS` (default: 30)
    pub fn from_env() -> Result<Self, ConfigError> {
        let api_token = std::env::var("MASS_API_TOKEN").map_err(|_| ConfigError::MissingToken)?;

        Ok(Self {
            organization_info_url: env_url(
                "MASS_ORG_INFO_URL",
                "https://organization-info.api.mass.inc",
            )?,
            investment_info_url: env_url(
                "MASS_INVESTMENT_INFO_URL",
                "https://investment-info-production-4f3779c81425.herokuapp.com",
            )?,
            treasury_info_url: env_url(
                "MASS_TREASURY_INFO_URL",
                "https://treasury-info.api.mass.inc",
            )?,
            consent_info_url: env_url("MASS_CONSENT_INFO_URL", "https://consent.api.mass.inc")?,
            templating_engine_url: env_url(
                "MASS_TEMPLATING_URL",
                "https://templating-engine-prod-5edc768c1f80.herokuapp.com",
            )?,
            api_token,
            timeout_secs: std::env::var("MASS_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
        })
    }

    /// Create a configuration pointing to local mock servers (for testing).
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::InvalidUrl` if the localhost URL cannot be parsed
    /// (should not occur for valid port numbers, but avoids `expect()`).
    pub fn local_mock(base_port: u16, token: &str) -> Result<Self, ConfigError> {
        let make_url = |port: u16| -> Result<Url, ConfigError> {
            Url::parse(&format!("http://127.0.0.1:{port}"))
                .map_err(|e| ConfigError::InvalidUrl("localhost".to_string(), e.to_string()))
        };
        Ok(Self {
            organization_info_url: make_url(base_port)?,
            investment_info_url: make_url(base_port + 1)?,
            treasury_info_url: make_url(base_port + 2)?,
            consent_info_url: make_url(base_port + 3)?,
            templating_engine_url: make_url(base_port + 4)?,
            api_token: token.to_string(),
            timeout_secs: 5,
        })
    }
}

fn env_url(var: &str, default: &str) -> Result<Url, ConfigError> {
    let raw = std::env::var(var).unwrap_or_else(|_| default.to_string());
    Url::parse(&raw).map_err(|e| ConfigError::InvalidUrl(var.to_string(), e.to_string()))
}

/// Configuration errors.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("MASS_API_TOKEN environment variable is required")]
    MissingToken,
    #[error("invalid URL for {0}: {1}")]
    InvalidUrl(String, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_mock_builds_valid_config() {
        let cfg = MassApiConfig::local_mock(9000, "test-token").unwrap();
        assert_eq!(cfg.api_token, "test-token");
        assert_eq!(cfg.timeout_secs, 5);
        assert_eq!(cfg.organization_info_url.as_str(), "http://127.0.0.1:9000/");
        assert_eq!(cfg.investment_info_url.as_str(), "http://127.0.0.1:9001/");
    }

    #[test]
    fn env_url_uses_default_when_var_absent() {
        let url = env_url("NONEXISTENT_VAR_12345", "https://example.com").unwrap();
        assert_eq!(url.as_str(), "https://example.com/");
    }

    #[test]
    fn env_url_rejects_invalid_url() {
        // Temporarily set an invalid URL.
        std::env::set_var("TEST_BAD_URL_MC", "not a url");
        let result = env_url("TEST_BAD_URL_MC", "https://example.com");
        std::env::remove_var("TEST_BAD_URL_MC");
        assert!(result.is_err());
    }
}
