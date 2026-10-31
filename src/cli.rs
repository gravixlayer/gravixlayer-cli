// src/cli.rs — Complete clap derive command tree.
//
// Every top-level command (`auth`, `config`, `runtime`, `template`, `agent`,
// `billing`, `validate`, `package`, `completions`, `update`) is declared here
// with ALL flags required by COMMANDS.md.  Handler dispatch lives in main.rs.
// Individual handler implementations live in the cmd/ module tree.

use crate::config::ResolvedConfig;
use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Root CLI
// ---------------------------------------------------------------------------

/// Official CLI for the GravixLayer AI agent platform.
///
/// Use `gravixlayer <command> --help` for detailed usage of each subcommand.
#[derive(Debug, Parser)]
#[command(
    name            = "gravixlayer",
    bin_name        = "gravixlayer",
    version,
    about           = "Manage GravixLayer runtimes, templates, snapshots, agents, and billing",
    long_about      = None,
    propagate_version = true,
    arg_required_else_help = true,
)]
pub struct Cli {
    /// Output format (table / json / quiet)
    #[arg(
        long,
        global = true,
        default_value = "table",
        env = "GRAVIXLAYER_OUTPUT"
    )]
    pub output: OutputFormat,

    /// GravixLayer API key (overrides config and keyring)
    #[arg(
        long,
        global = true,
        env = "GRAVIXLAYER_API_KEY",
        hide_env_values = true
    )]
    pub api_key: Option<String>,

    /// API base URL (default: https://api.gravixlayer.ai)
    #[arg(long, global = true, env = "GRAVIXLAYER_BASE_URL")]
    pub base_url: Option<String>,

    /// Profile name from ~/.gravixlayer/config.toml (default: active profile set by `config use-profile`)
    #[arg(long, global = true, env = "GRAVIXLAYER_PROFILE")]
    pub profile: Option<String>,

    /// Enable verbose logging
    #[arg(short = 'v', long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
    Quiet,
}

// ---------------------------------------------------------------------------
// Top-level commands
// ---------------------------------------------------------------------------

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Authenticate with GravixLayer
    Auth(AuthArgs),
    /// Manage CLI configuration and profiles
    Config(ConfigArgs),
    /// Manage cloud runtimes
    Runtime(RuntimeArgs),
    /// Manage reusable container templates
    Template(TemplateArgs),
    /// Manage named runtime snapshots
    Snapshot(SnapshotArgs),
    /// Manage secret providers for sandboxes
    Provider(ProviderArgs),
    /// Manage network policies (egress firewall) for sandboxes
    NetworkPolicy(NetworkPolicyArgs),
    /// Build, deploy, and manage AI agents
    Agent(AgentArgs),
    /// View billing, credits, and quotas
    Billing(BillingArgs),
    /// Validate a gravixlayer.json project file
    Validate(ValidateArgs),
    /// Package an agent project into a deployable archive
    Package(PackageArgs),
    /// Manage buckets and runtime mounts
    Storage(StorageArgs),
    /// Generate shell completions
    Completions(CompletionsArgs),
    /// Diagnose local CLI install / auth / config
    Doctor,
    /// Update the gravixlayer CLI to the latest version
    Update(UpdateArgs),
}

// ---------------------------------------------------------------------------
// auth
// ---------------------------------------------------------------------------

#[derive(Debug, Args)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub command: AuthCommand,
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Save an API key to the system keyring
    Login(AuthLoginArgs),
    /// Remove the stored API key
    Logout,
    /// Print the currently active API key (masked)
    Status,
    /// Print the API key in plain text (use carefully)
    Token,
    // TODO: re-enable once the backend exposes a stable API-key-compatible
    // whoami endpoint (currently /v1/users/me requires JWT, not API key).
    // Whoami,
}

#[derive(Debug, Args)]
pub struct AuthLoginArgs {
    /// API key to store (reads from stdin if omitted)
    #[arg(long)]
    pub api_key: Option<String>,
}

// ---------------------------------------------------------------------------
// config
// ---------------------------------------------------------------------------

#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Print the current configuration
    Show,
    /// Set a configuration value
    Set(ConfigSetArgs),
    /// Unset a configuration value (restore default)
    Unset(ConfigUnsetArgs),
    /// List all profiles
    Profiles,
    /// Set the active profile
    UseProfile(ConfigUseProfileArgs),
}

#[derive(Debug, Args)]
pub struct ConfigSetArgs {
    /// Key to set (e.g., default_cloud, default_region)
    pub key: String,
    /// Value
    pub value: String,
    /// Profile to modify (default: currently active profile)
    #[arg(long)]
    pub profile: Option<String>,
}

#[derive(Debug, Args)]
pub struct ConfigUnsetArgs {
    pub key: String,
    /// Profile to modify (default: currently active profile)
    #[arg(long)]
    pub profile: Option<String>,
}

#[derive(Debug, Args)]
pub struct ConfigUseProfileArgs {
    pub name: String,
}

// ---------------------------------------------------------------------------
// runtime
// ---------------------------------------------------------------------------

#[derive(Debug, Args)]
pub struct RuntimeArgs {
    #[command(subcommand)]
    pub command: RuntimeCommand,
}

#[derive(Debug, Subcommand)]
pub enum RuntimeCommand {
    /// Create a new runtime
    Create(RuntimeCreateArgs),
    /// List running runtimes
    List(RuntimeListArgs),
    /// Get details for a specific runtime
    Get(RuntimeGetArgs),
    /// Kill (terminate) a runtime
    Kill(RuntimeKillArgs),
    /// Pause a running runtime
    Pause(RuntimePauseArgs),
    /// Resume a paused runtime
    Resume(RuntimeResumeArgs),
    /// Show runtime resource metrics
    Metrics(RuntimeMetricsArgs),
    /// Establish/check runtime connection details
    Connect(RuntimeConnectArgs),
    /// Manage HTTPS web services on `*.service.gravixlayer.ai`
    Service(RuntimeServiceArgs),
    /// Execute a command inside a runtime (non-interactive)
    Exec(RuntimeExecArgs),
    /// Run a local script file inside a runtime
    Run(RuntimeRunArgs),
    /// Open an interactive shell session
    Shell(RuntimeShellArgs),
    /// Manage the current runtime context (default runtime ID)
    Context(RuntimeContextArgs),
    /// Manage persistent code execution contexts
    CodeContext(RuntimeCodeContextArgs),
    /// Manage SSH access
    Ssh(RuntimeSshArgs),
    /// Upload or download files from a runtime
    Files(RuntimeFilesArgs),
    /// Manage programmatic PTY sessions inside a runtime
    Pty(RuntimePtyArgs),
    /// Manage git operations inside a runtime
    Git(RuntimeGitArgs),
    /// Set the idle timeout for a runtime
    Timeout(RuntimeTimeoutArgs),
    /// Fork a running runtime into independent children
    Fork(RuntimeForkArgs),
    /// Inspect and signal background commands inside a runtime
    Commands(RuntimeCommandsArgs),
    /// Control the runtime's browser (open, navigate, act, snapshot, view)
    Browser(RuntimeBrowserArgs),
    /// Control the runtime's desktop (pixels, input, windows, apps, view, session components)
    Desktop(RuntimeDesktopArgs),
    /// Storage mounts on this runtime (bucket or git)
    Mounts(RuntimeMountsArgs),
}

#[derive(Debug, Args)]
pub struct RuntimeMountsArgs {
    #[command(subcommand)]
    pub command: RuntimeMountsCommand,
}

#[derive(Debug, Subcommand)]
pub enum RuntimeMountsCommand {
    /// Attach a bucket or git repository at a guest path
    Attach(RuntimeMountAttachArgs),
    /// List mounts attached to the runtime
    List(RuntimeMountsListArgs),
    /// Detach a mount by attachment id
    Detach(RuntimeMountDetachArgs),
}

#[derive(Debug, Args)]
pub struct RuntimeMountAttachArgs {
    /// Runtime ID (UUID)
    pub runtime_id: String,
    /// Mount as JSON — same shape as `runtime create --mount`
    #[arg(long, value_name = "JSON")]
    pub mount: String,
}

#[derive(Debug, Args)]
pub struct RuntimeMountsListArgs {
    /// Runtime ID (UUID)
    pub runtime_id: String,
}

#[derive(Debug, Args)]
pub struct RuntimeMountDetachArgs {
    /// Runtime ID (UUID)
    pub runtime_id: String,
    /// Attachment ID returned by `attach` or `list`
    pub attachment_id: String,
}

#[derive(Debug, Args)]
pub struct RuntimeBrowserArgs {
    #[command(subcommand)]
    pub command: RuntimeBrowserCommand,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopArgs {
    #[command(subcommand)]
    pub command: RuntimeDesktopCommand,
}

#[derive(Debug, Subcommand)]
pub enum RuntimeBrowserCommand {
    /// Open a browser (headed on the display by default)
    Open(RuntimeBrowserOpenArgs),
    /// Close a browser
    Close(RuntimeBrowserIdArgs),
    /// List the runtime's browsers
    List(RuntimeBrowserListArgs),
    /// Show a browser's status (url/title/targets)
    Status(RuntimeBrowserIdArgs),
    /// Navigate the browser to a URL
    Navigate(RuntimeBrowserNavigateArgs),
    /// History back / forward / reload
    Command(RuntimeBrowserCommandArgs),
    /// Screenshot the page (viewport, full-page, or an element)
    Screenshot(RuntimeBrowserShotArgs),
    /// Element digest — ref-addressed interactables for agents
    Snapshot(RuntimeBrowserSnapshotArgs),
    /// Evaluate JavaScript in the page
    Evaluate(RuntimeBrowserEvalArgs),
    /// Click/dblclick/hover/fill/type/press/hotkey/scroll an element
    Action(RuntimeBrowserActionArgs),
    /// List downloads (`--wait-name` blocks until a file lands)
    Downloads(RuntimeBrowserDownloadsArgs),
    /// Fetch a download's bytes to a local file
    Download(RuntimeBrowserDownloadArgs),
    /// Render a target to PDF
    Pdf(RuntimeBrowserPdfArgs),
    /// Mint a live-watch URL a human can open
    View(RuntimeBrowserViewArgs),
    /// Answer or dismiss a pending JavaScript dialog
    Dialog(RuntimeBrowserDialogArgs),
    /// List a browser's targets (tabs)
    Targets(RuntimeBrowserTargetsArgs),
    /// Stream browser events (one SSE page; re-page with --since)
    Events(RuntimeBrowserEventsArgs),
    /// List caller-created isolated contexts
    Contexts(RuntimeBrowserIdArgs),
    /// Create an isolated browser context
    ContextCreate(RuntimeBrowserContextCreateArgs),
    /// Delete an isolated browser context and its jars
    ContextDelete(RuntimeBrowserContextIdArgs),
    /// Export a context's cookies/local storage
    StorageExport(RuntimeBrowserStorageExportArgs),
    /// Import cookies/origins into a context
    StorageImport(RuntimeBrowserStorageImportArgs),
}

#[derive(Debug, Subcommand)]
pub enum RuntimeDesktopCommand {
    /// Start the display server (attach to the session's or spawn one)
    Enable(RuntimeDesktopEnableArgs),
    /// Stop the display server
    Disable(RuntimeDesktopIdArgs),
    /// Display status
    Status(RuntimeDesktopIdArgs),
    /// Live geometry as the display reports it now
    Info(RuntimeDesktopIdArgs),
    /// Live pointer coordinates
    Position(RuntimeDesktopIdArgs),
    /// Key names `keyboard press|hotkey` accept
    Keys(RuntimeDesktopIdArgs),
    /// Screenshot the screen to a local file
    Screenshot(RuntimeDesktopShotArgs),
    /// Live-watch URL a human can open
    View(RuntimeDesktopViewArgs),
    /// Stop the live view server and free its port
    StopView(RuntimeDesktopIdArgs),
    /// Wait until the screen stops changing
    WaitStable(RuntimeDesktopWaitArgs),
    /// Mouse input (click/down/up/drag/move/scroll)
    Mouse(RuntimeDesktopMouseArgs),
    /// Keyboard input (type/press/hotkey)
    Keyboard(RuntimeDesktopKeyboardArgs),
    /// Display clipboard (get/set, text or PNG image)
    Clipboard(RuntimeDesktopClipboardArgs),
    /// One agent step: batched inputs → settle → observation
    Act(RuntimeDesktopActArgs),
    /// Live-resize the display
    Resize(RuntimeDesktopResizeArgs),
    /// Top-level windows
    Windows(RuntimeDesktopWindowsArgs),
    /// Window op (focus/close/raise/lower/move/resize/maximize/unmaximize/minimize/fullscreen/unfullscreen)
    WindowOp(RuntimeDesktopWindowOpArgs),
    /// Installed applications
    Apps(RuntimeDesktopIdArgs),
    /// Launch an installed app by id
    Launch(RuntimeDesktopLaunchArgs),
    /// Open a file/URL with its registered handler
    Open(RuntimeDesktopOpenArgs),
    /// Declared session components and their live states
    Components(RuntimeDesktopIdArgs),
    /// Start a session component
    Start(RuntimeDesktopComponentArgs),
    /// Stop a session component
    Stop(RuntimeDesktopComponentArgs),
    /// Restart a session component
    Restart(RuntimeDesktopComponentArgs),
    /// Journal logs for a component
    Logs(RuntimeDesktopLogsArgs),
    /// Flat accessibility digest of the desktop tree
    A11ySnapshot(RuntimeDesktopA11ySnapshotArgs),
    /// Search the accessibility tree by role/name
    A11yFind(RuntimeDesktopA11yFindArgs),
    /// Ref-addressed accessibility action
    A11yAction(RuntimeDesktopA11yActionArgs),
    /// List recordings (active first, then completed)
    Recordings(RuntimeDesktopIdArgs),
    /// One recording's metadata
    Recording(RuntimeDesktopRecordingArgs),
    /// Start a display recording
    RecordStart(RuntimeDesktopRecordStartArgs),
    /// Finalize a recording's MP4 and seal its trajectory
    RecordStop(RuntimeDesktopRecordingArgs),
    /// Stamp a named mark into the recording
    RecordMark(RuntimeDesktopRecordMarkArgs),
    /// Delete a recording's artifacts
    RecordDelete(RuntimeDesktopRecordingArgs),
    /// Download a recording's video or trajectory artifact
    RecordDownload(RuntimeDesktopRecordDownloadArgs),
    /// Live-view takeover mode (shared|human|agent)
    ViewControl(RuntimeDesktopViewControlArgs),
}

// ── browser arg structs ─────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct RuntimeBrowserListArgs {
    /// Runtime ID (UUID)
    pub id: String,
}

