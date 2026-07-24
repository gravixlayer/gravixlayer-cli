// src/api/agent.rs — Agent build and deployment API methods.
//
// Agent builds are submitted as multipart/form-data with two parts:
//   "archive"  — tar.gz bytes of the source directory
//   "metadata" — JSON string of AgentBuildMetadata
//
// This mirrors `resources/agents.py` in the Python SDK exactly.

use std::time::Instant;

use reqwest::multipart;
use tokio::time::{sleep, Duration};

use super::{error::ApiError, types::*, ApiClient};

pub struct AgentApi<'a> {
    client: &'a ApiClient,
}

impl<'a> AgentApi<'a> {
    pub(crate) fn new(client: &'a ApiClient) -> Self {
        Self { client }
    }

    // ------------------------------------------------------------------
    // Build
    // ------------------------------------------------------------------

    /// `POST /v1/agents/template/build-agent` — multipart upload.
    ///
    /// `archive_bytes` is the raw tar.gz payload produced by
    /// `crate::scaffold::archive::create_source_archive`.
    pub async fn build(
        &self,
        archive_bytes: Vec<u8>,
        metadata: &AgentBuildMetadata,
    ) -> Result<AgentBuildResponse, ApiError> {
        let url = self.client.agents_url("template/build-agent");
        let metadata_json = serde_json::to_string(metadata).map_err(|e| ApiError::Server {
            status: 0,
            body: format!("failed to serialize build metadata: {e}"),
        })?;

        // Build multipart form — two named parts as expected by the Go handler.
        let archive_part = multipart::Part::bytes(archive_bytes)
            .file_name("archive.tar.gz")
            .mime_str("application/gzip")
            .map_err(|e| ApiError::Connection(e))?;
        let metadata_part = multipart::Part::text(metadata_json)
            .mime_str("application/json")
            .map_err(|e| ApiError::Connection(e))?;
        let form = multipart::Form::new()
            .part("archive", archive_part)
            .part("metadata", metadata_part);

        // Multipart uploads are not retried automatically (archive is consumed
        // once), so we send directly without the retry wrapper.
        use secrecy::ExposeSecret;
        let resp = self
            .client
            .http_client()
            .post(url)
            .bearer_auth(self.client.api_key().expose_secret())
            .multipart(form)
            .send()
            .await
            .map_err(ApiError::Connection)?;

        let status = resp.status();
        if status.is_success() {
            resp.json::<AgentBuildResponse>()
                .await
                .map_err(ApiError::Deserialization)
        } else {
            let body = resp.text().await.unwrap_or_default();
            Err(ApiError::from_response(status.as_u16(), body))
        }
    }

    // ------------------------------------------------------------------
    // Build status
    // ------------------------------------------------------------------

    /// `GET /v1/agents/template/builds/<build_id>/status`
    pub async fn build_status(&self, build_id: &str) -> Result<AgentBuildStatusResponse, ApiError> {
        let url = self
            .client
            .agents_url(&format!("template/builds/{build_id}/status"));
        self.client
            .execute("agent.build_status", || self.client.get(url.clone()))
            .await
    }

    /// Poll `build_status` until status is `"completed"` or `"failed"`.
    pub async fn wait_for_build(
        &self,
        build_id: &str,
        deadline: Instant,
    ) -> Result<AgentBuildStatusResponse, ApiError> {
        let mut interval = Duration::from_secs(3);
        loop {
            let status = self.build_status(build_id).await?;
            match status.status.as_str() {
                "completed" => return Ok(status),
                "failed" => {
                    let detail = status
                        .error
                        .as_deref()
                        .or(status.message.as_deref())
                        .unwrap_or("(no message)");
                    return Err(ApiError::Server {
                        status: 500,
                        body: format!("agent build {build_id} failed: {detail}"),
                    });
                }
                _ => {}
            }
            if Instant::now() >= deadline {
                return Err(ApiError::Server {
                    status: 408,
                    body: format!("timed out waiting for agent build {build_id}"),
                });
            }
            sleep(interval).await;
            interval = (interval * 2).min(Duration::from_secs(15));
        }
    }

