// src/api/provider.rs — Secret Providers API (/v1/identity/providers).

use super::{error::ApiError, types::*, ApiClient};

pub struct ProviderApi<'a> {
    client: &'a ApiClient,
}

impl<'a> ProviderApi<'a> {
    pub(crate) fn new(client: &'a ApiClient) -> Self {
        Self { client }
    }

    fn base(&self) -> String {
        self.client.url("v1/identity/providers")
    }

    /// `POST /v1/identity/providers`
    pub async fn create(
        &self,
        req: &CreateSecretProviderRequest,
        project_id: Option<&str>,
    ) -> Result<SecretProvider, ApiError> {
        let url = self.base();
        let env = self
            .client
            .execute::<SecretProviderEnvelope>("provider.create", || {
                let mut b = self.client.post(url.clone()).json(req);
                if let Some(pid) = project_id {
                    b = b.query(&[("project_id", pid)]);
                }
                b
            })
            .await?;
        Ok(env.provider)
    }

    /// `GET /v1/identity/providers`
    pub async fn list(
        &self,
        limit: u32,
        offset: u32,
        project_id: Option<&str>,
        search: Option<&str>,
    ) -> Result<SecretProviderList, ApiError> {
        let url = self.base();
        self.client
            .execute("provider.list", || {
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

    /// `GET /v1/identity/providers/:id`
    pub async fn get(&self, provider_id: &str) -> Result<SecretProvider, ApiError> {
        let url = format!("{}/{}", self.base(), provider_id);
        let env = self
            .client
            .execute::<SecretProviderEnvelope>("provider.get", || self.client.get(url.clone()))
            .await?;
        Ok(env.provider)
    }

    /// `PATCH /v1/identity/providers/:id`
    pub async fn update(
        &self,
        provider_id: &str,
        req: &UpdateSecretProviderRequest,
        project_id: Option<&str>,
    ) -> Result<SecretProvider, ApiError> {
        let url = format!("{}/{}", self.base(), provider_id);
        let env = self
            .client
            .execute::<SecretProviderEnvelope>("provider.update", || {
                let mut b = self.client.patch(url.clone()).json(req);
                if let Some(pid) = project_id {
                    b = b.query(&[("project_id", pid)]);
                }
                b
            })
            .await?;
        Ok(env.provider)
    }

    /// `DELETE /v1/identity/providers/:id`
    pub async fn delete(
        &self,
        provider_id: &str,
        project_id: Option<&str>,
    ) -> Result<SuccessEnvelope, ApiError> {
        let url = format!("{}/{}", self.base(), provider_id);
        self.client
            .execute("provider.delete", || {
                let mut b = self.client.delete(url.clone());
                if let Some(pid) = project_id {
                    b = b.query(&[("project_id", pid)]);
                }
                b
            })
            .await
    }

    /// `POST /v1/identity/providers/:id/secrets`
    pub async fn add_secret(
        &self,
        provider_id: &str,
        key: &str,
        value: &str,
        project_id: Option<&str>,
    ) -> Result<SecretInfo, ApiError> {
        let url = format!("{}/{}/secrets", self.base(), provider_id);
        let body = SecretPairRequest {
            key: key.to_string(),
            value: value.to_string(),
        };
        let env = self
            .client
            .execute::<SecretEnvelope>("provider.add_secret", || {
                let mut b = self.client.post(url.clone()).json(&body);
                if let Some(pid) = project_id {
                    b = b.query(&[("project_id", pid)]);
                }
                b
            })
            .await?;
        Ok(env.secret)
    }

    /// `GET /v1/identity/providers/:id/secrets`
    pub async fn list_secrets(&self, provider_id: &str) -> Result<SecretList, ApiError> {
        let url = format!("{}/{}/secrets", self.base(), provider_id);
        self.client
            .execute("provider.list_secrets", || self.client.get(url.clone()))
            .await
    }

    /// `PATCH /v1/identity/providers/:id/secrets/:secret_id`
    pub async fn update_secret(
        &self,
        provider_id: &str,
        secret_id: &str,
        req: &UpdateSecretRequest,
        project_id: Option<&str>,
    ) -> Result<SecretInfo, ApiError> {
        let url = format!("{}/{}/secrets/{}", self.base(), provider_id, secret_id);
        let env = self
            .client
            .execute::<SecretEnvelope>("provider.update_secret", || {
                let mut b = self.client.patch(url.clone()).json(req);
                if let Some(pid) = project_id {
                    b = b.query(&[("project_id", pid)]);
                }
                b
            })
            .await?;
        Ok(env.secret)
    }

    /// `DELETE /v1/identity/providers/:id/secrets/:secret_id`
    pub async fn delete_secret(
        &self,
        provider_id: &str,
        secret_id: &str,
        project_id: Option<&str>,
    ) -> Result<SuccessEnvelope, ApiError> {
        let url = format!("{}/{}/secrets/{}", self.base(), provider_id, secret_id);
        self.client
            .execute("provider.delete_secret", || {
                let mut b = self.client.delete(url.clone());
                if let Some(pid) = project_id {
                    b = b.query(&[("project_id", pid)]);
                }
                b
            })
            .await
    }

    /// `POST /v1/identity/providers/:id/attach`
    pub async fn attach(
        &self,
        provider_id: &str,
        runtime_id: &str,
        project_id: Option<&str>,
    ) -> Result<SuccessEnvelope, ApiError> {
        let url = format!("{}/{}/attach", self.base(), provider_id);
        let body = AttachProviderRequest {
            runtime_id: runtime_id.to_string(),
        };
        self.client
            .execute("provider.attach", || {
                let mut b = self.client.post(url.clone()).json(&body);
                if let Some(pid) = project_id {
                    b = b.query(&[("project_id", pid)]);
                }
                b
            })
            .await
    }

    /// `DELETE /v1/identity/providers/:id/attach/:runtime_id`
    pub async fn detach(
        &self,
        provider_id: &str,
        runtime_id: &str,
        project_id: Option<&str>,
    ) -> Result<SuccessEnvelope, ApiError> {
        let url = format!("{}/{}/attach/{}", self.base(), provider_id, runtime_id);
        self.client
            .execute("provider.detach", || {
                let mut b = self.client.delete(url.clone());
                if let Some(pid) = project_id {
                    b = b.query(&[("project_id", pid)]);
                }
                b
            })
            .await
    }

    /// `GET /v1/identity/runtimes/:runtime_id/providers`
    pub async fn list_for_runtime(&self, runtime_id: &str) -> Result<SecretProviderList, ApiError> {
        let url = self
            .client
            .url(&format!("v1/identity/runtimes/{runtime_id}/providers"));
        self.client
            .execute("provider.list_for_runtime", || self.client.get(url.clone()))
            .await
    }
}
