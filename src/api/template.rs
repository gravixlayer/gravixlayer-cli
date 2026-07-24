// src/api/template.rs — Template resource API methods.

use super::{error::ApiError, types::*, ApiClient};

pub struct TemplateApi<'a> {
    client: &'a ApiClient,
}

impl<'a> TemplateApi<'a> {
    pub(crate) fn new(client: &'a ApiClient) -> Self {
        Self { client }
    }

    /// `GET /v1/agents/template?limit=N&offset=N&kind=…`
    ///
    /// Control-plane default for omitted `kind` is `sandbox` (runtime templates).
    /// Pass `kind=agent` or `kind=all` to match the Python SDK.
    pub async fn list(
        &self,
        limit: u32,
        offset: u32,
        kind: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<TemplateList, ApiError> {
        let url = self.client.agents_url("template");
        let kind = kind.unwrap_or("sandbox");
        self.client
            .execute("template.list", || {
                let mut req = self.client.get(url.clone()).query(&[
                    ("limit", limit.to_string()),
                    ("offset", offset.to_string()),
                    ("kind", kind.to_string()),
                ]);
                if let Some(pid) = project_id {
                    req = req.query(&[("project_id", pid)]);
                }
                req
            })
            .await
    }

    /// `GET /v1/agents/template/<id>`
    pub async fn get(&self, id: &str) -> Result<Template, ApiError> {
        let url = self.client.agents_url(&format!("template/{id}"));
        self.client
            .execute("template.get", || self.client.get(url.clone()))
            .await
    }

    /// `GET /v1/agents/template/<id>/snapshot`
    pub async fn snapshot(&self, id: &str) -> Result<TemplateSnapshot, ApiError> {
        let url = self.client.agents_url(&format!("template/{id}/snapshot"));
        self.client
            .execute("template.snapshot", || self.client.get(url.clone()))
            .await
    }

    /// `DELETE /v1/agents/template/<id>`
    pub async fn delete(&self, id: &str) -> Result<(), ApiError> {
        let url = self.client.agents_url(&format!("template/{id}"));
        self.client
            .execute_empty("template.delete", || self.client.delete(url.clone()))
            .await
    }

    /// `GET /v1/agents/template/builds/<build_id>/status`
    pub async fn build_status(&self, build_id: &str) -> Result<TemplateBuildStatus, ApiError> {
        let url = self
            .client
            .agents_url(&format!("template/builds/{build_id}/status"));
        self.client
            .execute("template.build_status", || self.client.get(url.clone()))
            .await
    }

    /// Poll `build_status` until status is `"completed"` or `"failed"`.
    ///
    /// Returns the final `TemplateBuildStatus`.
    pub async fn wait_for_build(
        &self,
        build_id: &str,
        deadline: std::time::Instant,
    ) -> Result<TemplateBuildStatus, ApiError> {
        use tokio::time::{sleep, Duration};
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
                        body: format!("template build {build_id} failed: {detail}"),
                    });
                }
                _ => {}
            }
            if std::time::Instant::now() >= deadline {
                return Err(ApiError::Server {
                    status: 408,
                    body: format!("timed out waiting for template build {build_id}"),
                });
            }
            sleep(interval).await;
            interval = (interval * 2).min(Duration::from_secs(15));
        }
    }
}
