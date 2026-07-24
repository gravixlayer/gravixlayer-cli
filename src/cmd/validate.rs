// src/cmd/validate.rs — Validate gravixlayer.json.

use crate::cli::ValidateArgs;
use crate::config::project::GravixlayerProject;
use crate::ctx::AppContext;
use crate::framework;
use crate::output;

pub async fn handle(ctx: &AppContext, args: ValidateArgs) -> anyhow::Result<()> {
    // If the path is a directory, look for gravixlayer/gravixlayer.json inside it.
    let json_path = if args.path.is_dir() {
        args.path.join("gravixlayer").join("gravixlayer.json")
    } else {
        args.path.clone()
    };

    if !json_path.is_file() {
        anyhow::bail!("project file not found: {}", json_path.display());
    }

    let proj = GravixlayerProject::load(&json_path)?;

    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // Validate required fields
    if proj
        .name
        .as_deref()
        .map(|n| n.trim().is_empty())
        .unwrap_or(true)
    {
        errors.push("'name' field is required".into());
    }

    // Validate port range
    if let Some(port) = proj.port {
        if port < 1024 {
            warnings.push(format!("port {port} is below 1024 — prefer a port >= 1024"));
        }
    }

    // Validate framework value
    if let Some(fw) = proj.framework.as_deref() {
        if framework::canonical_framework(fw).is_none() {
            errors.push(framework::unknown_framework_message(fw));
        }
    }

    // Validate protocol values
    for protocol in &proj.protocols {
        if framework::canonical_protocol(protocol).is_none() {
            errors.push(format!(
                "unknown protocol '{protocol}' — expected one of: {}",
                framework::CANONICAL_PROTOCOLS.join(", ")
            ));
        }
    }

    // Validate python version format
    if let Some(py) = proj.python_version.as_deref() {
        let valid = py
            .split('.')
            .all(|part| part.chars().all(|c| c.is_ascii_digit()));
        if !valid {
            errors.push(format!(
                "invalid python_version '{py}' — expected format like '3.12'"
            ));
        }
    }

    // Report
    for err in &errors {
        output::error(format!("[ERROR] {err}"));
    }
    for warn in &warnings {
        output::warn(format!("[WARN]  {warn}"));
    }

    if !errors.is_empty() {
        anyhow::bail!(
            "{} validation error(s) found in {}",
            errors.len(),
            json_path.display()
        );
    }

    output::success(ctx.output, format!("{} is valid", json_path.display()));
    Ok(())
}