#[derive(Debug, Args)]
pub struct RuntimeBrowserIdArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Browser ID (e.g. b1)
    #[arg(long, default_value = "b1")]
    pub browser: String,
}

#[derive(Debug, Args)]
pub struct RuntimeBrowserOpenArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Run headless (no window on the display)
    #[arg(long)]
    pub headless: bool,
    /// Browser engine
    #[arg(long, default_value = "chromium")]
    pub engine: String,
    /// Persistent profile directory inside the runtime
    #[arg(long)]
    pub profile_dir: Option<String>,
    /// Extra engine arguments (repeatable)
    #[arg(long = "arg", allow_hyphen_values = true)]
    pub args: Vec<String>,
    /// Initial URL
    #[arg(long)]
    pub url: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeBrowserNavigateArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Browser ID (e.g. b1)
    #[arg(long, default_value = "b1")]
    pub browser: String,
    /// URL to navigate to
    pub url: String,
    /// Target (tab) ID — empty uses the active target
    #[arg(long)]
    pub target: Option<String>,
    /// load | domcontentloaded | networkidle | commit
    #[arg(long, default_value = "load")]
    pub wait_until: String,
    /// Navigation timeout (ms)
    #[arg(long, default_value_t = 30_000)]
    pub timeout_ms: u64,
}

#[derive(Debug, Args)]
pub struct RuntimeBrowserCommandArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Browser ID (e.g. b1)
    #[arg(long, default_value = "b1")]
    pub browser: String,
    /// back | forward | reload | activate
    pub verb: String,
    /// Target (tab) ID — empty uses the active target
    #[arg(long)]
    pub target: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeBrowserShotArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Browser ID (e.g. b1)
    #[arg(long, default_value = "b1")]
    pub browser: String,
    /// Output file (png/jpeg)
    #[arg(long, default_value = "screenshot.png")]
    pub out: String,
    /// Target (tab) ID
    #[arg(long)]
    pub target: Option<String>,
    /// png | jpeg
    #[arg(long, default_value = "png")]
    pub format: String,
    /// Capture the full scrollable page
    #[arg(long)]
    pub full_page: bool,
    /// Element ref/selector to clip to (e.g. e3, css:#el)
    #[arg(long)]
    pub element: Option<String>,
    /// Full element-target object as JSON (shadow DOM / iframe clips)
    #[arg(long, conflicts_with = "element")]
    pub element_json: Option<String>,
    /// Wait until pixel-stable before capturing (ms)
    #[arg(long)]
    pub wait_stable_ms: Option<u64>,
    /// Overall timeout (ms)
    #[arg(long)]
    pub timeout_ms: Option<u64>,
    /// JPEG quality 0–100 — ignored for PNG
    #[arg(long)]
    pub quality: Option<u32>,
}

#[derive(Debug, Args)]
pub struct RuntimeBrowserSnapshotArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Browser ID (e.g. b1)
    #[arg(long, default_value = "b1")]
    pub browser: String,
    /// Target (tab) ID
    #[arg(long)]
    pub target: Option<String>,
    /// Digest the whole page, not just the viewport
    #[arg(long)]
    pub full_page: bool,
    /// Diff against a previous snapshot rev — only changed elements
    #[arg(long)]
    pub rev: Option<String>,
    /// Keep only elements with this ARIA role
    #[arg(long)]
    pub filter_role: Option<String>,
    /// Keep only elements whose name contains this substring
    #[arg(long)]
    pub filter_name: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeBrowserEvalArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Browser ID (e.g. b1)
    #[arg(long, default_value = "b1")]
    pub browser: String,
    /// JavaScript expression
    pub expression: String,
    /// Target (tab) ID
    #[arg(long)]
    pub target: Option<String>,
    /// Await a returned promise
    #[arg(long, default_value_t = true)]
    pub await_promise: bool,
    /// JavaScript world: isolated (default) | main
    #[arg(long, default_value = "isolated")]
    pub world: String,
    /// Execution timeout (ms)
    #[arg(long, default_value_t = 30_000)]
    pub timeout_ms: u64,
}

#[derive(Debug, Args)]
pub struct RuntimeBrowserActionArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Browser ID (e.g. b1)
    #[arg(long, default_value = "b1")]
    pub browser: String,
    /// click | dblclick | hover | fill | type | press | hotkey | scroll
    pub verb: String,
    /// Element: ref (e3), selector (css:#x / text=Sign in), or x,y
    #[arg(long)]
    pub target: Option<String>,
    /// Full element-target object as JSON — `{"by":"css","value":"#x",
    /// "pierce":true,"inner":{"by":"frame","value":"f"}` for shadow
    /// DOM / iframe targets
    #[arg(long, conflicts_with = "target")]
    pub target_json: Option<String>,
    /// Target (tab) ID
    #[arg(long)]
    pub tab: Option<String>,
    /// Text for fill/type; key for press; "ctrl+c" for hotkey; "dx,dy" scroll
    #[arg(long)]
    pub value: Option<String>,
    /// Secret name — fills without the value crossing the client
    #[arg(long)]
    pub secret_ref: Option<String>,
    /// Mouse button for click/dblclick: left (default) | right | middle
    #[arg(long)]
    pub button: Option<String>,
    /// Click count for click (2 = double-click)
    #[arg(long)]
    pub click_count: Option<u32>,
    /// Skip the actionability gate
    #[arg(long)]
    pub force: bool,
    /// Skip the post-action settle wait
    #[arg(long)]
    pub no_settle: bool,
    /// Per-char delay (ms) for type
    #[arg(long)]
    pub delay_ms: Option<u64>,
    /// Settle-wait timeout (ms) — 0 uses the server default
    #[arg(long)]
    pub settle_timeout_ms: Option<u64>,
    /// Target-resolution bound (ms) — 0 uses the server default
    #[arg(long)]
    pub target_timeout_ms: Option<u64>,
}

#[derive(Debug, Args)]
pub struct RuntimeBrowserDownloadArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Browser ID (e.g. b1)
    #[arg(long, default_value = "b1")]
    pub browser: String,
    /// Download ID
    pub download_id: String,
    /// Output file (defaults to the download's filename)
    #[arg(long)]
    pub out: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeBrowserDownloadsArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Browser ID (e.g. b1)
    #[arg(long, default_value = "b1")]
    pub browser: String,
    /// Block until a download whose filename contains this lands
    #[arg(long)]
    pub wait_name: Option<String>,
    /// Wait bound (ms) — 0 uses the server default
    #[arg(long)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Args)]
pub struct RuntimeBrowserPdfArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Browser ID (e.g. b1)
    #[arg(long, default_value = "b1")]
    pub browser: String,
    /// Target (tab) ID
    pub target: String,
    /// Output file
    #[arg(long, default_value = "page.pdf")]
    pub out: String,
    /// Landscape orientation
    #[arg(long)]
    pub landscape: bool,
    /// Print background graphics
    #[arg(long, default_value_t = true)]
    pub print_background: bool,
    /// Page ranges (e.g. "1-3,5")
    #[arg(long)]
    pub page_ranges: Option<String>,
    /// Paper width in inches (0 = letter 8.5)
    #[arg(long)]
    pub paper_width_in: Option<f64>,
    /// Paper height in inches (0 = letter 11)
    #[arg(long)]
    pub paper_height_in: Option<f64>,
    /// Uniform margin in inches (sets all four sides)
    #[arg(long)]
    pub margin_in: Option<f64>,
    /// Top margin in inches (overrides --margin-in)
    #[arg(long)]
    pub margin_top_in: Option<f64>,
    /// Bottom margin in inches (overrides --margin-in)
    #[arg(long)]
    pub margin_bottom_in: Option<f64>,
    /// Left margin in inches (overrides --margin-in)
    #[arg(long)]
    pub margin_left_in: Option<f64>,
    /// Right margin in inches (overrides --margin-in)
    #[arg(long)]
    pub margin_right_in: Option<f64>,
}

#[derive(Debug, Args)]
pub struct RuntimeBrowserDialogArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Browser ID (e.g. b1)
    #[arg(long, default_value = "b1")]
    pub browser: String,
    /// Target (tab) ID
    #[arg(long)]
    pub target: Option<String>,
    /// Accept (default dismiss)
    #[arg(long)]
    pub accept: bool,
    /// Text for a prompt dialog
    #[arg(long)]
    pub prompt_text: Option<String>,
    /// Keep auto-dismissing future dialogs (default: handled here)
    #[arg(long)]
    pub auto_dismiss: bool,
}

#[derive(Debug, Args)]
pub struct RuntimeBrowserTargetsArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Browser ID (e.g. b1)
    #[arg(long, default_value = "b1")]
    pub browser: String,
    /// Open a new tab at this URL instead of listing
    #[arg(long)]
    pub url: Option<String>,
    /// Don't bring the new tab to the front
    #[arg(long)]
    pub no_activate: bool,
    /// Open the new tab inside an isolated context
    #[arg(long)]
    pub context: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeBrowserViewArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Browser ID (e.g. b1)
    #[arg(long, default_value = "b1")]
    pub browser: String,
    /// Target (tab) ID — headless screencast follows it
    #[arg(long)]
    pub target: Option<String>,
    /// Frames per second (max 30)
    #[arg(long)]
    pub fps: Option<u32>,
    /// JPEG quality 1-100
    #[arg(long)]
    pub quality: Option<u32>,
    /// Accepted for compatibility — every view now mints a fresh token
    /// (each mint invalidates the previous link)
    #[arg(long)]
    pub rotate_token: bool,
    /// control (input allowed) | watch (view-only link)
    #[arg(long)]
    pub role: Option<String>,
}

