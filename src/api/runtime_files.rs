use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::ApiClient;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: Option<String>,
    pub size: u64,
    pub is_dir: bool,
    pub modified_at: Option<String>,
    pub mode: Option<String>,
    pub permissions: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileListResponse {
    pub files: Vec<FileInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileReadResponse {
    pub content: String,
    pub path: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileWriteResponse {
    pub success: Option<bool>,
    pub path: Option<String>,
    pub bytes_written: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileDeleteResponse {
    pub success: Option<bool>,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirCreateResponse {
    pub success: Option<bool>,
    pub message: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChmodResponse {
    pub success: Option<bool>,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResult {
    pub path: String,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub file_type: Option<String>,
    pub size: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfoResponse {
    pub exists: bool,
    pub info: Option<FileInfo>,
}

// ---------------------------------------------------------------------------
// API implementation
// ---------------------------------------------------------------------------

/// Filesystem operations against `POST /v1/agents/runtime/{id}/files/...`
pub struct RuntimeFilesApi<'a> {
    client: &'a ApiClient,
}

impl<'a> RuntimeFilesApi<'a> {
    pub fn new(client: &'a ApiClient) -> Self {
        Self { client }
    }

    /// `POST /v1/agents/runtime/{id}/files/list`
    pub async fn list(&self, runtime_id: &str, path: &str) -> Result<FileListResponse> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/files/list"));
        let body = serde_json::json!({ "path": path });
        let resp = self
            .client
            .http_client()
            .post(&url)
            .bearer_auth(self.client.api_key_str())
            .json(&body)
            .send()
            .await
            .context("runtime files list request")?;
        ApiClient::check_status(resp)
            .await?
            .json()
            .await
            .context("parse file list")
    }

    /// `POST /v1/agents/runtime/{id}/files/read`
    pub async fn read(&self, runtime_id: &str, path: &str) -> Result<FileReadResponse> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/files/read"));
        let body = serde_json::json!({ "path": path });
        let resp = self
            .client
            .http_client()
            .post(&url)
            .bearer_auth(self.client.api_key_str())
            .json(&body)
            .send()
            .await
            .context("runtime files read request")?;
        ApiClient::check_status(resp)
            .await?
            .json()
            .await
            .context("parse file read")
    }

    /// `POST /v1/agents/runtime/{id}/files/write`
    pub async fn write(
        &self,
        runtime_id: &str,
        path: &str,
        content: &str,
    ) -> Result<FileWriteResponse> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/files/write"));
        let body = serde_json::json!({ "path": path, "content": content });
        let resp = self
            .client
            .http_client()
            .post(&url)
            .bearer_auth(self.client.api_key_str())
            .json(&body)
            .send()
            .await
            .context("runtime files write request")?;
        ApiClient::check_status(resp)
            .await?
            .json()
            .await
            .context("parse file write")
    }

    /// `POST /v1/agents/runtime/{id}/files/delete`
    pub async fn delete(&self, runtime_id: &str, path: &str) -> Result<FileDeleteResponse> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/files/delete"));
        let body = serde_json::json!({ "path": path });
        let resp = self
            .client
            .http_client()
            .post(&url)
            .bearer_auth(self.client.api_key_str())
            .json(&body)
            .send()
            .await
            .context("runtime files delete request")?;
        ApiClient::check_status(resp)
            .await?
            .json()
            .await
            .context("parse file delete")
    }

    /// `POST /v1/agents/runtime/{id}/files/create-directory`
    pub async fn mkdir(
        &self,
        runtime_id: &str,
        path: &str,
        recursive: bool,
        mode: Option<&str>,
    ) -> Result<DirCreateResponse> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/files/create-directory"));
        let mut body = serde_json::json!({ "path": path, "recursive": recursive });
        if let Some(mode) = mode {
            body["mode"] = mode.into();
        }
        let resp = self
            .client
            .http_client()
            .post(&url)
            .bearer_auth(self.client.api_key_str())
            .json(&body)
            .send()
            .await
            .context("runtime mkdir request")?;
        ApiClient::check_status(resp)
            .await?
            .json()
            .await
            .context("parse mkdir")
    }

    /// `POST /v1/agents/runtime/{id}/files/info`
    pub async fn info(&self, runtime_id: &str, path: &str) -> Result<FileInfoResponse> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/files/info"));
        let body = serde_json::json!({ "path": path });
        let resp = self
            .client
            .http_client()
            .post(&url)
            .bearer_auth(self.client.api_key_str())
            .json(&body)
            .send()
            .await
            .context("runtime file info request")?;
        ApiClient::check_status(resp)
            .await?
            .json()
            .await
            .context("parse file info")
    }

    /// `POST /v1/agents/runtime/{id}/files/set-mode`
    pub async fn chmod(&self, runtime_id: &str, path: &str, mode: &str) -> Result<ChmodResponse> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/files/set-mode"));
        let body = serde_json::json!({ "path": path, "mode": mode });
        let resp = self
            .client
            .http_client()
            .post(&url)
            .bearer_auth(self.client.api_key_str())
            .json(&body)
            .send()
            .await
            .context("runtime chmod request")?;
        ApiClient::check_status(resp)
            .await?
            .json()
            .await
            .context("parse chmod")
    }

    /// Multipart upload: `POST /v1/agents/runtime/{id}/files?path=<path>`
    pub async fn upload(
        &self,
        runtime_id: &str,
        local_path: &Path,
        remote_path: &str,
        user: Option<&str>,
        mode: Option<&str>,
    ) -> Result<UploadResult> {
        if !local_path.exists() {
            bail!("local path does not exist: {}", local_path.display());
        }
        let file_name = local_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());
        let part = reqwest::multipart::Part::file(local_path)
            .await
            .with_context(|| format!("open {}", local_path.display()))?
            .file_name(file_name)
            .mime_str("application/octet-stream")?;
        let form = reqwest::multipart::Form::new().part("file", part);

        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/files"));
        // URL-encode the remote path so special characters don't corrupt the query string.
        let mut query = vec![("path", remote_path.to_string())];
        if let Some(user) = user {
            query.push(("username", user.to_string()));
        }
        if let Some(mode) = mode {
            query.push(("mode", mode.to_string()));
        }
        let resp = self
            .client
            .http_client()
            .post(url)
            .query(&query)
            .bearer_auth(self.client.api_key_str())
            .multipart(form)
            .send()
            .await
            .context("runtime files upload")?;
        let checked = ApiClient::check_status(resp).await?;
        let json: serde_json::Value = checked.json().await.context("parse upload response")?;
        // API may return a list or a single object
        let entry = if json.is_array() {
            json.as_array()
                .and_then(|a| a.first().cloned())
                .unwrap_or(serde_json::json!({}))
        } else {
            json
        };
        Ok(UploadResult {
            path: entry
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or(remote_path)
                .to_string(),
            name: entry
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            file_type: entry
                .get("type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            size: entry.get("size").and_then(|v| v.as_u64()),
            error: entry
                .get("error")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        })
    }

    /// Multipart batch upload: `POST /v1/agents/runtime/{id}/files`.
    pub async fn write_many(
        &self,
        runtime_id: &str,
        files: &[(std::path::PathBuf, String)],
        user: Option<&str>,
    ) -> Result<Vec<UploadResult>> {
        let mut form = reqwest::multipart::Form::new();
        for (local_path, remote_path) in files {
            if !local_path.exists() {
                bail!("local path does not exist: {}", local_path.display());
            }
            let part = reqwest::multipart::Part::file(local_path)
                .await
                .with_context(|| format!("open {}", local_path.display()))?
                .file_name(remote_path.clone())
                .mime_str("application/octet-stream")?;
            form = form.part("file", part);
        }

        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/files"));
        let mut request = self
            .client
            .http_client()
            .post(url)
            .bearer_auth(self.client.api_key_str())
            .multipart(form);
        if let Some(user) = user {
            request = request.query(&[("username", user)]);
        }

        let resp = request.send().await.context("runtime files write-many")?;
        let checked = ApiClient::check_status(resp).await?;
        let json: serde_json::Value = checked.json().await.context("parse write-many response")?;
        Ok(parse_upload_results(json, None))
    }

    /// Download a file from the runtime and save to local disk.
    /// `GET /v1/agents/runtime/{id}/download?path=<path>` → write bytes to local_path.
    pub async fn download(
        &self,
        runtime_id: &str,
        remote_path: &str,
        local_path: &Path,
    ) -> Result<()> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/download"));
        let resp = self
            .client
            .http_client()
            .get(url)
            .bearer_auth(self.client.api_key_str())
            .query(&[("path", remote_path)])
            .send()
            .await
            .context("runtime file download request")?;
        let bytes = ApiClient::check_status(resp)
            .await?
            .bytes()
            .await
            .context("read download body")?;
        if let Some(parent) = local_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(local_path, &bytes)
            .await
            .with_context(|| format!("write {}", local_path.display()))?;
        Ok(())
    }
}

fn parse_upload_results(json: serde_json::Value, fallback_path: Option<&str>) -> Vec<UploadResult> {
    let entries = if let Some(array) = json.as_array() {
        array.clone()
    } else if let Some(array) = json.get("files").and_then(|files| files.as_array()) {
        array.clone()
    } else {
        vec![json]
    };

    entries
        .into_iter()
        .map(|entry| UploadResult {
            path: entry
                .get("path")
                .and_then(|v| v.as_str())
                .or(fallback_path)
                .unwrap_or_default()
                .to_string(),
            name: entry
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            file_type: entry
                .get("type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            size: entry.get("size").and_then(|v| v.as_u64()),
            error: entry
                .get("error")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        })
        .collect()
}
