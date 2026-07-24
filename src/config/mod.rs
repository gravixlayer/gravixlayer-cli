// src/config/mod.rs — User configuration.
//
// Reads `~/.gravixlayer/config.toml` which may contain multiple named profiles.
// Writes are done atomically (write to temp file then rename) to prevent
// corruption on interrupted writes.
//
// Schema example:
//
//   active_profile = "default"
//
//   [profiles.default]
//   api_key        = "grx_..."           # optional — keyring is preferred
//   base_url       = "https://api.gravixlayer.ai"
//   default_cloud  = "azure"             # renamed from default_provider
//   default_region = "eastus2"
//   default_template = "base-small"
//
//   [profiles.prod]
//   base_url = "https://api.gravixlayer.ai"

pub mod project;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::debug;

const CONFIG_DIR_NAME: &str = ".gravixlayer";
const CONFIG_FILE_NAME: &str = "config.toml";
const KEYRING_SERVICE: &str = "gravixlayer-cli";

// ---------------------------------------------------------------------------
// On-disk schema
// ---------------------------------------------------------------------------

/// The full content of `~/.gravixlayer/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfig {
    /// The name of the currently active profile.
    #[serde(default = "default_profile_name")]
    pub active_profile: String,

    /// Named profiles.
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

fn default_profile_name() -> String {
    "default".into()
}

/// A single named profile.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Profile {
    /// Optional inline API key (keyring is preferred).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// API base URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    /// Default cloud provider.
    #[serde(alias = "default_provider", skip_serializing_if = "Option::is_none")]
    pub default_cloud: Option<String>,

    /// Default deployment region.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_region: Option<String>,

    /// Default container template.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_template: Option<String>,

    /// Current context runtime ID (set by `runtime context set`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_runtime_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Resolved values (after applying CLI flags and env vars on top of config)
// ---------------------------------------------------------------------------

/// Effective configuration after precedence resolution:
///   1. CLI flag
///   2. Environment variable
///   3. Profile value
///   4. Hard-coded default
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ResolvedConfig {
    pub api_key: Option<String>,
    pub base_url: String,
    pub default_cloud: String,
    pub default_region: String,
    pub default_template: String,
    pub profile_name: String,
}

impl ResolvedConfig {
    pub const DEFAULT_BASE_URL: &'static str = "https://api.gravixlayer.ai";
    pub const DEFAULT_CLOUD: &'static str = "azure";
    pub const DEFAULT_REGION: &'static str = "eastus2";
    pub const DEFAULT_TEMPLATE: &'static str = "base-small";
}

// ---------------------------------------------------------------------------
// File I/O
// ---------------------------------------------------------------------------

impl UserConfig {
    /// Load from disk.  Returns a default config if the file does not exist.
    pub fn load() -> anyhow::Result<Self> {
        let path = config_path()?;
        if !path.exists() {
            debug!(
                "config file not found at {}, using defaults",
                path.display()
            );
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(&path)?;
        let cfg: Self = toml::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("failed to parse {}: {e}", path.display()))?;
        Ok(cfg)
    }

    /// Persist to disk atomically.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;

