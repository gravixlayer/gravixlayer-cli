// src/api/runtime.rs — Runtime resource API methods.
//
// All endpoints follow the Python SDK service URL pattern:
//   base_url + "/v1/agents/" + endpoint
//
// Verified against `gravixlayer-python/src/gravixlayer/resources/runtime.py`.

use secrecy::ExposeSecret;
use serde_json::json;

use super::{error::ApiError, types::*, ApiClient};

pub struct RuntimeApi<'a> {
    client: &'a ApiClient,
}

impl<'a> RuntimeApi<'a> {
    pub(crate) fn new(client: &'a ApiClient) -> Self {
        Self { client }
    }

    // ------------------------------------------------------------------
    // Create
    // ------------------------------------------------------------------

    /// `POST /v1/agents/runtime`
    ///
    /// Creates a new runtime.  Mandatory fields: `template`, `cloud`,
    /// `region`.  All optional fields are omitted when `None` / empty.
    pub async fn create(&self, req: CreateRuntimeRequest) -> Result<Runtime, ApiError> {
        let url = self.client.agents_url("runtime");
        self.client
            .execute("runtime.create", || {
                self.client.post(url.clone()).json(&req)
            })
            .await
    }

    // ------------------------------------------------------------------
    // List
    // ------------------------------------------------------------------

    /// `GET /v1/agents/runtime?limit=N&offset=N`
    pub async fn list(&self, limit: u32, offset: u32) -> Result<RuntimeList, ApiError> {
        let url = self.client.agents_url("runtime");
        self.client
            .execute("runtime.list", || {
                self.client
                    .get(url.clone())
                    .query(&[("limit", limit), ("offset", offset)])
            })
            .await
    }

    // ------------------------------------------------------------------
    // Get
    // ------------------------------------------------------------------

    /// `GET /v1/agents/runtime/<id>`
    pub async fn get(&self, runtime_id: &str) -> Result<Runtime, ApiError> {
        let url = self.client.agents_url(&format!("runtime/{runtime_id}"));
        self.client
            .execute("runtime.get", || self.client.get(url.clone()))
            .await
    }

    // ------------------------------------------------------------------
    // Kill / terminate
    // ------------------------------------------------------------------

    /// `DELETE /v1/agents/runtime/<id>`
    pub async fn kill(&self, runtime_id: &str) -> Result<RuntimeKillResponse, ApiError> {
        let url = self.client.agents_url(&format!("runtime/{runtime_id}"));
        self.client
            .execute("runtime.kill", || self.client.delete(url.clone()))
            .await
    }

    // ------------------------------------------------------------------
    // Pause
    // ------------------------------------------------------------------

