// src/api/billing.rs — Billing and quota API methods.
//
// Paths match Go `internal/agents/usage` routes and the control-plane contract:
//   GET /v1/agents/billing/summary?month=YYYY-MM&project_id=UUID
//   GET /v1/agents/billing/history?page=&page_size=&start_time=&end_time=&runtime_id=&status=&project_id=
//   GET /v1/agents/quota

use super::{error::ApiError, types::*, ApiClient};

pub struct BillingApi<'a> {
    client: &'a ApiClient,
}

impl<'a> BillingApi<'a> {
    pub(crate) fn new(client: &'a ApiClient) -> Self {
        Self { client }
    }

    /// `GET /v1/agents/billing/summary?month=YYYY-MM&project_id=UUID`
    pub async fn summary(
        &self,
        month: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<BillingSummary, ApiError> {
        let url = self.client.agents_url("billing/summary");
        self.client
            .execute("billing.summary", || {
                let mut req = self.client.get(url.clone());
                if let Some(m) = month {
                    req = req.query(&[("month", m)]);
                }
                if let Some(pid) = project_id {
                    req = req.query(&[("project_id", pid)]);
                }
                req
            })
            .await
    }

    /// `GET /v1/agents/billing/history`
    #[allow(clippy::too_many_arguments)]
    pub async fn history(
        &self,
        page: u32,
        page_size: u32,
        from: Option<&str>,
        to: Option<&str>,
        runtime_id: Option<&str>,
        status: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<BillingHistory, ApiError> {
        let url = self.client.agents_url("billing/history");
        self.client
            .execute("billing.history", || {
                let mut req = self
                    .client
                    .get(url.clone())
                    .query(&[("page", page), ("page_size", page_size)]);
                if let Some(f) = from {
                    req = req.query(&[("start_time", f)]);
                }
                if let Some(t) = to {
                    req = req.query(&[("end_time", t)]);
                }
                if let Some(rid) = runtime_id {
                    req = req.query(&[("runtime_id", rid)]);
                }
                if let Some(s) = status {
                    req = req.query(&[("status", s)]);
                }
                if let Some(pid) = project_id {
                    req = req.query(&[("project_id", pid)]);
                }
                req
            })
            .await
    }

    /// `GET /v1/agents/quota`
    pub async fn quotas(&self) -> Result<BillingQuota, ApiError> {
        let url = self.client.agents_url("quota");
        self.client
            .execute("billing.quotas", || self.client.get(url.clone()))
            .await
    }
}