// ── display arg structs ─────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct RuntimeDesktopIdArgs {
    /// Runtime ID (UUID)
    pub id: String,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopEnableArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Framebuffer width (default: platform display geometry)
    #[arg(long)]
    pub width: Option<u32>,
    /// Framebuffer height (default: platform display geometry)
    #[arg(long)]
    pub height: Option<u32>,
    /// X11 display name
    #[arg(long, default_value = ":0")]
    pub display: String,
    /// Window manager (default: none — a bare desktop)
    #[arg(long)]
    pub wm: Option<String>,
    /// auto (attach when the image ships a display, else spawn) | spawn | attach
    #[arg(long)]
    pub mode: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopShotArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Output file (png/jpeg)
    #[arg(long, default_value = "screen.png")]
    pub out: String,
    /// Wait until pixel-stable before capturing (ms)
    #[arg(long)]
    pub wait_stable_ms: Option<u64>,
    /// Overall timeout (ms)
    #[arg(long)]
    pub timeout_ms: Option<u64>,
    /// png | jpeg | webp
    #[arg(long, default_value = "png")]
    pub format: String,
    /// Crop region as x,y,w,h
    #[arg(long)]
    pub region: Option<String>,
    /// Downscale percentage (e.g. 50)
    #[arg(long)]
    pub scale_pct: Option<u32>,
    /// Fit inside W px preserving aspect
    #[arg(long)]
    pub max_width: Option<u32>,
    /// Fit inside H px preserving aspect
    #[arg(long)]
    pub max_height: Option<u32>,
    /// JPEG/WebP quality 1-100
    #[arg(long)]
    pub quality: Option<u32>,
    /// Composite the pointer into the shot
    #[arg(long)]
    pub cursor: bool,
    /// Skip encode/transfer when the frame still equals this revision
    #[arg(long)]
    pub if_rev: Option<u64>,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopViewArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Frames per second (max 30)
    #[arg(long)]
    pub fps: Option<u32>,
    /// JPEG quality 1-100
    #[arg(long)]
    pub quality: Option<u32>,
    /// Accepted for compatibility — every view now mints a fresh token
    /// (each mint invalidates the previous link)
    #[arg(long)]
    pub rotate_token: bool,
    /// control (input allowed) | watch (view-only link)
    #[arg(long)]
    pub role: Option<String>,
    /// Stream only this window's pixels
    #[arg(long)]
    pub window: Option<u32>,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopWaitArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// How long the screen must be pixel-quiet (ms)
    #[arg(long, default_value_t = 500)]
    pub stable_ms: u64,
    /// Overall timeout (ms)
    #[arg(long, default_value_t = 10_000)]
    pub timeout_ms: u64,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopMouseArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// click | down | up | drag | move | scroll
    pub verb: String,
    /// X coordinate (drag: start X — defaults to current position)
    #[arg(long)]
    pub x: Option<i32>,
    /// Y coordinate (drag: start Y — defaults to current position)
    #[arg(long)]
    pub y: Option<i32>,
    /// left | middle | right
    #[arg(long, default_value = "left")]
    pub button: String,
    /// Scroll delta X
    #[arg(long)]
    pub dx: Option<i32>,
    /// Scroll delta Y
    #[arg(long)]
    pub dy: Option<i32>,
    /// Drag destination X
    #[arg(long)]
    pub to_x: Option<i32>,
    /// Drag destination Y
    #[arg(long)]
    pub to_y: Option<i32>,
    /// Drag interpolation steps
    #[arg(long)]
    pub steps: Option<u32>,
    /// Drag duration (ms)
    #[arg(long)]
    pub duration_ms: Option<u64>,
    /// Click count (2 = double-click)
    #[arg(long)]
    pub click_count: Option<u32>,
    /// Comma-separated held modifiers (ctrl,shift,…)
    #[arg(long)]
    pub modifiers: Option<String>,
    /// display | screenshot (map fitted-screenshot pixels back)
    #[arg(long)]
    pub coordinate_space: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopKeyboardArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// type | press | hotkey
    pub verb: String,
    /// Text for `type`; key name for `press`; "ctrl,c" for `hotkey`
    #[arg(long)]
    pub value: Option<String>,
    /// Comma-separated keys for `hotkey`
    #[arg(long)]
    pub keys: Option<String>,
    /// Per-char delay (ms) for `type`
    #[arg(long)]
    pub delay_ms: Option<u64>,
    /// type mode: keys (keystrokes) | paste (clipboard + ctrl+v)
    #[arg(long)]
    pub mode: Option<String>,
    /// Comma-separated held modifiers (ctrl,shift,…)
    #[arg(long)]
    pub modifiers: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopClipboardArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// get | set
    pub verb: String,
    /// Text for `set`
    #[arg(long)]
    pub text: Option<String>,
    /// PNG file to `set` as the clipboard image
    #[arg(long)]
    pub png: Option<String>,
    /// Write a `get` image payload to this file
    #[arg(long)]
    pub png_out: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopActArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Actions as a JSON array, or a path to a JSON file (`-` for stdin)
    #[arg(long)]
    pub actions: String,
    /// Extra quiet time required after the last change (ms)
    #[arg(long)]
    pub settle_ms: Option<u64>,
    /// Overall budget for actions + settle (ms)
    #[arg(long)]
    pub timeout_ms: Option<u64>,
    /// Observation shot options as JSON (format/region/quality/…/if_rev);
    /// omit for no image
    #[arg(long)]
    pub observe: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopResizeArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// New framebuffer width
    pub width: u32,
    /// New framebuffer height
    pub height: u32,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopWindowsArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Filter by window-class substring
    #[arg(long)]
    pub class: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopWindowOpArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Window id (see `windows`)
    pub window: u32,
    /// focus | close | raise | lower | move | resize | maximize | unmaximize | minimize | fullscreen | unfullscreen
    pub op: String,
    /// Move destination X
    #[arg(long)]
    pub x: Option<i32>,
    /// Move destination Y
    #[arg(long)]
    pub y: Option<i32>,
    /// Resize width
    #[arg(long)]
    pub w: Option<u32>,
    /// Resize height
    #[arg(long)]
    pub h: Option<u32>,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopLaunchArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Application id (see `apps`)
    pub app: String,
    /// URI to substitute into the app's exec line
    #[arg(long)]
    pub uri: Option<String>,
    /// Wait until a new top-level window maps
    #[arg(long)]
    pub wait_window: bool,
    /// Window-wait timeout (ms)
    #[arg(long)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopOpenArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// File path or URL to open
    pub target: String,
    /// Wait until a new top-level window maps
    #[arg(long)]
    pub wait_window: bool,
    /// Window-wait timeout (ms)
    #[arg(long)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopComponentArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Component name (see `desktop status`)
    pub name: String,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopLogsArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Component name (see `desktop status`)
    pub name: String,
    /// Resume cursor from a previous page
    #[arg(long)]
    pub cursor: Option<String>,
    /// Cap returned entries
    #[arg(long)]
    pub max: Option<u32>,
    /// Priority filter (e.g. err)
    #[arg(long)]
    pub priority: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeBrowserEventsArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Browser ID (e.g. b1)
    #[arg(long, default_value = "b1")]
    pub browser: String,
    /// Resume strictly after this event sequence number
    #[arg(long, default_value_t = 0)]
    pub since: u64,
    /// Bound the long-poll page (ms; server default 25000, cap 30000)
    #[arg(long, default_value_t = 0)]
    pub wait_ms: u64,
}

#[derive(Debug, Args)]
pub struct RuntimeBrowserContextCreateArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Browser ID (e.g. b1)
    #[arg(long, default_value = "b1")]
    pub browser: String,
    /// Proxy server URL for the context
    #[arg(long)]
    pub proxy: Option<String>,
    /// Storage state seed (Playwright-compatible JSON object)
    #[arg(long)]
    pub storage_state: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeBrowserContextIdArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Browser ID (e.g. b1)
    #[arg(long, default_value = "b1")]
    pub browser: String,
    /// Context ID
    pub context_id: String,
}

#[derive(Debug, Args)]
pub struct RuntimeBrowserStorageExportArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Browser ID (e.g. b1)
    #[arg(long, default_value = "b1")]
    pub browser: String,
    /// Context ID — omitted exports the default context
    #[arg(long)]
    pub context: Option<String>,
    /// Write the storage state to this file instead of stdout
    #[arg(long)]
    pub out: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeBrowserStorageImportArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Browser ID (e.g. b1)
    #[arg(long, default_value = "b1")]
    pub browser: String,
    /// Context ID — omitted imports into the default context
    #[arg(long)]
    pub context: Option<String>,
    /// Storage state as inline JSON (object or pre-serialized string)
    #[arg(long, conflicts_with = "state_file")]
    pub state: Option<String>,
    /// Read the storage state from this file
    #[arg(long, conflicts_with = "state")]
    pub state_file: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopA11ySnapshotArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Scope: "full" (default) every app, "app" one app, "focus" the focused subtree
    #[arg(long)]
    pub scope: Option<String>,
    /// App name substring (scope=app)
    #[arg(long)]
    pub app: Option<String>,
    /// App pid (scope=app)
    #[arg(long)]
    pub pid: Option<u32>,
    /// Cap tree depth
    #[arg(long)]
    pub max_depth: Option<u32>,
    /// Cap returned elements
    #[arg(long)]
    pub limit: Option<u32>,
    /// Include hidden/non-visible elements (default shows only visible)
    #[arg(long)]
    pub all: bool,
    /// Only these roles (repeatable)
    #[arg(long = "role")]
    pub roles: Vec<String>,
    /// Name substring filter
    #[arg(long)]
    pub name: Option<String>,
    /// Only nodes changed since --rev (or the last-known tree when --rev is 0)
    #[arg(long)]
    pub diff: bool,
    /// Revision for --diff
    #[arg(long, default_value_t = 0)]
    pub rev: u64,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopA11yFindArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Role substring
    #[arg(long)]
    pub role: Option<String>,
    /// Name substring (case-insensitive)
    #[arg(long)]
    pub name: Option<String>,
    /// Scope the search below this element ref (omitted = whole tree)
    #[arg(long = "ref")]
    pub r#ref: Option<String>,
    /// Cap results
    #[arg(long)]
    pub max: Option<u32>,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopA11yActionArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// click | focus | invoke | set_value | type
    pub op: String,
    /// Target element ref (e.g. d12)
    #[arg(long = "ref")]
    pub r#ref: String,
    /// Named action for invoke (e.g. "press")
    #[arg(long)]
    pub action: Option<String>,
    /// Action index for invoke when no name is given
    #[arg(long)]
    pub action_index: Option<u32>,
    /// Text for type/set_value
    #[arg(long)]
    pub text: Option<String>,
    /// Numeric value for set_value
    #[arg(long)]
    pub value: Option<f64>,
    /// Settle timeout (ms)
    #[arg(long)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopRecordingArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Recording ID
    pub rec_id: String,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopRecordStartArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Components to record — video|input|window|app (repeatable; empty = all)
    #[arg(long = "include")]
    pub include: Vec<String>,
    /// Recording label
    #[arg(long)]
    pub label: Option<String>,
    /// Scope capture to one window's rect
    #[arg(long)]
    pub window_id: Option<u32>,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopRecordMarkArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Recording ID
    pub rec_id: String,
    /// Mark label
    pub label: String,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopRecordDownloadArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Recording ID
    pub rec_id: String,
    /// Local destination path
    pub local: PathBuf,
    /// Artifact to download: video | trajectory
    #[arg(long, default_value = "video", value_parser = ["video", "trajectory"])]
    pub artifact: String,
}

#[derive(Debug, Args)]
pub struct RuntimeDesktopViewControlArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// shared | human | agent
    #[arg(value_parser = ["shared", "human", "agent"])]
    pub mode: String,
}

#[derive(Debug, Args)]
pub struct RuntimeCreateArgs {
    /// Container template (default: base-small). Mutually exclusive with --snapshot.
    #[arg(short = 't', long)]
    pub template: Option<String>,
    /// Restore from a named snapshot (id or name). Mutually exclusive with --template.
    #[arg(long, conflicts_with = "template")]
    pub snapshot: Option<String>,
    /// Cloud provider
    #[arg(long, default_value = ResolvedConfig::DEFAULT_CLOUD)]
    pub cloud: String,
    /// Deployment region
    #[arg(long, default_value = ResolvedConfig::DEFAULT_REGION)]
    pub region: String,
    /// Idle timeout in seconds (0 = no timeout)
    #[arg(long)]
    pub timeout: Option<u64>,
    /// Allow internet access inside the runtime
    #[arg(long)]
    pub internet_access: Option<bool>,
    /// Key=Value environment variables (repeatable)
    #[arg(long = "env", short = 'e', value_name = "KEY=VALUE")]
    pub env_vars: Vec<String>,
    /// Removed — agent binding moved to the `mcp` attach spec; passing this
    /// flag fails with a migration hint instead of a server 400.
    #[arg(long)]
    pub agent_id: Option<String>,
    /// Provider IDs to attach at creation (repeatable)
    #[arg(long = "provider", value_name = "PROVIDER_ID")]
    pub providers: Vec<String>,
    /// Network policy IDs to attach at creation (repeatable)
    #[arg(long = "network-policy", value_name = "POLICY_ID")]
    pub network_policies: Vec<String>,
    /// Storage mount as JSON, e.g. '{"kind":"bucket","path":"/mnt/data","bucket":"<id-or-name>","mode":"rw"}' or '{"kind":"git","path":"/mnt/repo","url":"https://..."}' (repeatable)
    #[arg(long = "mount", value_name = "JSON")]
    pub mounts: Vec<String>,
    /// JSON metadata tags
    #[arg(long)]
    pub metadata: Option<String>,
    /// Wait for the runtime to reach RUNNING status
    #[arg(long, default_value_t = true)]
    pub wait: bool,
    /// Seconds to wait before timeout
    #[arg(long, default_value_t = 120)]
    pub wait_timeout: u64,
}

