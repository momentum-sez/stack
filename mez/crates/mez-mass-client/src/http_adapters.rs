//! # Real HTTP Adapter Clients for Pakistan National Systems
//!
//! Production-grade HTTP client implementations of the national system adapter
//! traits (FBR IRIS, NADRA, SBP Raast, SECP). These connect to the actual
//! government and central bank APIs in Pakistan's GovOS deployment.
//!
//! ## Architecture
//!
//! Each adapter wraps a `reqwest::Client` with the system-specific base URL,
//! authentication, and request/response mapping. All adapters are `Send + Sync`
//! and designed to be shared via `Arc` across async tasks.
//!
//! ## Error Handling
//!
//! HTTP errors are mapped to the domain-specific error types (FbrError,
//! NadraError, etc.) with diagnostic context including the endpoint URL,
//! HTTP status, and response body excerpt.
//!
//! ## Timeout & Retry
//!
//! Each adapter uses a per-request timeout (configurable, default 30s).
//! Retries are NOT built into the adapter — callers are responsible for
//! retry policy via the `mez_mass_client::retry` module.

use std::time::Duration;

// ─── FBR IRIS HTTP Client ───────────────────────────────────────────────

/// Configuration for the FBR IRIS HTTP adapter.
#[derive(Debug, Clone)]
pub struct FbrIrisConfig {
    /// Base URL of the FBR IRIS API (e.g., `https://iris.fbr.gov.pk/api/v1`).
    pub base_url: String,
    /// API key or bearer token for FBR IRIS authentication.
    pub api_key: String,
    /// Request timeout in seconds (default: 30).
    pub timeout_secs: u64,
}

impl FbrIrisConfig {
    /// Create a new configuration with default timeout.
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
            timeout_secs: 30,
        }
    }
}

/// Real HTTP client for FBR IRIS tax authority integration.
///
/// Connects to the live FBR IRIS API for NTN verification, tax event
/// submission, withholding rate queries, and taxpayer profile retrieval.
#[derive(Debug)]
pub struct HttpFbrIrisAdapter {
    client: reqwest::Client,
    base_url: String,
}

impl HttpFbrIrisAdapter {
    /// Create a new FBR IRIS HTTP adapter from configuration.
    pub fn new(config: FbrIrisConfig) -> Result<Self, crate::fbr::FbrError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {}", config.api_key))
                        .map_err(|_| crate::fbr::FbrError::NotConfigured {
                            reason: "invalid API key characters".into(),
                        })?,
                );
                headers.insert(
                    reqwest::header::CONTENT_TYPE,
                    reqwest::header::HeaderValue::from_static("application/json"),
                );
                headers
            })
            .build()
            .map_err(|e| crate::fbr::FbrError::ServiceUnavailable {
                reason: format!("failed to build HTTP client: {e}"),
            })?;

        let base_url = config.base_url.trim_end_matches('/').to_string();
        Ok(Self { client, base_url })
    }

    /// Send a request and handle HTTP errors consistently.
    async fn send_request(
        &self,
        request: reqwest::RequestBuilder,
        operation: &str,
    ) -> Result<reqwest::Response, crate::fbr::FbrError> {
        let resp = request.send().await.map_err(|e| {
            if e.is_timeout() {
                crate::fbr::FbrError::Timeout {
                    elapsed_ms: self.client_timeout_ms(),
                }
            } else {
                crate::fbr::FbrError::ServiceUnavailable {
                    reason: format!("{operation}: {e}"),
                }
            }
        })?;

        if resp.status().is_server_error() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(crate::fbr::FbrError::ServiceUnavailable {
                reason: format!("{operation}: HTTP {status} — {body}"),
            });
        }

        Ok(resp)
    }

    fn client_timeout_ms(&self) -> u64 {
        30_000 // Default; could be made configurable
    }
}