    /// `POST /v1/agents/runtime/<id>/pause`
    pub async fn pause(&self, runtime_id: &str) -> Result<RuntimeStatusResponse, ApiError> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/pause"));
        self.client
            .execute("runtime.pause", || {
                self.client.post(url.clone()).json(&json!({}))
            })
            .await
    }

    // ------------------------------------------------------------------
    // Resume
    // ------------------------------------------------------------------

    /// `POST /v1/agents/runtime/<id>/resume`
    pub async fn resume(&self, runtime_id: &str) -> Result<RuntimeStatusResponse, ApiError> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/resume"));
        self.client
            .execute("runtime.resume", || {
                self.client.post(url.clone()).json(&json!({}))
            })
            .await
    }

    // ------------------------------------------------------------------
    // Set timeout
    // ------------------------------------------------------------------

    /// `POST /v1/agents/runtime/<id>/timeout`
    pub async fn set_timeout(
        &self,
        runtime_id: &str,
        timeout_secs: u64,
    ) -> Result<RuntimeTimeoutResponse, ApiError> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/timeout"));
        let body = SetTimeoutRequest {
            timeout: timeout_secs,
        };
        self.client
            .execute("runtime.set_timeout", || {
                self.client.post(url.clone()).json(&body)
            })
            .await
    }

    // ------------------------------------------------------------------
    // Metrics
    // ------------------------------------------------------------------

    /// `GET /v1/agents/runtime/<id>/metrics`
    pub async fn metrics(&self, runtime_id: &str) -> Result<RuntimeMetrics, ApiError> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/metrics"));
        self.client
            .execute("runtime.metrics", || self.client.get(url.clone()))
            .await
    }

    // ------------------------------------------------------------------
    // Connect (used for exec/run operations)
    // ------------------------------------------------------------------

    /// `POST /v1/agents/runtime/<id>/connect`
    ///
    pub async fn connect(&self, runtime_id: &str) -> Result<RuntimeConnectResponse, ApiError> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/connect"));
        self.client
            .execute("runtime.connect", || self.client.post(url.clone()))
            .await
    }

    /// `POST /v1/agents/runtime/<id>/code/contexts`
    pub async fn create_code_context(
        &self,
        runtime_id: &str,
        language: Option<&str>,
        cwd: Option<&str>,
    ) -> Result<RuntimeCodeContext, ApiError> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/code/contexts"));
        self.client
            .execute("runtime.code_context.create", || {
                let mut body = serde_json::Map::new();
                if let Some(language) = language {
                    body.insert("language".into(), language.into());
                }
                if let Some(cwd) = cwd {
                    body.insert("cwd".into(), cwd.into());
                }
                self.client.post(url.clone()).json(&body)
            })
            .await
    }

    /// `GET /v1/agents/runtime/<id>/code/contexts/<context_id>`
    pub async fn get_code_context(
        &self,
        runtime_id: &str,
        context_id: &str,
    ) -> Result<RuntimeCodeContext, ApiError> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/code/contexts/{context_id}"));
        self.client
            .execute("runtime.code_context.get", || self.client.get(url.clone()))
            .await
    }

    /// `DELETE /v1/agents/runtime/<id>/code/contexts/<context_id>`
    pub async fn delete_code_context(
        &self,
        runtime_id: &str,
        context_id: &str,
    ) -> Result<RuntimeCodeContextDeleteResponse, ApiError> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/code/contexts/{context_id}"));
        self.client
            .execute("runtime.code_context.delete", || {
                self.client.delete(url.clone())
            })
            .await
    }

    // ------------------------------------------------------------------
    // Execute command (non-interactive)
    // ------------------------------------------------------------------

    /// `POST /v1/agents/runtime/<id>/commands/run`
    ///
    /// Runs a shell command inside the runtime and returns output.
    pub async fn exec_command(
        &self,
        runtime_id: &str,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value, ApiError> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/commands/run"));
        self.client
            .execute("runtime.exec_command", || {
                self.client.post(url.clone()).json(payload)
            })
            .await
    }

    /// `POST /v1/agents/runtime/<id>/commands/run?stream=true`
    pub async fn exec_command_stream(
        &self,
        runtime_id: &str,
        payload: &serde_json::Value,
    ) -> Result<reqwest::Response, ApiError> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/commands/run"));
        let resp = self
            .client
            .post(url)
            .query(&[("stream", "true")])
            .bearer_auth(self.client.api_key().expose_secret())
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
    // Run code
    // ------------------------------------------------------------------

    /// `POST /v1/agents/runtime/<id>/code/run`
    ///
    /// Executes a code snippet inside the runtime and returns output.
    pub async fn run_code(
        &self,
        runtime_id: &str,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value, ApiError> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/code/run"));
        self.client
            .execute("runtime.run_code", || {
                self.client.post(url.clone()).json(payload)
            })
            .await
    }

    /// `POST /v1/agents/runtime/<id>/ssh/enable`
    pub async fn enable_ssh(
        &self,
        runtime_id: &str,
        regenerate_keys: bool,
    ) -> Result<SshEnableResponse, ApiError> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/ssh/enable"));
        self.client
            .execute("runtime.ssh.enable", || {
                let builder = self.client.post(url.clone()).json(&serde_json::json!({}));
                if regenerate_keys {
                    builder.query(&[("regenerate_keys", "true")])
                } else {
                    builder
                }
            })
            .await
    }

    /// `POST /v1/agents/runtime/<id>/ssh/disable`
    pub async fn disable_ssh(&self, runtime_id: &str) -> Result<SshDisableResponse, ApiError> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/ssh/disable"));
        self.client
            .execute("runtime.ssh.disable", || {
                self.client.post(url.clone()).json(&serde_json::json!({}))
            })
            .await
    }

    /// `GET /v1/agents/runtime/<id>/ssh/status`
    pub async fn ssh_status(&self, runtime_id: &str) -> Result<SshStatusResponse, ApiError> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/ssh/status"));
        self.client
            .execute("runtime.ssh.status", || self.client.get(url.clone()))
            .await
    }

    // ------------------------------------------------------------------
    // Poll until RUNNING
    // ------------------------------------------------------------------

    /// Poll `GET /v1/agents/runtime/<id>` until `status == "running"` or the
    /// deadline is reached.
    ///
    /// Returns the final `Runtime` object.  Polls with 2-second intervals
    /// backed off using the same exponential formula as the retry layer to avoid
    /// thundering-herd on startup.
    pub async fn wait_until_running(
        &self,
        runtime_id: &str,
        deadline: std::time::Instant,
    ) -> Result<Runtime, ApiError> {
        use tokio::time::{sleep, Duration};
        let mut interval = Duration::from_secs(2);
        loop {
            let rt = self.get(runtime_id).await?;
            match rt.status.as_str() {
                "running" => return Ok(rt),
                "failed" | "terminated" | "error" => {
                    return Err(ApiError::Server {
                        status: 500,
                        body: format!("runtime entered terminal state: {}", rt.status),
                    });
                }
                _ => {}
            }
            if std::time::Instant::now() >= deadline {
                return Err(ApiError::Server {
                    status: 408,
                    body: format!(
                        "timed out waiting for runtime {runtime_id} to reach 'running' state (current: {})",
                        rt.status
                    ),
                });
            }
            sleep(interval).await;
            interval = (interval * 2).min(Duration::from_secs(10));
        }
    }
}