#[derive(Debug, Args)]
pub struct RuntimeListArgs {
    /// Maximum number of runtimes to return
    #[arg(long, default_value_t = 100)]
    pub limit: u32,
    /// Pagination offset
    #[arg(long, default_value_t = 0)]
    pub offset: u32,
}

#[derive(Debug, Args)]
pub struct RuntimeGetArgs {
    /// Runtime ID (UUID)
    pub id: String,
}

#[derive(Debug, Args)]
pub struct RuntimeConnectArgs {
    /// Runtime ID (UUID)
    pub id: String,
}

#[derive(Debug, Args)]
pub struct RuntimeServiceArgs {
    #[command(subcommand)]
    pub command: RuntimeServiceCommand,
}

#[derive(Debug, Subcommand)]
pub enum RuntimeServiceCommand {
    /// Open (or refresh) a public HTTPS web service URL for a guest port
    #[command(name = "web-url")]
    WebUrl(RuntimeServiceWebUrlArgs),
    /// List active web services for a runtime
    List(RuntimeServiceListArgs),
    /// Revoke a web service for a guest port
    Revoke(RuntimeServiceRevokeArgs),
}

#[derive(Debug, Args)]
pub struct RuntimeServiceWebUrlArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Guest HTTP port to expose
    pub port: u16,
    /// TTL in seconds (default 3600, max 86400)
    #[arg(long, default_value_t = 3600)]
    pub expires_in: u64,
    /// Allow unauthenticated public access (default: private + token)
    #[arg(long, default_value_t = false)]
    pub public: bool,
    /// Mint a new private token (invalidates previous token / browser cookies)
    #[arg(long, default_value_t = false)]
    pub rotate_token: bool,
}

#[derive(Debug, Args)]
pub struct RuntimeServiceListArgs {
    /// Runtime ID (UUID)
    pub id: String,
}

#[derive(Debug, Args)]
pub struct RuntimeServiceRevokeArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Guest HTTP port
    pub port: u16,
}

#[derive(Debug, Args)]
pub struct RuntimeKillArgs {
    /// Runtime ID (UUID) — omit when using --all
    #[arg(required_unless_present = "all")]
    pub id: Option<String>,
    /// Skip confirmation prompt
    #[arg(long, short = 'y')]
    pub yes: bool,
    /// Kill all runtimes (requires confirmation unless --yes is also given)
    #[arg(long, conflicts_with = "id")]
    pub all: bool,
}

#[derive(Debug, Args)]
pub struct RuntimePauseArgs {
    /// Runtime ID (UUID)
    pub id: String,
}

#[derive(Debug, Args)]
pub struct RuntimeResumeArgs {
    /// Runtime ID (UUID)
    pub id: String,
}

#[derive(Debug, Args)]
pub struct RuntimeMetricsArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Watch mode: refresh every N seconds
    #[arg(long, short = 'w')]
    pub watch: Option<u64>,
}

#[derive(Debug, Args)]
pub struct RuntimeExecArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Command and arguments to run
    #[arg(trailing_var_arg = true, required = true)]
    pub command: Vec<String>,
    /// Working directory inside the runtime
    #[arg(long)]
    pub workdir: Option<String>,
    /// Key=Value environment variables
    #[arg(long = "env", short = 'e', value_name = "KEY=VALUE")]
    pub env_vars: Vec<String>,
    /// Execution timeout in seconds. `0` with `--background` sets no deadline.
    #[arg(long, default_value_t = 300)]
    pub timeout: u64,
    /// Stream stdout/stderr incrementally using the backend SSE path
    #[arg(long)]
    pub stream: bool,
    /// Return as soon as the command is running. Prints the command record, including its pid.
    #[arg(long)]
    pub background: bool,
}

#[derive(Debug, Args)]
pub struct RuntimeRunArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Path to script file to upload and run. Omit when --code is given.
    #[arg(conflicts_with = "code")]
    pub script: Option<PathBuf>,
    /// Inline code to execute (alternative to a script file)
    #[arg(long, short = 'c', value_name = "CODE")]
    pub code: Option<String>,
    /// Language: python, javascript, typescript, or bash
    /// (inferred from the script extension when omitted)
    #[arg(long)]
    pub language: Option<String>,
    /// Persistent code context to execute in
    #[arg(long)]
    pub context_id: Option<String>,
    /// Key=Value environment variables
    #[arg(long = "env", short = 'e', value_name = "KEY=VALUE")]
    pub env_vars: Vec<String>,
    /// Execution timeout in seconds (sent to backend as `timeout`)
    #[arg(long, default_value_t = 300)]
    pub timeout: u64,
    /// Stream stdout/stderr incrementally using the backend SSE path
    #[arg(long)]
    pub stream: bool,
}