impl crate::fbr::FbrIrisAdapter for HttpFbrIrisAdapter {
    fn verify_ntn(
        &self,
        ntn: &mez_core::Ntn,
        entity_name: &str,
    ) -> Result<crate::fbr::NtnVerificationResponse, crate::fbr::FbrError> {
        let rt = tokio::runtime::Handle::try_current().map_err(|_| {
            crate::fbr::FbrError::ServiceUnavailable {
                reason: "no async runtime available for HTTP request".into(),
            }
        })?;

        let url = format!("{}/ntn/verify", self.base_url);
        let body = serde_json::json!({
            "ntn": ntn.as_str(),
            "entity_name": entity_name,
        });

        rt.block_on(async {
            let resp = self
                .send_request(self.client.post(&url).json(&body), "verify_ntn")
                .await?;

            if resp.status().is_client_error() {
                return Err(crate::fbr::FbrError::VerificationFailed {
                    reason: format!("HTTP {}", resp.status()),
                });
            }

            let result: crate::fbr::NtnVerificationResponse = resp
                .json()
                .await
                .map_err(|e| crate::fbr::FbrError::VerificationFailed {
                    reason: format!("response deserialization failed: {e}"),
                })?;

            Ok(result)
        })
    }

    fn submit_tax_event(
        &self,
        event: &crate::fbr::TaxEventSubmission,
    ) -> Result<crate::fbr::TaxEventResult, crate::fbr::FbrError> {
        crate::fbr::validate_ntn(&event.ntn)?;

        if event.idempotency_key.is_empty() {
            return Err(crate::fbr::FbrError::SubmissionRejected {
                reason: "idempotency_key must not be empty".into(),
            });
        }

        let rt = tokio::runtime::Handle::try_current().map_err(|_| {
            crate::fbr::FbrError::ServiceUnavailable {
                reason: "no async runtime available for HTTP request".into(),
            }
        })?;

        let url = format!("{}/tax-events", self.base_url);

        rt.block_on(async {
            let resp = self
                .send_request(self.client.post(&url).json(event), "submit_tax_event")
                .await?;

            if resp.status().is_client_error() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(crate::fbr::FbrError::SubmissionRejected {
                    reason: format!("HTTP {status} — {body}"),
                });
            }

            let result: crate::fbr::TaxEventResult = resp
                .json()
                .await
                .map_err(|e| crate::fbr::FbrError::SubmissionRejected {
                    reason: format!("response deserialization failed: {e}"),
                })?;

            Ok(result)
        })
    }

    fn query_withholding_rate(
        &self,
        params: &crate::fbr::WithholdingRateQuery,
    ) -> Result<crate::fbr::WithholdingRate, crate::fbr::FbrError> {
        let rt = tokio::runtime::Handle::try_current().map_err(|_| {
            crate::fbr::FbrError::ServiceUnavailable {
                reason: "no async runtime available for HTTP request".into(),
            }
        })?;

        let url = format!("{}/withholding-rates", self.base_url);

        rt.block_on(async {
            let resp = self
                .send_request(
                    self.client.get(&url).query(params),
                    "query_withholding_rate",
                )
                .await?;

            let result: crate::fbr::WithholdingRate = resp
                .json()
                .await
                .map_err(|e| crate::fbr::FbrError::ServiceUnavailable {
                    reason: format!("response deserialization failed: {e}"),
                })?;

            Ok(result)
        })
    }

    fn get_taxpayer_profile(
        &self,
        ntn: &mez_core::Ntn,
    ) -> Result<crate::fbr::TaxpayerProfile, crate::fbr::FbrError> {
        let rt = tokio::runtime::Handle::try_current().map_err(|_| {
            crate::fbr::FbrError::ServiceUnavailable {
                reason: "no async runtime available for HTTP request".into(),
            }
        })?;

        let url = format!("{}/taxpayers/{}", self.base_url, ntn.as_str());

        rt.block_on(async {
            let resp = self
                .send_request(self.client.get(&url), "get_taxpayer_profile")
                .await?;

            if resp.status().as_u16() == 404 {
                return Err(crate::fbr::FbrError::VerificationFailed {
                    reason: format!("NTN {} not found in FBR records", ntn.as_str()),
                });
            }

            let result: crate::fbr::TaxpayerProfile = resp
                .json()
                .await
                .map_err(|e| crate::fbr::FbrError::ServiceUnavailable {
                    reason: format!("response deserialization failed: {e}"),
                })?;

            Ok(result)
        })
    }

    fn adapter_name(&self) -> &str {
        "HttpFbrIrisAdapter"
    }
}

