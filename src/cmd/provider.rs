// src/cmd/provider.rs — Secret provider command handlers.

use crate::api::types::{
    CreateSecretProviderRequest, SecretPairRequest, UpdateSecretProviderRequest,
    UpdateSecretRequest,
};
use crate::cli::*;
use crate::ctx::AppContext;
use crate::output::{self, table};

pub async fn handle(ctx: &AppContext, cmd: ProviderCommand) -> anyhow::Result<()> {
    ctx.require_api_key()?;
    match cmd {
        ProviderCommand::Create(args) => create(ctx, args).await,
        ProviderCommand::List(args) => list(ctx, args).await,
        ProviderCommand::Get(args) => get(ctx, args).await,
        ProviderCommand::Update(args) => update(ctx, args).await,
        ProviderCommand::Delete(args) => delete(ctx, args).await,
        ProviderCommand::AddSecret(args) => add_secret(ctx, args).await,
        ProviderCommand::ListSecrets(args) => list_secrets(ctx, args).await,
        ProviderCommand::UpdateSecret(args) => update_secret(ctx, args).await,
        ProviderCommand::DeleteSecret(args) => delete_secret(ctx, args).await,
        ProviderCommand::Attach(args) => attach(ctx, args).await,
        ProviderCommand::Detach(args) => detach(ctx, args).await,
        ProviderCommand::ListAttached(args) => list_attached(ctx, args).await,
    }
}

async fn create(ctx: &AppContext, args: ProviderCreateArgs) -> anyhow::Result<()> {
    let secrets = parse_secret_pairs(&args.secret)?;
    let req = CreateSecretProviderRequest {
        name: args.name,
        provider_type: args.provider_type,
        secrets,
    };
    let provider = ctx
        .api
        .provider()
        .create(&req, args.project_id.as_deref())
        .await?;
    output::print_or_json(ctx.output, &provider, || {
        println!("{}", table::provider_detail_table(&provider));
    });
    Ok(())
}

async fn list(ctx: &AppContext, args: ProviderListArgs) -> anyhow::Result<()> {
    let result = ctx
        .api
        .provider()
        .list(
            args.limit,
            args.offset,
            args.project_id.as_deref(),
            args.search.as_deref(),
        )
        .await?;
    output::print_or_json(ctx.output, &result, || {
        println!("{}", table::provider_table(&result.providers));
        println!("  ({} total)", result.total);
    });
    Ok(())
}

async fn get(ctx: &AppContext, args: ProviderGetArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "provider")?;
    let provider = ctx.api.provider().get(&args.id).await?;
    output::print_or_json(ctx.output, &provider, || {
        println!("{}", table::provider_detail_table(&provider));
        if !provider.secrets.is_empty() {
            println!();
            println!("{}", table::secret_table(&provider.secrets));
        }
    });
    Ok(())
}

async fn update(ctx: &AppContext, args: ProviderUpdateArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "provider")?;
    let req = UpdateSecretProviderRequest {
        name: args.name,
        provider_type: args.provider_type,
        is_active: if args.enabled {
            Some(true)
        } else if args.disabled {
            Some(false)
        } else {
            None
        },
    };
    if req.name.is_none() && req.provider_type.is_none() && req.is_active.is_none() {
        anyhow::bail!("provide at least one of --name, --type, --enabled, or --disabled");
    }
    let provider = ctx
        .api
        .provider()
        .update(&args.id, &req, args.project_id.as_deref())
        .await?;
    output::print_or_json(ctx.output, &provider, || {
        println!("{}", table::provider_detail_table(&provider));
    });
    Ok(())
}

async fn delete(ctx: &AppContext, args: ProviderDeleteArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "provider")?;
    if !args.yes {
        anyhow::bail!("pass --yes / -y to confirm deletion");
    }
    let result = ctx
        .api
        .provider()
        .delete(&args.id, args.project_id.as_deref())
        .await?;
    output::print_or_json(ctx.output, &result, || {
        output::success(ctx.output, format!("Deleted provider {}", args.id));
    });
    Ok(())
}

