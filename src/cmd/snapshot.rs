// src/cmd/snapshot.rs — Named snapshot catalog command handlers.

use crate::api::error::ApiError;
use crate::api::types::CreateSnapshotRequest;
use crate::cli::{
    SnapshotActivateArgs, SnapshotCommand, SnapshotCreateArgs, SnapshotDeactivateArgs,
    SnapshotDeleteArgs, SnapshotGetArgs, SnapshotListArgs,
};
use crate::ctx::AppContext;
use crate::output::{self, table};

pub async fn handle(ctx: &AppContext, cmd: SnapshotCommand) -> anyhow::Result<()> {
    ctx.require_api_key()?;
    match cmd {
        SnapshotCommand::Create(args) => create(ctx, args).await,
        SnapshotCommand::List(args) => list(ctx, args).await,
        SnapshotCommand::Get(args) => get(ctx, args).await,
        SnapshotCommand::Delete(args) => delete(ctx, args).await,
        SnapshotCommand::Activate(args) => activate(ctx, args).await,
        SnapshotCommand::Deactivate(args) => deactivate(ctx, args).await,
    }
}

async fn create(ctx: &AppContext, args: SnapshotCreateArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    super::validate_resource_id(&args.name, "snapshot")?;
    let kind = args.kind.trim().to_ascii_lowercase();
    if !matches!(kind.as_str(), "hot" | "cold") {
        anyhow::bail!("invalid --kind '{kind}': expected hot or cold");
    }

    let spinner = output::Spinner::new(format!("Capturing {} snapshot {}…", kind, args.name));
    let snap = ctx
        .api
        .snapshot()
        .create(CreateSnapshotRequest {
            name: args.name,
            runtime_id: args.runtime_id,
            description: args.description,
            kind: Some(kind),
        })
        .await?;
    spinner.finish_ok(format!("Snapshot {} created", snap.id));
    output::print_or_json(ctx.output, &snap, || {
        println!("{}", table::snapshot_detail_table(&snap));
    });
    Ok(())
}

async fn list(ctx: &AppContext, args: SnapshotListArgs) -> anyhow::Result<()> {
    let kind = args.kind.as_deref().map(str::trim).filter(|k| !k.is_empty());
    if let Some(kind) = kind {
        if !matches!(kind, "hot" | "cold" | "all") {
            anyhow::bail!("invalid --kind '{kind}': expected hot, cold, or all");
        }
    }
    let kind = kind.filter(|k| *k != "all");
    let result = ctx
        .api
        .snapshot()
        .list(
            args.limit,
            args.offset,
            kind,
            args.runtime_id.as_deref(),
            args.state.as_deref(),
            args.source.as_deref(),
        )
        .await?;
    output::print_or_json(ctx.output, &result, || {
        println!("{}", table::snapshot_table(&result.snapshots));
        if let Some(total) = result.total {
            println!("  ({total} total)");
        }
    });
    Ok(())
}

async fn get(ctx: &AppContext, args: SnapshotGetArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "snapshot")?;
    let snap = ctx.api.snapshot().get(&args.id).await?;
    output::print_or_json(ctx.output, &snap, || {
        println!("{}", table::snapshot_detail_table(&snap));
    });
    Ok(())
}

async fn activate(ctx: &AppContext, args: SnapshotActivateArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "snapshot")?;
    let snap = ctx.api.snapshot().activate(&args.id).await?;
    output::print_or_json(ctx.output, &snap, || {
        output::success(ctx.output, format!("Snapshot {} activated", snap.id));
        println!("{}", table::snapshot_detail_table(&snap));
    });
    Ok(())
}

async fn deactivate(ctx: &AppContext, args: SnapshotDeactivateArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "snapshot")?;
    let snap = ctx.api.snapshot().deactivate(&args.id).await?;
    output::print_or_json(ctx.output, &snap, || {
        output::success(ctx.output, format!("Snapshot {} deactivated", snap.id));
        println!("{}", table::snapshot_detail_table(&snap));
    });
    Ok(())
}

async fn delete(ctx: &AppContext, args: SnapshotDeleteArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "snapshot")?;
    if !args.yes {
        eprint!("Delete snapshot {}? [y/N] ", args.id);
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }
    if let Err(err) = ctx.api.snapshot().delete(&args.id).await {
        if let ApiError::BadRequest { status: 404, .. } = err {
            anyhow::bail!("Snapshot {} not found", args.id);
        }
        return Err(err.into());
    }

    let resp = serde_json::json!({"snapshot_id": args.id, "deleted": true});
    output::print_or_json(ctx.output, &resp, || {
        output::success(ctx.output, format!("Snapshot {} deleted", args.id));
    });
    Ok(())
}
