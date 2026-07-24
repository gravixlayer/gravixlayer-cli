//! Runtime web services — `*.service.gravixlayer.ai`.

use serde::{Deserialize, Serialize};

use super::error::ApiError;
use super::ApiClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeWebServiceResponse {
    pub runtime_id: String,
    pub port: u16,
    pub url: String,
    #[serde(default)]
    pub web_url: Option<String>,
    #[serde(default)]
    pub browser_url: Option<String>,
    #[serde(default)]
    pub service_url: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub is_public: bool,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub subdomain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeWebServiceListResponse {
    #[serde(default)]
    pub services: Vec<RuntimeWebServiceResponse>,
}

#[derive(Debug, Clone, Serialize)]
struct OpenWebServiceBody {
    port: u16,
    expires_in_seconds: u64,
    is_public: bool,
    rotate_token: bool,
}

/// Web service operations against `/v1/agents/runtime/{id}/services`.
pub struct RuntimeServiceApi<'a> {
    client: &'a ApiClient,
}

impl<'a> RuntimeServiceApi<'a> {
    pub fn new(client: &'a ApiClient) -> Self {
        Self { client }
    }

    /// `POST /v1/agents/runtime/<id>/services` — open (or refresh) a web service.
    pub async fn open(
        &self,
        runtime_id: &str,
        port: u16,
        expires_in_seconds: u64,
        is_public: bool,
        rotate_token: bool,
    ) -> Result<RuntimeWebServiceResponse, ApiError> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/services"));
        self.client
            .execute("runtime.service.open", || {
                self.client.post(url.clone()).json(&OpenWebServiceBody {
                    port,
                    expires_in_seconds,
                    is_public,
                    rotate_token,
                })
            })
            .await
    }

    /// `GET /v1/agents/runtime/<id>/services`
    pub async fn list(&self, runtime_id: &str) -> Result<RuntimeWebServiceListResponse, ApiError> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/services"));
        self.client
            .execute("runtime.service.list", || self.client.get(url.clone()))
            .await
    }

    /// `DELETE /v1/agents/runtime/<id>/services/<port>`
    pub async fn revoke(&self, runtime_id: &str, port: u16) -> Result<(), ApiError> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/services/{port}"));
        self.client
            .execute_empty("runtime.service.revoke", || self.client.delete(url.clone()))
            .await
    }
}
