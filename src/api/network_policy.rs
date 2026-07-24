// src/api/network_policy.rs — Network Policies API (/v1/network-policies).

use super::{error::ApiError, types::*, ApiClient};

pub struct NetworkPolicyApi<'a> {
    client: &'a ApiClient,
}

impl<'a> NetworkPolicyApi<'a> {
    pub(crate) fn new(client: &'a ApiClient) -> Self {
        Self { client }
    }

    fn base(&self) -> String {
        self.client.url("v1/network-policies")
    }

    /// `POST /v1/network-policies`
    pub async fn create(
        &self,
        req: &CreateNetworkPolicyRequest,
        project_id: Option<&str>,
    ) -> Result<NetworkPolicy, ApiError> {
        let url = self.base();
        let env = self
            .client
            .execute::<NetworkPolicyEnvelope>("network_policy.create", || {
                let mut b = self.client.post(url.clone()).json(req);
                if let Some(pid) = project_id {
                    b = b.query(&[("project_id", pid)]);
                }
                b
            })
            .await?;
        Ok(env.policy)
    }

    /// `GET /v1/network-policies`
    pub async fn list(
        &self,
        limit: u32,
        offset: u32,
        project_id: Option<&str>,
        search: Option<&str>,
    ) -> Result<NetworkPolicyList, ApiError> {
        let url = self.base();
        self.client
            .execute("network_policy.list", || {
                let mut b = self
                    .client
                    .get(url.clone())
                    .query(&[("limit", limit), ("offset", offset)]);
                if let Some(pid) = project_id {
                    b = b.query(&[("project_id", pid)]);
                }
                if let Some(s) = search {
                    b = b.query(&[("search", s)]);
                }
                b
            })
            .await
    }

    /// `GET /v1/network-policies/:id`
    pub async fn get(&self, policy_id: &str) -> Result<NetworkPolicy, ApiError> {
        let url = format!("{}/{}", self.base(), policy_id);
        let env = self
            .client
            .execute::<NetworkPolicyEnvelope>("network_policy.get", || self.client.get(url.clone()))
            .await?;
        Ok(env.policy)
    }

    /// `PATCH /v1/network-policies/:id`
    pub async fn update(
        &self,
        policy_id: &str,
        req: &UpdateNetworkPolicyRequest,
        project_id: Option<&str>,
    ) -> Result<NetworkPolicy, ApiError> {
        let url = format!("{}/{}", self.base(), policy_id);
        let env = self
            .client
            .execute::<NetworkPolicyEnvelope>("network_policy.update", || {
                let mut b = self.client.patch(url.clone()).json(req);
                if let Some(pid) = project_id {
                    b = b.query(&[("project_id", pid)]);
                }
                b
            })
            .await?;
        Ok(env.policy)
    }

    /// `DELETE /v1/network-policies/:id`
    pub async fn delete(
        &self,
        policy_id: &str,
        project_id: Option<&str>,
    ) -> Result<SuccessEnvelope, ApiError> {
        let url = format!("{}/{}", self.base(), policy_id);
        self.client
            .execute("network_policy.delete", || {
                let mut b = self.client.delete(url.clone());
                if let Some(pid) = project_id {
                    b = b.query(&[("project_id", pid)]);
                }
                b
            })
            .await
    }

    /// `POST /v1/network-policies/:id/rules`
    pub async fn add_rule(
        &self,
        policy_id: &str,
        req: &AddNetworkPolicyRuleRequest,
        project_id: Option<&str>,
    ) -> Result<NetworkPolicyRule, ApiError> {
        let url = format!("{}/{}/rules", self.base(), policy_id);
        let env = self
            .client
            .execute::<NetworkPolicyRuleEnvelope>("network_policy.add_rule", || {
                let mut b = self.client.post(url.clone()).json(req);
                if let Some(pid) = project_id {
                    b = b.query(&[("project_id", pid)]);
                }
                b
            })
            .await?;
        Ok(env.rule)
    }

    /// `GET /v1/network-policies/:id/rules`
    pub async fn list_rules(&self, policy_id: &str) -> Result<NetworkPolicyRuleList, ApiError> {
        let url = format!("{}/{}/rules", self.base(), policy_id);
        self.client
            .execute("network_policy.list_rules", || self.client.get(url.clone()))
            .await
    }

    /// `PATCH /v1/network-policies/:id/rules/:rule_id`
    pub async fn update_rule(
        &self,
        policy_id: &str,
        rule_id: &str,
        req: &UpdateNetworkPolicyRuleRequest,
        project_id: Option<&str>,
    ) -> Result<NetworkPolicyRule, ApiError> {
        let url = format!("{}/{}/rules/{}", self.base(), policy_id, rule_id);
        let env = self
            .client
            .execute::<NetworkPolicyRuleEnvelope>("network_policy.update_rule", || {
                let mut b = self.client.patch(url.clone()).json(req);
                if let Some(pid) = project_id {
                    b = b.query(&[("project_id", pid)]);
                }
                b
            })
            .await?;
        Ok(env.rule)
    }

    /// `DELETE /v1/network-policies/:id/rules/:rule_id`
    pub async fn delete_rule(
        &self,
        policy_id: &str,
        rule_id: &str,
        project_id: Option<&str>,
    ) -> Result<SuccessEnvelope, ApiError> {
        let url = format!("{}/{}/rules/{}", self.base(), policy_id, rule_id);
        self.client
            .execute("network_policy.delete_rule", || {
                let mut b = self.client.delete(url.clone());
                if let Some(pid) = project_id {
                    b = b.query(&[("project_id", pid)]);
                }
                b
            })
            .await
    }

    /// `POST /v1/network-policies/:id/attach`
    pub async fn attach(
        &self,
        policy_id: &str,
        runtime_id: &str,
        project_id: Option<&str>,
    ) -> Result<SuccessEnvelope, ApiError> {
        let url = format!("{}/{}/attach", self.base(), policy_id);
        let body = AttachNetworkPolicyRequest {
            runtime_id: runtime_id.to_string(),
        };
        self.client
            .execute("network_policy.attach", || {
                let mut b = self.client.post(url.clone()).json(&body);
                if let Some(pid) = project_id {
                    b = b.query(&[("project_id", pid)]);
                }
                b
            })
            .await
    }

    /// `DELETE /v1/network-policies/:id/attach/:runtime_id`
    pub async fn detach(
        &self,
        policy_id: &str,
        runtime_id: &str,
        project_id: Option<&str>,
    ) -> Result<SuccessEnvelope, ApiError> {
        let url = format!("{}/{}/attach/{}", self.base(), policy_id, runtime_id);
        self.client
            .execute("network_policy.detach", || {
                let mut b = self.client.delete(url.clone());
                if let Some(pid) = project_id {
                    b = b.query(&[("project_id", pid)]);
                }
                b
            })
            .await
    }

    /// `GET /v1/network-policies/runtimes/:runtime_id`
    pub async fn list_for_runtime(&self, runtime_id: &str) -> Result<NetworkPolicyList, ApiError> {
        let url = format!("{}/runtimes/{}", self.base(), runtime_id);
        self.client
            .execute("network_policy.list_for_runtime", || {
                self.client.get(url.clone())
            })
            .await
    }
}
