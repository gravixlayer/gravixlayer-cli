// src/cmd/template.rs — Template command handlers.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::api::error::ApiError;
use crate::cli::{
    TemplateBuildArgs, TemplateBuildStatusArgs, TemplateCommand, TemplateDeleteArgs,
    TemplateGetArgs, TemplateListArgs,
};
use crate::ctx::AppContext;
use crate::output::{self, table};

pub async fn handle(ctx: &AppContext, cmd: TemplateCommand) -> anyhow::Result<()> {
    ctx.require_api_key()?;
    match cmd {
        TemplateCommand::List(args) => list(ctx, args).await,
        TemplateCommand::Get(args) => get(ctx, args).await,
        TemplateCommand::Snapshot(args) => snapshot(ctx, args).await,
        TemplateCommand::Build(args) => build(ctx, args).await,
        TemplateCommand::Delete(args) => delete(ctx, args).await,
        TemplateCommand::Status(args) => status(ctx, args).await,
    }
}

async fn list(ctx: &AppContext, args: TemplateListArgs) -> anyhow::Result<()> {
    let kind = args.kind.trim();
    if !matches!(kind, "sandbox" | "agent" | "all") {
        anyhow::bail!("invalid --kind '{kind}': expected sandbox, agent, or all");
    }
    let result = ctx
        .api
        .template()
        .list(
            args.limit,
            args.offset,
            Some(kind),
            args.project_id.as_deref(),
        )
        .await?;
    output::print_or_json(ctx.output, &result, || {
        println!("{}", table::template_table(&result.templates));
        if let Some(total) = result.total {
            println!("  ({total} total)");
        }
    });
    Ok(())
}

async fn get(ctx: &AppContext, args: TemplateGetArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "template")?;
    let tmpl = ctx.api.template().get(&args.id).await?;
    output::print_or_json(ctx.output, &tmpl, || {
        println!("{}", table::template_detail_table(&tmpl));
    });
    Ok(())
}

async fn snapshot(ctx: &AppContext, args: TemplateGetArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "template")?;
    let snap = ctx.api.template().snapshot(&args.id).await?;
    output::print_or_json(ctx.output, &snap, || {
        output::kv(ctx.output, "template_id", &snap.template_id);
        if let Some(ref name) = snap.name {
            output::kv(ctx.output, "name", name);
        }
        output::kv(
            ctx.output,
            "has_snapshot",
            if snap.has_snapshot { "true" } else { "false" },
        );
        if let Some(vcpu) = snap.vcpu_count {
            output::kv(ctx.output, "vcpu_count", &vcpu.to_string());
        }
        if let Some(mem) = snap.memory_mb {
            output::kv(ctx.output, "memory_mb", &mem.to_string());
        }
        if let Some(ref envd) = snap.envd_version {
            output::kv(ctx.output, "envd_version", envd);
        }
        if let Some(size) = snap.snapshot_size_bytes {
            output::kv(ctx.output, "snapshot_size_bytes", &size.to_string());
        }
    });
    Ok(())
}

