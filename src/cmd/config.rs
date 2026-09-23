// src/cmd/config.rs — Configuration command handlers.

use crate::cli::{ConfigCommand, ConfigSetArgs, ConfigUnsetArgs, ConfigUseProfileArgs};
use crate::ctx::AppContext;
use crate::output;

pub async fn handle(ctx: &mut AppContext, cmd: ConfigCommand) -> anyhow::Result<()> {
    match cmd {
        ConfigCommand::Show => show(ctx).await,
        ConfigCommand::Set(args) => set(ctx, args).await,
        ConfigCommand::Unset(args) => unset(ctx, args).await,
        ConfigCommand::Profiles => profiles(ctx).await,
        ConfigCommand::UseProfile(args) => use_profile(ctx, args).await,
    }
}

async fn show(ctx: &AppContext) -> anyhow::Result<()> {
    let mut redacted = ctx.user_config.clone();
    for profile in redacted.profiles.values_mut() {
        if profile.api_key.is_some() {
            profile.api_key = Some("<redacted>".to_string());
        }
    }
    let output = toml::to_string_pretty(&redacted)?;
    println!("{output}");
    Ok(())
}

async fn set(ctx: &mut AppContext, args: ConfigSetArgs) -> anyhow::Result<()> {
    let profile_name = args
        .profile
        .as_deref()
        .unwrap_or(&ctx.cfg.profile_name)
        .to_string();
    let value = args.value.clone();
    let profile = ctx.user_config.profile_mut(&profile_name);
    match args.key.as_str() {
        "base_url" => {
            // Validate: the client enforces HTTPS for non-WebSocket requests.
            // http:// is accepted only for a loopback API (local `gravix-api`
            // dev) — matching the client's `https_only` exemption.
            let v = value.trim();
            validate_base_url(v)?;
            profile.base_url = Some(v.to_string());
        }
        "default_cloud" | "default_provider" => profile.default_cloud = Some(value.clone()),
        "default_region" => profile.default_region = Some(value.clone()),
        "default_template" => profile.default_template = Some(value.clone()),
        other => anyhow::bail!("unknown config key: {other}  (valid: base_url, default_cloud, default_region, default_template)"),
    }
    ctx.user_config.save()?;
    output::success(
        ctx.output,
        format!("Set {}.{} = {}", profile_name, args.key, value),
    );
    Ok(())
}

async fn unset(ctx: &mut AppContext, args: ConfigUnsetArgs) -> anyhow::Result<()> {
    let profile_name = args
        .profile
        .as_deref()
        .unwrap_or(&ctx.cfg.profile_name)
        .to_string();
    let profile = ctx.user_config.profile_mut(&profile_name);
    match args.key.as_str() {
        "base_url" => profile.base_url = None,
        "default_cloud" | "default_provider" => profile.default_cloud = None,
        "default_region" => profile.default_region = None,
        "default_template" => profile.default_template = None,
        other => anyhow::bail!("unknown config key: {other}  (valid: base_url, default_cloud, default_region, default_template)"),
    }
    ctx.user_config.save()?;
    output::success(ctx.output, format!("Unset {}.{}", profile_name, args.key));
    Ok(())
}

async fn profiles(ctx: &AppContext) -> anyhow::Result<()> {
    let active = &ctx.user_config.active_profile;
    let mut names: Vec<&String> = ctx.user_config.profiles.keys().collect();
    names.sort();
    for name in names {
        let marker = if name == active { " *" } else { "  " };
        let p = &ctx.user_config.profiles[name];
        let url = p
            .base_url
            .as_deref()
            .unwrap_or(crate::config::ResolvedConfig::DEFAULT_BASE_URL);
        let provider = p
            .default_cloud
            .as_deref()
            .unwrap_or(crate::config::ResolvedConfig::DEFAULT_CLOUD);
        let region = p
            .default_region
            .as_deref()
            .unwrap_or(crate::config::ResolvedConfig::DEFAULT_REGION);
        let key_source = if p.api_key.is_some() {
            "config file"
        } else {
            "keyring / env"
        };
        println!("{marker} {name}");
        println!("     url      : {url}");
        println!("     provider : {provider} / {region}");
        println!("     api key  : {key_source}");
    }
    if ctx.user_config.profiles.is_empty() {
        println!("  (no profiles configured — run `gravixlayer auth login` to create one)");
    }
    Ok(())
}

async fn use_profile(ctx: &mut AppContext, args: ConfigUseProfileArgs) -> anyhow::Result<()> {
    ctx.user_config.active_profile = args.name.clone();
    ctx.user_config.save()?;
    output::success(ctx.output, format!("Active profile set to '{}'", args.name));
    Ok(())
}

/// `base_url` must be HTTPS, except a loopback API (local `gravix-api` dev has
/// no TLS). The client additionally enforces this via `https_only`, so this
/// check and `crate::api::is_http_loopback` must stay aligned.
fn validate_base_url(v: &str) -> anyhow::Result<()> {
    if v.contains(|c: char| c.is_ascii_control()) {
        anyhow::bail!("base_url contains invalid characters");
    }
    if v.starts_with("https://") || crate::api::is_http_loopback(v) {
        return Ok(());
    }
    anyhow::bail!("base_url must be https:// — http:// is allowed only for loopback (localhost, 127.0.0.1, [::1])");
}

#[cfg(test)]
mod tests {
    use super::validate_base_url;

    #[test]
    fn base_url_accepts_https_and_loopback_http() {
        for ok in [
            "https://api.gravixlayer.ai",
            "https://localhost:8443",
            "http://localhost:8000",
            "http://127.0.0.1:8000",
            "http://[::1]:8000",
            "http://127.34.56.78",
        ] {
            assert!(validate_base_url(ok).is_ok(), "{ok}");
        }
        for bad in [
            "http://api.gravixlayer.ai",
            "http://192.168.1.15:8000",
            "http://localhost.evil.com",
            "http://user@localhost@evil.com",
            "localhost:8000",
            "ftp://localhost:21",
            "https://bad\nhost",
        ] {
            assert!(validate_base_url(bad).is_err(), "{bad}");
        }
    }
}