        // Atomic write: temp file → rename.
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, &content)?;

        // Restrict permissions before the rename so the file is never world-readable.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))?;
        }

        fs::rename(&tmp, &path)?;

        debug!("config saved to {}", path.display());
        Ok(())
    }

    /// Get a mutable reference to a named profile, creating it if absent.
    pub fn profile_mut(&mut self, name: &str) -> &mut Profile {
        self.profiles.entry(name.to_string()).or_default()
    }

    /// Get an immutable reference to a named profile (may be absent).
    pub fn profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.get(name)
    }

    /// Resolve effective configuration for the given profile name, applying
    /// CLI overrides from environment variables.
    pub fn resolve(&self, profile_name: &str) -> ResolvedConfig {
        let profile = self.profiles.get(profile_name).cloned().unwrap_or_default();

        let base_url = std::env::var("GRAVIXLAYER_BASE_URL")
            .ok()
            .or(profile.base_url)
            .unwrap_or_else(|| ResolvedConfig::DEFAULT_BASE_URL.to_string());

        let default_cloud = profile
            .default_cloud
            .unwrap_or_else(|| ResolvedConfig::DEFAULT_CLOUD.to_string());

        let default_region = profile
            .default_region
            .unwrap_or_else(|| ResolvedConfig::DEFAULT_REGION.to_string());

        let default_template = profile
            .default_template
            .unwrap_or_else(|| ResolvedConfig::DEFAULT_TEMPLATE.to_string());

        // API key from env takes priority over profile inline key (never read
        // from keyring here — the ctx module handles keyring lookup).
        let api_key = std::env::var("GRAVIXLAYER_API_KEY")
            .ok()
            .or(profile.api_key);

        ResolvedConfig {
            api_key,
            base_url,
            default_cloud,
            default_region,
            default_template,
            profile_name: profile_name.to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Keyring helpers
// ---------------------------------------------------------------------------

/// Write an API key to the system keyring under `profile_name`.
pub fn keyring_set(profile_name: &str, api_key: &str) -> anyhow::Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, profile_name)?;
    entry.set_password(api_key)?;
    Ok(())
}

/// Read an API key from the system keyring for `profile_name`.
///
/// Returns `Ok(None)` when no key is stored (not an error).
pub fn keyring_get(profile_name: &str) -> anyhow::Result<Option<String>> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, profile_name)?;
    match entry.get_password() {
        Ok(k) => Ok(Some(k)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(keyring::Error::NoStorageAccess(e)) => {
            // Keyring unavailable (headless server, locked session, etc.).
            // Warn but don't fail — the API key may come from env or config file.
            tracing::warn!("system keyring unavailable ({e}); falling back to env/config");
            Ok(None)
        }
        Err(e) => Err(anyhow::anyhow!("keyring error: {e}")),
    }
}

/// Remove an API key from the system keyring for `profile_name`.
///
/// Silently succeeds if no key is stored (idempotent).
pub fn keyring_delete(profile_name: &str) -> anyhow::Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, profile_name)?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(anyhow::anyhow!("keyring error: {e}")),
    }
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Path to the config directory: `~/.gravixlayer/`.
pub fn config_dir() -> anyhow::Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    Ok(home.join(CONFIG_DIR_NAME))
}

/// Path to the config file: `~/.gravixlayer/config.toml`.
pub fn config_path() -> anyhow::Result<PathBuf> {
    Ok(config_dir()?.join(CONFIG_FILE_NAME))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config_with_profile() -> UserConfig {
        let mut cfg = UserConfig::default();
        cfg.active_profile = "default".into();
        let p = cfg.profile_mut("default");
        p.base_url = Some("https://api.example.com".into());
        p.default_cloud = Some("gcp".into());
        p.default_region = Some("us-central1".into());
        cfg
    }

    #[test]
    fn resolve_uses_profile_values() {
        let cfg = make_config_with_profile();
        let r = cfg.resolve("default");
        assert_eq!(r.base_url, "https://api.example.com");
        assert_eq!(r.default_cloud, "gcp");
        assert_eq!(r.default_region, "us-central1");
        assert_eq!(r.default_template, ResolvedConfig::DEFAULT_TEMPLATE);
    }

    #[test]
    fn resolve_falls_back_to_defaults_for_missing_profile() {
        let cfg = UserConfig::default();
        let r = cfg.resolve("nonexistent");
        assert_eq!(r.base_url, ResolvedConfig::DEFAULT_BASE_URL);
        assert_eq!(r.default_cloud, ResolvedConfig::DEFAULT_CLOUD);
        assert_eq!(r.default_region, ResolvedConfig::DEFAULT_REGION);
    }

    #[test]
    fn roundtrip_toml_serialization() {
        let cfg = make_config_with_profile();
        let s = toml::to_string_pretty(&cfg).unwrap();
        let restored: UserConfig = toml::from_str(&s).unwrap();
        assert_eq!(
            restored.profile("default").unwrap().base_url,
            Some("https://api.example.com".into())
        );
    }
}