// ─── NADRA HTTP Client ──────────────────────────────────────────────────

/// Configuration for the NADRA HTTP adapter.
#[derive(Debug, Clone)]
pub struct NadraConfig {
    /// Base URL of the NADRA verification API.
    pub base_url: String,
    /// API key or bearer token for NADRA authentication.
    pub api_key: String,
    /// Request timeout in seconds (default: 30).
    pub timeout_secs: u64,
}

impl NadraConfig {
    /// Create a new configuration with default timeout.
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
            timeout_secs: 30,
        }
    }
}

/// Real HTTP client for NADRA identity verification.
///
/// Connects to the live NADRA CNIC verification API for identity verification
/// and CNIC status checks.
#[derive(Debug)]
pub struct HttpNadraAdapter {
    client: reqwest::Client,
    base_url: String,
}

impl HttpNadraAdapter {
    /// Create a new NADRA HTTP adapter from configuration.
    pub fn new(config: NadraConfig) -> Result<Self, crate::nadra::NadraError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {}", config.api_key))
                        .map_err(|_| crate::nadra::NadraError::NotConfigured {
                            reason: "invalid API key characters".into(),
                        })?,
                );
                headers.insert(
                    reqwest::header::CONTENT_TYPE,
                    reqwest::header::HeaderValue::from_static("application/json"),
                );
                headers
            })
            .build()
            .map_err(|e| crate::nadra::NadraError::ServiceUnavailable {
                reason: format!("failed to build HTTP client: {e}"),
            })?;

        let base_url = config.base_url.trim_end_matches('/').to_string();
        Ok(Self { client, base_url })
    }

    async fn send_request(
        &self,
        request: reqwest::RequestBuilder,
        operation: &str,
    ) -> Result<reqwest::Response, crate::nadra::NadraError> {
        let resp = request.send().await.map_err(|e| {
            if e.is_timeout() {
                crate::nadra::NadraError::Timeout {
                    elapsed_ms: 30_000,
                }
            } else {
                crate::nadra::NadraError::ServiceUnavailable {
                    reason: format!("{operation}: {e}"),
                }
            }
        })?;

        if resp.status().is_server_error() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(crate::nadra::NadraError::ServiceUnavailable {
                reason: format!("{operation}: HTTP {status} — {body}"),
            });
        }

        Ok(resp)
    }
}

impl crate::nadra::NadraAdapter for HttpNadraAdapter {
    fn verify_identity(
        &self,
        request: &crate::nadra::NadraVerificationRequest,
    ) -> Result<crate::nadra::NadraVerificationResponse, crate::nadra::NadraError> {
        let _canonical_cnic = crate::nadra::validate_cnic(&request.cnic)?;

        let rt = tokio::runtime::Handle::try_current().map_err(|_| {
            crate::nadra::NadraError::ServiceUnavailable {
                reason: "no async runtime available for HTTP request".into(),
            }
        })?;

        let url = format!("{}/verify", self.base_url);

        rt.block_on(async {
            let resp = self
                .send_request(self.client.post(&url).json(request), "verify_identity")
                .await?;

            if resp.status().is_client_error() {
                return Err(crate::nadra::NadraError::VerificationFailed {
                    reason: format!("HTTP {}", resp.status()),
                });
            }

            let result: crate::nadra::NadraVerificationResponse = resp
                .json()
                .await
                .map_err(|e| crate::nadra::NadraError::VerificationFailed {
                    reason: format!("response deserialization failed: {e}"),
                })?;

            Ok(result)
        })
    }

