//! Runtime browser + display — computer-use planes.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::error::ApiError;
use super::ApiClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserInfo {
    pub browser_id: String,
    #[serde(default)]
    pub engine: String,
    #[serde(default)]
    pub headless: bool,
    #[serde(default)]
    pub pid: i64,
    #[serde(default)]
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserListResponse {
    #[serde(default)]
    pub browsers: Vec<BrowserInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserStatusResponse {
    pub browser_id: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub targets: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigateResponse {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub status: i64,
    #[serde(default)]
    pub navigated: bool,
    #[serde(default)]
    pub settled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotResponse {
    pub snapshot_id: String,
    #[serde(default)]
    pub rev: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub elements: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluateResponse {
    #[serde(default)]
    pub result: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResponse {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub navigated: bool,
    #[serde(default)]
    pub settled: bool,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageResponse {
    /// base64 payload.
    #[serde(default)]
    pub data: String,
    #[serde(default)]
    pub format: String,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewResponse {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub browser_url: Option<String>,
    #[serde(default)]
    pub port: u32,
    #[serde(default)]
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadsResponse {
    #[serde(default)]
    pub downloads: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadGetResponse {
    #[serde(default)]
    pub entry: Value,
    /// base64 file payload — null for `meta_only`.
    #[serde(default)]
    pub data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayStatusResponse {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub display: String,
    #[serde(default)]
    pub view_on: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitStableResponse {
    #[serde(default)]
    pub settled: bool,
    #[serde(default)]
    pub waited_ms: u32,
}

/// Browser operations against `/v1/agents/runtime/<id>/browser`.
pub struct RuntimeBrowserApi<'a> {
    client: &'a ApiClient,
}

impl<'a> RuntimeBrowserApi<'a> {
    pub fn new(client: &'a ApiClient) -> Self {
        Self { client }
    }

    fn url(&self, runtime_id: &str, tail: &str) -> String {
        self.client
            .agents_url(&format!("runtime/{runtime_id}/browser{tail}"))
    }

    /// `POST .../browser/open`
    pub async fn open(
        &self,
        runtime_id: &str,
        headless: bool,
        url: &str,
    ) -> Result<BrowserInfo, ApiError> {
        self.client
            .execute("runtime.browser.open", || {
                self.client
                    .post(self.url(runtime_id, "/open"))
                    .json(&json!({
                        "headless": headless,
                        "url": url,
                    }))
            })
            .await
    }

    /// `POST .../browser/<bid>/close`
    pub async fn close(&self, runtime_id: &str, browser_id: &str) -> Result<(), ApiError> {
        self.client
            .execute("runtime.browser.close", || {
                self.client
                    .post(self.url(runtime_id, &format!("/{browser_id}/close")))
                    .json(&json!({}))
            })
            .await
    }

    /// `GET .../browser`
    pub async fn list(&self, runtime_id: &str) -> Result<BrowserListResponse, ApiError> {
        self.client
            .execute("runtime.browser.list", || {
                self.client.get(self.url(runtime_id, ""))
            })
            .await
    }

    /// `GET .../browser/<bid>/status`
    pub async fn status(
        &self,
        runtime_id: &str,
        browser_id: &str,
    ) -> Result<BrowserStatusResponse, ApiError> {
        self.client
            .execute("runtime.browser.status", || {
                self.client
                    .get(self.url(runtime_id, &format!("/{browser_id}/status")))
            })
            .await
    }

    /// `POST .../browser/<bid>/navigate`
    pub async fn navigate(
        &self,
        runtime_id: &str,
        browser_id: &str,
        url: &str,
        wait_until: &str,
        timeout_ms: u64,
    ) -> Result<NavigateResponse, ApiError> {
        self.client
            .execute("runtime.browser.navigate", || {
                self.client
                    .post(self.url(runtime_id, &format!("/{browser_id}/navigate")))
                    .json(&json!({
                        "url": url,
                        "wait_until": wait_until,
                        "timeout_ms": timeout_ms,
                    }))
            })
            .await
    }

    /// `POST .../browser/<bid>/<verb>` — back / forward / reload.
    pub async fn command(
        &self,
        runtime_id: &str,
        browser_id: &str,
        verb: &str,
        target_id: &str,
    ) -> Result<NavigateResponse, ApiError> {
        self.client
            .execute("runtime.browser.command", || {
                self.client
                    .post(self.url(runtime_id, &format!("/{browser_id}/{verb}")))
                    .json(&json!({ "target_id": target_id }))
            })
            .await
    }

    /// `POST .../browser/<bid>/screenshot`
    pub async fn screenshot(
        &self,
        runtime_id: &str,
        browser_id: &str,
        body: Value,
    ) -> Result<ImageResponse, ApiError> {
        self.client
            .execute("runtime.browser.screenshot", || {
                self.client
                    .post(self.url(runtime_id, &format!("/{browser_id}/screenshot")))
                    .json(&body)
            })
            .await
    }

    /// `POST .../browser/<bid>/snapshot`
    pub async fn snapshot(
        &self,
        runtime_id: &str,
        browser_id: &str,
        target_id: &str,
        rev: &str,
    ) -> Result<SnapshotResponse, ApiError> {
        self.client
            .execute("runtime.browser.snapshot", || {
                self.client
                    .post(self.url(runtime_id, &format!("/{browser_id}/snapshot")))
                    .json(&json!({
                        "target_id": target_id,
                        "diff": rev,
                    }))
            })
            .await
    }

    /// `POST .../browser/<bid>/evaluate`
    pub async fn evaluate(
        &self,
        runtime_id: &str,
        browser_id: &str,
        expression: &str,
        await_promise: bool,
    ) -> Result<EvaluateResponse, ApiError> {
        self.client
            .execute("runtime.browser.evaluate", || {
                self.client
                    .post(self.url(runtime_id, &format!("/{browser_id}/evaluate")))
                    .json(&json!({
                        "expression": expression,
                        "await_promise": await_promise,
                    }))
            })
            .await
    }

    /// `POST .../browser/<bid>/actions/<verb>`
    pub async fn action(
        &self,
        runtime_id: &str,
        browser_id: &str,
        verb: &str,
        body: Value,
    ) -> Result<ActionResponse, ApiError> {
        self.client
            .execute("runtime.browser.action", || {
                self.client
                    .post(self.url(runtime_id, &format!("/{browser_id}/actions/{verb}")))
                    .json(&body)
            })
            .await
    }

    /// `GET .../browser/<bid>/downloads`
    pub async fn downloads(
        &self,
        runtime_id: &str,
        browser_id: &str,
    ) -> Result<DownloadsResponse, ApiError> {
        self.client
            .execute("runtime.browser.downloads", || {
                self.client
                    .get(self.url(runtime_id, &format!("/{browser_id}/downloads")))
            })
            .await
    }

    /// `GET .../browser/<bid>/downloads/<did>` — JSON `{entry, data:<b64>}`;
    /// the route returns JSON, not raw bytes.
    pub async fn download_get(
        &self,
        runtime_id: &str,
        browser_id: &str,
        download_id: &str,
    ) -> Result<DownloadGetResponse, ApiError> {
        let url = self.url(
            runtime_id,
            &format!("/{browser_id}/downloads/{download_id}"),
        );
        self.client
            .execute("runtime.browser.download_get", || {
                self.client.get(url.clone())
            })
            .await
    }

    /// `POST .../browser/<bid>/targets/<tid>/pdf`
    pub async fn pdf(
        &self,
        runtime_id: &str,
        browser_id: &str,
        target_id: &str,
        landscape: bool,
        print_background: bool,
    ) -> Result<ImageResponse, ApiError> {
        self.client
            .execute("runtime.browser.pdf", || {
                self.client
                    .post(self.url(
                        runtime_id,
                        &format!("/{browser_id}/targets/{target_id}/pdf"),
                    ))
                    .json(&json!({
                        "landscape": landscape,
                        "print_background": print_background,
                    }))
            })
            .await
    }

    /// `POST .../browser/<bid>/dialog`
    pub async fn dialog(
        &self,
        runtime_id: &str,
        browser_id: &str,
        target_id: &str,
        accept: bool,
        prompt_text: &str,
        auto_dismiss: bool,
    ) -> Result<(), ApiError> {
        self.client
            .execute("runtime.browser.dialog", || {
                self.client
                    .post(self.url(runtime_id, &format!("/{browser_id}/dialog")))
                    .json(&json!({
                        "target_id": target_id,
                        "accept": accept,
                        "prompt_text": prompt_text,
                        "auto_dismiss": auto_dismiss,
                    }))
            })
            .await
            .map(|_: Value| ())
    }

    /// `GET .../browser/<bid>/targets`
    pub async fn targets(&self, runtime_id: &str, browser_id: &str) -> Result<Value, ApiError> {
        self.client
            .execute("runtime.browser.targets", || {
                self.client
                    .get(self.url(runtime_id, &format!("/{browser_id}/targets")))
            })
            .await
    }

    /// `POST .../browser/<bid>/view` — mints a live-watch URL.
    pub async fn view(
        &self,
        runtime_id: &str,
        browser_id: &str,
        fps: u32,
        quality: u32,
    ) -> Result<ViewResponse, ApiError> {
        self.client
            .execute("runtime.browser.view", || {
                self.client
                    .post(self.url(runtime_id, &format!("/{browser_id}/view")))
                    .json(&json!({ "fps": fps, "quality": quality }))
            })
            .await
    }
}

/// Display operations against `/v1/agents/runtime/<id>/display`.
pub struct RuntimeDisplayApi<'a> {
    client: &'a ApiClient,
}

impl<'a> RuntimeDisplayApi<'a> {
    pub fn new(client: &'a ApiClient) -> Self {
        Self { client }
    }

    fn url(&self, runtime_id: &str, tail: &str) -> String {
        self.client
            .agents_url(&format!("runtime/{runtime_id}/display{tail}"))
    }

    /// `POST .../display/enable`
    pub async fn enable(
        &self,
        runtime_id: &str,
        width: u32,
        height: u32,
        display: &str,
        wm: &str,
    ) -> Result<DisplayStatusResponse, ApiError> {
        self.client
            .execute("runtime.display.enable", || {
                self.client
                    .post(self.url(runtime_id, "/enable"))
                    .json(&json!({
                        "width": width,
                        "height": height,
                        "display": display,
                        "wm": wm,
                    }))
            })
            .await
    }

    /// `POST .../display/disable`
    pub async fn disable(&self, runtime_id: &str) -> Result<(), ApiError> {
        self.client
            .execute("runtime.display.disable", || {
                self.client
                    .post(self.url(runtime_id, "/disable"))
                    .json(&json!({}))
            })
            .await
    }

    /// `GET .../display/status`
    pub async fn status(&self, runtime_id: &str) -> Result<DisplayStatusResponse, ApiError> {
        self.client
            .execute("runtime.display.status", || {
                self.client.get(self.url(runtime_id, "/status"))
            })
            .await
    }

    /// `POST .../display/screenshot`
    pub async fn screenshot(
        &self,
        runtime_id: &str,
        wait_stable_ms: u64,
        timeout_ms: u64,
        format: &str,
    ) -> Result<ImageResponse, ApiError> {
        self.client
            .execute("runtime.display.screenshot", || {
                self.client
                    .post(self.url(runtime_id, "/screenshot"))
                    .json(&json!({
                        "wait_stable_ms": wait_stable_ms,
                        "timeout_ms": timeout_ms,
                        "format": format,
                    }))
            })
            .await
    }

    /// `POST .../display/mouse|keyboard/...` input ops — one typed entry
    /// point; `kind` selects the verb, `body` carries mouse/key payloads.
    pub async fn input(&self, runtime_id: &str, path: &str, body: Value) -> Result<(), ApiError> {
        self.client
            .execute("runtime.display.input", || {
                self.client.post(self.url(runtime_id, path)).json(&body)
            })
            .await
    }

    /// `POST .../display/wait_stable`
    pub async fn wait_stable(
        &self,
        runtime_id: &str,
        stable_ms: u64,
        timeout_ms: u64,
    ) -> Result<WaitStableResponse, ApiError> {
        self.client
            .execute("runtime.display.wait_stable", || {
                self.client
                    .post(self.url(runtime_id, "/wait_stable"))
                    .json(&json!({
                        "stable_ms": stable_ms,
                        "timeout_ms": timeout_ms,
                    }))
            })
            .await
    }

    /// `POST .../display/view` — mints the live-watch URL.
    pub async fn view(
        &self,
        runtime_id: &str,
        fps: u32,
        quality: u32,
    ) -> Result<ViewResponse, ApiError> {
        self.client
            .execute("runtime.display.view", || {
                self.client
                    .post(self.url(runtime_id, "/view"))
                    .json(&json!({ "fps": fps, "quality": quality }))
            })
            .await
    }
}