    // ------------------------------------------------------------------
    // Deploy
    // ------------------------------------------------------------------

    /// `POST /v1/agents/deploy`
    pub async fn deploy(&self, req: &DeployAgentRequest) -> Result<AgentDeployResponse, ApiError> {
        let url = self.client.agents_url("deploy");
        self.client
            .execute("agent.deploy", || self.client.post(url.clone()).json(req))
            .await
    }

    // ------------------------------------------------------------------
    // Get endpoint
    // ------------------------------------------------------------------

    /// `GET /v1/agents/<agent_id>/endpoint`
    pub async fn get(&self, agent_id: &str) -> Result<AgentEndpoint, ApiError> {
        let url = self.client.agents_url(&format!("{agent_id}/endpoint"));
        self.client
            .execute("agent.get", || self.client.get(url.clone()))
            .await
    }

    // ------------------------------------------------------------------
    // Destroy
    // ------------------------------------------------------------------

    /// `DELETE /v1/agents/<agent_id>`
    pub async fn destroy(&self, agent_id: &str) -> Result<AgentDestroyResponse, ApiError> {
        let url = self.client.agents_url(agent_id);
        self.client
            .execute("agent.destroy", || self.client.delete(url.clone()))
            .await
    }

    // ------------------------------------------------------------------
    // Invoke
    // ------------------------------------------------------------------

    /// `POST <agent endpoint>/invoke`
    ///
    /// Sends a JSON payload to the deployed agent endpoint and returns the
    /// raw JSON response.
    pub async fn invoke(
        &self,
        agent_id: &str,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value, ApiError> {
        let endpoint = self.get(agent_id).await?;
        let base_url = endpoint.endpoint.ok_or_else(|| ApiError::Server {
            status: 404,
            body: format!("agent {agent_id} has no endpoint"),
        })?;
        let url = format!("{}/invoke", base_url.trim_end_matches('/'));
        self.client
            .execute("agent.invoke", || {
                self.client.post(url.clone()).json(payload)
            })
            .await
    }

    /// `POST <agent endpoint>/stream`
    pub async fn stream(
        &self,
        agent_id: &str,
        payload: &serde_json::Value,
    ) -> Result<reqwest::Response, ApiError> {
        let endpoint = self.get(agent_id).await?;
        let base_url = endpoint.endpoint.ok_or_else(|| ApiError::Server {
            status: 404,
            body: format!("agent {agent_id} has no endpoint"),
        })?;
        let url = format!("{}/stream", base_url.trim_end_matches('/'));
        let resp = self
            .client
            .post(url)
            .bearer_auth(self.client.api_key_str())
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .json(payload)
            .send()
            .await
            .map_err(ApiError::Connection)?;

        let status = resp.status();
        if status.is_success() {
            Ok(resp)
        } else {
            let body = resp.text().await.unwrap_or_default();
            Err(ApiError::from_response(status.as_u16(), body))
        }
    }

    // ------------------------------------------------------------------
    // Wait for agent to go ACTIVE
    // ------------------------------------------------------------------

    /// Poll until the agent endpoint reaches `status == "active"`.
    pub async fn wait_until_active(
        &self,
        agent_id: &str,
        deadline: Instant,
    ) -> Result<AgentEndpoint, ApiError> {
        let mut interval = Duration::from_secs(4);
        loop {
            let ep = self.get(agent_id).await?;
            match ep.status.as_deref() {
                Some("active") => return Ok(ep),
                Some("deleted") | Some("failed") => {
                    return Err(ApiError::Server {
                        status: 500,
                        body: format!(
                            "agent {agent_id} reached terminal state: {}",
                            ep.status.as_deref().unwrap_or("unknown")
                        ),
                    });
                }
                _ => {}
            }
            if Instant::now() >= deadline {
                return Err(ApiError::Server {
                    status: 408,
                    body: format!("timed out waiting for agent {agent_id} to become active"),
                });
            }
            sleep(interval).await;
            interval = (interval * 2).min(Duration::from_secs(20));
        }
    }
}