#[derive(Debug, Args)]
pub struct RuntimeShellArgs {
    /// Runtime ID (UUID)
    pub id: String,
    /// Shell binary (default: /bin/bash)
    #[arg(long, default_value = "/bin/bash")]
    pub shell: String,
    /// Project ID to associate with this terminal session
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeContextArgs {
    #[command(subcommand)]
    pub command: RuntimeContextCommand,
}

#[derive(Debug, Subcommand)]
pub enum RuntimeContextCommand {
    /// Show the current context runtime ID
    Show,
    /// Set the current context to a runtime ID
    Set(RuntimeContextSetArgs),
    /// Clear the current context
    Clear,
}

#[derive(Debug, Args)]
pub struct RuntimeContextSetArgs {
    pub id: String,
}

#[derive(Debug, Args)]
pub struct RuntimeCodeContextArgs {
    #[command(subcommand)]
    pub command: RuntimeCodeContextCommand,
}

#[derive(Debug, Subcommand)]
pub enum RuntimeCodeContextCommand {
    /// Create a persistent code execution context
    Create(RuntimeCodeContextCreateArgs),
    /// Get a code execution context
    Get(RuntimeCodeContextGetArgs),
    /// Delete a code execution context
    Delete(RuntimeCodeContextDeleteArgs),
}

#[derive(Debug, Args)]
pub struct RuntimeCodeContextCreateArgs {
    pub runtime_id: String,
    #[arg(long, default_value = "python")]
    pub language: String,
    #[arg(long)]
    pub cwd: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeCodeContextGetArgs {
    pub runtime_id: String,
    pub context_id: String,
}

#[derive(Debug, Args)]
pub struct RuntimeCodeContextDeleteArgs {
    pub runtime_id: String,
    pub context_id: String,
}

#[derive(Debug, Args)]
pub struct RuntimeSshArgs {
    #[command(subcommand)]
    pub command: RuntimeSshCommand,
}

#[derive(Debug, Subcommand)]
pub enum RuntimeSshCommand {
    /// Enable SSH access and return connection details
    Enable(RuntimeSshEnableArgs),
    /// Disable SSH access
    Disable(RuntimeSshDisableArgs),
    /// Show SSH status
    Status(RuntimeSshStatusArgs),
}

#[derive(Debug, Args)]
pub struct RuntimeSshEnableArgs {
    pub runtime_id: String,
    /// Regenerate SSH keys even if SSH is already enabled
    #[arg(long)]
    pub regenerate_keys: bool,
}

#[derive(Debug, Args)]
pub struct RuntimeSshDisableArgs {
    pub runtime_id: String,
}

#[derive(Debug, Args)]
pub struct RuntimeSshStatusArgs {
    pub runtime_id: String,
}

#[derive(Debug, Args)]
pub struct RuntimeFilesArgs {
    #[command(subcommand)]
    pub command: RuntimeFilesCommand,
}

#[derive(Debug, Subcommand)]
pub enum RuntimeFilesCommand {
    /// Upload a local file to a runtime
    Upload(RuntimeFilesUploadArgs),
    /// Download a file from a runtime to local disk
    Download(RuntimeFilesDownloadArgs),
    /// List files inside a runtime directory
    #[command(name = "ls")]
    List(RuntimeFilesListArgs),
    /// Print the contents of a file
    Cat(RuntimeFilesCatArgs),
    /// Write inline text to a file
    Write(RuntimeFilesWriteArgs),
    /// Upload multiple local files in one request
    WriteMany(RuntimeFilesWriteManyArgs),
    /// Show native stat metadata for a file or directory
    Info(RuntimeFilesInfoArgs),
    /// Delete a file inside a runtime
    #[command(name = "rm")]
    Delete(RuntimeFilesDeleteArgs),
    /// Create a directory
    Mkdir(RuntimeFilesMkdirArgs),
    /// Change file permissions
    Chmod(RuntimeFilesChmodArgs),
    /// Move or rename a path inside a runtime
    #[command(name = "mv")]
    Move(RuntimeFilesMoveArgs),
    /// Copy a file or directory inside a runtime
    #[command(name = "cp")]
    Copy(RuntimeFilesCopyArgs),
    /// Change the owning user and/or group of a path
    Chown(RuntimeFilesChownArgs),
    /// Watch a directory for filesystem changes (inotify backed)
    Watch(RuntimeFilesWatchArgs),
    /// Find files by name glob and/or content pattern
    Find(RuntimeFilesFindArgs),
    /// Replace a pattern across every matching file
    Replace(RuntimeFilesReplaceArgs),
}

#[derive(Debug, Args)]
pub struct RuntimeFilesMoveArgs {
    pub runtime_id: String,
    /// Existing path inside the runtime
    pub source: String,
    /// New path inside the runtime
    pub destination: String,
    /// Replace the destination if it already exists
    #[arg(long)]
    pub overwrite: bool,
}

#[derive(Debug, Args)]
pub struct RuntimeFilesCopyArgs {
    pub runtime_id: String,
    /// Existing path inside the runtime
    pub source: String,
    /// Destination path inside the runtime
    pub destination: String,
    /// Copy directories recursively
    #[arg(long, short = 'r')]
    pub recursive: bool,
    /// Replace the destination if it already exists
    #[arg(long)]
    pub overwrite: bool,
}

#[derive(Debug, Args)]
pub struct RuntimeFilesChownArgs {
    pub runtime_id: String,
    pub path: String,
    /// New owning user (name or numeric UID)
    #[arg(long)]
    pub user: Option<String>,
    /// New owning group (name or numeric GID)
    #[arg(long)]
    pub group: Option<String>,
    /// Apply recursively to a directory tree
    #[arg(long, short = 'r')]
    pub recursive: bool,
}

#[derive(Debug, Args)]
pub struct RuntimeFilesWatchArgs {
    pub runtime_id: String,
    /// Directory to watch
    pub path: String,
    /// Also watch subdirectories, including ones created later
    #[arg(long, short = 'r')]
    pub recursive: bool,
}

#[derive(Debug, Args)]
pub struct RuntimeFilesFindArgs {
    pub runtime_id: String,
    /// Directory to search under
    pub path: String,
    /// Content pattern to match inside files
    #[arg(long, short = 'p')]
    pub pattern: Option<String>,
    /// Shell-style name pattern, for example "*.py"
    #[arg(long, short = 'g')]
    pub glob: Option<String>,
    /// Treat the pattern as a regular expression
    #[arg(long)]
    pub regex: bool,
    /// Match case exactly (default is case-insensitive)
    #[arg(long)]
    pub case_sensitive: bool,
    /// Descend into and match dot-files
    #[arg(long)]
    pub include_hidden: bool,
    /// Stop after this many matches
    #[arg(long)]
    pub max_results: Option<u32>,
    /// Directory recursion limit
    #[arg(long)]
    pub max_depth: Option<u32>,
}

#[derive(Debug, Args)]
pub struct RuntimeFilesReplaceArgs {
    pub runtime_id: String,
    /// Directory to search under
    pub path: String,
    /// Pattern to replace
    pub pattern: String,
    /// Replacement text (use $1 for capture groups with --regex)
    pub replacement: String,
    /// Shell-style name pattern limiting which files are rewritten
    #[arg(long, short = 'g')]
    pub glob: Option<String>,
    /// Treat the pattern as a regular expression
    #[arg(long)]
    pub regex: bool,
    /// Match case exactly (default is case-insensitive)
    #[arg(long)]
    pub case_sensitive: bool,
    /// Descend into and match dot-files
    #[arg(long)]
    pub include_hidden: bool,
    /// Directory recursion limit
    #[arg(long)]
    pub max_depth: Option<u32>,
    /// Report what would change without writing any file
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct RuntimePtyArgs {
    #[command(subcommand)]
    pub command: RuntimePtyCommand,
}

#[derive(Debug, Subcommand)]
pub enum RuntimePtyCommand {
    /// Create a PTY session that outlives this command
    Create(RuntimePtyCreateArgs),
    /// List PTY sessions in a runtime
    #[command(name = "ls")]
    List(RuntimePtyListArgs),
    /// Show details for a single PTY session
    Get(RuntimePtyGetArgs),
    /// Write to a session's terminal
    Send(RuntimePtySendArgs),
    /// Resize a session's terminal
    Resize(RuntimePtyResizeArgs),
    /// Send a POSIX signal to a session (INT, TERM, KILL, HUP)
    Signal(RuntimePtySignalArgs),
    /// Attach to a session's output stream (scrollback first, then live)
    Attach(RuntimePtyAttachArgs),
    /// Terminate a PTY session
    Kill(RuntimePtyKillArgs),
}

#[derive(Debug, Args)]
pub struct RuntimePtyCreateArgs {
    pub runtime_id: String,
    /// Shell binary to launch
    #[arg(long)]
    pub shell: Option<String>,
    /// Arguments passed to the shell (repeatable, applied in order)
    #[arg(long = "arg", value_name = "ARG", allow_hyphen_values = true)]
    pub args: Vec<String>,
    /// Initial working directory
    #[arg(long)]
    pub workdir: Option<String>,
    /// Key=Value environment variables
    #[arg(long = "env", short = 'e', value_name = "KEY=VALUE")]
    pub env_vars: Vec<String>,
    /// Terminal width in columns
    #[arg(long)]
    pub cols: Option<u32>,
    /// Terminal height in rows
    #[arg(long)]
    pub rows: Option<u32>,
}

#[derive(Debug, Args)]
pub struct RuntimePtyListArgs {
    pub runtime_id: String,
}

#[derive(Debug, Args)]
pub struct RuntimePtyGetArgs {
    pub runtime_id: String,
    pub session_id: String,
}

#[derive(Debug, Args)]
pub struct RuntimePtySendArgs {
    pub runtime_id: String,
    pub session_id: String,
    /// Text to write (reads from stdin if omitted)
    pub data: Option<String>,
    /// Do not append a trailing newline
    #[arg(long)]
    pub no_newline: bool,
}

#[derive(Debug, Args)]
pub struct RuntimePtyResizeArgs {
    pub runtime_id: String,
    pub session_id: String,
    pub cols: u32,
    pub rows: u32,
}

#[derive(Debug, Args)]
pub struct RuntimePtySignalArgs {
    pub runtime_id: String,
    pub session_id: String,
    /// Signal name: INT, TERM, KILL or HUP
    pub signal: String,
}

#[derive(Debug, Args)]
pub struct RuntimePtyAttachArgs {
    pub runtime_id: String,
    pub session_id: String,
}

#[derive(Debug, Args)]
pub struct RuntimePtyKillArgs {
    pub runtime_id: String,
    pub session_id: String,
}

#[derive(Debug, Args)]
pub struct RuntimeFilesCatArgs {
    pub runtime_id: String,
    pub path: String,
}

#[derive(Debug, Args)]
pub struct RuntimeFilesWriteArgs {
    pub runtime_id: String,
    pub path: String,
    /// Content to write (reads from stdin if omitted)
    pub content: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeFilesMkdirArgs {
    pub runtime_id: String,
    pub path: String,
    /// Disable recursive parent directory creation
    #[arg(long)]
    pub no_recursive: bool,
    /// Octal directory mode (e.g. 0755)
    #[arg(long)]
    pub mode: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeFilesChmodArgs {
    pub runtime_id: String,
    pub path: String,
    /// Octal mode string (e.g. 0755)
    pub mode: String,
}

#[derive(Debug, Args)]
pub struct RuntimeFilesUploadArgs {
    pub runtime_id: String,
    /// Local source path
    pub local: PathBuf,
    /// Destination path inside the runtime
    pub remote: String,
    /// Runtime user/owner for the written file
    #[arg(long)]
    pub user: Option<String>,
    /// Octal file mode (e.g. 0644)
    #[arg(long)]
    pub mode: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeFilesWriteManyArgs {
    pub runtime_id: String,
    /// File mapping in LOCAL=REMOTE form (repeatable)
    #[arg(long = "file", value_name = "LOCAL=REMOTE", required = true)]
    pub files: Vec<String>,
    /// Runtime user/owner for written files
    #[arg(long)]
    pub user: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeFilesInfoArgs {
    pub runtime_id: String,
    pub path: String,
}

#[derive(Debug, Args)]
pub struct RuntimeFilesDownloadArgs {
    pub runtime_id: String,
    /// Source path inside the runtime
    pub remote: String,
    /// Local destination path
    pub local: PathBuf,
}

#[derive(Debug, Args)]
pub struct RuntimeFilesListArgs {
    pub runtime_id: String,
    #[arg(default_value = "/")]
    pub path: String,
}

#[derive(Debug, Args)]
pub struct RuntimeFilesDeleteArgs {
    pub runtime_id: String,
    pub path: String,
    /// Delete directories and their contents (required for non-empty dirs)
    #[arg(long, short = 'r')]
    pub recursive: bool,
    #[arg(long, short = 'y')]
    pub yes: bool,
}

#[derive(Debug, Args)]
pub struct RuntimeGitArgs {
    #[command(subcommand)]
    pub command: RuntimeGitCommand,
}

#[derive(Debug, Subcommand)]
pub enum RuntimeGitCommand {
    /// Clone a repository into a runtime
    Clone(RuntimeGitCloneArgs),
    /// Pull the latest changes inside a runtime
    Pull(RuntimeGitPullArgs),
    /// Show git status
    Status(RuntimeGitStatusArgs),
    /// List branches
    Branch(RuntimeGitBranchArgs),
    /// Checkout a branch or commit
    Checkout(RuntimeGitCheckoutArgs),
    /// Fetch from remote
    Fetch(RuntimeGitFetchArgs),
    /// Stage files
    Add(RuntimeGitAddArgs),
    /// Commit staged changes
    Commit(RuntimeGitCommitArgs),
    /// Push to remote
    Push(RuntimeGitPushArgs),
    /// Create a branch
    BranchCreate(RuntimeGitBranchCreateArgs),
    /// Delete a branch
    BranchDelete(RuntimeGitBranchDeleteArgs),
}

// Every subcommand below that operates on an existing repository requires
// `--path`. It used to default to `/`, which is never a repository, so the
// default could only ever produce a "not a git repository" failure from the
// runtime; requiring the value turns that into an immediate argument error.

#[derive(Debug, Args)]
pub struct RuntimeGitStatusArgs {
    pub runtime_id: String,
    /// Repository directory inside the runtime
    #[arg(long)]
    pub path: String,
}

#[derive(Debug, Args)]
pub struct RuntimeGitBranchArgs {
    pub runtime_id: String,
    /// Repository directory inside the runtime
    #[arg(long)]
    pub path: String,
    /// List remote branches
    #[arg(long)]
    pub remote: bool,
    /// List all branches
    #[arg(long)]
    pub all: bool,
}

#[derive(Debug, Args)]
pub struct RuntimeGitCheckoutArgs {
    pub runtime_id: String,
    pub branch: String,
    /// Repository directory inside the runtime
    #[arg(long)]
    pub path: String,
}

#[derive(Debug, Args)]
pub struct RuntimeGitFetchArgs {
    pub runtime_id: String,
    /// Repository directory inside the runtime
    #[arg(long)]
    pub path: String,
    #[arg(long)]
    pub remote: Option<String>,
    /// Token for a private HTTPS remote; used for this command only
    #[arg(long, env = "GRAVIXLAYER_GIT_TOKEN")]
    pub auth_token: Option<String>,
    /// Server-side deadline for the fetch, in seconds
    #[arg(long)]
    pub timeout_seconds: Option<u32>,
}

#[derive(Debug, Args)]
pub struct RuntimeGitAddArgs {
    pub runtime_id: String,
    /// Repository directory inside the runtime
    #[arg(long)]
    pub path: String,
    /// Paths to stage (default: all)
    #[arg(long, value_name = "FILE")]
    pub files: Vec<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeGitCommitArgs {
    pub runtime_id: String,
    #[arg(long, short = 'm', required = true)]
    pub message: String,
    /// Repository directory inside the runtime
    #[arg(long)]
    pub path: String,
    #[arg(long)]
    pub author_name: Option<String>,
    #[arg(long)]
    pub author_email: Option<String>,
    #[arg(long)]
    pub allow_empty: bool,
}

#[derive(Debug, Args)]
pub struct RuntimeGitPushArgs {
    pub runtime_id: String,
    /// Repository directory inside the runtime
    #[arg(long)]
    pub path: String,
    #[arg(long)]
    pub remote: Option<String>,
    #[arg(long)]
    pub refspec: Option<String>,
    #[arg(long)]
    pub username: Option<String>,
    #[arg(long)]
    pub password: Option<String>,
    /// Token for a private HTTPS remote; takes precedence over --username/--password
    #[arg(long, env = "GRAVIXLAYER_GIT_TOKEN")]
    pub auth_token: Option<String>,
    /// Server-side deadline for the push, in seconds
    #[arg(long)]
    pub timeout_seconds: Option<u32>,
}

#[derive(Debug, Args)]
pub struct RuntimeGitBranchCreateArgs {
    pub runtime_id: String,
    pub branch_name: String,
    /// Repository directory inside the runtime
    #[arg(long)]
    pub path: String,
    #[arg(long)]
    pub start_point: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeGitBranchDeleteArgs {
    pub runtime_id: String,
    pub branch_name: String,
    /// Repository directory inside the runtime
    #[arg(long)]
    pub path: String,
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct RuntimeGitCloneArgs {
    pub runtime_id: String,
    pub repo_url: String,
    /// Destination directory (default: /workspace/<repository name>, as `git clone` does)
    #[arg(long)]
    pub target_dir: Option<String>,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(long)]
    pub depth: Option<u32>,
    /// Token for a private HTTPS repository; used for this clone only and not
    /// stored in the checkout, so later pull/fetch/push need their own token
    #[arg(long, env = "GRAVIXLAYER_GIT_TOKEN")]
    pub auth_token: Option<String>,
    /// Server-side deadline for the clone, in seconds
    #[arg(long)]
    pub timeout_seconds: Option<u32>,
}

#[derive(Debug, Args)]
pub struct RuntimeGitPullArgs {
    pub runtime_id: String,
    /// Repository directory inside the runtime
    #[arg(long, alias = "workdir")]
    pub path: String,
    #[arg(long)]
    pub remote: Option<String>,
    #[arg(long)]
    pub branch: Option<String>,
    /// Token for a private HTTPS remote; used for this command only
    #[arg(long, env = "GRAVIXLAYER_GIT_TOKEN")]
    pub auth_token: Option<String>,
    /// Server-side deadline for the pull, in seconds
    #[arg(long)]
    pub timeout_seconds: Option<u32>,
}

#[derive(Debug, Args)]
pub struct RuntimeTimeoutArgs {
    pub id: String,
    /// New timeout in seconds
    pub seconds: u64,
}

#[derive(Debug, Args)]
pub struct RuntimeForkArgs {
    /// Runtime ID (UUID) to fork — must be running
    pub runtime_id: String,
    /// Number of children to start (1-100)
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..=100))]
    pub count: u32,
    /// Per-child idle timeout in seconds
    #[arg(long)]
    pub timeout: Option<u64>,
    /// Keep the fork's capture as a named snapshot for later reuse
    #[arg(long)]
    pub persist_snapshot: bool,
    /// Name for the persisted snapshot (requires --persist-snapshot)
    #[arg(long, requires = "persist_snapshot")]
    pub name: Option<String>,
    /// Key=Value environment overrides applied on the children (repeatable)
    #[arg(long = "env", short = 'e', value_name = "KEY=VALUE")]
    pub env_vars: Vec<String>,
    /// JSON metadata merged over the parent's metadata on the children
    #[arg(long)]
    pub metadata: Option<String>,
}

#[derive(Debug, Args)]
pub struct RuntimeCommandsArgs {
    #[command(subcommand)]
    pub command: RuntimeCommandsCommand,
}

#[derive(Debug, Subcommand)]
pub enum RuntimeCommandsCommand {
    /// List tracked commands (running and finished)
    List(RuntimeCommandsIdArgs),
    /// Show one tracked command by PID
    Get(RuntimeCommandsPidArgs),
    /// Signal a command (server defaults to KILL)
    Kill(RuntimeCommandsKillArgs),
    /// Attach to a command's output stream (scrollback first, then live)
    Attach(RuntimeCommandsPidArgs),
}

#[derive(Debug, Args)]
pub struct RuntimeCommandsIdArgs {
    /// Runtime ID (UUID)
    pub runtime_id: String,
}

#[derive(Debug, Args)]
pub struct RuntimeCommandsPidArgs {
    /// Runtime ID (UUID)
    pub runtime_id: String,
    /// Command PID
    pub pid: u32,
}

#[derive(Debug, Args)]
pub struct RuntimeCommandsKillArgs {
    /// Runtime ID (UUID)
    pub runtime_id: String,
    /// Command PID
    pub pid: u32,
    /// POSIX signal to send: KILL, TERM, INT, or HUP
    #[arg(long, value_parser = ["KILL", "TERM", "INT", "HUP"])]
    pub signal: Option<String>,
}

// ---------------------------------------------------------------------------
// template
// ---------------------------------------------------------------------------

#[derive(Debug, Args)]
pub struct TemplateArgs {
    #[command(subcommand)]
    pub command: TemplateCommand,
}

#[derive(Debug, Subcommand)]
pub enum TemplateCommand {
    /// List available templates
    List(TemplateListArgs),
    /// Get details for a specific template
    Get(TemplateGetArgs),
    /// Get template snapshot metadata
    Snapshot(TemplateGetArgs),
    /// Build a custom template from a directory
    Build(TemplateBuildArgs),
    /// Delete a custom template
    Delete(TemplateDeleteArgs),
    /// Poll build status
    Status(TemplateBuildStatusArgs),
    /// Stream a build's live event log (replays stored events, then follows)
    Events(TemplateBuildStatusArgs),
}

#[derive(Debug, Args)]
pub struct TemplateListArgs {
    #[arg(long, default_value_t = 100)]
    pub limit: u32,
    #[arg(long, default_value_t = 0)]
    pub offset: u32,
    /// Template kind filter: sandbox (default), agent, or all
    #[arg(long, default_value = "sandbox")]
    pub kind: String,
    /// Optional project filter (UUID) — forwarded when the control plane honors it
    #[arg(long)]
    pub project_id: Option<String>,
    /// Hide templates whose builds never published (pending, building,
    /// or failed) — list defaults to showing them since they hold names
    /// and accept retry/delete.
    #[arg(long)]
    pub active_only: bool,
}

#[derive(Debug, Args)]
pub struct TemplateGetArgs {
    pub id: String,
}

#[derive(Debug, Args)]
pub struct TemplateBuildArgs {
    /// Path to build context directory (default: .)
    #[arg(default_value = ".")]
    pub source: PathBuf,
    /// Path to Dockerfile — when supplied the file contents are sent directly
    /// to the template build API (bypasses auto-detection from source archive).
    #[arg(
        long,
        short = 'f',
        value_name = "FILE",
        conflicts_with = "docker_image"
    )]
    pub dockerfile: Option<PathBuf>,
    /// Docker image to use as base (e.g. python:3.12-slim). Mutually exclusive
    /// with --dockerfile. When supplied, sent as-is to the template build JSON API.
    #[arg(long, conflicts_with = "dockerfile")]
    pub docker_image: Option<String>,
    /// Optional custom template ID
    #[arg(long)]
    pub template_id: Option<String>,
    /// Name for the new template
    #[arg(long)]
    pub name: String,
    /// Description
    #[arg(long)]
    pub description: Option<String>,
    /// Framework label
    #[arg(long)]
    pub framework: Option<String>,
    /// Python version
    #[arg(long)]
    pub python_version: Option<String>,
    /// Number of vCPUs to allocate for the template VM (default: 2)
    #[arg(long)]
    pub vcpu_count: Option<u32>,
    /// Memory in MB (default: 1024)
    #[arg(long)]
    pub memory_mb: Option<u32>,
    /// Disk size in MB (default: 4096)
    #[arg(long)]
    pub disk_mb: Option<u32>,
    /// Command to run after VM starts
    #[arg(long)]
    pub start_cmd: Option<String>,
    /// Readiness check command
    #[arg(long)]
    pub ready_cmd: Option<String>,
    /// Readiness timeout in seconds
    #[arg(long)]
    pub ready_timeout_secs: Option<u32>,
    /// Environment variables in KEY=VALUE form (repeatable)
    #[arg(long = "env", short = 'e', value_name = "KEY=VALUE")]
    pub env_vars: Vec<String>,
    /// Tags in KEY=VALUE form (repeatable)
    #[arg(long = "tag", value_name = "KEY=VALUE")]
    pub tags: Vec<String>,
    /// Raw build step JSON object (repeatable)
    #[arg(long = "build-step", value_name = "JSON")]
    pub build_steps: Vec<String>,
    /// Wait for build to complete
    #[arg(long, default_value_t = true)]
    pub wait: bool,
    /// Build timeout in seconds
    #[arg(long, default_value_t = 600)]
    pub build_timeout: u64,
}

#[derive(Debug, Args)]
pub struct TemplateDeleteArgs {
    pub id: String,
    #[arg(long, short = 'y')]
    pub yes: bool,
}

#[derive(Debug, Args)]
pub struct TemplateBuildStatusArgs {
    pub build_id: String,
}

// ---------------------------------------------------------------------------
// snapshot
// ---------------------------------------------------------------------------

#[derive(Debug, Args)]
pub struct SnapshotArgs {
    #[command(subcommand)]
    pub command: SnapshotCommand,
}

#[derive(Debug, Subcommand)]
pub enum SnapshotCommand {
    /// Capture a running runtime into a named snapshot
    Create(SnapshotCreateArgs),
    /// List named snapshots
    List(SnapshotListArgs),
    /// Get a snapshot by ID or name
    Get(SnapshotGetArgs),
    /// Delete a named snapshot
    Delete(SnapshotDeleteArgs),
    /// Activate an inactive snapshot
    Activate(SnapshotActivateArgs),
    /// Deactivate a snapshot (stops new creates)
    Deactivate(SnapshotDeactivateArgs),
    /// Start runtimes from a named snapshot — no capture, parent not needed
    Fork(SnapshotForkArgs),
}

#[derive(Debug, Args)]
pub struct SnapshotCreateArgs {
    /// Project-unique snapshot name
    #[arg(long)]
    pub name: String,
    /// Source runtime UUID
    #[arg(long)]
    pub runtime_id: String,
    /// hot (memory + disk) or cold (disk only). Defaults to cold.
    #[arg(long, default_value = "cold")]
    pub kind: String,
    /// Optional description
    #[arg(long)]
    pub description: Option<String>,
}

#[derive(Debug, Args)]
pub struct SnapshotListArgs {
    #[arg(long, default_value_t = 20)]
    pub limit: u32,
    #[arg(long, default_value_t = 0)]
    pub offset: u32,
    /// Filter: hot, cold, or all
    #[arg(long)]
    pub kind: Option<String>,
    /// Filter by source runtime UUID
    #[arg(long)]
    pub runtime_id: Option<String>,
    /// Filter by catalog state
    #[arg(long)]
    pub state: Option<String>,
    /// Filter by source: runtime, template, or fork
    #[arg(long)]
    pub source: Option<String>,
}

#[derive(Debug, Args)]
pub struct SnapshotGetArgs {
    /// Snapshot UUID or project-unique name
    pub id: String,
}

#[derive(Debug, Args)]
pub struct SnapshotDeleteArgs {
    /// Snapshot UUID or project-unique name
    pub id: String,
    #[arg(long, short = 'y')]
    pub yes: bool,
}

#[derive(Debug, Args)]
pub struct SnapshotActivateArgs {
    /// Snapshot UUID or project-unique name
    pub id: String,
}

#[derive(Debug, Args)]
pub struct SnapshotDeactivateArgs {
    /// Snapshot UUID or project-unique name
    pub id: String,
}

#[derive(Debug, Args)]
pub struct SnapshotForkArgs {
    /// Snapshot UUID or project-unique name
    pub id: String,
    /// Number of runtimes to start (1-100)
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..=100))]
    pub count: u32,
    /// Per-runtime idle timeout in seconds
    #[arg(long)]
    pub timeout: Option<u64>,
    /// Key=Value environment overrides applied on the children (repeatable)
    #[arg(long = "env", short = 'e', value_name = "KEY=VALUE")]
    pub env_vars: Vec<String>,
    /// JSON metadata applied on the children
    #[arg(long)]
    pub metadata: Option<String>,
}

// ---------------------------------------------------------------------------
// agent
// ---------------------------------------------------------------------------

#[derive(Debug, Args)]
pub struct AgentArgs {
    #[command(subcommand)]
    pub command: AgentCommand,
}

#[derive(Debug, Subcommand)]
pub enum AgentCommand {
    /// Build an agent template from source
    Build(AgentBuildArgs),
    /// Deploy a built agent template
    Deploy(AgentDeployArgs),
    /// Get details for a specific deployed agent
    Get(AgentGetArgs),
    /// Invoke an agent synchronously
    Invoke(AgentInvokeArgs),
    /// Delete a deployed agent
    Destroy(AgentDestroyArgs),
    /// Show the build status for an agent build
    Status(AgentBuildStatusArgs),
    /// Initialize a new agent project scaffold (non-interactive)
    Init(AgentInitArgs),
    /// Create a new agent project interactively (guided wizard)
    Create(AgentCreateArgs),
    /// Start a local agent development server or use --runtime-sync for cloud sync
    Dev(AgentDevArgs),
    /// Build and run the agent locally in Docker
    Up(AgentUpArgs),
    /// Create a deployable source archive for an agent project
    Package(AgentPackageArgs),
    /// Generate a production Dockerfile for an agent project
    Dockerfile(AgentDockerfileArgs),
    /// Stream real-time output from a deployed agent via SSE
    Stream(AgentStreamArgs),
    /// Serve a native framework project for local development
    Serve(AgentServeArgs),
}

#[derive(Debug, Args)]
pub struct AgentBuildArgs {
    /// Path to agent project directory (default: .)
    #[arg(default_value = ".")]
    pub source: PathBuf,
    /// Name for the built agent template
    #[arg(long)]
    pub name: Option<String>,
    /// Description
    #[arg(long)]
    pub description: Option<String>,
    /// Agent framework
    #[arg(long)]
    pub framework: Option<AgentFrameworkArg>,
    /// Python version
    #[arg(long)]
    pub python_version: Option<String>,
    /// Application entrypoint command (e.g. "python -m simple_agent.app")
    #[arg(long)]
    pub entrypoint: Option<String>,
    /// Framework target when a project exposes multiple agents/graphs/apps (e.g. deep_agent)
    #[arg(long)]
    pub target: Option<String>,
    /// Ports the agent listens on (repeatable)
    #[arg(long = "port")]
    pub ports: Vec<u16>,
    /// Environment variables in KEY=VALUE form (repeatable)
    #[arg(long = "env", short = 'e', value_name = "KEY=VALUE")]
    pub env_vars: Vec<String>,
    /// Number of vCPUs for the template VM (default: 2)
    #[arg(long)]
    pub vcpu_count: Option<u32>,
    /// Memory in MB for the template VM (default: 1024)
    #[arg(long)]
    pub memory_mb: Option<u32>,
    /// Disk size in MB for the template VM (default: 4096)
    #[arg(long)]
    pub disk_mb: Option<u32>,
    /// Command to run after VM starts during snapshot phase
    #[arg(long)]
    pub start_cmd: Option<String>,
    /// Readiness command during snapshot phase
    #[arg(long)]
    pub ready_cmd: Option<String>,
    /// Readiness timeout in seconds
    #[arg(long)]
    pub ready_timeout_secs: Option<u32>,
    /// Build tags in KEY=VALUE form (repeatable)
    #[arg(long = "tag", value_name = "KEY=VALUE")]
    pub tags: Vec<String>,
    /// Wait for the build to complete
    #[arg(long, default_value_t = true)]
    pub wait: bool,
    /// Build timeout in seconds
    #[arg(long, default_value_t = 600)]
    pub build_timeout: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AgentFrameworkArg {
    Langgraph,
    Langchain,
    Crewai,
    #[value(name = "google-adk")]
    GoogleAdk,
    #[value(name = "openai-agents", alias = "openai")]
    OpenaiAgents,
    #[value(alias = "claude", alias = "claude-agent", alias = "claude-agent-sdk")]
    Anthropic,
    #[value(alias = "strands-agents")]
    Strands,
    Python,
}

impl std::fmt::Display for AgentFrameworkArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Langgraph => write!(f, "langgraph"),
            Self::Langchain => write!(f, "langchain"),
            Self::Crewai => write!(f, "crewai"),
            Self::GoogleAdk => write!(f, "google-adk"),
            Self::OpenaiAgents => write!(f, "openai-agents"),
            Self::Anthropic => write!(f, "anthropic"),
            Self::Strands => write!(f, "strands"),
            Self::Python => write!(f, "python"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AgentProtocolArg {
    Http,
    A2a,
    Mcp,
}

impl std::fmt::Display for AgentProtocolArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http => write!(f, "http"),
            Self::A2a => write!(f, "a2a"),
            Self::Mcp => write!(f, "mcp"),
        }
    }
}

