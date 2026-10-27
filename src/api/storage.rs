// src/api/storage.rs — Storage mounts API.
//
// Buckets live at /v1/agents/buckets; runtime mounts at
// /v1/agents/runtime/<id>/mounts. Credentials are secret-provider references —
// secret material never crosses this client.

use super::{error::ApiError, types::*, ApiClient};

pub struct StorageApi<'a> {
    client: &'a ApiClient,
}

impl<'a> StorageApi<'a> {
    pub(crate) fn new(client: &'a ApiClient) -> Self {
        Self { client }
    }

    // ── buckets ────────────────────────────────────────────────────────────

    /// `POST /v1/agents/buckets`
    pub async fn create_bucket(
        &self,
        req: &CreateBucketRequest,
    ) -> Result<StorageBucket, ApiError> {
        let url = self.client.agents_url("buckets");
        let env = self
            .client
            .execute::<BucketEnvelope>("storage.bucket.create", || {
                self.client.post(url.clone()).json(req)
            })
            .await?;
        Ok(env.bucket)
    }

    /// `GET /v1/agents/buckets`
    pub async fn list_buckets(
        &self,
        project_id: Option<&str>,
    ) -> Result<BucketListEnvelope, ApiError> {
        let url = self.client.agents_url("buckets");
        self.client
            .execute("storage.bucket.list", || {
                let mut b = self.client.get(url.clone());
                if let Some(pid) = project_id {
                    b = b.query(&[("project_id", pid)]);
                }
                b
            })
            .await
    }

    /// `GET /v1/agents/buckets/<id>`
    pub async fn get_bucket(&self, id_or_name: &str) -> Result<StorageBucket, ApiError> {
        let url = self.client.agents_url(&format!("buckets/{id_or_name}"));
        let env = self
            .client
            .execute::<BucketEnvelope>("storage.bucket.get", || self.client.get(url.clone()))
            .await?;
        Ok(env.bucket)
    }

    /// `POST /v1/agents/buckets/<id>/verify`
    pub async fn verify_bucket(&self, id_or_name: &str) -> Result<StorageBucket, ApiError> {
        let url = self
            .client
            .agents_url(&format!("buckets/{id_or_name}/verify"));
        let env = self
            .client
            .execute::<BucketEnvelope>("storage.bucket.verify", || self.client.post(url.clone()))
            .await?;
        Ok(env.bucket)
    }

    /// `DELETE /v1/agents/buckets/<id>`
    pub async fn delete_bucket(&self, id_or_name: &str) -> Result<bool, ApiError> {
        let url = self.client.agents_url(&format!("buckets/{id_or_name}"));
        let body: serde_json::Value = self
            .client
            .execute("storage.bucket.delete", || self.client.delete(url.clone()))
            .await?;
        Ok(body
            .get("deleted")
            .and_then(|v| v.as_bool())
            .unwrap_or(true))
    }

    // ── runtime mounts ─────────────────────────────────────────────────────

    /// `POST /v1/agents/runtime/<id>/mounts`
    pub async fn attach_mount(
        &self,
        runtime_id: &str,
        mount: &MountSpec,
    ) -> Result<RuntimeMount, ApiError> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/mounts"));
        let env = self
            .client
            .execute::<MountEnvelope>("storage.mount.attach", || {
                self.client.post(url.clone()).json(mount)
            })
            .await?;
        Ok(env.attachment)
    }

    /// `GET /v1/agents/runtime/<id>/mounts`
    pub async fn list_mounts(&self, runtime_id: &str) -> Result<MountListEnvelope, ApiError> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/mounts"));
        self.client
            .execute("storage.mount.list", || self.client.get(url.clone()))
            .await
    }

    /// `DELETE /v1/agents/runtime/<id>/mounts/<attachment_id>`
    pub async fn detach_mount(
        &self,
        runtime_id: &str,
        attachment_id: &str,
    ) -> Result<bool, ApiError> {
        let url = self
            .client
            .agents_url(&format!("runtime/{runtime_id}/mounts/{attachment_id}"));
        let body: serde_json::Value = self
            .client
            .execute("storage.mount.detach", || self.client.delete(url.clone()))
            .await?;
        Ok(body
            .get("detached")
            .and_then(|v| v.as_bool())
            .unwrap_or(true))
    }
}
