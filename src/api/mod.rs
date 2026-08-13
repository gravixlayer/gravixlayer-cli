// src/api/mod.rs — HTTP client for the GravixLayer API.
//
// `ApiClient` wraps a `reqwest::Client` (with rustls, no OpenSSL) and exposes
// typed sub-resource handles.  Every request goes through `execute_request`
// which handles:
//   • Authentication via `Authorization: Bearer` header
//   • Content-Type and Accept headers
//   • Automatic retry with exponential back-off (see retry.rs)
//   • Typed error mapping (see error.rs)
//
// Connection pooling is handled by `reqwest::Client` internally (idle timeout
// 90s, max idle per host 10) — matching the Python SDK's httpx pool settings.

pub mod agent;
pub mod billing;
pub mod error;
pub mod network_policy;
pub mod provider;
pub mod retry;
pub mod runtime;
pub mod runtime_files;
pub mod runtime_git;
pub mod runtime_pty;
pub mod runtime_service;
pub mod template;
pub mod types;

use std::time::Duration;

use reqwest::{header, Client, RequestBuilder, Response};
use secrecy::{ExposeSecret, SecretString};
use serde::de::DeserializeOwned;
use tracing::{debug, instrument};

use error::ApiError;
use retry::retry;

const DEFAULT_BASE_URL: &str = "https://api.gravixlayer.ai";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Default request timeout.  Raised to 300s to accommodate large archive
/// uploads (e.g. `agent build`) which can easily exceed 60s on slow networks.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);
const USER_AGENT: &str = concat!("gravixlayer-cli/", env!("CARGO_PKG_VERSION"));

// ---------------------------------------------------------------------------
// ApiClient
// ---------------------------------------------------------------------------

/// HTTP client for GravixLayer REST API.
///
/// Clone is cheap — the underlying `reqwest::Client` uses an `Arc` internally.
#[derive(Clone, Debug)]
pub struct ApiClient {
    http: Client,
    api_key: SecretString,
    base_url: String,
}