async fn build(ctx: &AppContext, args: TemplateBuildArgs) -> anyhow::Result<()> {
    use secrecy::ExposeSecret;

    let environment = parse_key_value_pairs(&args.env_vars, "env")?;
    let tags = parse_key_value_pairs(&args.tags, "tag")?;
    let build_steps = parse_json_objects(&args.build_steps, "build-step")?;

    // -----------------------------------------------------------------------
    // Two distinct backend endpoints:
    //
    //   POST /v1/agents/template/build       — JSON body, BuildTemplateRequest
    //     • Accepts `dockerfile` as raw Dockerfile CONTENT string
    //     • Also accepts `vcpu_count`, `memory_mb`, `disk_mb`
    //     • No source archive required
    //
    //   POST /v1/agents/template/build-agent — multipart, AgentBuildTemplateRequest
    //     • Accepts `archive` (tar.gz of source directory)
    //     • Auto-detects/generates Dockerfile from source
    //     • vcpu_count, memory_mb, disk_mb supported via `metadata` JSON
    //
    // Routing: if --dockerfile is provided we read its contents and hit the
    // JSON endpoint directly.  Otherwise we package the source directory and
    // use the build-agent multipart endpoint.
    // -----------------------------------------------------------------------

    if let Some(df_path) = &args.dockerfile {
        // ------------------------------------------------------------------
        // Dockerfile build → JSON endpoint /template/build
        // ------------------------------------------------------------------
        let resolved = df_path
            .canonicalize()
            .map_err(|_| anyhow::anyhow!("Dockerfile not found: {}", df_path.display()))?;
        if !resolved.is_file() {
            anyhow::bail!("--dockerfile path is not a file: {}", resolved.display());
        }
        let dockerfile_content = std::fs::read_to_string(&resolved)?;

        let mut body = template_build_body(&args, &environment, &tags, &build_steps)?;
        body["dockerfile"] = dockerfile_content.into();

        let spinner = output::Spinner::new("Submitting Dockerfile build…");
        let url = ctx.api.agents_url("template/build");
        let resp = ctx
            .api
            .http_client()
            .post(url)
            .bearer_auth(ctx.api.api_key().expose_secret())
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            anyhow::bail!(
                "build submission failed (HTTP {}): {}",
                status.as_u16(),
                body_text
            );
        }
        let build_resp: crate::api::types::TemplateBuildResponse = resp.json().await?;
        spinner.finish_ok(format!(
            "Build submitted: {}{}",
            build_resp.build_id,
            build_resp
                .template_id
                .as_deref()
                .map(|id| format!(" (template {id})"))
                .unwrap_or_default()
        ));
        wait_for_build_if_requested(ctx, &build_resp.build_id, args.wait, args.build_timeout).await
    } else if let Some(docker_image) = &args.docker_image {
        // ------------------------------------------------------------------
        // Docker image build → JSON endpoint /template/build
        // Backend BuildTemplateRequest supports `docker_image` as an
        // alternative to `dockerfile` (mutually exclusive).
        // ------------------------------------------------------------------
        let mut body = template_build_body(&args, &environment, &tags, &build_steps)?;
        body["docker_image"] = docker_image.clone().into();

        let spinner = output::Spinner::new(format!("Submitting build from image {docker_image}…"));
        let url = ctx.api.agents_url("template/build");
        let resp = ctx
            .api
            .http_client()
            .post(url)
            .bearer_auth(ctx.api.api_key().expose_secret())
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            anyhow::bail!(
                "build submission failed (HTTP {}): {}",
                status.as_u16(),
                body_text
            );
        }
        let build_resp: crate::api::types::TemplateBuildResponse = resp.json().await?;
        spinner.finish_ok(format!(
            "Build submitted: {}{}",
            build_resp.build_id,
            build_resp
                .template_id
                .as_deref()
                .map(|id| format!(" (template {id})"))
                .unwrap_or_default()
        ));
        wait_for_build_if_requested(ctx, &build_resp.build_id, args.wait, args.build_timeout).await
    } else {
        // Source-directory agent builds belong under `gravixlayer agent build`.
        // Routing them through `template build` would create kind=agent templates
        // under the sandbox/runtime command surface.
        anyhow::bail!(
            "template build requires --dockerfile or --docker-image. \
For agent projects from a source directory, use: gravixlayer agent build"
        );
    }
}

