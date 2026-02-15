//! Typed client for Mass templating-engine.
//!
//! Base URL: `templating-engine-prod-5edc768c1f80.herokuapp.com`

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::MassApiError;

// -- Types matching Mass API schemas ------------------------------------------

/// Request to generate a document from a template.
#[derive(Debug, Serialize)]
pub struct GenerateDocumentRequest {
    pub template_id: String,
    pub entity_id: Uuid,
    pub parameters: serde_json::Value,
}

/// Generated document response from the templating engine.
#[derive(Debug, Clone, Deserialize)]
pub struct GeneratedDocument {
    pub id: Uuid,
    pub template_id: String,
    pub entity_id: Uuid,
    pub format: String,
    pub content_url: Option<String>,
    pub status: String,
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

    /// Generate a document from a template.
    ///
    /// Calls `POST {base_url}/templating-engine/documents/generate`.
    pub async fn generate(
        &self,
        req: &GenerateDocumentRequest,
    ) -> Result<GeneratedDocument, MassApiError> {
        let endpoint = "POST /documents/generate";
        let url = format!(
            "{}templating-engine/documents/generate",
            self.base_url
        );

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
}