impl ApiClient {
    /// Construct a new `ApiClient`.
    ///
    /// `base_url` defaults to `https://api.gravixlayer.ai` if `None`.
    pub fn new(api_key: SecretString, base_url: Option<String>) -> Result<Self, ApiError> {
        let mut default_headers = header::HeaderMap::new();
        default_headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );
        default_headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );

        let http = Client::builder()
            .use_rustls_tls()
            .https_only(true)
            .user_agent(USER_AGENT)
            .default_headers(default_headers)
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .tcp_keepalive(Duration::from_secs(30))
            .build()
            .map_err(ApiError::Connection)?;

        Ok(Self {
            http,
            api_key,
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
        })
    }

    /// Build the full URL for a path relative to `/v1/agents/`.
    ///
    /// Example: `self.agents_url("runtime/abc")` → `https://api.gravixlayer.ai/v1/agents/runtime/abc`
    pub fn agents_url(&self, path: &str) -> String {
        format!("{}/v1/agents/{}", self.base_url, path)
    }

    /// Build the full URL for a path relative to the base (non-agents endpoints).
    pub fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    /// Build a WebSocket URL for the runtime terminal.
    ///
    /// The terminal endpoint is at `/v1/runtime/<id>/terminal` (no `/agents/` prefix).
    /// Token is passed via the `Authorization` header in the WebSocket upgrade request,
    /// not as a query param (that is the browser-only path).
    pub fn terminal_ws_url(
        &self,
        runtime_id: &str,
        shell: &str,
        project_id: Option<&str>,
    ) -> String {
        // Replace https:// with wss:// (or http:// with ws://)
        let ws_base = self
            .base_url
            .replace("https://", "wss://")
            .replace("http://", "ws://");
        let encoded_shell =
            url::form_urlencoded::byte_serialize(shell.as_bytes()).collect::<String>();
        let mut url = format!(
            "{}/v1/runtime/{}/terminal?shell={}",
            ws_base, runtime_id, encoded_shell
        );
        if let Some(pid) = project_id {
            // URL-encode the project_id to prevent query-string injection.
            let encoded_pid =
                url::form_urlencoded::byte_serialize(pid.as_bytes()).collect::<String>();
            url.push_str("&project_id=");
            url.push_str(&encoded_pid);
        }
        url
    }

    /// Inject the `Authorization` header onto a request builder.
    fn authorize(&self, builder: RequestBuilder) -> RequestBuilder {
        builder.bearer_auth(self.api_key.expose_secret())
    }

    /// Execute an HTTP request with retry on transient errors.
    ///
    /// `T` is the expected response body type.  The status code is checked and
    /// converted to an `ApiError` before attempting deserialization.
    #[instrument(skip(self, build_request), fields(method, url))]
    pub async fn execute<T>(
        &self,
        operation: &str,
        build_request: impl Fn() -> RequestBuilder,
    ) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
    {
        retry(operation, |attempt| {
            let builder = self.authorize(build_request());
            async move {
                debug!(attempt, "sending request");
                let resp: Response = builder.send().await.map_err(ApiError::Connection)?;
                Self::parse_response(resp).await
            }
        })
        .await
    }

    /// Execute an HTTP request that returns 204 No Content (no body).
    #[allow(dead_code)]
    pub async fn execute_empty(
        &self,
        operation: &str,
        build_request: impl Fn() -> RequestBuilder,
    ) -> Result<(), ApiError> {
        retry(operation, |attempt| {
            let builder = self.authorize(build_request());
            async move {
                debug!(attempt, "sending request (empty response expected)");
                let resp: Response = builder.send().await.map_err(ApiError::Connection)?;
                let status = resp.status();
                if status.is_success() {
                    return Ok(());
                }
                let body = resp.text().await.unwrap_or_default();
                Err(ApiError::from_response(status.as_u16(), body))
            }
        })
        .await
    }

    /// Parse an HTTP response into `T`, mapping non-success status codes to `ApiError`.
    async fn parse_response<T: DeserializeOwned>(resp: Response) -> Result<T, ApiError> {
        let status = resp.status();
        if status.is_success() {
            resp.json::<T>().await.map_err(ApiError::Deserialization)
        } else {
            let body = resp.text().await.unwrap_or_default();
            Err(ApiError::from_response(status.as_u16(), body))
        }
    }

    // ------------------------------------------------------------------
    // Convenience builders
    // ------------------------------------------------------------------

    pub fn get(&self, url: String) -> RequestBuilder {
        self.http.get(url)
    }

    pub fn post(&self, url: String) -> RequestBuilder {
        self.http.post(url)
    }

    #[allow(dead_code)]
    pub fn put(&self, url: String) -> RequestBuilder {
        self.http.put(url)
    }

    pub fn patch(&self, url: String) -> RequestBuilder {
        self.http.patch(url)
    }

    pub fn delete(&self, url: String) -> RequestBuilder {
        self.http.delete(url)
    }

    // ------------------------------------------------------------------
    // Sub-resource accessors
    // ------------------------------------------------------------------

    pub fn runtime(&self) -> runtime::RuntimeApi<'_> {
        runtime::RuntimeApi::new(self)
    }

    pub fn template(&self) -> template::TemplateApi<'_> {
        template::TemplateApi::new(self)
    }

    pub fn agent(&self) -> agent::AgentApi<'_> {
        agent::AgentApi::new(self)
    }

    pub fn billing(&self) -> billing::BillingApi<'_> {
        billing::BillingApi::new(self)
    }

    pub fn provider(&self) -> provider::ProviderApi<'_> {
        provider::ProviderApi::new(self)
    }

    pub fn network_policy(&self) -> network_policy::NetworkPolicyApi<'_> {
        network_policy::NetworkPolicyApi::new(self)
    }

    pub fn runtime_files(&self) -> runtime_files::RuntimeFilesApi<'_> {
        runtime_files::RuntimeFilesApi::new(self)
    }

    pub fn runtime_git(&self) -> runtime_git::RuntimeGitApi<'_> {
        runtime_git::RuntimeGitApi::new(self)
    }

    pub fn runtime_pty(&self) -> runtime_pty::RuntimePtyApi<'_> {
        runtime_pty::RuntimePtyApi::new(self)
    }

    pub fn runtime_service(&self) -> runtime_service::RuntimeServiceApi<'_> {
        runtime_service::RuntimeServiceApi::new(self)
    }

    /// Expose the underlying reqwest `Client` for WebSocket upgrades.
    ///
    /// The WS upgrade requires the same TLS stack and connection pool.
    pub fn http_client(&self) -> &Client {
        &self.http
    }

    /// Expose the API key for use in WebSocket upgrade requests.
    pub fn api_key(&self) -> &SecretString {
        &self.api_key
    }

    /// Expose the API key as a plain string reference (for building auth headers directly).
    pub fn api_key_str(&self) -> &str {
        self.api_key.expose_secret()
    }

    /// Check an HTTP response status and return an error for non-2xx responses.
    /// Returns the response on success so callers can continue parsing.
    pub async fn check_status(resp: reqwest::Response) -> anyhow::Result<reqwest::Response> {
        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }
        // Limit error body to 1 MB to protect against pathological server responses.
        let body = resp
            .bytes()
            .await
            .map(|b| {
                let truncated = b.slice(..b.len().min(1024 * 1024));
                String::from_utf8_lossy(&truncated).into_owned()
            })
            .unwrap_or_default();
        anyhow::bail!("API error {}: {}", status.as_u16(), body);
    }

    /// Base URL accessor.
    #[allow(dead_code)]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}
