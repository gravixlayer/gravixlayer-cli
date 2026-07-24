// src/cmd/doctor.rs — Local environment diagnostics (no network required).

use std::time::Duration;

use crate::cli::OutputFormat;
use crate::config::{self, UserConfig};
use crate::output;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const KEYRING_PROBE_TIMEOUT: Duration = Duration::from_secs(2);

pub async fn handle(output_fmt: OutputFormat) -> anyhow::Result<()> {
    output::info(output_fmt, format!("gravixlayer {CURRENT_VERSION}"));
    match std::env::current_exe() {
        Ok(path) => output::kv(output_fmt, "binary", path.display()),
        Err(err) => output::info(output_fmt, format!("binary: unknown ({err})")),
    }

    match UserConfig::load() {
        Ok(cfg) => {
            let path = config::config_path().unwrap_or_default();
            output::kv(output_fmt, "config", path.display());
            output::kv(output_fmt, "profile", &cfg.active_profile);
            let profile = cfg.profiles.get(&cfg.active_profile);
            let base = profile
                .and_then(|p| p.base_url.as_deref())
                .unwrap_or(config::ResolvedConfig::DEFAULT_BASE_URL);
            output::kv(output_fmt, "base_url", base);

            let env_key = std::env::var("GRAVIXLAYER_API_KEY")
                .map(|v| !v.is_empty())
                .unwrap_or(false);
            if env_key {
                output::success(output_fmt, "API key: present via GRAVIXLAYER_API_KEY");
            } else {
                match probe_keyring(&cfg.active_profile).await {
                    KeyringProbe::Present => {
                        output::success(output_fmt, "API key: present in OS keyring")
                    }
                    KeyringProbe::Absent => output::info(
                        output_fmt,
                        "API key: not found — run `gravixlayer auth login`",
                    ),
                    KeyringProbe::Unavailable(msg) => {
                        output::info(output_fmt, format!("API key: keyring unavailable ({msg})"))
                    }
                }
            }
        }
        Err(err) => {
            output::info(output_fmt, format!("config: failed to load ({err})"));
        }
    }

    let on_path = which("gravixlayer");
    output::kv(
        output_fmt,
        "on_path",
        if on_path { "yes" } else { "no (check PATH)" },
    );

    output::info(
        output_fmt,
        "Tip: run `gravixlayer auth status` after login to confirm credentials.",
    );
    Ok(())
}

enum KeyringProbe {
    Present,
    Absent,
    Unavailable(String),
}

async fn probe_keyring(profile: &str) -> KeyringProbe {
    let profile = profile.to_string();
    let handle = tokio::task::spawn_blocking(move || config::keyring_get(&profile));
    match tokio::time::timeout(KEYRING_PROBE_TIMEOUT, handle).await {
        Ok(Ok(Ok(Some(_)))) => KeyringProbe::Present,
        Ok(Ok(Ok(None))) => KeyringProbe::Absent,
        Ok(Ok(Err(err))) => KeyringProbe::Unavailable(err.to_string()),
        Ok(Err(err)) => KeyringProbe::Unavailable(format!("join error: {err}")),
        Err(_) => KeyringProbe::Unavailable(format!(
            "timed out after {}s — unlock the OS keychain and retry",
            KEYRING_PROBE_TIMEOUT.as_secs()
        )),
    }
}

fn which(bin: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|dir| {
                let candidate = dir.join(bin);
                candidate.is_file()
            })
        })
        .unwrap_or(false)
}