#[derive(Debug, Args)]
pub struct AgentDeployArgs {
    /// Agent project directory to build and deploy (default: current directory).
    /// Omit or use --template-id to deploy a pre-built template without rebuilding.
    #[arg(default_value = ".")]
    pub source: PathBuf,
    /// Deploy from a pre-built template ID, skipping the build step
    #[arg(long)]
    pub template_id: Option<String>,
    // --- Build-phase options (used when deploying from source) ---
    /// Agent name
    #[arg(long)]
    pub name: Option<String>,
    /// Description
    #[arg(long)]
    pub description: Option<String>,
    /// Agent framework
    #[arg(long)]
    pub framework: Option<AgentFrameworkArg>,
    /// Python version
    #[arg(long)]
    pub python_version: Option<String>,
    /// Application entrypoint command (e.g. "python -m simple_agent.app")
    #[arg(long)]
    pub entrypoint: Option<String>,
    /// Framework target when a project exposes multiple agents/graphs/apps (e.g. deep_agent)
    #[arg(long)]
    pub target: Option<String>,
    /// Ports the agent listens on (repeatable)
    #[arg(long = "port")]
    pub ports: Vec<u16>,
    /// Build timeout in seconds
    #[arg(long, default_value_t = 600)]
    pub build_timeout: u64,
    /// Number of vCPUs for the built template VM (default: 2)
    #[arg(long)]
    pub vcpu_count: Option<u32>,
    /// Memory in MB for the built template VM (default: 1024)
    #[arg(long)]
    pub memory_mb: Option<u32>,
    /// Disk size in MB for the built template VM (default: 4096)
    #[arg(long)]
    pub disk_mb: Option<u32>,
    /// Command to run after VM starts during snapshot phase
    #[arg(long)]
    pub start_cmd: Option<String>,
    /// Readiness command during snapshot phase
    #[arg(long)]
    pub ready_cmd: Option<String>,
    /// Readiness timeout in seconds
    #[arg(long)]
    pub ready_timeout_secs: Option<u32>,
    /// Build tags in KEY=VALUE form (repeatable)
    #[arg(long = "tag", value_name = "KEY=VALUE")]
    pub tags: Vec<String>,
    // --- Deploy-phase options ---
    /// HTTP port the agent listens on
    #[arg(long)]
    pub http_port: Option<u16>,
    /// A2A protocol port
    #[arg(long)]
    pub a2a_port: Option<u16>,
    /// MCP protocol port
    #[arg(long)]
    pub mcp_port: Option<u16>,
    /// Protocols to enable (http, a2a, mcp) — can be repeated
    #[arg(long = "protocol")]
    pub protocols: Vec<AgentProtocolArg>,
    /// Make the agent endpoint publicly accessible
    #[arg(long)]
    pub is_public: Option<bool>,
    /// Runtime environment variables (KEY=VALUE)
    #[arg(long = "env", short = 'e', value_name = "KEY=VALUE")]
    pub env_vars: Vec<String>,
    /// Deploy-only runtime environment variables in KEY=VALUE form
    #[arg(long = "deploy-env", value_name = "KEY=VALUE")]
    pub deploy_env_vars: Vec<String>,
    /// Agent runtime timeout in seconds
    #[arg(long)]
    pub timeout: Option<u64>,
    /// Wait for agent to reach ACTIVE status
    #[arg(long, default_value_t = true)]
    pub wait: bool,
    /// Wait timeout in seconds
    #[arg(long, default_value_t = 300)]
    pub wait_timeout: u64,
}