    fn check_cnic_status(
        &self,
        cnic: &str,
    ) -> Result<crate::nadra::CnicStatus, crate::nadra::NadraError> {
        let canonical = crate::nadra::validate_cnic(cnic)?;

        let rt = tokio::runtime::Handle::try_current().map_err(|_| {
            crate::nadra::NadraError::ServiceUnavailable {
                reason: "no async runtime available for HTTP request".into(),
            }
        })?;

        let url = format!("{}/cnic/{}/status", self.base_url, canonical);

        rt.block_on(async {
            let resp = self
                .send_request(self.client.get(&url), "check_cnic_status")
                .await?;

            let result: crate::nadra::CnicStatus = resp
                .json()
                .await
                .map_err(|e| crate::nadra::NadraError::VerificationFailed {
                    reason: format!("response deserialization failed: {e}"),
                })?;

            Ok(result)
        })
    }

    fn adapter_name(&self) -> &str {
        "HttpNadraAdapter"
    }
}

// ─── SECP HTTP Client ───────────────────────────────────────────────────

/// Configuration for the SECP HTTP adapter.
#[derive(Debug, Clone)]
pub struct SecpConfig {
    /// Base URL of the SECP eServices API.
    pub base_url: String,
    /// API key or bearer token for SECP authentication.
    pub api_key: String,
    /// Request timeout in seconds (default: 30).
    pub timeout_secs: u64,
}

impl SecpConfig {
    /// Create a new configuration with default timeout.
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
            timeout_secs: 30,
        }
    }
}

/// Real HTTP client for SECP corporate registry.
///
/// Connects to the live SECP eServices portal for company lookup, license
/// verification, filing status, and director queries.
#[derive(Debug)]
pub struct HttpSecpAdapter {
    client: reqwest::Client,
    base_url: String,
}

impl HttpSecpAdapter {
    /// Create a new SECP HTTP adapter from configuration.
    pub fn new(config: SecpConfig) -> Result<Self, crate::secp::SecpError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {}", config.api_key))
                        .map_err(|_| crate::secp::SecpError::NotConfigured {
                            reason: "invalid API key characters".into(),
                        })?,
                );
                headers.insert(
                    reqwest::header::CONTENT_TYPE,
                    reqwest::header::HeaderValue::from_static("application/json"),
                );
                headers
            })
            .build()
            .map_err(|e| crate::secp::SecpError::ServiceUnavailable {
                reason: format!("failed to build HTTP client: {e}"),
            })?;

        let base_url = config.base_url.trim_end_matches('/').to_string();
        Ok(Self { client, base_url })
    }

    async fn send_request(
        &self,
        request: reqwest::RequestBuilder,
        operation: &str,
    ) -> Result<reqwest::Response, crate::secp::SecpError> {
        let resp = request.send().await.map_err(|e| {
            if e.is_timeout() {
                crate::secp::SecpError::Timeout {
                    elapsed_ms: 30_000,
                }
            } else {
                crate::secp::SecpError::ServiceUnavailable {
                    reason: format!("{operation}: {e}"),
                }
            }
        })?;

        if resp.status().is_server_error() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(crate::secp::SecpError::ServiceUnavailable {
                reason: format!("{operation}: HTTP {status} — {body}"),
            });
        }

        Ok(resp)
    }
}

impl crate::secp::SecpAdapter for HttpSecpAdapter {
    fn lookup_company(
        &self,
        registration_no: &str,
    ) -> Result<crate::secp::CompanyRecord, crate::secp::SecpError> {
        crate::secp::validate_registration_no(registration_no)?;

        let rt = tokio::runtime::Handle::try_current().map_err(|_| {
            crate::secp::SecpError::ServiceUnavailable {
                reason: "no async runtime available for HTTP request".into(),
            }
        })?;

        let url = format!("{}/companies/{}", self.base_url, registration_no);

        rt.block_on(async {
            let resp = self
                .send_request(self.client.get(&url), "lookup_company")
                .await?;

            if resp.status().as_u16() == 404 {
                return Err(crate::secp::SecpError::CompanyNotFound {
                    registration_no: registration_no.to_string(),
                });
            }

            let result: crate::secp::CompanyRecord = resp
                .json()
                .await
                .map_err(|e| crate::secp::SecpError::ServiceUnavailable {
                    reason: format!("response deserialization failed: {e}"),
                })?;

            Ok(result)
        })
    }

