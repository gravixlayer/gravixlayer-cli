use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::ApiClient;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Outcome of one git invocation in the runtime.
///
/// Mirrors the API's git response exactly: the fields are what `git(1)` itself
/// produced, not a parsed interpretation of it. Callers that want structured
/// data (a branch list, a commit hash) read it out of `stdout`, which is the
/// porcelain output of the corresponding git command.
#[derive(Debug, Serialize, Deserialize)]
pub struct GitOperationResult {
    pub success: Option<bool>,
    /// Standard output from the git subprocess (JSON key: `stdout` or `output`).
    #[serde(alias = "output")]
    pub stdout: Option<String>,
    /// Standard error from the git subprocess (JSON key: `stderr`).
    pub stderr: Option<String>,
    /// Structural error message from the backend (JSON key: `error`).
    pub error: Option<String>,
    pub exit_code: Option<i32>,
}

// ---------------------------------------------------------------------------
// API implementation
// ---------------------------------------------------------------------------

/// Git operations against `POST /v1/agents/runtime/{id}/git/...`
pub struct RuntimeGitApi<'a> {
    client: &'a ApiClient,
}

impl<'a> RuntimeGitApi<'a> {
    pub fn new(client: &'a ApiClient) -> Self {
        Self { client }
    }

    async fn post_json(
        &self,
        runtime_id: &str,
        op: &str,
        body: serde_json::Value,
    ) -> Result<GitOperationResult> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/git/{op}"));
        let resp = self
            .client
            .http_client()
            .post(&url)
            .bearer_auth(self.client.api_key_str())
            .json(&body)
            .send()
            .await
            .with_context(|| format!("runtime git {op} request"))?;
        ApiClient::check_status(resp)
            .await?
            .json()
            .await
            .with_context(|| format!("parse git {op} response"))
    }

    /// Clone a repository into the runtime.
    ///
    /// `auth_token` authenticates this clone only; it is not stored in the
    /// checkout, so `pull`, `fetch`, and `push` each take their own token.
    pub async fn clone(
        &self,
        runtime_id: &str,
        url: &str,
        path: &str,
        branch: Option<&str>,
        depth: Option<u32>,
        auth_token: Option<&str>,
    ) -> Result<GitOperationResult> {
        let mut body = serde_json::json!({ "url": url, "path": path });
        if let Some(b) = branch {
            body["branch"] = b.into();
        }
        if let Some(d) = depth {
            body["depth"] = (d as i64).into();
        }
        if let Some(t) = auth_token {
            body["auth_token"] = t.into();
        }
        self.post_json(runtime_id, "clone", body).await
    }

    /// Git status for a repository path.
    pub async fn status(&self, runtime_id: &str, path: &str) -> Result<GitOperationResult> {
        let body = serde_json::json!({ "repository_path": path });
        self.post_json(runtime_id, "status", body).await
    }

    /// List branches.
    pub async fn branches(
        &self,
        runtime_id: &str,
        path: &str,
        scope: Option<&str>,
    ) -> Result<GitOperationResult> {
        let mut body = serde_json::json!({ "repository_path": path });
        if let Some(s) = scope {
            body["scope"] = s.into();
        }
        self.post_json(runtime_id, "branches", body).await
    }

    /// Checkout a branch or ref.
    pub async fn checkout(
        &self,
        runtime_id: &str,
        path: &str,
        ref_name: &str,
    ) -> Result<GitOperationResult> {
        let body = serde_json::json!({ "repository_path": path, "ref_name": ref_name });
        self.post_json(runtime_id, "checkout", body).await
    }

    /// Pull from remote. `auth_token` authenticates this operation only.
    pub async fn pull(
        &self,
        runtime_id: &str,
        path: &str,
        remote: Option<&str>,
        branch: Option<&str>,
        auth_token: Option<&str>,
    ) -> Result<GitOperationResult> {
        let mut body = serde_json::json!({ "repository_path": path });
        if let Some(r) = remote {
            body["remote"] = r.into();
        }
        if let Some(b) = branch {
            body["branch"] = b.into();
        }
        if let Some(t) = auth_token {
            body["auth_token"] = t.into();
        }
        self.post_json(runtime_id, "pull", body).await
    }

    /// Push to remote. `auth_token` takes precedence over `username`/`password`.
    #[allow(clippy::too_many_arguments)]
    pub async fn push(
        &self,
        runtime_id: &str,
        path: &str,
        remote: Option<&str>,
        refspec: Option<&str>,
        username: Option<&str>,
        password: Option<&str>,
        auth_token: Option<&str>,
    ) -> Result<GitOperationResult> {
        let mut body = serde_json::json!({ "repository_path": path });
        if let Some(r) = remote {
            body["remote"] = r.into();
        }
        if let Some(r) = refspec {
            body["refspec"] = r.into();
        }
        if let Some(u) = username {
            body["username"] = u.into();
        }
        if let Some(p) = password {
            body["password"] = p.into();
        }
        if let Some(t) = auth_token {
            body["auth_token"] = t.into();
        }
        self.post_json(runtime_id, "push", body).await
    }

    /// Fetch from remote. `auth_token` authenticates this operation only.
    pub async fn fetch(
        &self,
        runtime_id: &str,
        path: &str,
        remote: Option<&str>,
        auth_token: Option<&str>,
    ) -> Result<GitOperationResult> {
        let mut body = serde_json::json!({ "repository_path": path });
        if let Some(r) = remote {
            body["remote"] = r.into();
        }
        if let Some(t) = auth_token {
            body["auth_token"] = t.into();
        }
        self.post_json(runtime_id, "fetch", body).await
    }

    /// Stage files (`git add`).
    pub async fn add(
        &self,
        runtime_id: &str,
        path: &str,
        files: &[String],
    ) -> Result<GitOperationResult> {
        let mut body = serde_json::json!({ "repository_path": path });
        if !files.is_empty() {
            body["paths"] = serde_json::Value::Array(
                files
                    .iter()
                    .map(|f| serde_json::Value::String(f.clone()))
                    .collect(),
            );
        }
        self.post_json(runtime_id, "add", body).await
    }

    /// Create a commit.
    pub async fn commit(
        &self,
        runtime_id: &str,
        path: &str,
        message: &str,
        author_name: Option<&str>,
        author_email: Option<&str>,
        allow_empty: bool,
    ) -> Result<GitOperationResult> {
        let mut body = serde_json::json!({ "repository_path": path, "message": message });
        if let Some(n) = author_name {
            body["author_name"] = n.into();
        }
        if let Some(e) = author_email {
            body["author_email"] = e.into();
        }
        if allow_empty {
            body["allow_empty"] = true.into();
        }
        self.post_json(runtime_id, "commit", body).await
    }

    /// Create a new branch.
    pub async fn create_branch(
        &self,
        runtime_id: &str,
        path: &str,
        branch_name: &str,
        start_point: Option<&str>,
    ) -> Result<GitOperationResult> {
        let mut body = serde_json::json!({
            "repository_path": path,
            "branch_name": branch_name,
        });
        if let Some(s) = start_point {
            body["start_point"] = s.into();
        }
        self.post_json(runtime_id, "branch/create", body).await
    }

    /// Delete a branch.
    pub async fn delete_branch(
        &self,
        runtime_id: &str,
        path: &str,
        branch_name: &str,
        force: bool,
    ) -> Result<GitOperationResult> {
        let mut body = serde_json::json!({
            "repository_path": path,
            "branch_name": branch_name,
        });
        if force {
            body["force"] = true.into();
        }
        self.post_json(runtime_id, "branch/delete", body).await
    }
}
