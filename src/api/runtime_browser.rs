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
    pub targets: Vec<TargetInfo>,
}

/// A browser tab. Command verbs (`back`/`forward`/`reload`/`activate`) and
/// `targets` both return this shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetInfo {
    #[serde(default)]
    pub target_id: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub loading: bool,
}

/// `targets` reply — `target_id` is the newly opened tab when `url` was given.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetsResponse {
    #[serde(default)]
    pub target_id: String,
    #[serde(default)]
    pub targets: Vec<TargetInfo>,
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
    pub active: String,
    #[serde(default)]
    pub sub: String,
    #[serde(default)]
    pub result: String,
    #[serde(default)]
    pub restarts: u64,
    #[serde(default)]
    pub started_usec: u64,
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
    pub identifier: String,
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub usec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopLogsResponse {
    #[serde(default)]
    pub entries: Vec<DesktopLogEntry>,
    #[serde(default)]
    pub cursor: String,
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

/// Options for [`RuntimeBrowserApi::snapshot`].
#[derive(Debug, Default)]
pub struct SnapshotOptions<'a> {
    pub target_id: &'a str,
    /// Viewport-only digest (default true in the SDKs); `false` digests
    /// the whole page.
    pub viewport_only: bool,
    /// Diff against this previous snapshot rev.
    pub rev: &'a str,
    /// Keep only elements with this ARIA role.
    pub filter_role: &'a str,
    /// Keep only elements whose name contains this substring.
    pub filter_name: &'a str,
}

/// Options for [`RuntimeBrowserApi::evaluate`].
#[derive(Debug)]
pub struct EvaluateOptions<'a> {
    pub target_id: &'a str,
    pub await_promise: bool,
    /// `isolated` (default) or `main`.
    pub world: &'a str,
    pub timeout_ms: u64,
}