#[derive(Debug, Args)]
pub struct AgentGetArgs {
    pub id: String,
}

#[derive(Debug, Args)]
pub struct AgentInvokeArgs {
    pub id: String,
    /// JSON payload to send to the agent
    #[arg(long)]
    pub input: Option<String>,
    /// Plain text message (alternative to --input)
    #[arg(long, short = 'm')]
    pub message: Option<String>,
    /// Session/thread ID for stateful conversations
    #[arg(long)]
    pub session_id: Option<String>,
    /// Resume value for an interrupted stateful agent run
    #[arg(long)]
    pub resume: Option<String>,
    /// Metadata JSON object to pass to the agent
    #[arg(long)]
    pub metadata: Option<String>,
    /// Invocation timeout in seconds
    #[arg(long, default_value_t = 120)]
    pub timeout: u64,
}

#[derive(Debug, Args)]
pub struct AgentDestroyArgs {
    pub id: String,
    #[arg(long, short = 'y')]
    pub yes: bool,
}

#[derive(Debug, Args)]
pub struct AgentBuildStatusArgs {
    pub build_id: String,
}

#[derive(Debug, Args)]
pub struct AgentInitArgs {
    /// Project name (used as the directory name if path is not provided)
    pub name: String,
    /// Framework to scaffold
    #[arg(long, default_value = "python")]
    pub framework: AgentFrameworkArg,
    /// Directory to create the project in (default: ./<name>)
    #[arg(long)]
    pub output: Option<PathBuf>,
    /// Python version to target
    #[arg(long, default_value = "3.12")]
    pub python_version: String,
}

/// Args for the interactive `grx agent create` wizard.
#[derive(Debug, Args)]
pub struct AgentCreateArgs {
    /// Pre-fill the agent name (skips that prompt)
    #[arg(long)]
    pub name: Option<String>,
    /// Directory to create the project in (default: ./<name>)
    #[arg(long)]
    pub output: Option<PathBuf>,
}

/// Args for `grx agent dev` — live development with hot-reload.
#[derive(Debug, Args)]
pub struct AgentDevArgs {
    /// Agent project directory to watch (default: .)
    #[arg(default_value = ".")]
    pub source: PathBuf,
    /// Send one message to a local dev server, print the response, then stop it
    #[arg(long, short = 'm')]
    pub message: Option<String>,
    /// Host used by the local development server
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    /// Port used by the local development server
    #[arg(long, default_value_t = 8000)]
    pub port: u16,
    /// Disable framework/server reload flags when the configured command supports them
    #[arg(long)]
    pub no_reload: bool,
    /// Framework target when a project exposes multiple agents/graphs/apps (e.g. deep_agent)
    #[arg(long)]
    pub target: Option<String>,
    /// Use the legacy cloud runtime sync workflow instead of local dev
    #[arg(long)]
    pub runtime_sync: bool,
    /// Cloud provider
    #[arg(long, default_value = ResolvedConfig::DEFAULT_CLOUD)]
    pub cloud: String,
    /// Deployment region
    #[arg(long, default_value = ResolvedConfig::DEFAULT_REGION)]
    pub region: String,
    /// Subdirectory to watch for changes (default: app/)
    #[arg(long, default_value = "app")]
    pub watch_dir: String,
}

/// Args for `grx agent up` — local Docker parity for generated production images.
#[derive(Debug, Args)]
pub struct AgentUpArgs {
    /// Agent project directory to run (default: .)
    #[arg(default_value = ".")]
    pub source: PathBuf,
    /// Host interface for the Docker-published local endpoint
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    /// Port used by the local Docker server
    #[arg(long, default_value_t = 8000)]
    pub port: u16,
    /// Override the framework hint
    #[arg(long)]
    pub framework: Option<AgentFrameworkArg>,
    /// Override the base Python version for generated Dockerfiles
    #[arg(long)]
    pub python_version: Option<String>,
    /// Framework target when a project exposes multiple agents/graphs/apps (e.g. deep_agent)
    #[arg(long)]
    pub target: Option<String>,
    /// Environment variables in KEY=VALUE form (repeatable)
    #[arg(long = "env", short = 'e', value_name = "KEY=VALUE")]
    pub env_vars: Vec<String>,
}

#[derive(Debug, Args)]
pub struct AgentPackageArgs {
    /// Agent project directory to archive
    #[arg(default_value = ".")]
    pub source: PathBuf,
    /// Output path for the tar.gz archive
    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,
    /// Print the archive size and exit without saving
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct AgentDockerfileArgs {
    /// Agent project directory to inspect
    #[arg(default_value = ".")]
    pub source: PathBuf,
    /// Write Dockerfile to this path instead of stdout
    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,
    /// Override the base Python version
    #[arg(long)]
    pub python_version: Option<String>,
    /// Override the framework hint
    #[arg(long)]
    pub framework: Option<AgentFrameworkArg>,
    /// Framework target when a project exposes multiple agents/graphs/apps (e.g. deep_agent)
    #[arg(long)]
    pub target: Option<String>,
}

/// Args for `grx agent stream` — SSE streaming from a deployed agent.
#[derive(Debug, Args)]
pub struct AgentStreamArgs {
    /// Deployed agent ID
    pub id: String,
    /// JSON payload to send
    #[arg(long)]
    pub input: Option<String>,
    /// Plain text message
    #[arg(long, short = 'm')]
    pub message: Option<String>,
    /// Session/thread ID for stateful conversations
    #[arg(long)]
    pub session_id: Option<String>,
    /// Resume value for an interrupted stateful agent run
    #[arg(long)]
    pub resume: Option<String>,
    /// Metadata JSON object to pass to the agent
    #[arg(long)]
    pub metadata: Option<String>,
}

/// Args for `gravixlayer agent serve` — local development serving.
#[derive(Debug, Args, Clone)]
pub struct AgentServeArgs {
    /// Agent project directory to serve
    #[arg(default_value = ".")]
    pub source: PathBuf,
    /// Agent framework (auto-detected from project when omitted)
    #[arg(long)]
    pub framework: Option<AgentFrameworkArg>,
    /// Host interface for the local server
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,
    /// Port for HTTP and A2A endpoints
    #[arg(long, default_value_t = 8000)]
    pub port: u16,
    /// Framework target when a project exposes multiple graphs/apps
    #[arg(long)]
    pub target: Option<String>,
    /// Protocols to enable (http, a2a, mcp) — can be repeated
    #[arg(long = "protocol")]
    pub protocols: Vec<AgentProtocolArg>,
    /// Protocols to enable as comma-separated values
    #[arg(long = "protocols")]
    pub protocols_csv: Option<String>,
    /// Public base URL to write into the A2A agent card
    #[arg(long)]
    pub public_url: Option<String>,
    /// Python executable used to run the user's framework code
    #[arg(long, default_value = "python")]
    pub python: String,
    /// Number of long-lived Python workers to keep warm
    #[arg(long, env = "GRAVIXLAYER_AGENT_WORKERS", default_value_t = 4)]
    pub workers: usize,
    /// Per-request framework invocation timeout in seconds
    #[arg(
        long,
        env = "GRAVIXLAYER_AGENT_REQUEST_TIMEOUT_SECS",
        default_value_t = 300
    )]
    pub request_timeout_secs: u64,
    /// Worker startup/import timeout in seconds
    #[arg(
        long,
        env = "GRAVIXLAYER_AGENT_WORKER_START_TIMEOUT_SECS",
        default_value_t = 60
    )]
    pub worker_start_timeout_secs: u64,
}

// ---------------------------------------------------------------------------
// provider (secret providers)
// ---------------------------------------------------------------------------

#[derive(Debug, Args)]
pub struct ProviderArgs {
    #[command(subcommand)]
    pub command: ProviderCommand,
}

#[derive(Debug, Subcommand)]
pub enum ProviderCommand {
    /// Create a secret provider
    Create(ProviderCreateArgs),
    /// List secret providers
    List(ProviderListArgs),
    /// Get a secret provider (includes masked secrets)
    Get(ProviderGetArgs),
    /// Update a secret provider
    Update(ProviderUpdateArgs),
    /// Delete a secret provider
    Delete(ProviderDeleteArgs),
    /// Add a secret key/value pair to a provider
    AddSecret(ProviderAddSecretArgs),
    /// List masked secrets for a provider
    ListSecrets(ProviderListSecretsArgs),
    /// Update a secret key and/or value
    UpdateSecret(ProviderUpdateSecretArgs),
    /// Delete a secret from a provider
    DeleteSecret(ProviderDeleteSecretArgs),
    /// Attach a provider to a runtime (sandbox)
    Attach(ProviderAttachArgs),
    /// Detach a provider from a runtime
    Detach(ProviderDetachArgs),
    /// List providers attached to a runtime
    ListAttached(ProviderListAttachedArgs),
}

