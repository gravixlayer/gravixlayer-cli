// src/api/snapshot.rs — Named snapshot catalog API methods.

use std::time::Duration;

use super::{error::ApiError, types::*, ApiClient};

/// Capture can pause the guest, pack overlay extents, and write a Full
/// Firecracker snapshot. Matches the control-plane 10-minute gRPC deadline.
const SNAPSHOT_CREATE_TIMEOUT: Duration = Duration::from_secs(600);

pub struct SnapshotApi<'a> {
    client: &'a ApiClient,
}

impl<'a> SnapshotApi<'a> {
    pub(crate) fn new(client: &'a ApiClient) -> Self {
        Self { client }
    }

    /// `POST /v1/agents/snapshots`
    pub async fn create(&self, req: CreateSnapshotRequest) -> Result<Snapshot, ApiError> {
        let url = self.client.agents_url("snapshots");
        self.client
            .execute("snapshot.create", || {
                self.client
                    .post(url.clone())
                    .timeout(SNAPSHOT_CREATE_TIMEOUT)
                    .json(&req)
            })
            .await
    }

    /// `GET /v1/agents/snapshots?limit=&offset=&kind=&runtime_id=&state=&source=`
    pub async fn list(
        &self,
        limit: u32,
        offset: u32,
        kind: Option<&str>,
        runtime_id: Option<&str>,
        state: Option<&str>,
        source: Option<&str>,
    ) -> Result<SnapshotList, ApiError> {
        let url = self.client.agents_url("snapshots");
        self.client
            .execute("snapshot.list", || {
                let mut req = self
                    .client
                    .get(url.clone())
                    .query(&[("limit", limit.to_string()), ("offset", offset.to_string())]);
                if let Some(kind) = kind {
                    req = req.query(&[("kind", kind)]);
                }
                if let Some(runtime_id) = runtime_id {
                    req = req.query(&[("runtime_id", runtime_id)]);
                }
                if let Some(state) = state {
                    req = req.query(&[("state", state)]);
                }
                if let Some(source) = source {
                    req = req.query(&[("source", source)]);
                }
                req
            })
            .await
    }

    /// `GET /v1/agents/snapshots/<id_or_name>`
    pub async fn get(&self, id: &str) -> Result<Snapshot, ApiError> {
        let url = self.client.agents_url(&format!("snapshots/{id}"));
        self.client
            .execute("snapshot.get", || self.client.get(url.clone()))
            .await
    }

    /// `POST /v1/agents/snapshots/<id>/activate`
    pub async fn activate(&self, id: &str) -> Result<Snapshot, ApiError> {
        let url = self.client.agents_url(&format!("snapshots/{id}/activate"));
        self.client
            .execute("snapshot.activate", || {
                self.client.post(url.clone()).json(&serde_json::json!({}))
            })
            .await
    }

    /// `POST /v1/agents/snapshots/<id>/deactivate`
    pub async fn deactivate(&self, id: &str) -> Result<Snapshot, ApiError> {
        let url = self
            .client
            .agents_url(&format!("snapshots/{id}/deactivate"));
        self.client
            .execute("snapshot.deactivate", || {
                self.client.post(url.clone()).json(&serde_json::json!({}))
            })
            .await
    }

    /// `DELETE /v1/agents/snapshots/<id_or_name>`
    pub async fn delete(&self, id: &str) -> Result<(), ApiError> {
        let url = self.client.agents_url(&format!("snapshots/{id}"));
        self.client
            .execute_empty("snapshot.delete", || self.client.delete(url.clone()))
            .await
    }
}
