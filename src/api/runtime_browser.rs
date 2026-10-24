//! Runtime browser + display — computer-use planes.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::error::ApiError;
use super::ApiClient;

/// Percent-encode one URL path segment — caller-supplied ids (browser,
/// target, download) must never carry `/`, `?` or `#` into a path.
fn seg(id: &str) -> String {
    url::form_urlencoded::byte_serialize(id.as_bytes()).collect()
}

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
    /// The expression's result, already parsed from JSON.
    #[serde(default)]
    pub value: Value,
    #[serde(default)]
    pub error: String,
    /// The expression threw — `error` carries the message.
    #[serde(default)]
    pub is_error: bool,
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
    /// Applied downscale factor (1 = full resolution).
    #[serde(default)]
    pub scale: Option<f64>,
    /// Frame revision — feed back via `if_rev` to skip unchanged shots.
    #[serde(default)]
    pub rev: u64,
    /// `if_rev` matched the current frame — `data` is empty.
    #[serde(default)]
    pub unchanged: bool,
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
    /// Role-scoped token when `role` was requested — share as `&vt=` on the URL.
    #[serde(default)]
    pub view_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayInfoResponse {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub depth: u32,
    #[serde(default)]
    pub dpi: u32,
    #[serde(default)]
    pub display: String,
    #[serde(default)]
    pub attached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointerResponse {
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysResponse {
    #[serde(default)]
    pub keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardResponse {
    #[serde(default)]
    pub text: String,
    /// base64 PNG payload when the clipboard holds an image.
    #[serde(default)]
    pub png: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopComponentInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub sub_state: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopStatusResponse {
    #[serde(default)]
    pub present: bool,
    #[serde(default)]
    pub display: String,
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub components: Vec<DesktopComponentInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopLogEntry {
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopLogsResponse {
    #[serde(default)]
    pub entries: Vec<DesktopLogEntry>,
    #[serde(default)]
    pub next_cursor: String,
    #[serde(default)]
    pub truncated: bool,
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
    #[serde(default)]
    pub rev: u64,
}

/// `POST .../display/act` reply — one agent step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActResponse {
    #[serde(default)]
    pub settled: bool,
    #[serde(default)]
    pub rev: u64,
    #[serde(default)]
    pub screenshot: Option<ImageResponse>,
}

/// A top-level window from `GET .../display/windows`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayWindowInfo {
    #[serde(default)]
    pub id: u32,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(default)]
    pub w: u32,
    #[serde(default)]
    pub h: u32,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub window_class: String,
    #[serde(default)]
    pub instance: String,
    #[serde(default)]
    pub desktop: i32,
    #[serde(default)]
    pub focused: bool,
    #[serde(default)]
    pub states: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsResponse {
    #[serde(default)]
    pub windows: Vec<DisplayWindowInfo>,
    /// Currently focused window id (0 = none).
    #[serde(default)]
    pub focused: u32,
}

/// An installed application (freedesktop entry).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayAppInfo {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub generic_name: String,
    #[serde(default)]
    pub exec: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub comment: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub terminal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppsResponse {
    #[serde(default)]
    pub apps: Vec<DisplayAppInfo>,
}

/// `POST .../display/apps/launch` / `.../display/open` reply.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppLaunchResponse {
    #[serde(default)]
    pub pid: u32,
    #[serde(default)]
    pub window_id: u32,
    #[serde(default)]
    pub app: String,
    #[serde(default)]
    pub mime: String,
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

    /// `POST .../browser/<bid>/close` — replies `{"ok": true}`.
    pub async fn close(&self, runtime_id: &str, browser_id: &str) -> Result<(), ApiError> {
        self.client
            .execute("runtime.browser.close", || {
                self.client
                    .post(self.url(runtime_id, &format!("/{}/close", seg(browser_id))))
                    .json(&json!({}))
            })
            .await
            .map(|_: Value| ())
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
                    .get(self.url(runtime_id, &format!("/{}/status", seg(browser_id))))
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
                    .post(self.url(runtime_id, &format!("/{}/navigate", seg(browser_id))))
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
                    .post(self.url(runtime_id, &format!("/{}/{}", seg(browser_id), seg(verb))))
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
                    .post(self.url(runtime_id, &format!("/{}/screenshot", seg(browser_id))))
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
                    .post(self.url(runtime_id, &format!("/{}/snapshot", seg(browser_id))))
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
                    .post(self.url(runtime_id, &format!("/{}/evaluate", seg(browser_id))))
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
                    .post(self.url(
                        runtime_id,
                        &format!("/{}/actions/{}", seg(browser_id), seg(verb)),
                    ))
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
                    .get(self.url(runtime_id, &format!("/{}/downloads", seg(browser_id))))
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
            &format!("/{}/downloads/{}", seg(browser_id), seg(download_id)),
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
                        &format!("/{}/targets/{}/pdf", seg(browser_id), seg(target_id)),
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
                    .post(self.url(runtime_id, &format!("/{}/dialog", seg(browser_id))))
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

    /// `POST .../browser/<bid>/targets` — an empty body lists the tabs
    /// (the route is POST-only; a non-empty `url` would open a new tab).
    pub async fn targets(&self, runtime_id: &str, browser_id: &str) -> Result<Value, ApiError> {
        self.client
            .execute("runtime.browser.targets", || {
                self.client
                    .post(self.url(runtime_id, &format!("/{}/targets", seg(browser_id))))
                    .json(&json!({}))
            })
            .await
    }

    /// `POST .../browser/<bid>/view` — mints a live-watch URL.
    /// `rotate_token` re-mints the auth token (invalidating the prior URL);
    /// `role` `watch` mints a view-only link returned in `view_token`
    /// (share as `&vt=` — those connections see frames, never inject input).
    pub async fn view(
        &self,
        runtime_id: &str,
        browser_id: &str,
        fps: u32,
        quality: u32,
        rotate_token: bool,
        role: &str,
    ) -> Result<ViewResponse, ApiError> {
        self.client
            .execute("runtime.browser.view", || {
                let mut body = json!({
                    "fps": fps,
                    "quality": quality,
                    "rotate_token": rotate_token,
                });
                if !role.is_empty() {
                    body["role"] = role.into();
                }
                self.client
                    .post(self.url(runtime_id, &format!("/{}/view", seg(browser_id))))
                    .json(&body)
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

    fn desktop_url(&self, runtime_id: &str, tail: &str) -> String {
        self.client
            .agents_url(&format!("runtime/{runtime_id}/desktop{tail}"))
    }

    /// `POST .../display/enable`
    pub async fn enable(
        &self,
        runtime_id: &str,
        width: u32,
        height: u32,
        display: &str,
        wm: &str,
        mode: &str,
    ) -> Result<DisplayStatusResponse, ApiError> {
        self.client
            .execute("runtime.display.enable", || {
                let mut body = json!({
                    "width": width,
                    "height": height,
                    "display": display,
                    "wm": wm,
                });
                if !mode.is_empty() {
                    body["mode"] = mode.into();
                }
                self.client
                    .post(self.url(runtime_id, "/enable"))
                    .json(&body)
            })
            .await
    }

    /// `POST .../display/disable` — replies `{"ok": true}`.
    pub async fn disable(&self, runtime_id: &str) -> Result<(), ApiError> {
        self.client
            .execute("runtime.display.disable", || {
                self.client
                    .post(self.url(runtime_id, "/disable"))
                    .json(&json!({}))
            })
            .await
            .map(|_: Value| ())
    }

    /// `GET .../display/status`
    pub async fn status(&self, runtime_id: &str) -> Result<DisplayStatusResponse, ApiError> {
        self.client
            .execute("runtime.display.status", || {
                self.client.get(self.url(runtime_id, "/status"))
            })
            .await
    }

    /// `GET .../display/info` — live geometry as the display reports it now.
    pub async fn info(&self, runtime_id: &str) -> Result<DisplayInfoResponse, ApiError> {
        self.client
            .execute("runtime.display.info", || {
                self.client.get(self.url(runtime_id, "/info"))
            })
            .await
    }

    /// `GET .../display/position` — live pointer coordinates.
    pub async fn position(&self, runtime_id: &str) -> Result<PointerResponse, ApiError> {
        self.client
            .execute("runtime.display.position", || {
                self.client.get(self.url(runtime_id, "/position"))
            })
            .await
    }

    /// `GET .../display/keys` — key names press/hotkey accept.
    pub async fn keys(&self, runtime_id: &str) -> Result<KeysResponse, ApiError> {
        self.client
            .execute("runtime.display.keys", || {
                self.client.get(self.url(runtime_id, "/keys"))
            })
            .await
    }

    /// `POST .../display/clipboard` — `op` get|set.
    pub async fn clipboard(
        &self,
        runtime_id: &str,
        op: &str,
        text: &str,
        png_b64: &str,
    ) -> Result<ClipboardResponse, ApiError> {
        self.client
            .execute("runtime.display.clipboard", || {
                self.client
                    .post(self.url(runtime_id, "/clipboard"))
                    .json(&json!({"op": op, "text": text, "png": png_b64}))
            })
            .await
    }

    /// `POST .../display/screenshot` — the caller composes the body
    /// (format, region, fit, quality, cursor, wait-stable).
    pub async fn screenshot(
        &self,
        runtime_id: &str,
        body: Value,
    ) -> Result<ImageResponse, ApiError> {
        self.client
            .execute("runtime.display.screenshot", || {
                self.client
                    .post(self.url(runtime_id, "/screenshot"))
                    .json(&body)
            })
            .await
    }

    /// `POST .../display/mouse|keyboard/...` input ops — one typed entry
    /// point; `kind` selects the verb, `body` carries mouse/key payloads.
    /// Each op replies `{"ok": true}`.
    pub async fn input(&self, runtime_id: &str, path: &str, body: Value) -> Result<(), ApiError> {
        self.client
            .execute("runtime.display.input", || {
                self.client.post(self.url(runtime_id, path)).json(&body)
            })
            .await
            .map(|_: Value| ())
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
    /// `rotate_token` re-mints the auth token (invalidating the prior URL);
    /// `role` `watch` mints a view-only link returned in `view_token`;
    /// `window_id` streams just that window's pixels.
    pub async fn view(
        &self,
        runtime_id: &str,
        fps: u32,
        quality: u32,
        rotate_token: bool,
        role: &str,
        window_id: u32,
    ) -> Result<ViewResponse, ApiError> {
        self.client
            .execute("runtime.display.view", || {
                let mut body = json!({
                    "fps": fps,
                    "quality": quality,
                    "rotate_token": rotate_token,
                });
                if !role.is_empty() {
                    body["role"] = role.into();
                }
                if window_id != 0 {
                    body["window_id"] = window_id.into();
                }
                self.client.post(self.url(runtime_id, "/view")).json(&body)
            })
            .await
    }

    /// `POST .../display/view/stop` — stops the view server, frees the port.
    pub async fn stop_view(&self, runtime_id: &str) -> Result<(), ApiError> {
        self.client
            .execute("runtime.display.stop_view", || {
                self.client
                    .post(self.url(runtime_id, "/view/stop"))
                    .json(&json!({}))
            })
            .await
            .map(|_: Value| ())
    }

    /// `POST .../display/act` — one agent step: ordered inputs → settle →
    /// observation. `body` carries `actions`/`settle_ms`/`timeout_ms`/`observe`.
    pub async fn act(&self, runtime_id: &str, body: Value) -> Result<ActResponse, ApiError> {
        self.client
            .execute("runtime.display.act", || {
                self.client.post(self.url(runtime_id, "/act")).json(&body)
            })
            .await
    }

    /// `POST .../display/resize` — live display resize; returns live geometry.
    pub async fn resize(
        &self,
        runtime_id: &str,
        width: u32,
        height: u32,
    ) -> Result<DisplayInfoResponse, ApiError> {
        self.client
            .execute("runtime.display.resize", || {
                self.client
                    .post(self.url(runtime_id, "/resize"))
                    .json(&json!({"width": width, "height": height}))
            })
            .await
    }

    /// `GET .../display/windows` — top-level windows; `window_class` filters.
    pub async fn windows(
        &self,
        runtime_id: &str,
        window_class: &str,
    ) -> Result<WindowsResponse, ApiError> {
        self.client
            .execute("runtime.display.windows", || {
                let req = self.client.get(self.url(runtime_id, "/windows"));
                if window_class.is_empty() {
                    req
                } else {
                    req.query(&[("window_class", window_class)])
                }
            })
            .await
    }

    /// `POST .../display/windows/op` — window op; `body` carries
    /// `id`/`op`/`x`/`y`/`w`/`h`. Replies the post-op window record.
    pub async fn window_op(
        &self,
        runtime_id: &str,
        body: Value,
    ) -> Result<DisplayWindowInfo, ApiError> {
        self.client
            .execute("runtime.display.window_op", || {
                self.client
                    .post(self.url(runtime_id, "/windows/op"))
                    .json(&body)
            })
            .await
    }

    /// `GET .../display/apps` — installed applications.
    pub async fn apps(&self, runtime_id: &str) -> Result<AppsResponse, ApiError> {
        self.client
            .execute("runtime.display.apps", || {
                self.client.get(self.url(runtime_id, "/apps"))
            })
            .await
    }

    /// `POST .../display/apps/launch` — launch by app id; `body` carries
    /// `app`/`uri`/`wait_window`/`timeout_ms`.
    pub async fn app_launch(
        &self,
        runtime_id: &str,
        body: Value,
    ) -> Result<AppLaunchResponse, ApiError> {
        self.client
            .execute("runtime.display.app_launch", || {
                self.client
                    .post(self.url(runtime_id, "/apps/launch"))
                    .json(&body)
            })
            .await
    }

    /// `POST .../display/open` — open a path/URL with its handler.
    pub async fn open(&self, runtime_id: &str, body: Value) -> Result<AppLaunchResponse, ApiError> {
        self.client
            .execute("runtime.display.open", || {
                self.client.post(self.url(runtime_id, "/open")).json(&body)
            })
            .await
    }

    /// `GET .../desktop/status` — declared components and live states.
    pub async fn desktop_status(
        &self,
        runtime_id: &str,
    ) -> Result<DesktopStatusResponse, ApiError> {
        self.client
            .execute("runtime.desktop.status", || {
                self.client.get(self.desktop_url(runtime_id, "/status"))
            })
            .await
    }

    /// `POST .../desktop/component` — `op` start|stop|restart.
    pub async fn desktop_component(
        &self,
        runtime_id: &str,
        name: &str,
        op: &str,
    ) -> Result<DesktopComponentInfo, ApiError> {
        self.client
            .execute("runtime.desktop.component", || {
                self.client
                    .post(self.desktop_url(runtime_id, "/component"))
                    .json(&json!({"name": name, "op": op}))
            })
            .await
    }

    /// `POST .../desktop/logs` — journal page; pass `next_cursor` for the next.
    pub async fn desktop_logs(
        &self,
        runtime_id: &str,
        name: &str,
        cursor: &str,
        max: u32,
        priority: &str,
    ) -> Result<DesktopLogsResponse, ApiError> {
        self.client
            .execute("runtime.desktop.logs", || {
                self.client
                    .post(self.desktop_url(runtime_id, "/logs"))
                    .json(&json!({
                        "name": name,
                        "cursor": cursor,
                        "max": max,
                        "priority": priority,
                    }))
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use httpmock::prelude::*;
    use secrecy::SecretString;
    use serde_json::json;

    use super::ApiClient;

    const RT: &str = "11111111-2222-3333-4444-555555555555";

    fn client(server: &MockServer) -> ApiClient {
        ApiClient::new(SecretString::from("test-key"), Some(server.base_url())).unwrap()
    }

    fn path(tail: &str) -> String {
        format!("/v1/agents/runtime/{RT}/{tail}")
    }

    #[tokio::test]
    async fn snapshot_parses_the_api_reply() {
        // The API reply carries no snapshot id — only url/title/viewport/rev/elements.
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(POST).path(path("browser/b1/snapshot"));
                then.status(200).json_body(json!({
                    "url": "https://example.com/", "title": "Example Domain",
                    "viewport": {"width": 1440, "height": 900}, "rev": "3",
                    "elements": [{"ref": "e1", "tag": "a", "role": "link", "name": "Learn more"}],
                }));
            })
            .await;
        let api = client(&server);
        let r = api
            .runtime_browser()
            .snapshot(RT, "b1", "", "")
            .await
            .unwrap();
        assert_eq!(r.rev, "3");
        assert_eq!(r.elements[0]["ref"], "e1");
    }

    #[tokio::test]
    async fn ok_replies_are_success_for_unit_ops() {
        // close, display disable, and display input all reply `{"ok": true}`.
        let server = MockServer::start_async().await;
        for tail in [
            "browser/b1/close",
            "display/disable",
            "display/mouse/click",
            "display/keyboard/type",
        ] {
            server
                .mock_async(|when, then| {
                    when.method(POST).path(path(tail));
                    then.status(200).json_body(json!({"ok": true}));
                })
                .await;
        }
        let api = client(&server);
        api.runtime_browser().close(RT, "b1").await.unwrap();
        let display = api.runtime_display();
        display.disable(RT).await.unwrap();
        display
            .input(
                RT,
                "/mouse/click",
                json!({"x": 1, "y": 2, "button": "left"}),
            )
            .await
            .unwrap();
        display
            .input(RT, "/keyboard/type", json!({"text": "hi"}))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn targets_lists_with_a_post() {
        // The route is POST-only; an empty body lists the tabs.
        let server = MockServer::start_async().await;
        let listed = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path(path("browser/b1/targets"))
                    .json_body(json!({}));
                then.status(200).json_body(json!({
                    "target_id": "",
                    "targets": [{"target_id": "A1B2", "url": "https://example.com/"}],
                }));
            })
            .await;
        let r = client(&server)
            .runtime_browser()
            .targets(RT, "b1")
            .await
            .unwrap();
        listed.assert_async().await;
        assert_eq!(r["targets"][0]["target_id"], "A1B2");
    }

    #[tokio::test]
    async fn evaluate_reads_value_and_error() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(POST).path(path("browser/b1/evaluate"));
                then.status(200)
                    .json_body(json!({"value": {"n": 1}, "error": "", "is_error": false}));
            })
            .await;
        let r = client(&server)
            .runtime_browser()
            .evaluate(RT, "b1", "({n: 1})", true)
            .await
            .unwrap();
        assert_eq!(r.value, json!({"n": 1}));
        assert!(!r.is_error);

        let threw: super::EvaluateResponse = serde_json::from_value(json!({
            "value": null, "error": "ReferenceError: x is not defined", "is_error": true,
        }))
        .unwrap();
        assert!(threw.is_error);
        assert!(threw.error.starts_with("ReferenceError"));
    }

    #[tokio::test]
    async fn view_sends_rotate_token() {
        // `rotate_token` re-mints the watch URL's auth token server-side.
        let server = MockServer::start_async().await;
        let svc = json!({
            "url": "https://x.service.example", "service_url": "https://x.service.example/",
            "browser_url": "https://x.service.example/_ws/auth?token=t",
            "port": 9871, "expires_at": "2099-01-01T00:00:00Z",
        });
        let display_view = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path(path("display/view"))
                    .json_body(json!({"fps": 0, "quality": 0, "rotate_token": true}));
                then.status(201).json_body(svc.clone());
            })
            .await;
        let browser_view = server
            .mock_async(|when, then| {
                when.method(POST).path(path("browser/b1/view")).json_body(
                    json!({"fps": 0, "quality": 0, "rotate_token": true, "role": "watch"}),
                );
                then.status(201).json_body(svc);
            })
            .await;
        let api = client(&server);
        api.runtime_display()
            .view(RT, 0, 0, true, "", 0)
            .await
            .unwrap();
        api.runtime_browser()
            .view(RT, "b1", 0, 0, true, "watch")
            .await
            .unwrap();
        display_view.assert_async().await;
        browser_view.assert_async().await;
    }
}