#[derive(Debug, Args)]
pub struct ProviderCreateArgs {
    /// Provider name
    pub name: String,
    /// Auth kind (phase 1: api_key only)
    #[arg(long = "type", default_value = "api_key")]
    pub provider_type: String,
    /// Secret KEY=VALUE pairs (repeatable)
    #[arg(long = "secret", value_name = "KEY=VALUE")]
    pub secret: Vec<String>,
    /// Project ID for scoping
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct ProviderListArgs {
    #[arg(long, default_value_t = 100)]
    pub limit: u32,
    #[arg(long, default_value_t = 0)]
    pub offset: u32,
    #[arg(long)]
    pub search: Option<String>,
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct ProviderGetArgs {
    /// Provider ID (UUID)
    pub id: String,
}

#[derive(Debug, Args)]
pub struct ProviderUpdateArgs {
    /// Provider ID (UUID)
    pub id: String,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long = "type")]
    pub provider_type: Option<String>,
    /// Enable the provider (injects secrets into sandboxes)
    #[arg(long, conflicts_with = "disabled")]
    pub enabled: bool,
    /// Disable the provider
    #[arg(long, conflicts_with = "enabled")]
    pub disabled: bool,
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct ProviderDeleteArgs {
    pub id: String,
    #[arg(long, short = 'y')]
    pub yes: bool,
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct ProviderAddSecretArgs {
    /// Provider ID (UUID)
    pub id: String,
    /// Secret key (env var name), e.g. OPENAI_API_KEY
    #[arg(long)]
    pub key: String,
    /// Secret value
    #[arg(long)]
    pub value: String,
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct ProviderListSecretsArgs {
    pub id: String,
}

#[derive(Debug, Args)]
pub struct ProviderUpdateSecretArgs {
    /// Provider ID (UUID)
    pub id: String,
    /// Secret ID (UUID)
    pub secret_id: String,
    #[arg(long)]
    pub key: Option<String>,
    #[arg(long)]
    pub value: Option<String>,
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct ProviderDeleteSecretArgs {
    pub id: String,
    pub secret_id: String,
    #[arg(long, short = 'y')]
    pub yes: bool,
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct ProviderAttachArgs {
    /// Provider ID (UUID)
    pub id: String,
    /// Runtime ID (UUID) to attach to
    pub runtime_id: String,
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct ProviderDetachArgs {
    pub id: String,
    pub runtime_id: String,
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct ProviderListAttachedArgs {
    /// Runtime ID (UUID)
    pub runtime_id: String,
}

// ---------------------------------------------------------------------------
// network-policy (egress firewall policies)
// ---------------------------------------------------------------------------

#[derive(Debug, Args)]
pub struct NetworkPolicyArgs {
    #[command(subcommand)]
    pub command: NetworkPolicyCommand,
}

#[derive(Debug, Subcommand)]
pub enum NetworkPolicyCommand {
    /// Create a network policy
    Create(NetworkPolicyCreateArgs),
    /// List network policies
    List(NetworkPolicyListArgs),
    /// Get a network policy
    Get(NetworkPolicyGetArgs),
    /// Update a network policy
    Update(NetworkPolicyUpdateArgs),
    /// Delete a network policy
    Delete(NetworkPolicyDeleteArgs),
    /// Add an egress rule to a policy
    AddRule(NetworkPolicyAddRuleArgs),
    /// List rules for a policy
    ListRules(NetworkPolicyListRulesArgs),
    /// Update an egress rule
    UpdateRule(NetworkPolicyUpdateRuleArgs),
    /// Delete an egress rule
    DeleteRule(NetworkPolicyDeleteRuleArgs),
    /// Attach a network policy to a runtime (sandbox)
    Attach(NetworkPolicyAttachArgs),
    /// Detach a network policy from a runtime
    Detach(NetworkPolicyDetachArgs),
    /// List network policies attached to a runtime
    ListAttached(NetworkPolicyListAttachedArgs),
}

#[derive(Debug, Args)]
pub struct NetworkPolicyCreateArgs {
    /// Policy name
    pub name: String,
    /// Egress mode: allowlist | denylist | allow_all | deny_all
    #[arg(long = "egress-mode", default_value = "allowlist")]
    pub egress_mode: String,
    /// Optional description
    #[arg(long)]
    pub description: Option<String>,
    /// Mark as the default policy for the account/project
    #[arg(long)]
    pub is_default: bool,
    /// Project ID for scoping
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct NetworkPolicyListArgs {
    #[arg(long, default_value_t = 100)]
    pub limit: u32,
    #[arg(long, default_value_t = 0)]
    pub offset: u32,
    #[arg(long)]
    pub search: Option<String>,
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct NetworkPolicyGetArgs {
    /// Policy ID (UUID)
    pub id: String,
}

#[derive(Debug, Args)]
pub struct NetworkPolicyUpdateArgs {
    /// Policy ID (UUID)
    pub id: String,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long = "egress-mode")]
    pub egress_mode: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    /// Enable the policy
    #[arg(long, conflicts_with = "disabled")]
    pub enabled: bool,
    /// Disable the policy
    #[arg(long, conflicts_with = "enabled")]
    pub disabled: bool,
    /// Mark as default
    #[arg(long, conflicts_with = "unset_default")]
    pub set_default: bool,
    /// Clear default flag
    #[arg(long, conflicts_with = "set_default")]
    pub unset_default: bool,
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct NetworkPolicyDeleteArgs {
    pub id: String,
    #[arg(long, short = 'y')]
    pub yes: bool,
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct NetworkPolicyAddRuleArgs {
    /// Policy ID (UUID)
    pub id: String,
    /// Destination hostname, IP, or CIDR
    #[arg(long)]
    pub destination: String,
    /// Destination port (0 = any)
    #[arg(long, default_value_t = 0)]
    pub port: i64,
    /// Protocol: tcp | udp | any
    #[arg(long, default_value = "tcp")]
    pub protocol: String,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct NetworkPolicyListRulesArgs {
    pub id: String,
}

#[derive(Debug, Args)]
pub struct NetworkPolicyUpdateRuleArgs {
    /// Policy ID (UUID)
    pub id: String,
    /// Rule ID (UUID)
    pub rule_id: String,
    #[arg(long)]
    pub destination: Option<String>,
    #[arg(long)]
    pub port: Option<i64>,
    #[arg(long)]
    pub protocol: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct NetworkPolicyDeleteRuleArgs {
    pub id: String,
    pub rule_id: String,
    #[arg(long, short = 'y')]
    pub yes: bool,
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct NetworkPolicyAttachArgs {
    /// Policy ID (UUID)
    pub id: String,
    /// Runtime ID (UUID) to attach to
    pub runtime_id: String,
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct NetworkPolicyDetachArgs {
    pub id: String,
    pub runtime_id: String,
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct NetworkPolicyListAttachedArgs {
    /// Runtime ID (UUID)
    pub runtime_id: String,
}

// ---------------------------------------------------------------------------
// billing
// ---------------------------------------------------------------------------
#[derive(Debug, Args)]
pub struct BillingArgs {
    #[command(subcommand)]
    pub command: BillingCommand,
}

#[derive(Debug, Subcommand)]
pub enum BillingCommand {
    /// Show usage/cost summary for a billing month
    Summary(BillingSummaryArgs),
    /// Show billing history / invoices
    History(BillingHistoryArgs),
    /// Show quota and limit details
    Quotas,
}

#[derive(Debug, Args)]
pub struct BillingSummaryArgs {
    /// Billing month in YYYY-MM (default: current month on the control plane)
    #[arg(long)]
    pub month: Option<String>,
    /// Optional project filter (UUID)
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct BillingHistoryArgs {
    #[arg(long, default_value_t = 1)]
    pub page: u32,
    #[arg(long, default_value_t = 100)]
    pub page_size: u32,
    /// Start date filter (RFC 3339) — maps to `start_time`
    #[arg(long)]
    pub from: Option<String>,
    /// End date filter (RFC 3339) — maps to `end_time`
    #[arg(long)]
    pub to: Option<String>,
    /// Filter by runtime ID
    #[arg(long)]
    pub runtime_id: Option<String>,
    /// Filter by billing status
    #[arg(long)]
    pub status: Option<String>,
    /// Optional project filter (UUID)
    #[arg(long)]
    pub project_id: Option<String>,
}

// ---------------------------------------------------------------------------
// validate
// ---------------------------------------------------------------------------

#[derive(Debug, Args)]
pub struct ValidateArgs {
    /// Path to gravixlayer.json (default: ./gravixlayer/gravixlayer.json)
    #[arg(default_value = ".")]
    pub path: PathBuf,
}

// ---------------------------------------------------------------------------
// package
// ---------------------------------------------------------------------------

#[derive(Debug, Args)]
pub struct PackageArgs {
    /// Agent project directory to archive
    #[arg(default_value = ".")]
    pub source: PathBuf,
    /// Output path for the tar.gz file
    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,
    /// Print the archive size and exit without saving
    #[arg(long)]
    pub dry_run: bool,
}

// ---------------------------------------------------------------------------
// completions
// ---------------------------------------------------------------------------

#[derive(Debug, Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    pub shell: Shell,
}

// ---------------------------------------------------------------------------
// update
// ---------------------------------------------------------------------------

#[derive(Debug, Args)]
#[command(disable_version_flag = true)]
pub struct UpdateArgs {
    /// Check for a new version without installing
    #[arg(long)]
    pub check: bool,
    /// Install a specific version (e.g. 0.1.0 or v0.1.0). Default: latest release.
    #[arg(long = "version", value_name = "VERSION")]
    pub version: Option<String>,
}

// ---------------------------------------------------------------------------
// storage (buckets)
// ---------------------------------------------------------------------------

#[derive(Debug, Args)]
pub struct StorageArgs {
    #[command(subcommand)]
    pub command: StorageCommand,
}

#[derive(Debug, Subcommand)]
pub enum StorageCommand {
    /// Manage S3 and S3-compatible buckets
    Bucket(StorageBucketArgs),
}

#[derive(Debug, Args)]
pub struct StorageBucketArgs {
    #[command(subcommand)]
    pub command: StorageBucketCommand,
}

#[derive(Debug, Subcommand)]
pub enum StorageBucketCommand {
    /// Create a managed bucket
    Create(StorageBucketCreateArgs),
    /// Connect an existing S3 or S3-compatible bucket
    Connect(StorageBucketConnectArgs),
    /// List buckets
    List(StorageBucketListArgs),
    /// Show one bucket
    Get(StorageBucketIdArgs),
    /// Verify an external bucket's credentials
    Verify(StorageBucketIdArgs),
    /// Delete or disconnect a bucket (refused while mounted)
    Delete(StorageBucketIdArgs),
}

#[derive(Debug, Args)]
pub struct StorageBucketCreateArgs {
    /// Bucket name
    pub name: String,
    /// Region for the bucket (defaults to the runtime region)
    #[arg(long)]
    pub region: Option<String>,
    /// Project ID for scoping
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct StorageBucketListArgs {
    /// Project ID for scoping
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct StorageBucketIdArgs {
    /// Bucket ID (UUID for delete; ID or name for get)
    pub id: String,
}

#[derive(Debug, Args)]
pub struct StorageBucketConnectArgs {
    /// Bucket name shown in Gravix Layer
    pub name: String,
    /// The existing bucket name at the provider
    pub bucket: String,
    /// Provider: s3 (default) or s3_compatible
    #[arg(long, default_value = "s3")]
    pub provider: String,
    /// Object prefix the bucket connection is rooted at
    #[arg(long)]
    pub prefix: Option<String>,
    /// Bucket region
    #[arg(long)]
    pub region: Option<String>,
    /// https endpoint (required for s3_compatible)
    #[arg(long)]
    pub endpoint: Option<String>,
    /// Path-style addressing for s3_compatible endpoints
    #[arg(long)]
    pub path_style: bool,
    /// Auth mode: static (secret references) or oidc (role assumption)
    #[arg(long, default_value = "static")]
    pub auth_mode: String,
    /// Secret provider ID holding the credentials (auth static)
    #[arg(long)]
    pub secret_provider_id: Option<String>,
    /// Secret key holding the access key id (auth static)
    #[arg(long)]
    pub access_key_secret: Option<String>,
    /// Secret key holding the secret key (auth static)
    #[arg(long)]
    pub secret_key_secret: Option<String>,
    /// Secret key holding the session token (auth static)
    #[arg(long)]
    pub session_token_secret: Option<String>,
    /// Role to assume (auth oidc)
    #[arg(long)]
    pub role_arn: Option<String>,
    /// External id for the assume (auth oidc)
    #[arg(long)]
    pub external_id: Option<String>,
    /// Project ID for scoping
    #[arg(long)]
    pub project_id: Option<String>,
}