    fn verify_license(
        &self,
        registration_no: &str,
        license_type: &str,
    ) -> Result<crate::secp::LicenseVerification, crate::secp::SecpError> {
        crate::secp::validate_registration_no(registration_no)?;

        let rt = tokio::runtime::Handle::try_current().map_err(|_| {
            crate::secp::SecpError::ServiceUnavailable {
                reason: "no async runtime available for HTTP request".into(),
            }
        })?;

        let url = format!(
            "{}/companies/{}/licenses/{}",
            self.base_url, registration_no, license_type
        );

        rt.block_on(async {
            let resp = self
                .send_request(self.client.get(&url), "verify_license")
                .await?;

            if resp.status().as_u16() == 404 {
                return Err(crate::secp::SecpError::CompanyNotFound {
                    registration_no: registration_no.to_string(),
                });
            }

            let result: crate::secp::LicenseVerification = resp
                .json()
                .await
                .map_err(|e| crate::secp::SecpError::ServiceUnavailable {
                    reason: format!("response deserialization failed: {e}"),
                })?;

            Ok(result)
        })
    }

    fn check_filing_status(
        &self,
        registration_no: &str,
    ) -> Result<crate::secp::FilingStatus, crate::secp::SecpError> {
        crate::secp::validate_registration_no(registration_no)?;

        let rt = tokio::runtime::Handle::try_current().map_err(|_| {
            crate::secp::SecpError::ServiceUnavailable {
                reason: "no async runtime available for HTTP request".into(),
            }
        })?;

        let url = format!(
            "{}/companies/{}/filing-status",
            self.base_url, registration_no
        );

        rt.block_on(async {
            let resp = self
                .send_request(self.client.get(&url), "check_filing_status")
                .await?;

            if resp.status().as_u16() == 404 {
                return Err(crate::secp::SecpError::CompanyNotFound {
                    registration_no: registration_no.to_string(),
                });
            }

            let result: crate::secp::FilingStatus = resp
                .json()
                .await
                .map_err(|e| crate::secp::SecpError::ServiceUnavailable {
                    reason: format!("response deserialization failed: {e}"),
                })?;

            Ok(result)
        })
    }

    fn get_directors(
        &self,
        registration_no: &str,
    ) -> Result<Vec<crate::secp::DirectorRecord>, crate::secp::SecpError> {
        crate::secp::validate_registration_no(registration_no)?;

        let rt = tokio::runtime::Handle::try_current().map_err(|_| {
            crate::secp::SecpError::ServiceUnavailable {
                reason: "no async runtime available for HTTP request".into(),
            }
        })?;

        let url = format!(
            "{}/companies/{}/directors",
            self.base_url, registration_no
        );

        rt.block_on(async {
            let resp = self
                .send_request(self.client.get(&url), "get_directors")
                .await?;

            if resp.status().as_u16() == 404 {
                return Err(crate::secp::SecpError::CompanyNotFound {
                    registration_no: registration_no.to_string(),
                });
            }

            let result: Vec<crate::secp::DirectorRecord> = resp
                .json()
                .await
                .map_err(|e| crate::secp::SecpError::ServiceUnavailable {
                    reason: format!("response deserialization failed: {e}"),
                })?;

            Ok(result)
        })
    }

    fn adapter_name(&self) -> &str {
        "HttpSecpAdapter"
    }
}

// ─── SBP Raast HTTP Client ─────────────────────────────────────────────

