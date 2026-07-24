// src/ctx.rs — Application context.
//
// `AppContext` is constructed once in `main.rs` from the parsed CLI arguments
// and passed by reference to every command handler.  It holds:
//   • The typed API client (ready to make requests)
//   • The resolved configuration (effective after flag/env/file precedence)
//   • The loaded project file (if found in the directory tree)
//   • Output mode (table | json | quiet)

use std::path::PathBuf;

use secrecy::SecretString;

use crate::api::ApiClient;
use crate::cli::OutputFormat;
use crate::config::project::GravixlayerProject;
use crate::config::{keyring_get, ResolvedConfig, UserConfig};

// ---------------------------------------------------------------------------
// AppContext
// ---------------------------------------------------------------------------

/// All runtime dependencies available to command handlers.
pub struct AppContext {
    /// Ready-to-use HTTP client.
    pub api: ApiClient,

    /// Resolved (effective) configuration.
    pub cfg: ResolvedConfig,

    /// User config file (for write-back operations such as `config set`).
    pub user_config: UserConfig,

    /// Project file discovered by walking up from cwd (may be absent).
    pub project: Option<GravixlayerProject>,

    /// Root directory of the project (parent of `gravixlayer/`).
    #[allow(dead_code)]
    pub project_root: Option<PathBuf>,

    /// Output format selected by the user.
    pub output: OutputFormat,
}

impl AppContext {
    /// Build `AppContext` from CLI globals.
    ///
    /// API key resolution order:
    ///   1. `--api-key` CLI flag
    ///   2. `GRAVIXLAYER_API_KEY` environment variable (via `ResolvedConfig`)
    ///   3. System keyring (profile-scoped)
    ///
    /// Profile resolution order:
    ///   1. `--profile` CLI flag / `GRAVIXLAYER_PROFILE` env var (handled by clap)
    ///   2. `active_profile` field in `~/.gravixlayer/config.toml` (set by `config use-profile`)
    ///   3. Hard-coded fallback: `"default"`
    ///
    /// Returns an error if no API key can be found AND the user is not running
    /// an unauthenticated command (e.g., `completions` or `update --check`).
    pub fn build(
        cli_api_key: Option<String>,
        cli_base_url: Option<String>,
        cli_profile: Option<String>,
        output: OutputFormat,
    ) -> anyhow::Result<Self> {
        let user_config = UserConfig::load()?;

        // Resolve which profile to use: explicit flag/env → active_profile → "default".
        let profile_name = cli_profile.unwrap_or_else(|| user_config.active_profile.clone());

        let mut cfg = user_config.resolve(&profile_name);

        // CLI --api-key flag takes highest precedence.
        if let Some(key) = cli_api_key {
            cfg.api_key = Some(key);
        }

        // CLI --base-url flag takes highest precedence over env/profile.
        if let Some(url) = cli_base_url {
            cfg.base_url = url;
        }

        // Fall back to keyring when neither flag nor env provided a key.
        if cfg.api_key.is_none() {
            cfg.api_key = keyring_get(&profile_name)?;
        }

        let api_key_str = cfg.api_key.clone().unwrap_or_default();
        let api = ApiClient::new(
            SecretString::new(api_key_str.into()),
            Some(cfg.base_url.clone()),
        )
        .map_err(|e| anyhow::anyhow!("failed to build API client: {e}"))?;

        let (project, project_root) = GravixlayerProject::find_from_cwd()
            .map(|(p, r)| (Some(p), Some(r)))
            .unwrap_or((None, None));

        Ok(Self {
            api,
            cfg,
            user_config,
            project,
            project_root,
            output,
        })
    }

    /// Returns `true` if an API key is configured.
    pub fn has_api_key(&self) -> bool {
        !self.cfg.api_key.as_deref().unwrap_or("").is_empty()
    }

    /// Abort with a user-friendly error when no API key is available.
    pub fn require_api_key(&self) -> anyhow::Result<()> {
        if !self.has_api_key() {
            anyhow::bail!(
                "no API key found\n\
                 Run `gravixlayer auth login` or set GRAVIXLAYER_API_KEY"
            );
        }
        Ok(())
    }

    /// Save a modified `user_config` back to disk.
    #[allow(dead_code)]
    pub fn save_config(&self) -> anyhow::Result<()> {
        self.user_config.save()
    }
}
