//! Programmatic PTY sessions: `/v1/agents/runtime/{id}/pty/...`
//!
//! A PTY session is a real pseudo-terminal allocated inside the runtime and owned by
//! the execution plane rather than by the connection that created it. It survives
//! client disconnects, so a session can be created, driven, detached from, and
//! re-attached to later with its scrollback intact.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ApiClient;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct PtySession {
    pub session_id: String,
    #[serde(default)]
    pub runtime_id: String,
    #[serde(default)]
    pub pid: u32,
    #[serde(default)]
    pub shell: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub working_dir: String,
    #[serde(default)]
    pub cols: u32,
    #[serde(default)]
    pub rows: u32,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub exit_code: i32,
    pub created_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PtySessionList {
    #[serde(default)]
    pub sessions: Vec<PtySession>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PtyInputResponse {
    pub success: Option<bool>,
    pub bytes_written: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PtyAckResponse {
    pub success: Option<bool>,
}

/// Optional parameters for a new PTY session. `None` means "use the runtime default".
#[derive(Debug, Default)]
pub struct PtyCreateParams {
    pub shell: Option<String>,
    pub working_dir: Option<String>,
    pub environment: HashMap<String, String>,
    pub cols: Option<u32>,
    pub rows: Option<u32>,
}

// ---------------------------------------------------------------------------
// API implementation
// ---------------------------------------------------------------------------

pub struct RuntimePtyApi<'a> {
    client: &'a ApiClient,
}

impl<'a> RuntimePtyApi<'a> {
    pub fn new(client: &'a ApiClient) -> Self {
        Self { client }
    }

    /// `POST /v1/agents/runtime/{id}/pty`
    pub async fn create(&self, runtime_id: &str, params: &PtyCreateParams) -> Result<PtySession> {
        let url = self.client.agents_url(&format!("runtime/{runtime_id}/pty"));
        let mut body = serde_json::Map::new();
        if let Some(shell) = &params.shell {
            body.insert("shell".into(), serde_json::Value::String(shell.clone()));
        }
        if let Some(working_dir) = &params.working_dir {
            body.insert(
                "working_dir".into(),
                serde_json::Value::String(working_dir.clone()),
            );
        }
        if !params.environment.is_empty() {
            body.insert("environment".into(), serde_json::json!(params.environment));
        }
        if let Some(cols) = params.cols {
            body.insert("cols".into(), serde_json::json!(cols));
        }
        if let Some(rows) = params.rows {
            body.insert("rows".into(), serde_json::json!(rows));
        }

        let resp = self
            .client
            .http_client()
            .post(&url)
            .bearer_auth(self.client.api_key_str())
            .json(&serde_json::Value::Object(body))
            .send()
            .await
            .context("pty create request")?;
        ApiClient::check_status(resp)
            .await?
            .json()
            .await
            .context("parse pty session")
    }

    /// `GET /v1/agents/runtime/{id}/pty`
    pub async fn list(&self, runtime_id: &str) -> Result<PtySessionList> {
        let url = self.client.agents_url(&format!("runtime/{runtime_id}/pty"));
        let resp = self
            .client
            .http_client()
            .get(&url)
            .bearer_auth(self.client.api_key_str())
            .send()
            .await
            .context("pty list request")?;
        ApiClient::check_status(resp)
            .await?
            .json()
            .await
            .context("parse pty session list")
    }

    /// `GET /v1/agents/runtime/{id}/pty/{session_id}`
    pub async fn get(&self, runtime_id: &str, session_id: &str) -> Result<PtySession> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/pty/{session_id}"));
        let resp = self
            .client
            .http_client()
            .get(&url)
            .bearer_auth(self.client.api_key_str())
            .send()
            .await
            .context("pty get request")?;
        ApiClient::check_status(resp)
            .await?
            .json()
            .await
            .context("parse pty session")
    }

    /// `POST /v1/agents/runtime/{id}/pty/{session_id}/input`
    ///
    /// Bytes are base64 encoded so control characters and escape sequences survive
    /// the JSON transport intact.
    pub async fn send_input(
        &self,
        runtime_id: &str,
        session_id: &str,
        data: &[u8],
    ) -> Result<PtyInputResponse> {
        use base64::Engine as _;

        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/pty/{session_id}/input"));
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        let body = serde_json::json!({ "data_base64": encoded });
        let resp = self
            .client
            .http_client()
            .post(&url)
            .bearer_auth(self.client.api_key_str())
            .json(&body)
            .send()
            .await
            .context("pty input request")?;
        ApiClient::check_status(resp)
            .await?
            .json()
            .await
            .context("parse pty input response")
    }

    /// `POST /v1/agents/runtime/{id}/pty/{session_id}/resize`
    pub async fn resize(
        &self,
        runtime_id: &str,
        session_id: &str,
        cols: u32,
        rows: u32,
    ) -> Result<PtyAckResponse> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/pty/{session_id}/resize"));
        let body = serde_json::json!({ "cols": cols, "rows": rows });
        let resp = self
            .client
            .http_client()
            .post(&url)
            .bearer_auth(self.client.api_key_str())
            .json(&body)
            .send()
            .await
            .context("pty resize request")?;
        ApiClient::check_status(resp)
            .await?
            .json()
            .await
            .context("parse pty resize response")
    }

    /// `POST /v1/agents/runtime/{id}/pty/{session_id}/signal`
    pub async fn signal(
        &self,
        runtime_id: &str,
        session_id: &str,
        signal: &str,
    ) -> Result<PtyAckResponse> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/pty/{session_id}/signal"));
        let body = serde_json::json!({ "signal": signal });
        let resp = self
            .client
            .http_client()
            .post(&url)
            .bearer_auth(self.client.api_key_str())
            .json(&body)
            .send()
            .await
            .context("pty signal request")?;
        ApiClient::check_status(resp)
            .await?
            .json()
            .await
            .context("parse pty signal response")
    }

    /// `GET /v1/agents/runtime/{id}/pty/{session_id}/stream`
    ///
    /// Returns the raw streaming response; the caller consumes the SSE frames.
    /// The stream begins with the session's retained scrollback.
    pub async fn stream(&self, runtime_id: &str, session_id: &str) -> Result<reqwest::Response> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/pty/{session_id}/stream"));
        let resp = self
            .client
            .http_client()
            .get(&url)
            .bearer_auth(self.client.api_key_str())
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .send()
            .await
            .context("pty stream request")?;
        ApiClient::check_status(resp).await
    }

    /// `DELETE /v1/agents/runtime/{id}/pty/{session_id}`
    pub async fn kill(&self, runtime_id: &str, session_id: &str) -> Result<PtyAckResponse> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/pty/{session_id}"));
        let resp = self
            .client
            .http_client()
            .delete(&url)
            .bearer_auth(self.client.api_key_str())
            .send()
            .await
            .context("pty kill request")?;
        ApiClient::check_status(resp)
            .await?
            .json()
            .await
            .context("parse pty kill response")
    }
}