async fn add_secret(ctx: &AppContext, args: ProviderAddSecretArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "provider")?;
    let secret = ctx
        .api
        .provider()
        .add_secret(&args.id, &args.key, &args.value, args.project_id.as_deref())
        .await?;
    output::print_or_json(ctx.output, &secret, || {
        output::success(ctx.output, format!("Added secret {}", secret.key));
        println!("{}", table::secret_table(std::slice::from_ref(&secret)));
    });
    Ok(())
}

async fn list_secrets(ctx: &AppContext, args: ProviderListSecretsArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "provider")?;
    let result = ctx.api.provider().list_secrets(&args.id).await?;
    output::print_or_json(ctx.output, &result, || {
        println!("{}", table::secret_table(&result.secrets));
    });
    Ok(())
}

async fn update_secret(ctx: &AppContext, args: ProviderUpdateSecretArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "provider")?;
    super::validate_resource_id(&args.secret_id, "secret")?;
    let req = UpdateSecretRequest {
        key: args.key,
        value: args.value,
    };
    if req.key.is_none() && req.value.is_none() {
        anyhow::bail!("provide --key and/or --value");
    }
    let secret = ctx
        .api
        .provider()
        .update_secret(&args.id, &args.secret_id, &req, args.project_id.as_deref())
        .await?;
    output::print_or_json(ctx.output, &secret, || {
        output::success(ctx.output, format!("Updated secret {}", secret.key));
        println!("{}", table::secret_table(std::slice::from_ref(&secret)));
    });
    Ok(())
}

async fn delete_secret(ctx: &AppContext, args: ProviderDeleteSecretArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "provider")?;
    super::validate_resource_id(&args.secret_id, "secret")?;
    if !args.yes {
        anyhow::bail!("pass --yes / -y to confirm deletion");
    }
    let result = ctx
        .api
        .provider()
        .delete_secret(&args.id, &args.secret_id, args.project_id.as_deref())
        .await?;
    output::print_or_json(ctx.output, &result, || {
        output::success(ctx.output, format!("Deleted secret {}", args.secret_id));
    });
    Ok(())
}

async fn attach(ctx: &AppContext, args: ProviderAttachArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "provider")?;
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let result = ctx
        .api
        .provider()
        .attach(&args.id, &args.runtime_id, args.project_id.as_deref())
        .await?;
    output::print_or_json(ctx.output, &result, || {
        output::success(
            ctx.output,
            format!(
                "Attached provider {} to runtime {}",
                args.id, args.runtime_id
            ),
        );
    });
    Ok(())
}

async fn detach(ctx: &AppContext, args: ProviderDetachArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "provider")?;
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let result = ctx
        .api
        .provider()
        .detach(&args.id, &args.runtime_id, args.project_id.as_deref())
        .await?;
    output::print_or_json(ctx.output, &result, || {
        output::success(
            ctx.output,
            format!(
                "Detached provider {} from runtime {}",
                args.id, args.runtime_id
            ),
        );
    });
    Ok(())
}

async fn list_attached(ctx: &AppContext, args: ProviderListAttachedArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let result = ctx
        .api
        .provider()
        .list_for_runtime(&args.runtime_id)
        .await?;
    output::print_or_json(ctx.output, &result, || {
        println!("{}", table::provider_table(&result.providers));
    });
    Ok(())
}

fn parse_secret_pairs(pairs: &[String]) -> anyhow::Result<Vec<SecretPairRequest>> {
    let mut out = Vec::new();
    for pair in pairs {
        let (key, value) = pair
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("invalid --secret '{pair}': expected KEY=VALUE"))?;
        if key.is_empty() {
            anyhow::bail!("invalid --secret '{pair}': key must not be empty");
        }
        out.push(SecretPairRequest {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
    Ok(out)
}
