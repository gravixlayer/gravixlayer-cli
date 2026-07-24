// src/cmd/network_policy.rs — Network policy command handlers.

use crate::api::types::{
    AddNetworkPolicyRuleRequest, CreateNetworkPolicyRequest, UpdateNetworkPolicyRequest,
    UpdateNetworkPolicyRuleRequest,
};
use crate::cli::*;
use crate::ctx::AppContext;
use crate::output::{self, table};

pub async fn handle(ctx: &AppContext, cmd: NetworkPolicyCommand) -> anyhow::Result<()> {
    ctx.require_api_key()?;
    match cmd {
        NetworkPolicyCommand::Create(args) => create(ctx, args).await,
        NetworkPolicyCommand::List(args) => list(ctx, args).await,
        NetworkPolicyCommand::Get(args) => get(ctx, args).await,
        NetworkPolicyCommand::Update(args) => update(ctx, args).await,
        NetworkPolicyCommand::Delete(args) => delete(ctx, args).await,
        NetworkPolicyCommand::AddRule(args) => add_rule(ctx, args).await,
        NetworkPolicyCommand::ListRules(args) => list_rules(ctx, args).await,
        NetworkPolicyCommand::UpdateRule(args) => update_rule(ctx, args).await,
        NetworkPolicyCommand::DeleteRule(args) => delete_rule(ctx, args).await,
        NetworkPolicyCommand::Attach(args) => attach(ctx, args).await,
        NetworkPolicyCommand::Detach(args) => detach(ctx, args).await,
        NetworkPolicyCommand::ListAttached(args) => list_attached(ctx, args).await,
    }
}

async fn create(ctx: &AppContext, args: NetworkPolicyCreateArgs) -> anyhow::Result<()> {
    let req = CreateNetworkPolicyRequest {
        name: args.name,
        egress_mode: args.egress_mode,
        description: args.description,
        is_default: args.is_default,
    };
    let policy = ctx
        .api
        .network_policy()
        .create(&req, args.project_id.as_deref())
        .await?;
    output::print_or_json(ctx.output, &policy, || {
        println!("{}", table::network_policy_detail_table(&policy));
    });
    Ok(())
}

async fn list(ctx: &AppContext, args: NetworkPolicyListArgs) -> anyhow::Result<()> {
    let result = ctx
        .api
        .network_policy()
        .list(
            args.limit,
            args.offset,
            args.project_id.as_deref(),
            args.search.as_deref(),
        )
        .await?;
    output::print_or_json(ctx.output, &result, || {
        println!("{}", table::network_policy_table(&result.policies));
        println!("  ({} total)", result.total);
    });
    Ok(())
}

async fn get(ctx: &AppContext, args: NetworkPolicyGetArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "network policy")?;
    let policy = ctx.api.network_policy().get(&args.id).await?;
    output::print_or_json(ctx.output, &policy, || {
        println!("{}", table::network_policy_detail_table(&policy));
    });
    Ok(())
}

async fn update(ctx: &AppContext, args: NetworkPolicyUpdateArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "network policy")?;
    let req = UpdateNetworkPolicyRequest {
        name: args.name,
        egress_mode: args.egress_mode,
        description: args.description,
        is_active: if args.enabled {
            Some(true)
        } else if args.disabled {
            Some(false)
        } else {
            None
        },
        is_default: if args.set_default {
            Some(true)
        } else if args.unset_default {
            Some(false)
        } else {
            None
        },
    };
    if req.name.is_none()
        && req.egress_mode.is_none()
        && req.description.is_none()
        && req.is_active.is_none()
        && req.is_default.is_none()
    {
        anyhow::bail!(
            "provide at least one of --name, --egress-mode, --description, --enabled, --disabled, --set-default, or --unset-default"
        );
    }
    let policy = ctx
        .api
        .network_policy()
        .update(&args.id, &req, args.project_id.as_deref())
        .await?;
    output::print_or_json(ctx.output, &policy, || {
        println!("{}", table::network_policy_detail_table(&policy));
    });
    Ok(())
}

