//! Typed client for Mass templating-engine.
//!
//! Base URL: `templating-engine-prod-5edc768c1f80.herokuapp.com`
//! Context path: `/templating-engine`
//! Swagger: `/templating-engine/swagger-ui/index.html`
//! API docs: `/templating-engine/v3/api-docs`
//!
//! ## Live API Paths (from Swagger spec, February 2026)
//!
//! | Method | Path | Operation |
//! |--------|------|-----------|
//! | POST   | `/api/v1/template/sign` | Sign/generate document from template |
//! | GET    | `/api/v1/template/{id}` | Get template by ID |
//! | GET    | `/api/v1/template/available` | Get available templates |
//! | POST   | `/api/v1/template/requirements` | Get field requirements |
//! | GET    | `/api/v1/submission/{submissionId}` | Get submission |
//! | GET    | `/api/v1/submission/all` | Get submissions by filter |

use serde::{Deserialize, Serialize};

use crate::error::MassApiError;

/// API version path for templating-engine service.
const API_PREFIX: &str = "templating-engine/api/v1";

// -- Types matching Mass API schemas ------------------------------------------

/// Request to sign/generate a document from a template.
///
/// Matches `POST /api/v1/template/sign` on templating-engine.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignTemplateRequest {
    pub entity_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signing_order: Option<SigningOrder>,
    pub template_types: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub signers: Vec<TemplateSigner>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_submission_id: Option<String>,
}

/// Signing order for a template signing request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SigningOrder {
    Random,
    Preserved,
}

/// A signer for a template signing request.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSigner {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<SignerName>,
    pub email: String,
    pub signing_role: SigningRole,
}

/// Name structure for a signer.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignerName {
    pub first_name: String,
    pub last_name: String,
}

/// Role of a signer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SigningRole {
    Officer,
    Recipient,
    Spouse,
    Witness,
    /// Forward-compatible catch-all.
    #[serde(other)]
    Unknown,
}

/// Submission response from the templating engine.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionResponse {
    pub id: String,
    #[serde(default)]
    pub entity_id: Option<String>,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub signing_order: Option<SigningOrder>,
    #[serde(default)]
    pub signers: Vec<serde_json::Value>,
    #[serde(default)]
    pub document_uri: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Template record.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub entity_id: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(rename = "type", default)]
    pub template_type: Option<String>,
    #[serde(default)]
    pub grouping: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

/// Available template option.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateOption {
    pub template_type: String,
    #[serde(default)]
    pub template_name: Option<String>,
    #[serde(default)]
    pub entity_id: Option<String>,
    #[serde(default)]
    pub template_id: Option<String>,
    #[serde(default)]
    pub document_uri: Option<String>,
}

/// Legacy request type for backwards compatibility with existing code.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateDocumentRequest {
    pub template_id: String,
    pub entity_id: String,
    pub parameters: serde_json::Value,
}

/// Legacy response type for backwards compatibility.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedDocument {
    pub id: String,
    #[serde(default)]
    pub template_id: Option<String>,
    #[serde(default)]
    pub entity_id: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub content_url: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

// -- Client -------------------------------------------------------------------

/// Client for the Mass templating-engine API.
#[derive(Debug, Clone)]
pub struct TemplatingClient {
    http: reqwest::Client,
    base_url: url::Url,
}

impl TemplatingClient {
    pub(crate) fn new(http: reqwest::Client, base_url: url::Url) -> Self {
        Self { http, base_url }
    }

    /// Sign/generate a document from a template.
    ///
    /// Calls `POST {base_url}/templating-engine/api/v1/template/sign`.
    pub async fn sign(
        &self,
        req: &SignTemplateRequest,
    ) -> Result<SubmissionResponse, MassApiError> {
        let endpoint = "POST /template/sign";
        let url = format!("{}{}/template/sign", self.base_url, API_PREFIX);

        let resp = crate::retry::retry_send(|| self.http.post(&url).json(req).send())
            .await
            .map_err(|e| MassApiError::Http {
                endpoint: endpoint.into(),
                source: e,
            })?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(MassApiError::ApiError {
                endpoint: endpoint.into(),
                status,
                body,
            });
        }

        resp.json().await.map_err(|e| MassApiError::Deserialization {
            endpoint: endpoint.into(),
            source: e,
        })
    }

    /// Get a template by ID.
    ///
    /// Calls `GET {base_url}/templating-engine/api/v1/template/{id}`.
    pub async fn get_template(&self, id: &str) -> Result<Option<Template>, MassApiError> {
        let endpoint = format!("GET /template/{id}");
        let url = format!("{}{}/template/{id}", self.base_url, API_PREFIX);

        let resp = crate::retry::retry_send(|| self.http.get(&url).send())
            .await
            .map_err(|e| MassApiError::Http {
                endpoint: endpoint.clone(),
                source: e,
            })?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(MassApiError::ApiError {
                endpoint,
                status,
                body,
            });
        }

        resp.json()
            .await
            .map(Some)
            .map_err(|e| MassApiError::Deserialization {
                endpoint,
                source: e,
            })
    }

    /// Get available templates.
    ///
    /// Calls `GET {base_url}/templating-engine/api/v1/template/available`.
    pub async fn available_templates(
        &self,
        entity_id: Option<&str>,
        filter_grouping: Option<&str>,
    ) -> Result<Vec<TemplateOption>, MassApiError> {
        let endpoint = "GET /template/available";
        let mut url = format!("{}{}/template/available", self.base_url, API_PREFIX);

        let mut params = Vec::new();
        if let Some(eid) = entity_id {
            params.push(format!("entityId={eid}"));
        }
        if let Some(fg) = filter_grouping {
            params.push(format!("filterGrouping={fg}"));
        }
        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        let resp = crate::retry::retry_send(|| self.http.get(&url).send())
            .await
            .map_err(|e| MassApiError::Http {
                endpoint: endpoint.into(),
                source: e,
            })?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(MassApiError::ApiError {
                endpoint: endpoint.into(),
                status,
                body,
            });
        }

        resp.json().await.map_err(|e| MassApiError::Deserialization {
            endpoint: endpoint.into(),
            source: e,
        })
    }

    /// Get a submission by ID.
    ///
    /// Calls `GET {base_url}/templating-engine/api/v1/submission/{id}`.
    pub async fn get_submission(
        &self,
        submission_id: &str,
    ) -> Result<Option<SubmissionResponse>, MassApiError> {
        let endpoint = format!("GET /submission/{submission_id}");
        let url = format!(
            "{}{}/submission/{submission_id}",
            self.base_url, API_PREFIX
        );

        let resp = crate::retry::retry_send(|| self.http.get(&url).send())
            .await
            .map_err(|e| MassApiError::Http {
                endpoint: endpoint.clone(),
                source: e,
            })?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(MassApiError::ApiError {
                endpoint,
                status,
                body,
            });
        }

        resp.json()
            .await
            .map(Some)
            .map_err(|e| MassApiError::Deserialization {
                endpoint,
                source: e,
            })
    }
}
