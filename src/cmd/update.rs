// src/cmd/update.rs — Self-update via GitHub Releases using the self_update crate.

use anyhow::{Context, Result};

use crate::cli::{OutputFormat, UpdateArgs};
use crate::output;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const REPO_OWNER: &str = "gravixlayer";
const REPO_NAME: &str = "gravixlayer-cli";
const BIN_NAME: &str = "gravixlayer";

pub async fn handle(output_fmt: OutputFormat, args: UpdateArgs) -> Result<()> {
    output::info(output_fmt, format!("Current version: {CURRENT_VERSION}"));

    let check_only = args.check;
    let target_version = args
        .version
        .as_deref()
        .map(|v| v.trim().trim_start_matches('v').to_string());

    if check_only && target_version.is_some() {
        anyhow::bail!("--check and --version cannot be used together");
    }

    let result = tokio::task::spawn_blocking(move || -> Result<UpdateOutcome> {
        let mut builder = self_update::backends::github::Update::configure();
        builder
            .repo_owner(REPO_OWNER)
            .repo_name(REPO_NAME)
            .bin_name(BIN_NAME)
            .current_version(CURRENT_VERSION)
            .no_confirm(true);

        if let Some(ref ver) = target_version {
            builder.target_version_tag(&format!("v{ver}"));
        }

        let updater = builder.build().context("configure updater")?;

        if check_only {
            let release = updater
                .get_latest_release()
                .context("fetch latest release from GitHub")?;
            let latest = release.version.trim_start_matches('v').to_string();
            if latest == CURRENT_VERSION {
                return Ok(UpdateOutcome::UpToDate);
            }
            return Ok(UpdateOutcome::Available(latest));
        }

        // Performing an update (latest or pinned tag).
        let status = updater.update().context("perform update")?;
        match status {
            self_update::Status::UpToDate(v) => {
                let v = v.trim_start_matches('v').to_string();
                if v == CURRENT_VERSION {
                    Ok(UpdateOutcome::UpToDate)
                } else {
                    Ok(UpdateOutcome::Updated(v))
                }
            }
            self_update::Status::Updated(v) => Ok(UpdateOutcome::Updated(
                v.trim_start_matches('v').to_string(),
            )),
        }
    })
    .await
    .context("update task panicked")??;

    match result {
        UpdateOutcome::UpToDate => {
            output::success(
                output_fmt,
                format!("Already up to date ({CURRENT_VERSION})."),
            );
        }
        UpdateOutcome::Available(latest) => {
            output::info(output_fmt, format!("New version available: {latest}"));
            output::info(output_fmt, "Run `gravixlayer update` to upgrade.");
        }
        UpdateOutcome::Updated(latest) => {
            output::success(
                output_fmt,
                format!("Updated to {latest}. Run `gravixlayer --version` to confirm."),
            );
        }
    }
    Ok(())
}

enum UpdateOutcome {
    UpToDate,
    Available(String),
    Updated(String),
}