/// Options for [`RuntimeBrowserApi::view`].
#[derive(Debug, Default)]
pub struct ViewOptions<'a> {
    pub target_id: &'a str,
    pub fps: u32,
    pub quality: u32,
    /// Re-mint the auth token (invalidates the prior URL).
    pub rotate_token: bool,
    /// `watch` mints a view-only link.
    pub role: &'a str,
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

    /// `POST .../browser/open` — `engine`/`profile_dir`/`args`/`url` are
    /// optional server-side; empty means defaults and no initial navigation.
    pub async fn open(
        &self,
        runtime_id: &str,
        engine: &str,
        headless: bool,
        profile_dir: &str,
        args: &[String],
        url: &str,
    ) -> Result<BrowserInfo, ApiError> {
        self.client
            .execute("runtime.browser.open", || {
                let mut body = json!({
                    "engine": engine,
                    "headless": headless,
                });
                if !profile_dir.is_empty() {
                    body["profile_dir"] = profile_dir.into();
                }
                if !args.is_empty() {
                    body["args"] = args.to_vec().into();
                }
                if !url.is_empty() {
                    body["url"] = url.into();
                }
                self.client.post(self.url(runtime_id, "/open")).json(&body)
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
        target_id: &str,
        wait_until: &str,
        timeout_ms: u64,
    ) -> Result<NavigateResponse, ApiError> {
        self.client
            .execute("runtime.browser.navigate", || {
                self.client
                    .post(self.url(runtime_id, &format!("/{}/navigate", seg(browser_id))))
                    .json(&json!({
                        "target_id": target_id,
                        "url": url,
                        "wait_until": wait_until,
                        "timeout_ms": timeout_ms,
                    }))
            })
            .await
    }

    /// `POST .../browser/<bid>/<verb>` — back / forward / reload / activate.
    pub async fn command(
        &self,
        runtime_id: &str,
        browser_id: &str,
        verb: &str,
        target_id: &str,
    ) -> Result<TargetInfo, ApiError> {
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
        options: SnapshotOptions<'_>,
    ) -> Result<SnapshotResponse, ApiError> {
        self.client
            .execute("runtime.browser.snapshot", || {
                self.client
                    .post(self.url(runtime_id, &format!("/{}/snapshot", seg(browser_id))))
                    .json(&json!({
                        "target_id": options.target_id,
                        "viewport_only": options.viewport_only,
                        "diff": options.rev,
                        "filter_role": options.filter_role,
                        "filter_name": options.filter_name,
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
        options: EvaluateOptions<'_>,
    ) -> Result<EvaluateResponse, ApiError> {
        self.client
            .execute("runtime.browser.evaluate", || {
                self.client
                    .post(self.url(runtime_id, &format!("/{}/evaluate", seg(browser_id))))
                    .json(&json!({
                        "target_id": options.target_id,
                        "expression": expression,
                        "await_promise": options.await_promise,
                        "timeout_ms": options.timeout_ms,
                        "world": options.world,
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

    /// `POST .../browser/<bid>/downloads` — empty body lists; `wait_name`
    /// blocks until a matching download completes.
    pub async fn downloads(
        &self,
        runtime_id: &str,
        browser_id: &str,
        wait_name: &str,
        timeout_ms: u64,
    ) -> Result<DownloadsResponse, ApiError> {
        self.client
            .execute("runtime.browser.downloads", || {
                let mut body = json!({});
                if !wait_name.is_empty() {
                    body["wait_name"] = wait_name.into();
                    body["timeout_ms"] = timeout_ms.into();
                }
                self.client
                    .post(self.url(runtime_id, &format!("/{}/downloads", seg(browser_id))))
                    .json(&body)
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
        body: Value,
    ) -> Result<ImageResponse, ApiError> {
        self.client
            .execute("runtime.browser.pdf", || {
                self.client
                    .post(self.url(
                        runtime_id,
                        &format!("/{}/targets/{}/pdf", seg(browser_id), seg(target_id)),
                    ))
                    .json(&body)
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

    /// `POST .../browser/<bid>/targets` — empty `url` lists the tabs; a
    /// non-empty `url` opens a new tab (`activate` brings it to front).
    pub async fn targets(
        &self,
        runtime_id: &str,
        browser_id: &str,
        url: &str,
        activate: bool,
    ) -> Result<TargetsResponse, ApiError> {
        self.client
            .execute("runtime.browser.targets", || {
                self.client
                    .post(self.url(runtime_id, &format!("/{}/targets", seg(browser_id))))
                    .json(&json!({ "url": url, "activate": activate }))
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
        options: ViewOptions<'_>,
    ) -> Result<ViewResponse, ApiError> {
        self.client
            .execute("runtime.browser.view", || {
                let mut body = json!({
                    "target_id": options.target_id,
                    "fps": options.fps,
                    "quality": options.quality,
                    "rotate_token": options.rotate_token,
                });
                if !options.role.is_empty() {
                    body["role"] = options.role.into();
                }
                self.client
                    .post(self.url(runtime_id, &format!("/{}/view", seg(browser_id))))
                    .json(&body)
            })
            .await
    }
}

/// Display operations against `/v1/agents/runtime/<id>/display`.
pub struct RuntimeDesktopApi<'a> {
    client: &'a ApiClient,
}

impl<'a> RuntimeDesktopApi<'a> {
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
            .snapshot(
                RT,
                "b1",
                super::SnapshotOptions {
                    viewport_only: true,
                    ..Default::default()
                },
            )
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
        let display = api.runtime_desktop();
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
                    .json_body(json!({"url": "", "activate": true}));
                then.status(200).json_body(json!({
                    "target_id": "",
                    "targets": [{"target_id": "A1B2", "url": "https://example.com/"}],
                }));
            })
            .await;
        let r = client(&server)
            .runtime_browser()
            .targets(RT, "b1", "", true)
            .await
            .unwrap();
        listed.assert_async().await;
        assert_eq!(r.targets[0].target_id, "A1B2");
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
            .evaluate(
                RT,
                "b1",
                "({n: 1})",
                super::EvaluateOptions {
                    target_id: "",
                    await_promise: true,
                    world: "isolated",
                    timeout_ms: 30_000,
                },
            )
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
                    json!({"target_id": "", "fps": 0, "quality": 0, "rotate_token": true, "role": "watch"}),
                );
                then.status(201).json_body(svc);
            })
            .await;
        let api = client(&server);
        api.runtime_desktop()
            .view(RT, 0, 0, true, "", 0)
            .await
            .unwrap();
        api.runtime_browser()
            .view(
                RT,
                "b1",
                super::ViewOptions {
                    rotate_token: true,
                    role: "watch",
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        display_view.assert_async().await;
        browser_view.assert_async().await;
    }

    #[tokio::test]
    async fn navigate_forwards_target_wait_and_timeout() {
        let server = MockServer::start_async().await;
        let m = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path(path("browser/b1/navigate"))
                    .json_body(json!({
                        "target_id": "T1",
                        "url": "https://example.com/",
                        "wait_until": "networkidle",
                        "timeout_ms": 12_000,
                    }));
                then.status(200).json_body(json!({
                    "url": "https://example.com/", "title": "Example",
                    "status": 200, "navigated": true, "settled": true,
                }));
            })
            .await;
        let r = client(&server)
            .runtime_browser()
            .navigate(
                RT,
                "b1",
                "https://example.com/",
                "T1",
                "networkidle",
                12_000,
            )
            .await
            .unwrap();
        m.assert_async().await;
        assert!(r.navigated);
        assert_eq!(r.status, 200);
    }

    #[tokio::test]
    async fn command_uses_the_verb_path_and_returns_target_info() {
        // The wire reply is a TargetInfo — url/title/active/loading, not a
        // navigate record.
        let server = MockServer::start_async().await;
        let m = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path(path("browser/b1/back"))
                    .json_body(json!({"target_id": "T7"}));
                then.status(200).json_body(json!({
                    "target_id": "T7", "url": "https://prev.example/",
                    "title": "Prev", "type": "page",
                    "active": true, "loading": false,
                }));
            })
            .await;
        let r = client(&server)
            .runtime_browser()
            .command(RT, "b1", "back", "T7")
            .await
            .unwrap();
        m.assert_async().await;
        assert_eq!(r.target_id, "T7");
        assert!(r.active);
        assert_eq!(r.url, "https://prev.example/");
    }

    #[tokio::test]
    async fn open_forwards_engine_profile_args_and_url() {
        let server = MockServer::start_async().await;
        let m = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path(path("browser/open"))
                    .json_body(json!({
                        "engine": "chromium",
                        "headless": true,
                        "profile_dir": "/tmp/prof",
                        "args": ["--window-size=800,600"],
                        "url": "https://example.com/",
                    }));
                then.status(200).json_body(json!({
                    "browser_id": "b9", "engine": "chromium", "headless": true,
                }));
            })
            .await;
        let b = client(&server)
            .runtime_browser()
            .open(
                RT,
                "chromium",
                true,
                "/tmp/prof",
                &["--window-size=800,600".to_string()],
                "https://example.com/",
            )
            .await
            .unwrap();
        m.assert_async().await;
        assert_eq!(b.browser_id, "b9");
    }

    #[tokio::test]
    async fn snapshot_forwards_all_filters_and_the_diff_rev() {
        let server = MockServer::start_async().await;
        let m = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path(path("browser/b1/snapshot"))
                    .json_body(json!({
                        "target_id": "T1",
                        "viewport_only": false,
                        "diff": "2",
                        "filter_role": "link",
                        "filter_name": "next",
                    }));
                then.status(200).json_body(json!({
                    "url": "u", "title": "t", "rev": "3", "elements": [],
                }));
            })
            .await;
        client(&server)
            .runtime_browser()
            .snapshot(
                RT,
                "b1",
                super::SnapshotOptions {
                    target_id: "T1",
                    viewport_only: false,
                    rev: "2",
                    filter_role: "link",
                    filter_name: "next",
                },
            )
            .await
            .unwrap();
        m.assert_async().await;
    }

    #[tokio::test]
    async fn evaluate_forwards_target_world_and_timeout() {
        let server = MockServer::start_async().await;
        let m = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path(path("browser/b1/evaluate"))
                    .json_body(json!({
                        "target_id": "T2",
                        "expression": "document.title",
                        "await_promise": true,
                        "timeout_ms": 5_000,
                        "world": "main",
                    }));
                then.status(200).json_body(json!({
                    "value": "Example", "error": "", "is_error": false,
                }));
            })
            .await;
        let r = client(&server)
            .runtime_browser()
            .evaluate(
                RT,
                "b1",
                "document.title",
                super::EvaluateOptions {
                    target_id: "T2",
                    await_promise: true,
                    world: "main",
                    timeout_ms: 5_000,
                },
            )
            .await
            .unwrap();
        m.assert_async().await;
        assert_eq!(r.value, "Example");
    }

    #[tokio::test]
    async fn action_posts_the_body_verbatim_to_the_verb_route() {
        let server = MockServer::start_async().await;
        let m = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path(path("browser/b1/actions/fill"))
                    .json_body(json!({
                        "target_id": "T1", "selector": "#q", "text": "hi",
                        "secret_ref": "", "force": false,
                        "settle_timeout_ms": 0, "target_timeout_ms": 0,
                    }));
                then.status(200)
                    .json_body(json!({"settled": true, "url": "https://x/"}));
            })
            .await;
        let r = client(&server)
            .runtime_browser()
            .action(
                RT,
                "b1",
                "fill",
                json!({
                    "target_id": "T1", "selector": "#q", "text": "hi",
                    "secret_ref": "", "force": false,
                    "settle_timeout_ms": 0, "target_timeout_ms": 0,
                }),
            )
            .await
            .unwrap();
        m.assert_async().await;
        assert!(r.settled);
        assert_eq!(r.url, "https://x/");
    }

    #[tokio::test]
    async fn targets_with_a_url_opens_and_activates() {
        let server = MockServer::start_async().await;
        let m = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path(path("browser/b1/targets"))
                    .json_body(json!({"url": "https://new.example/", "activate": true}));
                then.status(200).json_body(json!({
                    "target_id": "T9",
                    "targets": [{"target_id": "T9", "url": "https://new.example/", "type": "page"}],
                }));
            })
            .await;
        let r = client(&server)
            .runtime_browser()
            .targets(RT, "b1", "https://new.example/", true)
            .await
            .unwrap();
        m.assert_async().await;
        assert_eq!(r.target_id, "T9");
        assert_eq!(r.targets[0].r#type, "page");
    }

    #[tokio::test]
    async fn downloads_waits_only_when_asked() {
        // With a wait_name the wait fields go on the wire; without it the
        // body stays empty so the server returns the current list.
        let server = MockServer::start_async().await;
        let listed = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path(path("browser/b1/downloads"))
                    .json_body(json!({}));
                then.status(200).json_body(json!({"downloads": []}));
            })
            .await;
        let waited = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path(path("browser/b1/downloads"))
                    .json_body(json!({"wait_name": "report", "timeout_ms": 60_000}));
                then.status(200).json_body(json!({
                    "downloads": [{"download_id": "d1", "filename": "report.csv"}],
                }));
            })
            .await;
        let api = client(&server);
        api.runtime_browser()
            .downloads(RT, "b1", "", 0)
            .await
            .unwrap();
        let r = api
            .runtime_browser()
            .downloads(RT, "b1", "report", 60_000)
            .await
            .unwrap();
        listed.assert_async().await;
        waited.assert_async().await;
        assert_eq!(r.downloads.len(), 1);
    }

    #[tokio::test]
    async fn dialog_forwards_every_flag() {
        let server = MockServer::start_async().await;
        let m = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path(path("browser/b1/dialog"))
                    .json_body(json!({
                        "target_id": "T3", "accept": true,
                        "prompt_text": "answer", "auto_dismiss": true,
                    }));
                then.status(200).json_body(json!({"ok": true}));
            })
            .await;
        client(&server)
            .runtime_browser()
            .dialog(RT, "b1", "T3", true, "answer", true)
            .await
            .unwrap();
        m.assert_async().await;
    }

    #[tokio::test]
    async fn pdf_uses_the_target_scoped_route() {
        // Target ids with path-unsafe bytes must stay inside one segment.
        let server = MockServer::start_async().await;
        let m = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path(path("browser/b1/targets/T%2F1/pdf"))
                    .json_body(json!({"landscape": true}));
                then.status(200)
                    .json_body(json!({"data": "aGk=", "format": "pdf"}));
            })
            .await;
        let r = client(&server)
            .runtime_browser()
            .pdf(RT, "b1", "T/1", json!({"landscape": true}))
            .await
            .unwrap();
        m.assert_async().await;
        assert_eq!(r.data, "aGk=");
    }

    #[tokio::test]
    async fn ids_with_path_unsafe_bytes_stay_in_one_segment() {
        // `/`, `?`, `#` in a browser or download id must not inject routes.
        let server = MockServer::start_async().await;
        let m = server
            .mock_async(|when, then| {
                when.method(GET).path(path("browser/b%3F1/downloads/d%23x"));
                then.status(200).json_body(json!({
                    "entry": {"download_id": "d#x"}, "data": null,
                }));
            })
            .await;
        client(&server)
            .runtime_browser()
            .download_get(RT, "b?1", "d#x")
            .await
            .unwrap();
        m.assert_async().await;
    }

    #[tokio::test]
    async fn view_omits_an_empty_role() {
        // `role` is conditional — absent means a full-control watch link.
        let server = MockServer::start_async().await;
        let m = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path(path("browser/b1/view"))
                    .json_body(json!({
                        "target_id": "T1", "fps": 15, "quality": 70,
                        "rotate_token": false,
                    }));
                then.status(201).json_body(json!({
                    "url": "https://x.service.example", "service_url": "",
                    "browser_url": "", "port": 0, "expires_at": "",
                }));
            })
            .await;
        client(&server)
            .runtime_browser()
            .view(
                RT,
                "b1",
                super::ViewOptions {
                    target_id: "T1",
                    fps: 15,
                    quality: 70,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        m.assert_async().await;
    }
}
