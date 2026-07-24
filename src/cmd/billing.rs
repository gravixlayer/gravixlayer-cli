// src/cmd/billing.rs — Billing command handlers.

use crate::cli::{BillingCommand, BillingHistoryArgs, BillingSummaryArgs};
use crate::ctx::AppContext;
use crate::output::{self, table};

pub async fn handle(ctx: &AppContext, cmd: BillingCommand) -> anyhow::Result<()> {
    ctx.require_api_key()?;
    match cmd {
        BillingCommand::Summary(args) => summary(ctx, args).await,
        BillingCommand::History(args) => history(ctx, args).await,
        BillingCommand::Quotas => quotas(ctx).await,
    }
}

async fn summary(ctx: &AppContext, args: BillingSummaryArgs) -> anyhow::Result<()> {
    if let Some(ref month) = args.month {
        validate_month(month)?;
    }
    let s = ctx
        .api
        .billing()
        .summary(args.month.as_deref(), args.project_id.as_deref())
        .await?;
    output::print_or_json(ctx.output, &s, || {
        println!("{}", table::billing_summary_table(&s));
    });
    Ok(())
}

async fn history(ctx: &AppContext, args: BillingHistoryArgs) -> anyhow::Result<()> {
    let h = ctx
        .api
        .billing()
        .history(
            args.page,
            args.page_size,
            args.from.as_deref(),
            args.to.as_deref(),
            args.runtime_id.as_deref(),
            args.status.as_deref(),
            args.project_id.as_deref(),
        )
        .await?;
    output::print_or_json(ctx.output, &h, || {
        println!("{}", table::billing_history_table(&h.items));
        if let Some(total) = h.total {
            println!("  ({total} total)");
        }
    });
    Ok(())
}

async fn quotas(ctx: &AppContext) -> anyhow::Result<()> {
    let q = ctx.api.billing().quotas().await?;
    output::print_or_json(ctx.output, &q, || {
        println!("{}", table::billing_quota_table(&q));
    });
    Ok(())
}

fn validate_month(month: &str) -> anyhow::Result<()> {
    let parts: Vec<_> = month.split('-').collect();
    if parts.len() != 2 || parts[0].len() != 4 || parts[1].len() != 2 {
        anyhow::bail!("invalid --month '{month}': expected YYYY-MM");
    }
    let year: u32 = parts[0]
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid --month '{month}': year must be numeric"))?;
    let mon: u32 = parts[1]
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid --month '{month}': month must be numeric"))?;
    if !(1..=12).contains(&mon) || year < 2000 {
        anyhow::bail!("invalid --month '{month}': out of range");
    }
    Ok(())
}