fn template_build_body(
    args: &TemplateBuildArgs,
    environment: &HashMap<String, String>,
    tags: &HashMap<String, String>,
    build_steps: &[serde_json::Value],
) -> anyhow::Result<serde_json::Value> {
    let mut body = serde_json::Map::new();
    body.insert("name".into(), args.name.clone().into());
    insert_if_some(&mut body, "template_id", args.template_id.clone());
    insert_if_some(&mut body, "description", args.description.clone());
    insert_if_some_u32(&mut body, "vcpu_count", args.vcpu_count);
    insert_if_some_u32(&mut body, "memory_mb", args.memory_mb);
    insert_if_some_u32(&mut body, "disk_mb", args.disk_mb);
    insert_if_some(&mut body, "start_cmd", args.start_cmd.clone());
    insert_if_some(&mut body, "ready_cmd", args.ready_cmd.clone());
    insert_if_some_u32(&mut body, "ready_timeout_secs", args.ready_timeout_secs);
    if !environment.is_empty() {
        body.insert("environment".into(), serde_json::to_value(environment)?);
    }
    if !build_steps.is_empty() {
        body.insert(
            "build_steps".into(),
            serde_json::Value::Array(build_steps.to_vec()),
        );
    }
    if !tags.is_empty() {
        body.insert("tags".into(), serde_json::to_value(tags)?);
    }
    Ok(serde_json::Value::Object(body))
}

fn insert_if_some(
    map: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: Option<String>,
) {
    if let Some(value) = value {
        map.insert(key.to_string(), value.into());
    }
}

fn insert_if_some_u32(
    map: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: Option<u32>,
) {
    if let Some(value) = value {
        map.insert(key.to_string(), serde_json::Value::Number(value.into()));
    }
}

fn parse_key_value_pairs(
    pairs: &[String],
    flag_name: &str,
) -> anyhow::Result<HashMap<String, String>> {
    pairs
        .iter()
        .map(|pair| {
            let separator = pair.find('=').ok_or_else(|| {
                anyhow::anyhow!("invalid --{flag_name} '{}': expected KEY=VALUE", pair)
            })?;
            let key = &pair[..separator];
            let value = &pair[separator + 1..];
            if key.is_empty() {
                anyhow::bail!("invalid --{flag_name} '{}': key must not be empty", pair);
            }
            Ok((key.to_string(), value.to_string()))
        })
        .collect()
}

fn parse_json_objects(
    raw_values: &[String],
    flag_name: &str,
) -> anyhow::Result<Vec<serde_json::Value>> {
    raw_values
        .iter()
        .map(|raw| {
            let value: serde_json::Value = serde_json::from_str(raw)
                .map_err(|e| anyhow::anyhow!("invalid --{flag_name} JSON: {e}"))?;
            if !value.is_object() {
                anyhow::bail!("--{flag_name} must be a JSON object");
            }
            Ok(value)
        })
        .collect()
}

async fn wait_for_build_if_requested(
    ctx: &AppContext,
    build_id: &str,
    wait: bool,
    build_timeout: u64,
) -> anyhow::Result<()> {
    if wait {
        let sp = output::Spinner::new(format!("Waiting for build {build_id}…"));
        let deadline = Instant::now() + Duration::from_secs(build_timeout);
        let final_status = ctx
            .api
            .template()
            .wait_for_build(build_id, deadline)
            .await?;
        sp.finish_ok(format!("Build {} completed", final_status.build_id));
        output::print_or_json(ctx.output, &final_status, || {
            println!("{}", table::build_status_table(&final_status));
        });
    } else {
        output::info(
            ctx.output,
            format!("Track build progress with: gravixlayer template status {build_id}"),
        );
    }
    Ok(())
}

async fn delete(ctx: &AppContext, args: TemplateDeleteArgs) -> anyhow::Result<()> {
    if !args.yes {
        eprint!("Delete template {}? [y/N] ", args.id);
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }
    if let Err(err) = ctx.api.template().delete(&args.id).await {
        if let ApiError::BadRequest { status: 404, .. } = err {
            anyhow::bail!("Template {} not found", args.id);
        }
        return Err(err.into());
    }

    let resp = serde_json::json!({"template_id": args.id, "deleted": true});
    output::print_or_json(ctx.output, &resp, || {
        output::success(ctx.output, format!("Template {} deleted", args.id));
    });
    Ok(())
}

async fn status(ctx: &AppContext, args: TemplateBuildStatusArgs) -> anyhow::Result<()> {
    let s = ctx.api.template().build_status(&args.build_id).await?;
    output::print_or_json(ctx.output, &s, || {
        println!("{}", table::build_status_table(&s));
    });
    Ok(())
}