async fn delete(ctx: &AppContext, args: NetworkPolicyDeleteArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "network policy")?;
    if !args.yes {
        anyhow::bail!("pass --yes / -y to confirm deletion");
    }
    let result = ctx
        .api
        .network_policy()
        .delete(&args.id, args.project_id.as_deref())
        .await?;
    output::print_or_json(ctx.output, &result, || {
        output::success(ctx.output, format!("Deleted network policy {}", args.id));
    });
    Ok(())
}

async fn add_rule(ctx: &AppContext, args: NetworkPolicyAddRuleArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "network policy")?;
    let req = AddNetworkPolicyRuleRequest {
        destination: args.destination,
        port: args.port,
        protocol: args.protocol,
        description: args.description,
    };
    let rule = ctx
        .api
        .network_policy()
        .add_rule(&args.id, &req, args.project_id.as_deref())
        .await?;
    output::print_or_json(ctx.output, &rule, || {
        output::success(
            ctx.output,
            format!(
                "Added rule {} ({}:{}/{})",
                rule.id, rule.destination, rule.port, rule.protocol
            ),
        );
        println!(
            "{}",
            table::network_policy_rule_table(std::slice::from_ref(&rule))
        );
    });
    Ok(())
}

async fn list_rules(ctx: &AppContext, args: NetworkPolicyListRulesArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "network policy")?;
    let result = ctx.api.network_policy().list_rules(&args.id).await?;
    output::print_or_json(ctx.output, &result, || {
        println!("{}", table::network_policy_rule_table(&result.rules));
    });
    Ok(())
}

async fn update_rule(ctx: &AppContext, args: NetworkPolicyUpdateRuleArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "network policy")?;
    super::validate_resource_id(&args.rule_id, "rule")?;
    let req = UpdateNetworkPolicyRuleRequest {
        destination: args.destination,
        port: args.port,
        protocol: args.protocol,
        description: args.description,
    };
    if req.destination.is_none()
        && req.port.is_none()
        && req.protocol.is_none()
        && req.description.is_none()
    {
        anyhow::bail!(
            "provide at least one of --destination, --port, --protocol, or --description"
        );
    }
    let rule = ctx
        .api
        .network_policy()
        .update_rule(&args.id, &args.rule_id, &req, args.project_id.as_deref())
        .await?;
    output::print_or_json(ctx.output, &rule, || {
        output::success(ctx.output, format!("Updated rule {}", rule.id));
        println!(
            "{}",
            table::network_policy_rule_table(std::slice::from_ref(&rule))
        );
    });
    Ok(())
}

async fn delete_rule(ctx: &AppContext, args: NetworkPolicyDeleteRuleArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "network policy")?;
    super::validate_resource_id(&args.rule_id, "rule")?;
    if !args.yes {
        anyhow::bail!("pass --yes / -y to confirm deletion");
    }
    let result = ctx
        .api
        .network_policy()
        .delete_rule(&args.id, &args.rule_id, args.project_id.as_deref())
        .await?;
    output::print_or_json(ctx.output, &result, || {
        output::success(ctx.output, format!("Deleted rule {}", args.rule_id));
    });
    Ok(())
}

async fn attach(ctx: &AppContext, args: NetworkPolicyAttachArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "network policy")?;
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let result = ctx
        .api
        .network_policy()
        .attach(&args.id, &args.runtime_id, args.project_id.as_deref())
        .await?;
    output::print_or_json(ctx.output, &result, || {
        output::success(
            ctx.output,
            format!(
                "Attached network policy {} to runtime {}",
                args.id, args.runtime_id
            ),
        );
    });
    Ok(())
}

async fn detach(ctx: &AppContext, args: NetworkPolicyDetachArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "network policy")?;
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let result = ctx
        .api
        .network_policy()
        .detach(&args.id, &args.runtime_id, args.project_id.as_deref())
        .await?;
    output::print_or_json(ctx.output, &result, || {
        output::success(
            ctx.output,
            format!(
                "Detached network policy {} from runtime {}",
                args.id, args.runtime_id
            ),
        );
    });
    Ok(())
}

async fn list_attached(
    ctx: &AppContext,
    args: NetworkPolicyListAttachedArgs,
) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let result = ctx
        .api
        .network_policy()
        .list_for_runtime(&args.runtime_id)
        .await?;
    output::print_or_json(ctx.output, &result, || {
        println!("{}", table::network_policy_table(&result.policies));
    });
    Ok(())
}