/// Configuration for the SBP Raast HTTP adapter.
#[derive(Debug, Clone)]
pub struct RaastConfig {
    /// Base URL of the SBP Raast API.
    pub base_url: String,
    /// API key or certificate identifier for SBP Raast authentication.
    pub api_key: String,
    /// Participant bank code (4-character IBAN bank code).
    pub bank_code: String,
    /// Request timeout in seconds (default: 30).
    pub timeout_secs: u64,
}

impl RaastConfig {
    /// Create a new configuration with default timeout.
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        bank_code: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
            bank_code: bank_code.into(),
            timeout_secs: 30,
        }
    }
}

/// Real HTTP client for SBP Raast instant payment system.
///
/// Connects to the live SBP Raast API for payment initiation, status queries,
/// account verification, and alias-based lookups.
#[derive(Debug)]
pub struct HttpRaastAdapter {
    client: reqwest::Client,
    base_url: String,
    bank_code: String,
}

impl HttpRaastAdapter {
    /// Create a new SBP Raast HTTP adapter from configuration.
    pub fn new(config: RaastConfig) -> Result<Self, crate::raast::RaastError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {}", config.api_key))
                        .map_err(|_| crate::raast::RaastError::NotConfigured {
                            reason: "invalid API key characters".into(),
                        })?,
                );
                headers.insert(
                    reqwest::header::CONTENT_TYPE,
                    reqwest::header::HeaderValue::from_static("application/json"),
                );
                headers
            })
            .build()
            .map_err(|e| crate::raast::RaastError::ServiceUnavailable {
                reason: format!("failed to build HTTP client: {e}"),
            })?;

        let base_url = config.base_url.trim_end_matches('/').to_string();
        Ok(Self {
            client,
            base_url,
            bank_code: config.bank_code,
        })
    }

    async fn send_request(
        &self,
        request: reqwest::RequestBuilder,
        operation: &str,
    ) -> Result<reqwest::Response, crate::raast::RaastError> {
        let resp = request.send().await.map_err(|e| {
            if e.is_timeout() {
                crate::raast::RaastError::Timeout {
                    elapsed_ms: 30_000,
                }
            } else {
                crate::raast::RaastError::ServiceUnavailable {
                    reason: format!("{operation}: {e}"),
                }
            }
        })?;

        if resp.status().is_server_error() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(crate::raast::RaastError::ServiceUnavailable {
                reason: format!("{operation}: HTTP {status} — {body}"),
            });
        }

        Ok(resp)
    }

    /// Accessor for the bank code (useful for constructing payment references).
    pub fn bank_code(&self) -> &str {
        &self.bank_code
    }
}

impl crate::raast::RaastAdapter for HttpRaastAdapter {
    fn initiate_payment(
        &self,
        instruction: &crate::raast::RaastPaymentInstruction,
    ) -> Result<crate::raast::RaastPaymentResult, crate::raast::RaastError> {
        crate::raast::validate_iban(&instruction.from_iban)?;
        crate::raast::validate_iban(&instruction.to_iban)?;

        if instruction.amount <= 0 {
            return Err(crate::raast::RaastError::PaymentRejected {
                reason: "amount must be positive".into(),
            });
        }

        let rt = tokio::runtime::Handle::try_current().map_err(|_| {
            crate::raast::RaastError::ServiceUnavailable {
                reason: "no async runtime available for HTTP request".into(),
            }
        })?;

        let url = format!("{}/payments", self.base_url);

        rt.block_on(async {
            let resp = self
                .send_request(
                    self.client
                        .post(&url)
                        .json(instruction)
                        .header("X-Bank-Code", &self.bank_code),
                    "initiate_payment",
                )
                .await?;

            if resp.status().is_client_error() {
                let body = resp.text().await.unwrap_or_default();
                return Err(crate::raast::RaastError::PaymentRejected {
                    reason: body,
                });
            }

            let result: crate::raast::RaastPaymentResult = resp
                .json()
                .await
                .map_err(|e| crate::raast::RaastError::ServiceUnavailable {
                    reason: format!("response deserialization failed: {e}"),
                })?;

            Ok(result)
        })
    }

