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
            let v = value.trim();
            if !v.starts_with("https://") {
                anyhow::bail!("base_url must start with https://");
            }
            if v.contains(|c: char| c.is_ascii_control()) {
                anyhow::bail!("base_url contains invalid characters");
            }
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
