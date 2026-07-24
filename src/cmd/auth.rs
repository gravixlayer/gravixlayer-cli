// src/cmd/auth.rs — Authentication command handlers.

use anyhow::Context;
use rpassword::read_password;
use secrecy::SecretString;

use crate::api::error::ApiError;
use crate::api::ApiClient;
use crate::cli::{AuthCommand, AuthLoginArgs};
use crate::config::{keyring_delete, keyring_get, keyring_set};
use crate::ctx::AppContext;
use crate::output;

pub async fn handle(ctx: &mut AppContext, cmd: AuthCommand) -> anyhow::Result<()> {
    match cmd {
        AuthCommand::Login(args) => login(ctx, args).await,
        AuthCommand::Logout => logout(ctx).await,
        AuthCommand::Status => status(ctx).await,
        AuthCommand::Token => token(ctx).await,
        // AuthCommand::Whoami => whoami(ctx).await,
    }
}

async fn login(ctx: &mut AppContext, args: AuthLoginArgs) -> anyhow::Result<()> {
    let key = if let Some(k) = args.api_key {
        k
    } else {
        eprint!("Enter your GravixLayer API key: ");
        read_password().context("failed to read API key from stdin")?
    };

    let key = key.trim().to_string();
    if key.is_empty() {
        anyhow::bail!("API key cannot be empty");
    }

    // Verify the key against the API before persisting it to the keyring.
    // Uses a lightweight authenticated list call (API-key compatible).
    verify_api_key(&key, &ctx.cfg.base_url).await?;

    keyring_set(&ctx.cfg.profile_name, &key)?;
    ctx.cfg.api_key = Some(key);

    // Rebuild the HTTP client so subsequent commands in this process use the
    // newly stored key (important when login is composed with other flows).
    ctx.api = ApiClient::new(
        SecretString::new(ctx.cfg.api_key.clone().unwrap_or_default().into()),
        Some(ctx.cfg.base_url.clone()),
    )
    .context("failed to rebuild API client after login")?;

    output::success(
        ctx.output,
        format!(
            "API key verified and saved for profile '{}'",
            ctx.cfg.profile_name
        ),
    );
    Ok(())
}

/// Confirm the key is accepted by the control plane before storing it.
async fn verify_api_key(api_key: &str, base_url: &str) -> anyhow::Result<()> {
    let client = ApiClient::new(
        SecretString::new(api_key.to_string().into()),
        Some(base_url.to_string()),
    )
    .context("failed to build API client for key verification")?;

    match client.runtime().list(1, 0).await {
        Ok(_) => Ok(()),
        Err(ApiError::Auth { message }) => {
            anyhow::bail!("invalid API key: {message}");
        }
        Err(err) => {
            anyhow::bail!("failed to verify API key against {base_url}: {err}");
        }
    }
}

async fn logout(ctx: &mut AppContext) -> anyhow::Result<()> {
    keyring_delete(&ctx.cfg.profile_name)?;
    ctx.cfg.api_key = None;
    output::success(
        ctx.output,
        format!("API key removed for profile '{}'", ctx.cfg.profile_name),
    );
    Ok(())
}

async fn status(ctx: &AppContext) -> anyhow::Result<()> {
    match active_api_key(ctx)? {
        None => {
            output::info(ctx.output, "No API key stored in keyring");
            output::info(
                ctx.output,
                "Run `gravixlayer auth login` or set GRAVIXLAYER_API_KEY to authenticate",
            );
        }
        Some(key) => {
            let masked = mask_key(&key);
            output::info(
                ctx.output,
                format!(
                    "Authenticated (profile: {}, key: {})",
                    ctx.cfg.profile_name, masked
                ),
            );
        }
    }
    Ok(())
}

async fn token(ctx: &AppContext) -> anyhow::Result<()> {
    match active_api_key(ctx)? {
        None => anyhow::bail!(
            "no API key found — run `gravixlayer auth login` or set GRAVIXLAYER_API_KEY"
        ),
        Some(key) => {
            // Warn on stderr so pipelines that capture stdout still get only the key,
            // while interactive users see the sensitivity notice.
            eprintln!(
                "warning: printing API key to stdout — do not share this value or commit it to logs"
            );
            println!("{key}");
        }
    }
    Ok(())
}

fn active_api_key(ctx: &AppContext) -> anyhow::Result<Option<String>> {
    if let Some(key) = ctx.cfg.api_key.as_ref().filter(|key| !key.is_empty()) {
        return Ok(Some(key.clone()));
    }
    keyring_get(&ctx.cfg.profile_name)
}

fn mask_key(key: &str) -> String {
    if key.len() > 4 {
        format!("{}…{}", "*".repeat(8), &key[key.len() - 4..])
    } else {
        "*".repeat(key.len())
    }
}

// TODO: re-enable once the backend exposes a stable API-key-compatible
// whoami endpoint. Currently /v1/users/me requires JWT authentication.
// The WhoAmI type and API call are preserved here for when that lands.
//
// async fn whoami(ctx: &AppContext) -> anyhow::Result<()> {
//     ctx.require_api_key()?;
//     let profile = ctx.api.whoami().await?;
//     output::print_or_json(ctx.output, &profile, || {
//         if let Some(ref email) = profile.email {
//             output::kv(ctx.output, "email", email);
//         }
//         if let Some(ref account_id) = profile.account_id {
//             output::kv(ctx.output, "account_id", account_id);
//         }
//         if let Some(ref plan) = profile.plan {
//             output::kv(ctx.output, "plan", plan);
//         }
//         if let Some(ref created_at) = profile.created_at {
//             output::kv(ctx.output, "created_at", created_at);
//         }
//     });
//     Ok(())
// }

#[cfg(test)]
mod tests {
    use super::mask_key;

    #[test]
    fn mask_key_hides_prefix() {
        let masked = mask_key("gl_abcdefghijklmnop");
        assert!(masked.ends_with("mnop"));
        assert!(!masked.contains("abcdef"));
    }

    #[test]
    fn mask_key_short() {
        assert_eq!(mask_key("ab"), "**");
    }
}