    fn check_payment_status(
        &self,
        raast_reference: &str,
    ) -> Result<crate::raast::RaastPaymentResult, crate::raast::RaastError> {
        let rt = tokio::runtime::Handle::try_current().map_err(|_| {
            crate::raast::RaastError::ServiceUnavailable {
                reason: "no async runtime available for HTTP request".into(),
            }
        })?;

        let url = format!("{}/payments/{}", self.base_url, raast_reference);

        rt.block_on(async {
            let resp = self
                .send_request(self.client.get(&url), "check_payment_status")
                .await?;

            if resp.status().as_u16() == 404 {
                return Err(crate::raast::RaastError::PaymentNotFound {
                    reference: raast_reference.to_string(),
                });
            }

            let result: crate::raast::RaastPaymentResult = resp
                .json()
                .await
                .map_err(|e| crate::raast::RaastError::ServiceUnavailable {
                    reason: format!("response deserialization failed: {e}"),
                })?;

            Ok(result)
        })
    }

    fn verify_account(
        &self,
        iban: &str,
    ) -> Result<crate::raast::AccountVerification, crate::raast::RaastError> {
        crate::raast::validate_iban(iban)?;

        let rt = tokio::runtime::Handle::try_current().map_err(|_| {
            crate::raast::RaastError::ServiceUnavailable {
                reason: "no async runtime available for HTTP request".into(),
            }
        })?;

        let url = format!("{}/accounts/{}/verify", self.base_url, iban);

        rt.block_on(async {
            let resp = self
                .send_request(self.client.get(&url), "verify_account")
                .await?;

            if resp.status().is_client_error() {
                return Err(crate::raast::RaastError::AccountVerificationFailed {
                    reason: format!("HTTP {}", resp.status()),
                });
            }

            let result: crate::raast::AccountVerification = resp
                .json()
                .await
                .map_err(|e| crate::raast::RaastError::AccountVerificationFailed {
                    reason: format!("response deserialization failed: {e}"),
                })?;

            Ok(result)
        })
    }

    fn lookup_by_alias(
        &self,
        alias: &str,
        alias_type: crate::raast::AliasType,
    ) -> Result<crate::raast::AliasLookupResult, crate::raast::RaastError> {
        let rt = tokio::runtime::Handle::try_current().map_err(|_| {
            crate::raast::RaastError::ServiceUnavailable {
                reason: "no async runtime available for HTTP request".into(),
            }
        })?;

        let url = format!("{}/aliases/lookup", self.base_url);
        let body = serde_json::json!({
            "alias": alias,
            "alias_type": alias_type,
        });

        rt.block_on(async {
            let resp = self
                .send_request(self.client.post(&url).json(&body), "lookup_by_alias")
                .await?;

            if resp.status().as_u16() == 404 {
                return Err(crate::raast::RaastError::AliasNotFound {
                    alias: alias.to_string(),
                });
            }

            let result: crate::raast::AliasLookupResult = resp
                .json()
                .await
                .map_err(|e| crate::raast::RaastError::ServiceUnavailable {
                    reason: format!("response deserialization failed: {e}"),
                })?;

            Ok(result)
        })
    }

    fn adapter_name(&self) -> &str {
        "HttpRaastAdapter"
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fbr::FbrIrisAdapter;
    use crate::nadra::NadraAdapter;
    use crate::raast::RaastAdapter;
    use crate::secp::SecpAdapter;

    #[test]
    fn fbr_config_new() {
        let config = FbrIrisConfig::new("https://iris.fbr.gov.pk/api/v1", "test-key");
        assert_eq!(config.base_url, "https://iris.fbr.gov.pk/api/v1");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.timeout_secs, 30);
    }

