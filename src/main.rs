// src/main.rs — Entry point for the `gravixlayer` CLI binary.

use anyhow::Context;
use clap::{CommandFactory, Parser};
use clap_complete::generate;
use tracing_subscriber::{fmt, EnvFilter};

mod api;
mod cli;
mod cmd;
mod config;
mod ctx;
mod framework;
mod output;
mod scaffold;
mod terminal;

use cli::{Cli, Commands};
use ctx::AppContext;

#[tokio::main]
async fn main() {
    // rustls 0.23: both aws-lc-rs and ring appear in the dependency tree, so
    // the crate cannot auto-pick a CryptoProvider. Install one before any TLS
    // (reqwest HTTPS, tokio-tungstenite WebSocket for `runtime shell`).
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    // Parse first so we can read --verbose before building the context.
    let cli = Cli::parse();

    // Initialise tracing.  `RUST_LOG` takes precedence; `--verbose` bumps
    // the effective level to `debug`.
    let default_level = if cli.verbose { "debug" } else { "warn" };
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));
    fmt::SubscriberBuilder::default()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    let exit_code = run(cli).await;
    std::process::exit(exit_code);
}

async fn run(cli: Cli) -> i32 {
    match dispatch(cli).await {
        Ok(()) => 0,
        Err(err) => {
            // Print the full error chain to stderr.
            eprintln!("error: {err:#}");
            1
        }
    }
}

async fn dispatch(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        // ---------------------------------------------------------------------------
        // Commands that do NOT require authentication or network access.
        // ---------------------------------------------------------------------------
        Commands::Completions(args) => {
            let mut cmd = Cli::command();
            let shell = args.shell;
            let name = cmd.get_name().to_string();
            generate(shell, &mut cmd, name, &mut std::io::stdout());
            return Ok(());
        }

        Commands::Doctor => {
            cmd::doctor::handle(cli.output).await?;
            return Ok(());
        }

        Commands::Update(args) => {
            cmd::update::handle(cli.output, args).await?;
            return Ok(());
        }

        // ---------------------------------------------------------------------------
        // All other commands need a fully-configured AppContext.
        // ---------------------------------------------------------------------------
        other => {
            let mut ctx = AppContext::build(cli.api_key, cli.base_url, cli.profile, cli.output)
                .context("failed to initialise CLI context")?;

            match other {
                Commands::Auth(args) => cmd::auth::handle(&mut ctx, args.command).await?,
                Commands::Config(args) => cmd::config::handle(&mut ctx, args.command).await?,
                Commands::Runtime(args) => cmd::runtime::handle(&mut ctx, args.command).await?,
                Commands::Template(args) => cmd::template::handle(&ctx, args.command).await?,
                Commands::Snapshot(args) => cmd::snapshot::handle(&ctx, args.command).await?,
                Commands::Provider(args) => cmd::provider::handle(&ctx, args.command).await?,
                Commands::NetworkPolicy(args) => {
                    cmd::network_policy::handle(&ctx, args.command).await?
                }
                Commands::Agent(args) => cmd::agent::handle(&ctx, args.command).await?,
                Commands::Billing(args) => cmd::billing::handle(&ctx, args.command).await?,
                Commands::Validate(args) => cmd::validate::handle(&ctx, args).await?,
                Commands::Package(args) => cmd::package::handle(&ctx, args).await?,
                // Already handled above.
                Commands::Completions(_) | Commands::Doctor | Commands::Update(_) => unreachable!(),
            }
        }
    }
    Ok(())
}