    #[test]
    fn nadra_config_new() {
        let config = NadraConfig::new("https://nadra.gov.pk/api/v1", "test-key");
        assert_eq!(config.base_url, "https://nadra.gov.pk/api/v1");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.timeout_secs, 30);
    }

    #[test]
    fn secp_config_new() {
        let config = SecpConfig::new("https://eservices.secp.gov.pk/api/v1", "test-key");
        assert_eq!(config.base_url, "https://eservices.secp.gov.pk/api/v1");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.timeout_secs, 30);
    }

    #[test]
    fn raast_config_new() {
        let config = RaastConfig::new("https://raast.sbp.org.pk/api/v1", "test-key", "HABB");
        assert_eq!(config.base_url, "https://raast.sbp.org.pk/api/v1");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.bank_code, "HABB");
        assert_eq!(config.timeout_secs, 30);
    }

    #[test]
    fn http_fbr_adapter_builds_with_valid_config() {
        let config = FbrIrisConfig::new("https://iris.fbr.gov.pk/api/v1", "test-key");
        let adapter = HttpFbrIrisAdapter::new(config);
        assert!(adapter.is_ok());
        let adapter = adapter.expect("adapter should build");
        assert_eq!(adapter.adapter_name(), "HttpFbrIrisAdapter");
    }

    #[test]
    fn http_nadra_adapter_builds_with_valid_config() {
        let config = NadraConfig::new("https://nadra.gov.pk/api/v1", "test-key");
        let adapter = HttpNadraAdapter::new(config);
        assert!(adapter.is_ok());
        let adapter = adapter.expect("adapter should build");
        assert_eq!(adapter.adapter_name(), "HttpNadraAdapter");
    }

    #[test]
    fn http_secp_adapter_builds_with_valid_config() {
        let config = SecpConfig::new("https://eservices.secp.gov.pk/api/v1", "test-key");
        let adapter = HttpSecpAdapter::new(config);
        assert!(adapter.is_ok());
        let adapter = adapter.expect("adapter should build");
        assert_eq!(adapter.adapter_name(), "HttpSecpAdapter");
    }

    #[test]
    fn http_raast_adapter_builds_with_valid_config() {
        let config = RaastConfig::new("https://raast.sbp.org.pk/api/v1", "test-key", "HABB");
        let adapter = HttpRaastAdapter::new(config);
        assert!(adapter.is_ok());
        let adapter = adapter.expect("adapter should build");
        assert_eq!(adapter.adapter_name(), "HttpRaastAdapter");
        assert_eq!(adapter.bank_code(), "HABB");
    }

    #[test]
    fn http_fbr_adapter_is_trait_object_safe() {
        let config = FbrIrisConfig::new("https://test.example.com", "key");
        let adapter = HttpFbrIrisAdapter::new(config).expect("build");
        let _boxed: Box<dyn crate::fbr::FbrIrisAdapter> = Box::new(adapter);
    }

    #[test]
    fn http_nadra_adapter_is_trait_object_safe() {
        let config = NadraConfig::new("https://test.example.com", "key");
        let adapter = HttpNadraAdapter::new(config).expect("build");
        let _boxed: Box<dyn crate::nadra::NadraAdapter> = Box::new(adapter);
    }

    #[test]
    fn http_secp_adapter_is_trait_object_safe() {
        let config = SecpConfig::new("https://test.example.com", "key");
        let adapter = HttpSecpAdapter::new(config).expect("build");
        let _boxed: Box<dyn crate::secp::SecpAdapter> = Box::new(adapter);
    }

    #[test]
    fn http_raast_adapter_is_trait_object_safe() {
        let config = RaastConfig::new("https://test.example.com", "key", "TEST");
        let adapter = HttpRaastAdapter::new(config).expect("build");
        let _boxed: Box<dyn crate::raast::RaastAdapter> = Box::new(adapter);
    }

    #[test]
    fn base_url_trailing_slash_trimmed() {
        let config = FbrIrisConfig::new("https://iris.fbr.gov.pk/api/v1/", "key");
        let adapter = HttpFbrIrisAdapter::new(config).expect("build");
        assert_eq!(adapter.base_url, "https://iris.fbr.gov.pk/api/v1");
    }
}
