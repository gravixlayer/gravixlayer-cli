// src/cmd/runtime.rs — Runtime command handlers.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::Context;

use crate::api::guest_deadline_budget;
use crate::api::types::CreateRuntimeRequest;
use crate::cli::{
    RuntimeBrowserCommand, RuntimeCodeContextCommand, RuntimeCommand, RuntimeCommandsCommand,
    RuntimeCommandsIdArgs, RuntimeCommandsKillArgs, RuntimeCommandsPidArgs, RuntimeConnectArgs,
    RuntimeContextCommand, RuntimeCreateArgs, RuntimeDesktopCommand, RuntimeDisplayCommand,
    RuntimeExecArgs, RuntimeFilesCatArgs, RuntimeFilesChmodArgs, RuntimeFilesChownArgs,
    RuntimeFilesCommand, RuntimeFilesCopyArgs, RuntimeFilesDeleteArgs, RuntimeFilesDownloadArgs,
    RuntimeFilesFindArgs, RuntimeFilesInfoArgs, RuntimeFilesListArgs, RuntimeFilesMkdirArgs,
    RuntimeFilesMoveArgs, RuntimeFilesReplaceArgs, RuntimeFilesUploadArgs, RuntimeFilesWatchArgs,
    RuntimeFilesWriteArgs, RuntimeFilesWriteManyArgs, RuntimeForkArgs, RuntimeGetArgs,
    RuntimeGitAddArgs, RuntimeGitBranchArgs, RuntimeGitBranchCreateArgs,
    RuntimeGitBranchDeleteArgs, RuntimeGitCheckoutArgs, RuntimeGitCloneArgs, RuntimeGitCommand,
    RuntimeGitCommitArgs, RuntimeGitFetchArgs, RuntimeGitPullArgs, RuntimeGitPushArgs,
    RuntimeGitStatusArgs, RuntimeKillArgs, RuntimeListArgs, RuntimeMetricsArgs,
    RuntimeMountsCommand, RuntimePauseArgs, RuntimePtyAttachArgs, RuntimePtyCommand,
    RuntimePtyCreateArgs, RuntimePtyGetArgs, RuntimePtyKillArgs, RuntimePtyListArgs,
    RuntimePtyResizeArgs, RuntimePtySendArgs, RuntimePtySignalArgs, RuntimeResumeArgs,
    RuntimeRunArgs, RuntimeServiceCommand, RuntimeServiceListArgs, RuntimeServiceRevokeArgs,
    RuntimeServiceWebUrlArgs, RuntimeShellArgs, RuntimeSshCommand, RuntimeTimeoutArgs,
};
use crate::ctx::AppContext;
use crate::output::{self, table};
use crate::terminal::pty;

pub async fn handle(ctx: &mut AppContext, cmd: RuntimeCommand) -> anyhow::Result<()> {
    ctx.require_api_key()?;
    match cmd {
        RuntimeCommand::Create(args) => create(ctx, args).await,
        RuntimeCommand::List(args) => list(ctx, args).await,
        RuntimeCommand::Get(args) => get(ctx, args).await,
        RuntimeCommand::Kill(args) => kill(ctx, args).await,
        RuntimeCommand::Pause(args) => pause(ctx, args).await,
        RuntimeCommand::Resume(args) => resume(ctx, args).await,
        RuntimeCommand::Metrics(args) => metrics(ctx, args).await,
        RuntimeCommand::Connect(args) => connect(ctx, args).await,
        RuntimeCommand::Service(args) => service(ctx, args.command).await,
        RuntimeCommand::Shell(args) => shell(ctx, args).await,
        RuntimeCommand::Exec(args) => exec(ctx, args).await,
        RuntimeCommand::Run(args) => run(ctx, args).await,
        RuntimeCommand::Context(args) => context(ctx, args.command).await,
        RuntimeCommand::CodeContext(args) => code_context(ctx, args.command).await,
        RuntimeCommand::Ssh(args) => ssh(ctx, args.command).await,
        RuntimeCommand::Files(args) => files(ctx, args.command).await,
        RuntimeCommand::Pty(args) => pty_command(ctx, args.command).await,
        RuntimeCommand::Git(args) => git(ctx, args.command).await,
        RuntimeCommand::Timeout(args) => set_timeout(ctx, args).await,
        RuntimeCommand::Fork(args) => fork(ctx, args).await,
        RuntimeCommand::Commands(args) => commands(ctx, args.command).await,
        RuntimeCommand::Browser(args) => browser(ctx, args.command).await,
        RuntimeCommand::Display(args) => display(ctx, args.command).await,
        RuntimeCommand::Desktop(args) => desktop(ctx, args.command).await,
        RuntimeCommand::Mounts(args) => mounts(ctx, args.command).await,
    }
}

// ---------------------------------------------------------------------------
// Create
// ---------------------------------------------------------------------------

async fn create(ctx: &AppContext, args: RuntimeCreateArgs) -> anyhow::Result<()> {
    // Parse KEY=VALUE env vars
    let env_vars = parse_env_vars(&args.env_vars)?;
    let metadata = parse_string_map_json(args.metadata.as_deref(), "metadata")?;

    // Mounts arrive as JSON specs; validate each before the create leaves.
    if args.mounts.len() > 8 {
        anyhow::bail!("at most 8 mounts per runtime, got {}", args.mounts.len());
    }
    let mut mounts = Vec::with_capacity(args.mounts.len());
    for raw in &args.mounts {
        let spec: crate::api::types::MountSpec =
            serde_json::from_str(raw).map_err(|e| anyhow::anyhow!("invalid --mount JSON: {e}"))?;
        crate::api::types::validate_mount(&spec)
            .map_err(|e| anyhow::anyhow!("invalid --mount: {e}"))?;
        mounts.push(spec);
    }

    let template = if args.snapshot.is_some() {
        String::new()
    } else {
        args.template.unwrap_or_else(|| "base-small".to_string())
    };

    if args.agent_id.is_some() {
        anyhow::bail!(
            "--agent-id was removed: agent binding is replaced by the `mcp` attach spec              (see the API changelog for the new create body)"
        );
    }
    let req = CreateRuntimeRequest {
        template,
        cloud: args.cloud,
        region: args.region,
        timeout: args.timeout,
        internet_access: args.internet_access,
        env_vars,
        metadata,
        providers: args.providers,
        network_policy_ids: args.network_policies,
        snapshot: args.snapshot,
        mounts,
    };

    let spinner = if ctx.output == crate::cli::OutputFormat::Table {
        Some(output::Spinner::new("Creating runtime…"))
    } else {
        None
    };

    let mut rt = ctx.api.runtime().create(req).await?;

    if args.wait {
        if let Some(ref sp) = spinner {
            sp.set_message(format!("Waiting for runtime {} to start…", rt.runtime_id));
        }
        let deadline = Instant::now() + Duration::from_secs(args.wait_timeout);
        rt = ctx
            .api
            .runtime()
            .wait_until_running(&rt.runtime_id, deadline)
            .await?;
    }

    if let Some(ref sp) = spinner {
        sp.finish_ok(format!("Runtime {} is ready", rt.runtime_id));
    }

    output::print_or_json(ctx.output, &rt, || {
        println!("{}", table::runtime_detail_table(&rt));
    });
    Ok(())
}

// ---------------------------------------------------------------------------
// List
// ---------------------------------------------------------------------------

async fn list(ctx: &AppContext, args: RuntimeListArgs) -> anyhow::Result<()> {
    let result = ctx.api.runtime().list(args.limit, args.offset).await?;
    output::print_or_json(ctx.output, &result, || {
        println!("{}", table::runtime_table(&result.runtimes));
        if let Some(total) = result.total {
            println!("  ({total} total)");
        }
    });
    Ok(())
}

// ---------------------------------------------------------------------------
// Get
// ---------------------------------------------------------------------------

async fn get(ctx: &AppContext, args: RuntimeGetArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "runtime")?;
    let rt = ctx.api.runtime().get(&args.id).await?;
    output::print_or_json(ctx.output, &rt, || {
        println!("{}", table::runtime_detail_table(&rt));
    });
    Ok(())
}

// ---------------------------------------------------------------------------
// Kill
// ---------------------------------------------------------------------------

async fn kill(ctx: &AppContext, args: RuntimeKillArgs) -> anyhow::Result<()> {
    if args.all {
        return kill_all(ctx, args.yes).await;
    }

    // `id` is guaranteed by clap's `required_unless_present = "all"`.
    let id = args.id.as_deref().unwrap();
    super::validate_resource_id(id, "runtime")?;
    if !args.yes {
        eprint!("Kill runtime {}? [y/N] ", id);
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }
    let resp = ctx.api.runtime().kill(id).await?;
    output::print_or_json(ctx.output, &resp, || {
        output::success(
            ctx.output,
            format!(
                "Runtime {} terminated: {}",
                id,
                resp.message.as_deref().unwrap_or("ok")
            ),
        );
    });
    Ok(())
}

async fn kill_all(ctx: &AppContext, yes: bool) -> anyhow::Result<()> {
    // Fetch up to 1 000 runtimes in one call; enough for any realistic workspace.
    let result = ctx.api.runtime().list(1000, 0).await?;
    if result.runtimes.is_empty() {
        println!("No runtimes found.");
        return Ok(());
    }

    println!("Found {} runtime(s):", result.runtimes.len());
    for rt in &result.runtimes {
        println!("  {}  {}", rt.runtime_id, rt.status);
    }

    if !yes {
        eprint!("Kill all {} runtime(s)? [y/N] ", result.runtimes.len());
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    let mut killed = 0usize;
    let mut errors = 0usize;
    for rt in &result.runtimes {
        match ctx.api.runtime().kill(&rt.runtime_id).await {
            Ok(_) => {
                output::success(ctx.output, format!("Runtime {} terminated", rt.runtime_id));
                killed += 1;
            }
            Err(e) => {
                eprintln!("error killing {}: {e}", rt.runtime_id);
                errors += 1;
            }
        }
    }

    if errors > 0 {
        anyhow::bail!("Killed {killed}, failed to kill {errors}");
    }
    output::success(ctx.output, format!("All {killed} runtime(s) terminated"));
    Ok(())
}

// ---------------------------------------------------------------------------
// Pause / Resume
// ---------------------------------------------------------------------------

async fn pause(ctx: &AppContext, args: RuntimePauseArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "runtime")?;
    let resp = ctx.api.runtime().pause(&args.id).await?;
    output::print_or_json(ctx.output, &resp, || {
        output::success(ctx.output, format!("Runtime {} paused", args.id));
    });
    Ok(())
}

async fn resume(ctx: &AppContext, args: RuntimeResumeArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "runtime")?;
    let resp = ctx.api.runtime().resume(&args.id).await?;
    output::print_or_json(ctx.output, &resp, || {
        output::success(ctx.output, format!("Runtime {} resumed", args.id));
    });
    Ok(())
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

async fn metrics(ctx: &AppContext, args: RuntimeMetricsArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "runtime")?;
    if let Some(interval_secs) = args.watch {
        loop {
            let m = ctx.api.runtime().metrics(&args.id).await?;
            // Clear screen for watch mode
            print!("\x1b[2J\x1b[H");
            output::print_or_json(ctx.output, &m, || {
                println!("{}", table::runtime_metrics_table(&m));
            });
            // Wait for the next poll interval or Ctrl-C — whichever comes first.
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(interval_secs)) => {}
                _ = tokio::signal::ctrl_c() => {
                    break;
                }
            }
        }
    } else {
        let m = ctx.api.runtime().metrics(&args.id).await?;
        output::print_or_json(ctx.output, &m, || {
            println!("{}", table::runtime_metrics_table(&m));
        });
    }
    Ok(())
}

async fn connect(ctx: &AppContext, args: RuntimeConnectArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "runtime")?;
    let resp = ctx.api.runtime().connect(&args.id).await?;
    output::print_or_json(ctx.output, &resp, || {
        output::kv(ctx.output, "runtime_id", &resp.runtime_id);
        output::kv(ctx.output, "status", resp.status.as_deref().unwrap_or("—"));
        output::kv(ctx.output, "domain", resp.domain.as_deref().unwrap_or("—"));
        output::kv(
            ctx.output,
            "message",
            resp.message.as_deref().unwrap_or("—"),
        );
    });
    Ok(())
}

async fn service(ctx: &AppContext, cmd: RuntimeServiceCommand) -> anyhow::Result<()> {
    match cmd {
        RuntimeServiceCommand::WebUrl(args) => service_web_url(ctx, args).await,
        RuntimeServiceCommand::List(args) => service_list(ctx, args).await,
        RuntimeServiceCommand::Revoke(args) => service_revoke(ctx, args).await,
    }
}

async fn service_web_url(ctx: &AppContext, args: RuntimeServiceWebUrlArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "runtime")?;
    let resp = ctx
        .api
        .runtime_service()
        .open(
            &args.id,
            args.port,
            args.expires_in,
            args.public,
            args.rotate_token,
        )
        .await?;
    output::print_or_json(ctx.output, &resp, || {
        println!("{}", resp.url);
        if let Some(ref token) = resp.token {
            eprintln!("token: {token}");
        }
        if let Some(ref browser) = resp.browser_url {
            if browser != &resp.url {
                eprintln!("browser: {browser}");
            }
        }
    });
    Ok(())
}

async fn service_list(ctx: &AppContext, args: RuntimeServiceListArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "runtime")?;
    let resp = ctx.api.runtime_service().list(&args.id).await?;
    output::print_or_json(ctx.output, &resp, || {
        if resp.services.is_empty() {
            println!("No active web services");
            return;
        }
        for s in &resp.services {
            let access = if s.is_public { "public" } else { "private" };
            let expires = s.expires_at.as_deref().unwrap_or("—");
            println!("{:<6} {:<8} {:<24} {}", s.port, access, expires, s.url);
        }
    });
    Ok(())
}

async fn service_revoke(ctx: &AppContext, args: RuntimeServiceRevokeArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "runtime")?;
    ctx.api
        .runtime_service()
        .revoke(&args.id, args.port)
        .await?;
    output::success(
        ctx.output,
        format!("Revoked web service for port {} on {}", args.port, args.id),
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Shell (interactive WebSocket terminal)
// ---------------------------------------------------------------------------

async fn shell(ctx: &AppContext, args: RuntimeShellArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "runtime")?;
    use secrecy::ExposeSecret;
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;

    let (cols, rows) = pty::terminal_size();
    let ws_url = ctx
        .api
        .terminal_ws_url(&args.id, &args.shell, args.project_id.as_deref());

    output::info(ctx.output, format!("Connecting to runtime {}…", args.id));

    // Build the WebSocket request with the Authorization header.
    let mut request = ws_url.as_str().into_client_request()?;
    request.headers_mut().insert(
        "Authorization",
        format!("Bearer {}", ctx.api.api_key().expose_secret())
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid auth header: {e}"))?,
    );

    let (ws_stream, _response) = connect_async(request)
        .await
        .map_err(|e| anyhow::anyhow!("WebSocket connect failed: {e}"))?;

    let result = pty::run_session(ws_stream, cols, rows).await?;

    if let Some(code) = result.exit_code {
        if code != 0 {
            std::process::exit(code);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Exec (non-interactive command execution)
// ---------------------------------------------------------------------------

async fn exec(ctx: &AppContext, args: RuntimeExecArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "runtime")?;
    if args.command.is_empty() {
        anyhow::bail!("at least one command argument is required");
    }
    let env_vars = parse_env_vars(&args.env_vars)?;
    // Backend expects: command (executable string), args (rest), working_dir, environment, timeout (ms)
    let (cmd_bin, cmd_args) = args.command.split_first().unwrap();
    if args.background && args.stream {
        anyhow::bail!("--background cannot be combined with --stream; attach with the command pid");
    }
    let mut payload = serde_json::json!({
        "command": cmd_bin,
        "args": cmd_args,
        "working_dir": args.workdir,
        "environment": env_vars,
        "timeout": args.timeout * 1000,
    });
    if args.background {
        payload["background"] = serde_json::json!(true);
    }
    if args.stream {
        return exec_stream(ctx, &args.id, &payload).await;
    }

    let resp = ctx
        .api
        .runtime()
        .exec_command(&args.id, &payload, guest_deadline_budget(args.timeout))
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    output::print_or_json(ctx.output, &resp, || {
        println!(
            "{}",
            serde_json::to_string_pretty(&resp).unwrap_or_default()
        );
    });
    // Propagate the remote process exit code so shell pipelines and scripts
    // can observe a non-zero result (mirrors what the stream path already does).
    if let Some(code) = resp.get("exit_code").and_then(|v| v.as_i64()) {
        if code != 0 {
            std::process::exit(code as i32);
        }
    }
    Ok(())
}

async fn exec_stream(
    ctx: &AppContext,
    runtime_id: &str,
    payload: &serde_json::Value,
) -> anyhow::Result<()> {
    let response = ctx
        .api
        .runtime()
        .exec_command_stream(runtime_id, payload)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    consume_command_stream(response).await
}

/// Render a live command SSE stream: stdout to stdout, stderr to stderr,
/// `end` to the process exit code, `error` to a failure. Shared by
/// `exec --stream` and `commands attach` — both consume the same
/// `live_command_frames` wire format.
/// One chunk from an SSE byte stream under the per-read deadline.
///
/// Only command streams get this treatment — the server pings them every
/// 15 s, so `STREAM_READ_GRACE` fires only on a genuinely stalled connection
/// (the SDKs' read-grace). `files watch`/`pty attach` never ping — silence is
/// their normal state — so those consumers read without a deadline.
async fn next_sse_chunk(
    stream: &mut (impl futures_util::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin),
) -> anyhow::Result<Option<Result<bytes::Bytes, reqwest::Error>>> {
    use futures_util::StreamExt;
    tokio::time::timeout(crate::api::STREAM_READ_GRACE, stream.next())
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "stream stalled — no frames for {}s",
                crate::api::STREAM_READ_GRACE.as_secs()
            )
        })
}

async fn consume_command_stream(response: reqwest::Response) -> anyhow::Result<()> {
    use std::io::Write;

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut exit_code = 0i32;

    while let Some(chunk) = next_sse_chunk(&mut stream).await? {
        let chunk = chunk.map_err(|e| anyhow::anyhow!("stream command response: {e}"))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(separator_index) = buffer.find("\n\n") {
            let event = buffer[..separator_index].to_string();
            buffer.drain(..separator_index + 2);

            for line in event.lines() {
                let Some(data) = line.strip_prefix("data:") else {
                    continue;
                };
                let payload: serde_json::Value = serde_json::from_str(data.trim())?;
                match payload.get("type").and_then(|value| value.as_str()) {
                    Some("stdout") => {
                        if let Some(data) = payload.get("data").and_then(|value| value.as_str()) {
                            print!("{data}");
                            std::io::stdout().flush()?;
                        }
                    }
                    Some("stderr") => {
                        if let Some(data) = payload.get("data").and_then(|value| value.as_str()) {
                            eprint!("{data}");
                            std::io::stderr().flush()?;
                        }
                    }
                    Some("end") => {
                        exit_code = payload
                            .get("exit_code")
                            .and_then(|value| value.as_i64())
                            .unwrap_or_default() as i32;
                    }
                    Some("error") => {
                        let message = payload
                            .get("message")
                            .and_then(|value| value.as_str())
                            .unwrap_or("runtime command stream failed");
                        anyhow::bail!(message.to_string());
                    }
                    _ => {}
                }
            }
        }
    }

    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Run (upload + execute a script)
// ---------------------------------------------------------------------------

async fn run(ctx: &AppContext, args: RuntimeRunArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "runtime")?;
    let env_vars = parse_env_vars(&args.env_vars)?;

    let (content, language) = match (&args.code, &args.script) {
        (Some(code), None) => (code.clone(), args.language.clone()),
        (None, Some(script)) => {
            if !script.is_file() {
                anyhow::bail!("script not found: {}", script.display());
            }
            let content = std::fs::read_to_string(script)?;

            // Infer language from file extension so the backend can route to
            // the right interpreter; --language overrides.
            let inferred = script
                .extension()
                .and_then(|e| e.to_str())
                .and_then(|ext| match ext {
                    "py" => Some("python"),
                    "js" => Some("javascript"),
                    "ts" => Some("typescript"),
                    "sh" | "bash" => Some("bash"),
                    _ => None,
                })
                .map(str::to_string);
            (content, args.language.clone().or(inferred))
        }
        _ => anyhow::bail!("provide a script path or --code"),
    };

    // Field names match the public Runtime run-code API contract.
    let mut payload = serde_json::json!({
        "code": content,
        "environment": env_vars,
        "timeout": args.timeout,
    });
    if let Some(lang) = language {
        payload["language"] = serde_json::Value::String(lang);
    }
    if let Some(context_id) = &args.context_id {
        payload["context_id"] = serde_json::Value::String(context_id.clone());
    }

    if args.stream {
        return run_code_stream(ctx, &args.id, &payload, args.timeout).await;
    }

    let resp = ctx
        .api
        .runtime()
        .run_code(&args.id, &payload, guest_deadline_budget(args.timeout))
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    output::print_or_json(ctx.output, &resp, || {
        println!(
            "{}",
            serde_json::to_string_pretty(&resp).unwrap_or_default()
        );
    });
    Ok(())
}

/// Render an incrementally streamed code execution.
///
/// stdout and stderr arrive as `text` fields (the command stream uses `data`),
/// and a failing cell surfaces as a structured `error` frame rather than a
/// non-zero exit code, so it is mapped onto a process exit code here.
async fn run_code_stream(
    ctx: &AppContext,
    runtime_id: &str,
    payload: &serde_json::Value,
    timeout_secs: u64,
) -> anyhow::Result<()> {
    use std::io::Write;

    let response = ctx
        .api
        .runtime()
        .run_code_stream(runtime_id, payload, guest_deadline_budget(timeout_secs))
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut failed = false;

    while let Some(chunk) = next_sse_chunk(&mut stream).await? {
        let chunk = chunk.map_err(|e| anyhow::anyhow!("stream code response: {e}"))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(separator_index) = buffer.find("\n\n") {
            let frame = buffer[..separator_index].to_string();
            buffer.drain(..separator_index + 2);

            for line in frame.lines() {
                let Some(data) = line.strip_prefix("data:") else {
                    continue;
                };
                let event: serde_json::Value = serde_json::from_str(data.trim())?;
                match event.get("type").and_then(|value| value.as_str()) {
                    Some("stdout") => {
                        if let Some(text) = event.get("text").and_then(|value| value.as_str()) {
                            print!("{text}");
                            std::io::stdout().flush()?;
                        }
                    }
                    Some("stderr") => {
                        if let Some(text) = event.get("text").and_then(|value| value.as_str()) {
                            eprint!("{text}");
                            std::io::stderr().flush()?;
                        }
                    }
                    Some("result") => {
                        // Rich results (charts, images, HTML) have no terminal
                        // representation, so only the plain-text form is shown.
                        if let Some(text) = event
                            .get("result")
                            .and_then(|value| value.get("text"))
                            .and_then(|value| value.as_str())
                        {
                            println!("{text}");
                            std::io::stdout().flush()?;
                        }
                    }
                    Some("error") => {
                        failed = true;
                        let error = event.get("error");
                        let name = error
                            .and_then(|value| value.get("name"))
                            .and_then(|value| value.as_str())
                            .unwrap_or("Error");
                        let value = error
                            .and_then(|value| value.get("value"))
                            .and_then(|value| value.as_str())
                            .unwrap_or_default();
                        eprintln!("{name}: {value}");
                        if let Some(traceback) = error
                            .and_then(|value| value.get("traceback"))
                            .and_then(|value| value.as_array())
                        {
                            for entry in traceback {
                                if let Some(entry) = entry.as_str() {
                                    eprintln!("{entry}");
                                }
                            }
                        }
                        std::io::stderr().flush()?;
                    }
                    _ => {}
                }
            }
        }
    }

    if failed {
        std::process::exit(1);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------

async fn context(ctx: &mut AppContext, cmd: RuntimeContextCommand) -> anyhow::Result<()> {
    use crate::cli::RuntimeContextCommand::*;
    match cmd {
        Show => {
            let profile = ctx.user_config.profile(&ctx.cfg.profile_name);
            let id = profile
                .and_then(|p| p.current_runtime_id.as_deref())
                .unwrap_or("(none)");
            println!("{id}");
        }
        Set(args) => {
            super::validate_resource_id(&args.id, "runtime")?;
            let profile = ctx.user_config.profile_mut(&ctx.cfg.profile_name);
            profile.current_runtime_id = Some(args.id.clone());
            ctx.user_config.save()?;
            output::success(ctx.output, format!("Context set to runtime {}", args.id));
        }
        Clear => {
            let profile = ctx.user_config.profile_mut(&ctx.cfg.profile_name);
            profile.current_runtime_id = None;
            ctx.user_config.save()?;
            output::success(ctx.output, "Context cleared");
        }
    }
    Ok(())
}

async fn code_context(ctx: &AppContext, cmd: RuntimeCodeContextCommand) -> anyhow::Result<()> {
    match cmd {
        RuntimeCodeContextCommand::Create(args) => {
            super::validate_resource_id(&args.runtime_id, "runtime")?;
            let resp = ctx
                .api
                .runtime()
                .create_code_context(&args.runtime_id, Some(&args.language), args.cwd.as_deref())
                .await?;
            output::print_or_json(ctx.output, &resp, || {
                output::kv(ctx.output, "context_id", &resp.context_id);
                output::kv(
                    ctx.output,
                    "language",
                    resp.language.as_deref().unwrap_or("—"),
                );
                output::kv(ctx.output, "cwd", resp.cwd.as_deref().unwrap_or("—"));
            });
            Ok(())
        }
        RuntimeCodeContextCommand::Get(args) => {
            super::validate_resource_id(&args.runtime_id, "runtime")?;
            let resp = ctx
                .api
                .runtime()
                .get_code_context(&args.runtime_id, &args.context_id)
                .await?;
            output::print_or_json(ctx.output, &resp, || {
                output::kv(ctx.output, "context_id", &resp.context_id);
                output::kv(
                    ctx.output,
                    "language",
                    resp.language.as_deref().unwrap_or("—"),
                );
                output::kv(ctx.output, "cwd", resp.cwd.as_deref().unwrap_or("—"));
            });
            Ok(())
        }
        RuntimeCodeContextCommand::Delete(args) => {
            super::validate_resource_id(&args.runtime_id, "runtime")?;
            let resp = ctx
                .api
                .runtime()
                .delete_code_context(&args.runtime_id, &args.context_id)
                .await?;
            output::print_or_json(ctx.output, &resp, || {
                output::success(
                    ctx.output,
                    resp.message
                        .as_deref()
                        .unwrap_or("Context deleted successfully"),
                );
            });
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// SSH
// ---------------------------------------------------------------------------

async fn ssh(ctx: &AppContext, cmd: RuntimeSshCommand) -> anyhow::Result<()> {
    match cmd {
        RuntimeSshCommand::Enable(args) => {
            super::validate_resource_id(&args.runtime_id, "runtime")?;
            let resp = ctx
                .api
                .runtime()
                .enable_ssh(&args.runtime_id, args.regenerate_keys)
                .await?;
            output::print_or_json(ctx.output, &resp, || {
                output::success(ctx.output, resp.message.as_deref().unwrap_or("SSH enabled"));
                if let Some(cmd) = resp.connect_cmd.as_deref() {
                    println!("{cmd}");
                }
                if let Some(private_key) = resp.private_key.as_deref() {
                    println!("\nPrivate key:\n{private_key}");
                }
            });
            Ok(())
        }
        RuntimeSshCommand::Disable(args) => {
            super::validate_resource_id(&args.runtime_id, "runtime")?;
            let resp = ctx.api.runtime().disable_ssh(&args.runtime_id).await?;
            output::print_or_json(ctx.output, &resp, || {
                output::success(
                    ctx.output,
                    resp.message.as_deref().unwrap_or("SSH disabled"),
                );
            });
            Ok(())
        }
        RuntimeSshCommand::Status(args) => {
            super::validate_resource_id(&args.runtime_id, "runtime")?;
            let resp = ctx.api.runtime().ssh_status(&args.runtime_id).await?;
            output::print_or_json(ctx.output, &resp, || {
                println!("runtime_id\t{}", resp.runtime_id);
                println!("enabled\t{}", resp.enabled);
                println!(
                    "port\t{}",
                    resp.port.map_or("-".to_string(), |port| port.to_string())
                );
                println!("username\t{}", resp.username.as_deref().unwrap_or("-"));
                println!(
                    "daemon_running\t{}",
                    resp.daemon_running
                        .map_or("-".to_string(), |running| running.to_string())
                );
            });
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Files
// ---------------------------------------------------------------------------

async fn files(ctx: &AppContext, cmd: RuntimeFilesCommand) -> anyhow::Result<()> {
    match cmd {
        RuntimeFilesCommand::List(args) => files_list(ctx, args).await,
        RuntimeFilesCommand::Cat(args) => files_cat(ctx, args).await,
        RuntimeFilesCommand::Write(args) => files_write(ctx, args).await,
        RuntimeFilesCommand::WriteMany(args) => files_write_many(ctx, args).await,
        RuntimeFilesCommand::Info(args) => files_info(ctx, args).await,
        RuntimeFilesCommand::Upload(args) => files_upload(ctx, args).await,
        RuntimeFilesCommand::Download(args) => files_download(ctx, args).await,
        RuntimeFilesCommand::Delete(args) => files_delete(ctx, args).await,
        RuntimeFilesCommand::Mkdir(args) => files_mkdir(ctx, args).await,
        RuntimeFilesCommand::Chmod(args) => files_chmod(ctx, args).await,
        RuntimeFilesCommand::Move(args) => files_move(ctx, args).await,
        RuntimeFilesCommand::Copy(args) => files_copy(ctx, args).await,
        RuntimeFilesCommand::Chown(args) => files_chown(ctx, args).await,
        RuntimeFilesCommand::Watch(args) => files_watch(ctx, args).await,
        RuntimeFilesCommand::Find(args) => files_find(ctx, args).await,
        RuntimeFilesCommand::Replace(args) => files_replace(ctx, args).await,
    }
}

async fn files_list(ctx: &AppContext, args: RuntimeFilesListArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let resp = ctx
        .api
        .runtime_files()
        .list(&args.runtime_id, &args.path)
        .await?;
    output::print_or_json(ctx.output, &resp, || {
        for entry in &resp.files {
            let kind = if entry.is_dir { "d" } else { "-" };
            let size = entry.size.to_string();
            let path = entry.path.as_deref().unwrap_or(&entry.name);
            println!("{kind}  {:<10}  {}", size, path);
        }
    });
    Ok(())
}

async fn files_cat(ctx: &AppContext, args: RuntimeFilesCatArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let resp = ctx
        .api
        .runtime_files()
        .read(&args.runtime_id, &args.path)
        .await?;
    print!("{}", resp.content);
    Ok(())
}

async fn files_write(ctx: &AppContext, args: RuntimeFilesWriteArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let content = match args.content {
        Some(c) => c,
        None => {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            buf
        }
    };
    let resp = ctx
        .api
        .runtime_files()
        .write(&args.runtime_id, &args.path, &content)
        .await?;
    output::print_or_json(ctx.output, &resp, || {
        output::success(ctx.output, format!("Written: {}", args.path));
    });
    Ok(())
}

async fn files_upload(ctx: &AppContext, args: RuntimeFilesUploadArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    if !args.local.exists() {
        anyhow::bail!("local path not found: {}", args.local.display());
    }
    let resp = ctx
        .api
        .runtime_files()
        .upload(
            &args.runtime_id,
            &args.local,
            &args.remote,
            args.user.as_deref(),
            args.mode.as_deref(),
        )
        .await?;
    output::print_or_json(ctx.output, &resp, || {
        output::success(
            ctx.output,
            format!("Uploaded {} → {}", args.local.display(), args.remote),
        );
    });
    Ok(())
}

async fn files_write_many(ctx: &AppContext, args: RuntimeFilesWriteManyArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let files = parse_file_mappings(&args.files)?;
    let resp = ctx
        .api
        .runtime_files()
        .write_many(&args.runtime_id, &files, args.user.as_deref())
        .await?;
    output::print_or_json(ctx.output, &resp, || {
        for entry in &resp {
            if let Some(error) = entry.error.as_deref() {
                output::warn(&format!("{}: {}", entry.path, error));
            } else {
                output::success(ctx.output, format!("Written: {}", entry.path));
            }
        }
    });
    Ok(())
}

async fn files_info(ctx: &AppContext, args: RuntimeFilesInfoArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let resp = ctx
        .api
        .runtime_files()
        .info(&args.runtime_id, &args.path)
        .await?;
    output::print_or_json(ctx.output, &resp, || {
        if !resp.exists {
            output::warn("path does not exist");
            return;
        }
        if let Some(info) = &resp.info {
            output::kv(ctx.output, "name", &info.name);
            output::kv(ctx.output, "path", info.path.as_deref().unwrap_or("—"));
            output::kv(ctx.output, "size", &info.size.to_string());
            output::kv(ctx.output, "is_dir", &info.is_dir.to_string());
            output::kv(ctx.output, "mode", info.mode.as_deref().unwrap_or("—"));
            output::kv(
                ctx.output,
                "permissions",
                info.permissions.as_deref().unwrap_or("—"),
            );
            output::kv(
                ctx.output,
                "modified_at",
                info.modified_at.as_deref().unwrap_or("—"),
            );
        }
    });
    Ok(())
}

async fn files_download(ctx: &AppContext, args: RuntimeFilesDownloadArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    ctx.api
        .runtime_files()
        .download(&args.runtime_id, &args.remote, &args.local)
        .await?;
    output::success(
        ctx.output,
        format!("Downloaded {} → {}", args.remote, args.local.display()),
    );
    Ok(())
}

async fn files_delete(ctx: &AppContext, args: RuntimeFilesDeleteArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    if !args.yes {
        eprint!("Delete {}? [y/N] ", args.path);
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }
    let resp = ctx
        .api
        .runtime_files()
        .delete(&args.runtime_id, &args.path, args.recursive)
        .await?;
    output::print_or_json(ctx.output, &resp, || {
        output::success(ctx.output, format!("Deleted: {}", args.path));
    });
    Ok(())
}

async fn files_mkdir(ctx: &AppContext, args: RuntimeFilesMkdirArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let resp = ctx
        .api
        .runtime_files()
        .mkdir(
            &args.runtime_id,
            &args.path,
            !args.no_recursive,
            args.mode.as_deref(),
        )
        .await?;
    output::print_or_json(ctx.output, &resp, || {
        output::success(ctx.output, format!("Created directory: {}", args.path));
    });
    Ok(())
}

async fn files_chmod(ctx: &AppContext, args: RuntimeFilesChmodArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let resp = ctx
        .api
        .runtime_files()
        .chmod(&args.runtime_id, &args.path, &args.mode)
        .await?;
    output::print_or_json(ctx.output, &resp, || {
        output::success(ctx.output, format!("chmod {} {}", args.mode, args.path));
    });
    Ok(())
}

async fn files_move(ctx: &AppContext, args: RuntimeFilesMoveArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let resp = ctx
        .api
        .runtime_files()
        .move_path(
            &args.runtime_id,
            &args.source,
            &args.destination,
            args.overwrite,
        )
        .await?;
    output::print_or_json(ctx.output, &resp, || {
        output::success(
            ctx.output,
            format!("Moved {} -> {}", args.source, args.destination),
        );
    });
    Ok(())
}

async fn files_copy(ctx: &AppContext, args: RuntimeFilesCopyArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let resp = ctx
        .api
        .runtime_files()
        .copy_path(
            &args.runtime_id,
            &args.source,
            &args.destination,
            args.recursive,
            args.overwrite,
        )
        .await?;
    output::print_or_json(ctx.output, &resp, || {
        output::success(
            ctx.output,
            format!("Copied {} -> {}", args.source, args.destination),
        );
    });
    Ok(())
}

async fn files_chown(ctx: &AppContext, args: RuntimeFilesChownArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let resp = ctx
        .api
        .runtime_files()
        .chown(
            &args.runtime_id,
            &args.path,
            args.user.as_deref(),
            args.group.as_deref(),
            args.recursive,
        )
        .await?;
    output::print_or_json(ctx.output, &resp, || {
        let owner = match (args.user.as_deref(), args.group.as_deref()) {
            (Some(user), Some(group)) => format!("{user}:{group}"),
            (Some(user), None) => user.to_string(),
            (None, Some(group)) => format!(":{group}"),
            (None, None) => String::new(),
        };
        output::success(ctx.output, format!("chown {} {}", owner, args.path));
    });
    Ok(())
}

/// Stream inotify events from a runtime directory until interrupted.
///
/// Runs until the user cancels or the runtime goes away; there is no natural
/// end to a watch, so a clean `Ok(())` on stream close is the correct result.
async fn files_watch(ctx: &AppContext, args: RuntimeFilesWatchArgs) -> anyhow::Result<()> {
    use futures_util::StreamExt;
    use std::io::Write;
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let json_mode = ctx.output == crate::cli::OutputFormat::Json;

    let response = ctx
        .api
        .runtime_files()
        .watch(&args.runtime_id, &args.path, args.recursive)
        .await?;

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    // No per-read deadline — the watch stream never pings; silence is its
    // normal state. The transport is unbounded once headers arrive.
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| anyhow::anyhow!("stream watch response: {e}"))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(separator_index) = buffer.find("\n\n") {
            let frame = buffer[..separator_index].to_string();
            buffer.drain(..separator_index + 2);

            for line in frame.lines() {
                let Some(data) = line.strip_prefix("data:") else {
                    continue;
                };
                let payload: serde_json::Value = serde_json::from_str(data.trim())?;
                if json_mode {
                    println!("{payload}");
                    std::io::stdout().flush()?;
                    continue;
                }

                let event_type = payload
                    .get("type")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown");
                if event_type == "error" {
                    let message = payload
                        .get("message")
                        .and_then(|value| value.as_str())
                        .unwrap_or("runtime watch stream failed");
                    anyhow::bail!(message.to_string());
                }
                let path = payload
                    .get("path")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                match payload.get("new_path").and_then(|value| value.as_str()) {
                    Some(new_path) if !new_path.is_empty() => {
                        println!("{event_type:<8} {path} -> {new_path}")
                    }
                    _ => println!("{event_type:<8} {path}"),
                }
                std::io::stdout().flush()?;
            }
        }
    }

    Ok(())
}

/// Find files by name glob and/or content pattern, natively inside the guest.
async fn files_find(ctx: &AppContext, args: RuntimeFilesFindArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let resp = ctx
        .api
        .runtime_files()
        .find(
            &args.runtime_id,
            &args.path,
            args.pattern.as_deref(),
            args.glob.as_deref(),
            args.regex,
            args.case_sensitive,
            args.include_hidden,
            args.max_results,
            args.max_depth,
        )
        .await?;
    output::print_or_json(ctx.output, &resp, || {
        if resp.matches.is_empty() {
            output::success(ctx.output, "No matches".to_string());
            return;
        }
        for entry in &resp.matches {
            match entry.line.unwrap_or(0) {
                0 => println!("{}", entry.path),
                line => println!(
                    "{}:{}: {}",
                    entry.path,
                    line,
                    entry.content.as_deref().unwrap_or_default()
                ),
            }
        }
        if resp.truncated.unwrap_or(false) {
            output::success(
                ctx.output,
                format!(
                    "{} matches shown (truncated; raise --max-results for more)",
                    resp.matches.len()
                ),
            );
        }
    });
    Ok(())
}

/// Replace a pattern across every matching file, atomically per file.
async fn files_replace(ctx: &AppContext, args: RuntimeFilesReplaceArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let resp = ctx
        .api
        .runtime_files()
        .search_replace(
            &args.runtime_id,
            &args.path,
            &args.pattern,
            &args.replacement,
            args.glob.as_deref(),
            args.regex,
            args.case_sensitive,
            args.include_hidden,
            args.max_depth,
            args.dry_run,
        )
        .await?;
    output::print_or_json(ctx.output, &resp, || {
        for entry in &resp.files {
            println!("{}: {}", entry.path, entry.replacements.unwrap_or(0));
        }
        let verb = if args.dry_run {
            "would replace"
        } else {
            "replaced"
        };
        output::success(
            ctx.output,
            format!(
                "{} {} occurrence(s) across {} file(s)",
                verb,
                resp.total_replacements.unwrap_or(0),
                resp.files.len()
            ),
        );
    });
    Ok(())
}

// ---------------------------------------------------------------------------
// PTY sessions
// ---------------------------------------------------------------------------

async fn pty_command(ctx: &AppContext, cmd: RuntimePtyCommand) -> anyhow::Result<()> {
    match cmd {
        RuntimePtyCommand::Create(args) => pty_create(ctx, args).await,
        RuntimePtyCommand::List(args) => pty_list(ctx, args).await,
        RuntimePtyCommand::Get(args) => pty_get(ctx, args).await,
        RuntimePtyCommand::Send(args) => pty_send(ctx, args).await,
        RuntimePtyCommand::Resize(args) => pty_resize(ctx, args).await,
        RuntimePtyCommand::Signal(args) => pty_signal(ctx, args).await,
        RuntimePtyCommand::Attach(args) => pty_attach(ctx, args).await,
        RuntimePtyCommand::Kill(args) => pty_kill(ctx, args).await,
    }
}

fn print_pty_session(ctx: &AppContext, session: &crate::api::runtime_pty::PtySession) {
    output::kv(ctx.output, "session_id", &session.session_id);
    output::kv(ctx.output, "status", &session.status);
    output::kv(ctx.output, "pid", &session.pid.to_string());
    output::kv(ctx.output, "shell", &session.shell);
    output::kv(ctx.output, "working_dir", &session.working_dir);
    output::kv(
        ctx.output,
        "size",
        &format!("{}x{}", session.cols, session.rows),
    );
    if session.status == "exited" {
        output::kv(ctx.output, "exit_code", &session.exit_code.to_string());
    }
    output::kv(
        ctx.output,
        "created_at",
        session.created_at.as_deref().unwrap_or("-"),
    );
}

async fn pty_create(ctx: &AppContext, args: RuntimePtyCreateArgs) -> anyhow::Result<()> {
    use crate::api::runtime_pty::PtyCreateParams;

    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let params = PtyCreateParams {
        shell: args.shell,
        args: args.args,
        working_dir: args.workdir,
        environment: parse_env_vars(&args.env_vars)?,
        cols: args.cols,
        rows: args.rows,
    };
    let resp = ctx
        .api
        .runtime_pty()
        .create(&args.runtime_id, &params)
        .await?;
    output::print_or_json(ctx.output, &resp, || print_pty_session(ctx, &resp));
    Ok(())
}

async fn pty_list(ctx: &AppContext, args: RuntimePtyListArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let resp = ctx.api.runtime_pty().list(&args.runtime_id).await?;
    output::print_or_json(ctx.output, &resp, || {
        if resp.sessions.is_empty() {
            output::info(ctx.output, "No PTY sessions");
            return;
        }
        println!("{}", table::pty_session_table(&resp.sessions));
    });
    Ok(())
}

async fn pty_get(ctx: &AppContext, args: RuntimePtyGetArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let resp = ctx
        .api
        .runtime_pty()
        .get(&args.runtime_id, &args.session_id)
        .await?;
    output::print_or_json(ctx.output, &resp, || print_pty_session(ctx, &resp));
    Ok(())
}

async fn pty_send(ctx: &AppContext, args: RuntimePtySendArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    // Reading raw bytes from stdin keeps control sequences and non-UTF-8 input intact.
    let mut data = match args.data {
        Some(text) => text.into_bytes(),
        None => {
            use std::io::Read;
            let mut buf = Vec::new();
            std::io::stdin().read_to_end(&mut buf)?;
            buf
        }
    };
    if !args.no_newline && !data.ends_with(b"\n") {
        data.push(b'\n');
    }
    let resp = ctx
        .api
        .runtime_pty()
        .send_input(&args.runtime_id, &args.session_id, &data)
        .await?;
    output::print_or_json(ctx.output, &resp, || {
        output::success(
            ctx.output,
            format!("Sent {} bytes", resp.bytes_written.unwrap_or_default()),
        );
    });
    Ok(())
}

async fn pty_resize(ctx: &AppContext, args: RuntimePtyResizeArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let resp = ctx
        .api
        .runtime_pty()
        .resize(&args.runtime_id, &args.session_id, args.cols, args.rows)
        .await?;
    output::print_or_json(ctx.output, &resp, || {
        output::success(
            ctx.output,
            format!("Resized to {}x{}", args.cols, args.rows),
        );
    });
    Ok(())
}

async fn pty_signal(ctx: &AppContext, args: RuntimePtySignalArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let resp = ctx
        .api
        .runtime_pty()
        .signal(&args.runtime_id, &args.session_id, &args.signal)
        .await?;
    output::print_or_json(ctx.output, &resp, || {
        output::success(
            ctx.output,
            format!("Sent SIG{}", args.signal.to_uppercase()),
        );
    });
    Ok(())
}

/// Replay a session's scrollback and then follow its live output.
///
/// Output is written to stdout verbatim as raw bytes so terminal escape
/// sequences produced inside the runtime render correctly.
async fn pty_attach(ctx: &AppContext, args: RuntimePtyAttachArgs) -> anyhow::Result<()> {
    use base64::Engine as _;
    use futures_util::StreamExt;
    use std::io::Write;

    let response = ctx
        .api
        .runtime_pty()
        .stream(&args.runtime_id, &args.session_id)
        .await?;

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut exit_code = 0i32;

    // Same rule as `files watch` — no pings on this stream, so idle time is
    // legitimate and reads are not deadline'd.
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| anyhow::anyhow!("stream pty output: {e}"))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(separator_index) = buffer.find("\n\n") {
            let frame = buffer[..separator_index].to_string();
            buffer.drain(..separator_index + 2);

            for line in frame.lines() {
                let Some(data) = line.strip_prefix("data:") else {
                    continue;
                };
                let payload: serde_json::Value = serde_json::from_str(data.trim())?;
                match payload.get("type").and_then(|value| value.as_str()) {
                    Some("data") => {
                        let Some(encoded) = payload.get("data").and_then(|value| value.as_str())
                        else {
                            continue;
                        };
                        let decoded = base64::engine::general_purpose::STANDARD
                            .decode(encoded)
                            .map_err(|e| anyhow::anyhow!("decode pty output: {e}"))?;
                        let mut stdout = std::io::stdout();
                        stdout.write_all(&decoded)?;
                        stdout.flush()?;
                    }
                    Some("exit") => {
                        exit_code = payload
                            .get("exit_code")
                            .and_then(|value| value.as_i64())
                            .unwrap_or_default() as i32;
                    }
                    Some("error") => {
                        let message = payload
                            .get("message")
                            .and_then(|value| value.as_str())
                            .unwrap_or("pty output stream failed");
                        anyhow::bail!(message.to_string());
                    }
                    _ => {}
                }
            }
        }
    }

    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

async fn pty_kill(ctx: &AppContext, args: RuntimePtyKillArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let resp = ctx
        .api
        .runtime_pty()
        .kill(&args.runtime_id, &args.session_id)
        .await?;
    output::print_or_json(ctx.output, &resp, || {
        output::success(
            ctx.output,
            format!("Killed PTY session {}", args.session_id),
        );
    });
    Ok(())
}

// ---------------------------------------------------------------------------
// Git
// ---------------------------------------------------------------------------

async fn git(ctx: &AppContext, cmd: RuntimeGitCommand) -> anyhow::Result<()> {
    match cmd {
        RuntimeGitCommand::Clone(args) => git_clone(ctx, args).await,
        RuntimeGitCommand::Pull(args) => git_pull(ctx, args).await,
        RuntimeGitCommand::Status(args) => git_status(ctx, args).await,
        RuntimeGitCommand::Branch(args) => git_branch(ctx, args).await,
        RuntimeGitCommand::Checkout(args) => git_checkout(ctx, args).await,
        RuntimeGitCommand::Fetch(args) => git_fetch(ctx, args).await,
        RuntimeGitCommand::Add(args) => git_add(ctx, args).await,
        RuntimeGitCommand::Commit(args) => git_commit(ctx, args).await,
        RuntimeGitCommand::Push(args) => git_push(ctx, args).await,
        RuntimeGitCommand::BranchCreate(args) => git_branch_create(ctx, args).await,
        RuntimeGitCommand::BranchDelete(args) => git_branch_delete(ctx, args).await,
    }
}

/// Working directory of a runtime: the sandbox user's home, and where commands
/// run by default, so it is where a clone belongs when no destination is given.
const RUNTIME_WORKSPACE_DIR: &str = "/workspace";

/// Whether the git invocation itself failed.
///
/// `exit_code` is authoritative when the API sends one; `success` covers a
/// response that omits it. A response carrying neither is treated as a success
/// so a missing field can never turn into a spurious pipeline failure.
fn git_failed(result: &crate::api::runtime_git::GitOperationResult) -> bool {
    match (result.exit_code, result.success) {
        (Some(code), _) => code != 0,
        (None, Some(ok)) => !ok,
        (None, None) => false,
    }
}

/// Exit code to report for a git operation that ran and failed.
///
/// Git's own exit code passes through so a script can branch on it exactly as
/// it would running git locally. A negative code means the process was killed
/// by a signal rather than exiting; that has no meaningful code to forward, so
/// it collapses to 1.
fn git_exit_code(result: &crate::api::runtime_git::GitOperationResult) -> i32 {
    match result.exit_code {
        Some(code) if code > 0 => code,
        _ => 1,
    }
}

fn print_git_result(ctx: &AppContext, result: &crate::api::runtime_git::GitOperationResult) {
    output::print_or_json(ctx.output, result, || {
        if let Some(ref out) = result.stdout {
            if !out.is_empty() {
                print!("{out}");
            }
        }
        if let Some(ref err) = result.stderr {
            if !err.is_empty() {
                eprint!("{err}");
            }
        }
        if let Some(ref api_err) = result.error {
            if !api_err.is_empty() {
                eprint!("error: {api_err}");
            }
        }
        if git_failed(result) {
            let code = result.exit_code.unwrap_or(-1);
            output::warn(format!("git exited with code {code}"));
        }
    });
}

/// Print the result, then leave the process with git's own status.
///
/// A failed git command has to fail the CLI too, otherwise `git clone && make`
/// in a pipeline runs `make` against a checkout that was never created. This
/// matches how `runtime exec` forwards a remote command's exit code.
fn finish_git(
    ctx: &AppContext,
    result: &crate::api::runtime_git::GitOperationResult,
) -> anyhow::Result<()> {
    print_git_result(ctx, result);
    if git_failed(result) {
        // Git's output does not always end in a newline, and stdout is
        // line-buffered, so the tail of a diagnostic can still be sitting in the
        // buffer here. `exit` does not unwind and would drop it — precisely on
        // the path where the message matters most.
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        std::process::exit(git_exit_code(result));
    }
    Ok(())
}

/// Directory `git clone` would create for `url`, without a destination given.
///
/// Same rule as git: drop a trailing slash and a `.git` suffix, then take the
/// last path segment — `https://host/org/repo.git`, `git@host:org/repo.git`,
/// and `ssh://git@host/org/repo/` all yield `repo`. Returns `None` when nothing
/// usable is left, so the caller can ask for `--target-dir` instead of guessing.
fn repository_directory_name(url: &str) -> Option<&str> {
    let trimmed = url.trim_end_matches('/');
    let trimmed = trimmed.strip_suffix(".git").unwrap_or(trimmed);
    let name = trimmed.rsplit(['/', ':']).next()?;
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

async fn git_clone(ctx: &AppContext, args: RuntimeGitCloneArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    // Reject file:// URLs immediately — the runtime operates on remote repos only.
    if args.repo_url.starts_with("file://") {
        anyhow::bail!("file:// URLs are not supported; only remote git URLs are allowed");
    }
    let target = match args.target_dir.clone() {
        Some(dir) => dir,
        None => {
            let name = repository_directory_name(&args.repo_url).ok_or_else(|| {
                anyhow::anyhow!(
                    "cannot derive a directory name from {}; pass --target-dir",
                    args.repo_url
                )
            })?;
            format!("{RUNTIME_WORKSPACE_DIR}/{name}")
        }
    };
    let spinner = output::Spinner::new(format!("Cloning {} …", args.repo_url));
    let result = ctx
        .api
        .runtime_git()
        .clone(
            &args.runtime_id,
            &args.repo_url,
            &target,
            args.branch.as_deref(),
            args.depth,
            args.auth_token.as_deref(),
            args.timeout_seconds,
        )
        .await?;
    drop(spinner);
    finish_git(ctx, &result)
}

async fn git_pull(ctx: &AppContext, args: RuntimeGitPullArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let result = ctx
        .api
        .runtime_git()
        .pull(
            &args.runtime_id,
            &args.path,
            args.remote.as_deref(),
            args.branch.as_deref(),
            args.auth_token.as_deref(),
            args.timeout_seconds,
        )
        .await?;
    finish_git(ctx, &result)
}

async fn git_status(ctx: &AppContext, args: RuntimeGitStatusArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let result = ctx
        .api
        .runtime_git()
        .status(&args.runtime_id, &args.path)
        .await?;
    finish_git(ctx, &result)
}

async fn git_branch(ctx: &AppContext, args: RuntimeGitBranchArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let scope = if args.all {
        Some("all")
    } else if args.remote {
        Some("remote")
    } else {
        None
    };
    let result = ctx
        .api
        .runtime_git()
        .branches(&args.runtime_id, &args.path, scope)
        .await?;
    // `git branch` already marks the current branch with `*`, so its output is
    // printed as-is rather than re-rendered.
    finish_git(ctx, &result)
}

async fn git_checkout(ctx: &AppContext, args: RuntimeGitCheckoutArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let result = ctx
        .api
        .runtime_git()
        .checkout(&args.runtime_id, &args.path, &args.branch)
        .await?;
    finish_git(ctx, &result)
}

async fn git_fetch(ctx: &AppContext, args: RuntimeGitFetchArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let result = ctx
        .api
        .runtime_git()
        .fetch(
            &args.runtime_id,
            &args.path,
            args.remote.as_deref(),
            args.auth_token.as_deref(),
            args.timeout_seconds,
        )
        .await?;
    finish_git(ctx, &result)
}

async fn git_add(ctx: &AppContext, args: RuntimeGitAddArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let result = ctx
        .api
        .runtime_git()
        .add(&args.runtime_id, &args.path, &args.files)
        .await?;
    finish_git(ctx, &result)
}

async fn git_commit(ctx: &AppContext, args: RuntimeGitCommitArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let result = ctx
        .api
        .runtime_git()
        .commit(
            &args.runtime_id,
            &args.path,
            &args.message,
            args.author_name.as_deref(),
            args.author_email.as_deref(),
            args.allow_empty,
        )
        .await?;
    finish_git(ctx, &result)
}

async fn git_push(ctx: &AppContext, args: RuntimeGitPushArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let spinner = output::Spinner::new("Pushing …");
    let result = ctx
        .api
        .runtime_git()
        .push(
            &args.runtime_id,
            &args.path,
            args.remote.as_deref(),
            args.refspec.as_deref(),
            args.username.as_deref(),
            args.password.as_deref(),
            args.auth_token.as_deref(),
            args.timeout_seconds,
        )
        .await?;
    drop(spinner);
    finish_git(ctx, &result)
}

async fn git_branch_create(
    ctx: &AppContext,
    args: RuntimeGitBranchCreateArgs,
) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let result = ctx
        .api
        .runtime_git()
        .create_branch(
            &args.runtime_id,
            &args.path,
            &args.branch_name,
            args.start_point.as_deref(),
        )
        .await?;
    finish_git(ctx, &result)
}

async fn git_branch_delete(
    ctx: &AppContext,
    args: RuntimeGitBranchDeleteArgs,
) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let result = ctx
        .api
        .runtime_git()
        .delete_branch(&args.runtime_id, &args.path, &args.branch_name, args.force)
        .await?;
    finish_git(ctx, &result)
}

// ---------------------------------------------------------------------------
// Set timeout
// ---------------------------------------------------------------------------

async fn set_timeout(ctx: &AppContext, args: RuntimeTimeoutArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.id, "runtime")?;
    let resp = ctx
        .api
        .runtime()
        .set_timeout(&args.id, args.seconds)
        .await?;
    output::print_or_json(ctx.output, &resp, || {
        let mut msg = format!("Timeout for runtime {} set to {}s", args.id, args.seconds);
        if let Some(ref at) = resp.timeout_at {
            msg.push_str(&format!(" (timeout_at: {at})"));
        }
        output::success(ctx.output, msg);
    });
    Ok(())
}

// ---------------------------------------------------------------------------
// Fork
// ---------------------------------------------------------------------------

/// Shared live-fork / snapshot-fork body — same wire fields the SDKs send.
pub(super) fn fork_payload(
    count: u32,
    timeout_secs: Option<u64>,
    persist_snapshot: Option<bool>,
    name: Option<String>,
    env_vars: &[String],
    metadata: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let env = parse_env_vars(env_vars)?;
    let meta = parse_string_map_json(metadata, "metadata")?;
    let mut payload = serde_json::json!({ "count": count });
    if let Some(timeout) = timeout_secs {
        payload["timeout_seconds"] = serde_json::json!(timeout);
    }
    if let Some(persist) = persist_snapshot {
        payload["persist_snapshot"] = serde_json::json!(persist);
    }
    if let Some(name) = name {
        payload["name"] = serde_json::json!(name);
    }
    if !env.is_empty() {
        payload["env_vars"] = serde_json::json!(env);
    }
    if !meta.is_empty() {
        payload["metadata"] = serde_json::json!(meta);
    }
    Ok(payload)
}

/// Render a fork result: runtime rows for started children, one stderr line
/// per failed child. Returns the number that started.
pub(super) fn print_fork_response(
    ctx: &AppContext,
    response: &crate::api::types::ForkResponse,
) -> usize {
    let started = response.runtimes().count();
    output::print_or_json(ctx.output, response, || {
        if started > 0 {
            println!("{}", table::fork_children_table(response));
        }
        for (error, code) in response.errors() {
            eprintln!("child failed ({code}): {error}");
        }
    });
    started
}

async fn fork(ctx: &AppContext, args: RuntimeForkArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let payload = fork_payload(
        args.count,
        args.timeout,
        if args.persist_snapshot {
            Some(true)
        } else {
            None
        },
        args.name,
        &args.env_vars,
        args.metadata.as_deref(),
    )?;

    let spinner = if ctx.output == crate::cli::OutputFormat::Table {
        Some(output::Spinner::new(format!(
            "Forking {} runtime{}…",
            args.count,
            if args.count == 1 { "" } else { "s" }
        )))
    } else {
        None
    };

    let response = ctx
        .api
        .runtime()
        .fork(&args.runtime_id, &payload)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if let Some(ref sp) = spinner {
        sp.finish_ok(format!(
            "Forked {} runtime{}",
            response.runtimes().count(),
            if response.runtimes().count() == 1 {
                ""
            } else {
                "s"
            }
        ));
    }

    let started = print_fork_response(ctx, &response);
    if started == 0 {
        anyhow::bail!("fork produced no children");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Commands (background command management)
// ---------------------------------------------------------------------------

async fn commands(ctx: &AppContext, cmd: RuntimeCommandsCommand) -> anyhow::Result<()> {
    match cmd {
        RuntimeCommandsCommand::List(args) => commands_list(ctx, args).await,
        RuntimeCommandsCommand::Get(args) => commands_get(ctx, args).await,
        RuntimeCommandsCommand::Kill(args) => commands_kill(ctx, args).await,
        RuntimeCommandsCommand::Attach(args) => commands_attach(ctx, args).await,
    }
}

async fn commands_list(ctx: &AppContext, args: RuntimeCommandsIdArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let list = ctx
        .api
        .runtime()
        .list_commands(&args.runtime_id)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    output::print_or_json(ctx.output, &list, || {
        println!("{}", table::command_table(&list.commands));
    });
    Ok(())
}

async fn commands_get(ctx: &AppContext, args: RuntimeCommandsPidArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let cmd = ctx
        .api
        .runtime()
        .get_command(&args.runtime_id, args.pid)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    output::print_or_json(ctx.output, &cmd, || {
        println!("{}", table::command_detail_table(&cmd));
    });
    Ok(())
}

async fn commands_kill(ctx: &AppContext, args: RuntimeCommandsKillArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let cmd = ctx
        .api
        .runtime()
        .kill_command(&args.runtime_id, args.pid, args.signal.as_deref())
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    output::print_or_json(ctx.output, &cmd, || {
        let signal = args.signal.as_deref().unwrap_or("KILL");
        output::success(
            ctx.output,
            format!(
                "Sent {signal} to command {} on runtime {}",
                args.pid, args.runtime_id
            ),
        );
    });
    Ok(())
}

async fn commands_attach(ctx: &AppContext, args: RuntimeCommandsPidArgs) -> anyhow::Result<()> {
    super::validate_resource_id(&args.runtime_id, "runtime")?;
    let response = ctx
        .api
        .runtime()
        .command_stream(&args.runtime_id, args.pid)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    consume_command_stream(response).await
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_env_vars(pairs: &[String]) -> anyhow::Result<HashMap<String, String>> {
    pairs
        .iter()
        .map(|kv| {
            let eq = kv
                .find('=')
                .ok_or_else(|| anyhow::anyhow!("invalid env var '{}': expected KEY=VALUE", kv))?;
            let key = &kv[..eq];
            let val = &kv[eq + 1..];
            if key.is_empty() {
                anyhow::bail!("invalid env var '{}': key must not be empty", kv);
            }
            Ok((key.to_string(), val.to_string()))
        })
        .collect()
}

fn parse_string_map_json(
    raw: Option<&str>,
    field_name: &str,
) -> anyhow::Result<HashMap<String, String>> {
    let Some(raw) = raw else {
        return Ok(HashMap::new());
    };
    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| anyhow::anyhow!("invalid {field_name} JSON: {e}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("{field_name} must be a JSON object"))?;

    let mut parsed = HashMap::with_capacity(object.len());
    for (key, value) in object {
        let Some(string_value) = value.as_str() else {
            anyhow::bail!("{field_name}.{key} must be a string");
        };
        parsed.insert(key.clone(), string_value.to_string());
    }
    Ok(parsed)
}

fn parse_file_mappings(pairs: &[String]) -> anyhow::Result<Vec<(std::path::PathBuf, String)>> {
    pairs
        .iter()
        .map(|pair| {
            let separator = pair.find('=').ok_or_else(|| {
                anyhow::anyhow!("invalid --file '{}': expected LOCAL=REMOTE", pair)
            })?;
            let local = &pair[..separator];
            let remote = &pair[separator + 1..];
            if local.is_empty() || remote.is_empty() {
                anyhow::bail!(
                    "invalid --file '{}': local and remote paths are required",
                    pair
                );
            }
            Ok((std::path::PathBuf::from(local), remote.to_string()))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Browser
// ---------------------------------------------------------------------------

fn browser_target(spec: &str) -> serde_json::Value {
    // e3 → ref; "css:x" / "text:x" / "xpath:x" / "role:x" → selector; "x,y" → coords.
    if let Some(v) = spec.strip_prefix("css:") {
        serde_json::json!({"by": "css", "value": v})
    } else if let Some(v) = spec.strip_prefix("text:") {
        serde_json::json!({"by": "text", "value": v})
    } else if let Some(v) = spec.strip_prefix("xpath:") {
        serde_json::json!({"by": "xpath", "value": v})
    } else if let Some(v) = spec.strip_prefix("role:") {
        serde_json::json!({"by": "role", "value": v})
    } else if spec.contains(',') {
        let (x, y) = spec.split_once(',').unwrap();
        serde_json::json!({"by": "coords", "attrs": {"x": x.trim(), "y": y.trim()}})
    } else {
        serde_json::json!({"by": "ref", "value": spec})
    }
}

/// `action scroll --value dx,dy` — the server reads integer deltas; a float
/// on the wire is read as 0, so anything else is rejected up front.
fn scroll_deltas(value: &str) -> anyhow::Result<(i64, i64)> {
    let parsed = value
        .split_once(',')
        .and_then(|(dx, dy)| Some((dx.trim().parse().ok()?, dy.trim().parse().ok()?)));
    parsed.ok_or_else(|| anyhow::anyhow!("scroll --value must be integer `dx,dy`, got {value:?}"))
}

/// Ops the server's `window_op` accepts — one list drives validation, the
/// error message and the tests, so they can't drift apart.
const WINDOW_OPS: &[&str] = &[
    "focus",
    "close",
    "raise",
    "lower",
    "move",
    "resize",
    "maximize",
    "unmaximize",
    "minimize",
    "fullscreen",
    "unfullscreen",
];

async fn browser(ctx: &AppContext, cmd: RuntimeBrowserCommand) -> anyhow::Result<()> {
    match cmd {
        RuntimeBrowserCommand::Open(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let b = ctx
                .api
                .runtime_browser()
                .open(&a.id, a.headless, a.url.as_deref().unwrap_or(""))
                .await?;
            output::print_or_json(ctx.output, &b, || {
                println!("browser_id: {}", b.browser_id);
                println!("engine: {}", b.engine);
                println!("headless: {}", b.headless);
            });
        }
        RuntimeBrowserCommand::Close(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            ctx.api.runtime_browser().close(&a.id, &a.browser).await?;
            println!("closed {}", a.browser);
        }
        RuntimeBrowserCommand::List(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let r = ctx.api.runtime_browser().list(&a.id).await?;
            output::print_or_json(ctx.output, &r, || {
                for b in &r.browsers {
                    println!("{}\t{}\t{}", b.browser_id, b.engine, b.state);
                }
            });
        }
        RuntimeBrowserCommand::Status(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let s = ctx.api.runtime_browser().status(&a.id, &a.browser).await?;
            output::print_or_json(ctx.output, &s, || {
                println!("state: {}", s.state);
                for t in &s.targets {
                    println!(
                        "{}	{}	{}",
                        t["target_id"].as_str().unwrap_or(""),
                        t["title"].as_str().unwrap_or(""),
                        t["url"].as_str().unwrap_or(""),
                    );
                }
            });
        }
        RuntimeBrowserCommand::Navigate(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let r = ctx
                .api
                .runtime_browser()
                .navigate(&a.id, &a.browser, &a.url, &a.wait_until, a.timeout_ms)
                .await?;
            output::print_or_json(ctx.output, &r, || {
                println!("{} {}", r.status, r.url);
                println!("title: {}", r.title);
            });
        }
        RuntimeBrowserCommand::Command(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            match a.verb.as_str() {
                "back" | "forward" | "reload" | "activate" => {}
                _ => anyhow::bail!("verb must be back|forward|reload|activate"),
            }
            let r = ctx
                .api
                .runtime_browser()
                .command(
                    &a.id,
                    &a.browser,
                    &a.verb,
                    a.target.as_deref().unwrap_or(""),
                )
                .await?;
            output::print_or_json(ctx.output, &r, || println!("{}", r.url));
        }
        RuntimeBrowserCommand::Screenshot(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let mut body = serde_json::json!({
                "format": a.format,
                "full_page": a.full_page,
            });
            if let Some(t) = a.target.as_deref() {
                body["target_id"] = serde_json::json!(t);
            }
            if let Some(e) = a.element.as_deref() {
                body["element"] = browser_target(e);
            }
            if let Some(w) = a.wait_stable_ms {
                body["wait_stable_ms"] = serde_json::json!(w);
            }
            if let Some(t) = a.timeout_ms {
                body["timeout_ms"] = serde_json::json!(t);
            }
            let r = ctx
                .api
                .runtime_browser()
                .screenshot(&a.id, &a.browser, body)
                .await?;
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&r.data)
                .context("decode screenshot")?;
            tokio::fs::write(&a.out, &bytes).await?;
            println!("wrote {} ({}x{})", a.out, r.width, r.height);
        }
        RuntimeBrowserCommand::Snapshot(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let r = ctx
                .api
                .runtime_browser()
                .snapshot(
                    &a.id,
                    &a.browser,
                    a.target.as_deref().unwrap_or(""),
                    a.rev.as_deref().unwrap_or(""),
                )
                .await?;
            output::print_or_json(ctx.output, &r, || {
                for el in &r.elements {
                    println!(
                        "{}\t{}\t{}",
                        el["ref"].as_str().unwrap_or(""),
                        el["tag"].as_str().unwrap_or(""),
                        el["name"]
                            .as_str()
                            .or_else(|| el["text"].as_str())
                            .unwrap_or("")
                    );
                }
            });
        }
        RuntimeBrowserCommand::Evaluate(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let r = ctx
                .api
                .runtime_browser()
                .evaluate(&a.id, &a.browser, &a.expression, a.await_promise)
                .await?;
            if r.is_error {
                anyhow::bail!("expression threw: {}", r.error);
            }
            output::print_or_json(ctx.output, &r, || {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&r.value).unwrap_or_default()
                );
            });
        }
        RuntimeBrowserCommand::Action(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let mut body = serde_json::json!({
                "settle": !a.no_settle,
                "force": a.force,
                "target_id": a.tab.as_deref().unwrap_or(""),
            });
            if let Some(t) = a.target.as_deref() {
                body["target"] = browser_target(t);
            }
            match a.verb.as_str() {
                "click" | "dblclick" | "hover" | "scroll" => {}
                "fill" => {
                    if let Some(sr) = a.secret_ref.as_deref() {
                        body["secret_ref"] = serde_json::json!(sr);
                    } else {
                        body["text"] = serde_json::json!(a.value.as_deref().unwrap_or(""));
                    }
                }
                "type" => {
                    body["text"] = serde_json::json!(a.value.as_deref().unwrap_or(""));
                    if let Some(d) = a.delay_ms {
                        body["delay_ms"] = serde_json::json!(d);
                    }
                }
                "press" => body["key"] = serde_json::json!(a.value.as_deref().unwrap_or("")),
                "hotkey" => {
                    let keys: Vec<String> = a
                        .value
                        .as_deref()
                        .unwrap_or("")
                        .split(',')
                        .map(|k| k.trim().to_string())
                        .filter(|k| !k.is_empty())
                        .collect();
                    body["keys"] = serde_json::json!(keys);
                }
                _ => {
                    anyhow::bail!("verb must be click|dblclick|hover|fill|type|press|hotkey|scroll")
                }
            }
            if a.verb == "scroll" {
                if let Some(v) = a.value.as_deref() {
                    let (dx, dy) = scroll_deltas(v)?;
                    body["dx"] = serde_json::json!(dx);
                    body["dy"] = serde_json::json!(dy);
                }
            }
            let r = ctx
                .api
                .runtime_browser()
                .action(&a.id, &a.browser, &a.verb, body)
                .await?;
            output::print_or_json(ctx.output, &r, || {
                println!(
                    "ok={} navigated={} settled={}",
                    r.ok, r.navigated, r.settled
                );
            });
        }
        RuntimeBrowserCommand::Downloads(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let r = ctx
                .api
                .runtime_browser()
                .downloads(&a.id, &a.browser)
                .await?;
            output::print_or_json(ctx.output, &r, || {
                for d in &r.downloads {
                    println!(
                        "{}\t{}\t{}\t{}B",
                        d["id"].as_str().unwrap_or(""),
                        d["state"].as_str().unwrap_or(""),
                        d["filename"].as_str().unwrap_or(""),
                        d["bytes"].as_u64().unwrap_or(0),
                    );
                }
            });
        }
        RuntimeBrowserCommand::Download(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let r = ctx
                .api
                .runtime_browser()
                .download_get(&a.id, &a.browser, &a.download_id)
                .await?;
            let out = a
                .out
                .clone()
                .or_else(|| {
                    a.out.clone().or_else(|| {
                        r.entry
                            .get("filename")
                            .and_then(|f| f.as_str().map(str::to_string))
                    })
                })
                .unwrap_or_else(|| a.download_id.clone());
            let b64 = r
                .data
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("download {} has no payload yet", a.download_id))?;
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(b64)
                .context("decode download")?;
            tokio::fs::write(&out, &bytes).await?;
            println!("wrote {} ({}B)", out, bytes.len());
        }
        RuntimeBrowserCommand::Pdf(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let r = ctx
                .api
                .runtime_browser()
                .pdf(
                    &a.id,
                    &a.browser,
                    &a.target,
                    a.landscape,
                    a.print_background,
                )
                .await?;
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&r.data)
                .context("decode pdf")?;
            tokio::fs::write(&a.out, &bytes).await?;
            println!("wrote {} ({}B)", a.out, bytes.len());
        }
        RuntimeBrowserCommand::View(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let r = ctx
                .api
                .runtime_browser()
                .view(
                    &a.id,
                    &a.browser,
                    a.fps.unwrap_or(0),
                    a.quality.unwrap_or(0),
                    a.rotate_token,
                    a.role.as_deref().unwrap_or(""),
                )
                .await?;
            output::print_or_json(ctx.output, &r, || {
                let url = r.browser_url.as_deref().unwrap_or(&r.url);
                println!("{url}");
                if let Some(vt) = r.view_token.as_deref() {
                    println!("view token (append as &vt=): {vt}");
                }
            });
        }
        RuntimeBrowserCommand::Dialog(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            ctx.api
                .runtime_browser()
                .dialog(
                    &a.id,
                    &a.browser,
                    a.target.as_deref().unwrap_or(""),
                    a.accept,
                    a.prompt_text.as_deref().unwrap_or(""),
                    a.auto_dismiss,
                )
                .await?;
            println!("ok");
        }
        RuntimeBrowserCommand::Targets(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let r = ctx.api.runtime_browser().targets(&a.id, &a.browser).await?;
            output::print_or_json(ctx.output, &r, || {
                for t in r["targets"].as_array().cloned().unwrap_or_default() {
                    println!(
                        "{}\t{}\t{}",
                        t["target_id"].as_str().unwrap_or(""),
                        t["type"].as_str().unwrap_or(""),
                        t["title"].as_str().unwrap_or(""),
                    );
                }
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

async fn display(ctx: &AppContext, cmd: RuntimeDisplayCommand) -> anyhow::Result<()> {
    match cmd {
        RuntimeDisplayCommand::Enable(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let s = ctx
                .api
                .runtime_display()
                .enable(
                    &a.id,
                    a.width.unwrap_or(0),
                    a.height.unwrap_or(0),
                    &a.display,
                    a.wm.as_deref().unwrap_or(""),
                    a.mode.as_deref().unwrap_or(""),
                )
                .await?;
            output::print_or_json(ctx.output, &s, || {
                println!("enabled: {}", s.enabled);
                println!("{}x{} on {}", s.width, s.height, s.display);
            });
        }
        RuntimeDisplayCommand::Disable(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            ctx.api.runtime_display().disable(&a.id).await?;
            println!("display disabled");
        }
        RuntimeDisplayCommand::Status(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let s = ctx.api.runtime_display().status(&a.id).await?;
            output::print_or_json(ctx.output, &s, || {
                println!("enabled: {}", s.enabled);
                if s.enabled {
                    println!("{}x{} on {}", s.width, s.height, s.display);
                }
            });
        }
        RuntimeDisplayCommand::Info(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let s = ctx.api.runtime_display().info(&a.id).await?;
            output::print_or_json(ctx.output, &s, || {
                println!("enabled: {}", s.enabled);
                if s.enabled {
                    println!(
                        "{}x{} @{}dpi on {}{}",
                        s.width,
                        s.height,
                        s.dpi,
                        s.display,
                        if s.attached { " (attached)" } else { "" }
                    );
                }
            });
        }
        RuntimeDisplayCommand::Position(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let p = ctx.api.runtime_display().position(&a.id).await?;
            output::print_or_json(ctx.output, &p, || {
                println!("{},{}", p.x, p.y);
            });
        }
        RuntimeDisplayCommand::Keys(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let k = ctx.api.runtime_display().keys(&a.id).await?;
            output::print_or_json(ctx.output, &k, || {
                println!("{}", k.keys.join(" "));
            });
        }
        RuntimeDisplayCommand::Screenshot(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let mut body = serde_json::json!({
                "wait_stable_ms": a.wait_stable_ms.unwrap_or(0),
                "timeout_ms": a.timeout_ms.unwrap_or(0),
                "format": a.format,
            });
            if let Some(region) = a.region.as_deref() {
                let nums: Vec<i32> = region
                    .split(',')
                    .map(|s| s.trim().parse().context("region must be x,y,w,h"))
                    .collect::<anyhow::Result<_>>()?;
                anyhow::ensure!(nums.len() == 4, "region must be x,y,w,h");
                body["region"] =
                    serde_json::json!({"x": nums[0], "y": nums[1], "w": nums[2], "h": nums[3]});
            }
            if let Some(v) = a.scale_pct {
                body["scale_pct"] = v.into();
            }
            if let Some(v) = a.max_width {
                body["max_width"] = v.into();
            }
            if let Some(v) = a.max_height {
                body["max_height"] = v.into();
            }
            if let Some(v) = a.quality {
                body["quality"] = v.into();
            }
            if a.cursor {
                body["cursor"] = true.into();
            }
            if let Some(v) = a.if_rev {
                body["if_rev"] = v.into();
            }
            let r = ctx.api.runtime_display().screenshot(&a.id, body).await?;
            if r.unchanged {
                println!("unchanged (rev {})", r.rev);
            } else {
                use base64::Engine;
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(&r.data)
                    .context("decode screenshot")?;
                tokio::fs::write(&a.out, &bytes).await?;
                println!("wrote {} ({}x{}, rev {})", a.out, r.width, r.height, r.rev);
            }
        }
        RuntimeDisplayCommand::View(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let r = ctx
                .api
                .runtime_display()
                .view(
                    &a.id,
                    a.fps.unwrap_or(0),
                    a.quality.unwrap_or(0),
                    a.rotate_token,
                    a.role.as_deref().unwrap_or(""),
                    a.window.unwrap_or(0),
                )
                .await?;
            output::print_or_json(ctx.output, &r, || {
                let url = r.browser_url.as_deref().unwrap_or(&r.url);
                println!("{url}");
                if let Some(vt) = r.view_token.as_deref() {
                    println!("view token (append as &vt=): {vt}");
                }
            });
        }
        RuntimeDisplayCommand::StopView(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            ctx.api.runtime_display().stop_view(&a.id).await?;
            println!("view stopped");
        }
        RuntimeDisplayCommand::WaitStable(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let r = ctx
                .api
                .runtime_display()
                .wait_stable(&a.id, a.stable_ms, a.timeout_ms)
                .await?;
            output::print_or_json(ctx.output, &r, || {
                println!("settled: {} in {}ms", r.settled, r.waited_ms);
            });
        }
        RuntimeDisplayCommand::Mouse(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let mods = split_csv(a.modifiers.as_deref());
            let mut body = serde_json::json!({"button": a.button});
            if let Some(v) = a.x {
                body["x"] = v.into();
            }
            if let Some(v) = a.y {
                body["y"] = v.into();
            }
            if let Some(v) = a.click_count {
                body["click_count"] = v.into();
            }
            if let Some(v) = a.coordinate_space.as_deref() {
                body["coordinate_space"] = v.into();
            }
            if let Some(v) = mods.clone() {
                body["modifiers"] = v;
            }
            let path = match a.verb.as_str() {
                "click" => "/mouse/click",
                "down" => "/mouse/down",
                "up" => "/mouse/up",
                "drag" => {
                    body["to_x"] = a.to_x.context("drag needs --to-x")?.into();
                    body["to_y"] = a.to_y.context("drag needs --to-y")?.into();
                    if let Some(v) = a.steps {
                        body["steps"] = v.into();
                    }
                    if let Some(v) = a.duration_ms {
                        body["duration_ms"] = v.into();
                    }
                    "/mouse/drag"
                }
                "move" => "/mouse/move",
                "scroll" => {
                    body["dx"] = a.dx.unwrap_or(0).into();
                    body["dy"] = a.dy.unwrap_or(0).into();
                    "/mouse/scroll"
                }
                _ => anyhow::bail!("verb must be click|down|up|drag|move|scroll"),
            };
            ctx.api.runtime_display().input(&a.id, path, body).await?;
            println!("ok");
        }
        RuntimeDisplayCommand::Keyboard(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let mods = split_csv(a.modifiers.as_deref());
            let (path, body) = match a.verb.as_str() {
                "type" => {
                    let mut b = serde_json::json!({"text": a.value.as_deref().unwrap_or(""), "delay_ms": a.delay_ms.unwrap_or(0)});
                    if let Some(m) = a.mode.as_deref() {
                        b["mode"] = m.into();
                    }
                    ("/keyboard/type", b)
                }
                "press" => {
                    let mut b = serde_json::json!({"key": a.value.as_deref().unwrap_or("")});
                    if let Some(v) = mods.clone() {
                        b["modifiers"] = v;
                    }
                    ("/keyboard/press", b)
                }
                "hotkey" => {
                    let keys = split_csv(a.keys.as_deref().or(a.value.as_deref()))
                        .unwrap_or_else(|| serde_json::json!([]));
                    let mut b = serde_json::json!({"keys": keys});
                    if let Some(v) = mods.clone() {
                        b["modifiers"] = v;
                    }
                    ("/keyboard/hotkey", b)
                }
                _ => anyhow::bail!("verb must be type|press|hotkey"),
            };
            ctx.api.runtime_display().input(&a.id, path, body).await?;
            println!("ok");
        }
        RuntimeDisplayCommand::Clipboard(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            match a.verb.as_str() {
                "get" => {
                    let r = ctx
                        .api
                        .runtime_display()
                        .clipboard(&a.id, "get", "", "")
                        .await?;
                    if !r.text.is_empty() {
                        println!("{}", r.text);
                    }
                    if !r.png.is_empty() {
                        use base64::Engine;
                        let bytes = base64::engine::general_purpose::STANDARD
                            .decode(&r.png)
                            .context("decode clipboard png")?;
                        match a.png_out.as_deref() {
                            Some(path) => {
                                tokio::fs::write(path, &bytes).await?;
                                println!("wrote {path} ({} bytes)", bytes.len());
                            }
                            None => println!("image: {} bytes png", bytes.len()),
                        }
                    }
                }
                "set" => {
                    let png = match a.png.as_deref() {
                        Some(path) => {
                            use base64::Engine;
                            let bytes = tokio::fs::read(path)
                                .await
                                .with_context(|| format!("read {path}"))?;
                            base64::engine::general_purpose::STANDARD.encode(bytes)
                        }
                        None => String::new(),
                    };
                    ctx.api
                        .runtime_display()
                        .clipboard(&a.id, "set", a.text.as_deref().unwrap_or(""), &png)
                        .await?;
                    println!("ok");
                }
                _ => anyhow::bail!("verb must be get|set"),
            }
        }
        RuntimeDisplayCommand::Act(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let raw = match a.actions.as_str() {
                "-" => {
                    use std::io::Read;
                    let mut s = String::new();
                    std::io::stdin().read_to_string(&mut s)?;
                    s
                }
                v if v.trim_start().starts_with('[') => v.to_string(),
                path => tokio::fs::read_to_string(path)
                    .await
                    .with_context(|| format!("read actions {path}"))?,
            };
            let actions: serde_json::Value =
                serde_json::from_str(&raw).context("actions must be a JSON array")?;
            anyhow::ensure!(actions.is_array(), "actions must be a JSON array");
            let mut body = serde_json::json!({"actions": actions});
            if let Some(v) = a.settle_ms {
                body["settle_ms"] = v.into();
            }
            if let Some(v) = a.timeout_ms {
                body["timeout_ms"] = v.into();
            }
            if let Some(o) = a.observe.as_deref() {
                body["observe"] =
                    serde_json::from_str(o).context("observe must be a JSON object")?;
            }
            let r = ctx.api.runtime_display().act(&a.id, body).await?;
            output::print_or_json(ctx.output, &r, || {
                println!("settled: {} (rev {})", r.settled, r.rev);
                if let Some(s) = &r.screenshot {
                    if s.unchanged {
                        println!("screenshot: unchanged (rev {})", s.rev);
                    } else {
                        println!(
                            "screenshot: {}x{} {} (rev {})",
                            s.width, s.height, s.format, s.rev
                        );
                    }
                }
            });
        }
        RuntimeDisplayCommand::Resize(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let s = ctx
                .api
                .runtime_display()
                .resize(&a.id, a.width, a.height)
                .await?;
            output::print_or_json(ctx.output, &s, || {
                println!("{}x{} @{}dpi on {}", s.width, s.height, s.dpi, s.display);
            });
        }
        RuntimeDisplayCommand::Windows(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let r = ctx
                .api
                .runtime_display()
                .windows(&a.id, a.class.as_deref().unwrap_or(""))
                .await?;
            output::print_or_json(ctx.output, &r, || {
                for w in &r.windows {
                    println!(
                        "{:<10} {:>4},{:<4} {}x{}  {}{}{}",
                        w.id,
                        w.x,
                        w.y,
                        w.w,
                        w.h,
                        w.window_class,
                        if w.focused { " *" } else { "" },
                        if w.title.is_empty() {
                            String::new()
                        } else {
                            format!("  {}", w.title)
                        },
                    );
                }
            });
        }
        RuntimeDisplayCommand::WindowOp(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            anyhow::ensure!(
                WINDOW_OPS.contains(&a.op.as_str()),
                "op must be {}",
                WINDOW_OPS.join("|")
            );
            let mut body = serde_json::json!({"id": a.window, "op": a.op});
            if let Some(v) = a.x {
                body["x"] = v.into();
            }
            if let Some(v) = a.y {
                body["y"] = v.into();
            }
            if let Some(v) = a.w {
                body["w"] = v.into();
            }
            if let Some(v) = a.h {
                body["h"] = v.into();
            }
            let w = ctx.api.runtime_display().window_op(&a.id, body).await?;
            output::print_or_json(ctx.output, &w, || {
                println!("{} {}x{} at {},{}", w.id, w.w, w.h, w.x, w.y);
            });
        }
        RuntimeDisplayCommand::Apps(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let r = ctx.api.runtime_display().apps(&a.id).await?;
            output::print_or_json(ctx.output, &r, || {
                for app in &r.apps {
                    println!("{:<40} {}", app.id, app.name);
                }
            });
        }
        RuntimeDisplayCommand::Launch(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let mut body = serde_json::json!({"app": a.app});
            if let Some(v) = a.uri.as_deref() {
                body["uri"] = v.into();
            }
            if a.wait_window {
                body["wait_window"] = true.into();
                body["timeout_ms"] = a.timeout_ms.unwrap_or(10_000).into();
            }
            let r = ctx.api.runtime_display().app_launch(&a.id, body).await?;
            output::print_or_json(ctx.output, &r, || {
                println!("pid: {}", r.pid);
                if r.window_id != 0 {
                    println!("window: {}", r.window_id);
                }
            });
        }
        RuntimeDisplayCommand::Open(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let mut body = serde_json::json!({"target": a.target});
            if a.wait_window {
                body["wait_window"] = true.into();
                body["timeout_ms"] = a.timeout_ms.unwrap_or(10_000).into();
            }
            let r = ctx.api.runtime_display().open(&a.id, body).await?;
            output::print_or_json(ctx.output, &r, || {
                println!("pid: {} ({})", r.pid, r.app);
                if r.window_id != 0 {
                    println!("window: {}", r.window_id);
                }
            });
        }
    }
    Ok(())
}

/// `"a, b ,c"` → `["a","b","c"]` (None → None).
fn split_csv(s: Option<&str>) -> Option<serde_json::Value> {
    s.map(|v| {
        serde_json::json!(v
            .split(',')
            .map(|k| k.trim().to_string())
            .filter(|k| !k.is_empty())
            .collect::<Vec<_>>())
    })
}

async fn desktop(ctx: &AppContext, cmd: RuntimeDesktopCommand) -> anyhow::Result<()> {
    match cmd {
        RuntimeDesktopCommand::Status(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let s = ctx.api.runtime_display().desktop_status(&a.id).await?;
            output::print_or_json(ctx.output, &s, || {
                println!("present: {}", s.present);
                for c in &s.components {
                    println!("{:<24} {} ({}/{})", c.name, c.unit, c.state, c.sub_state);
                }
            });
        }
        RuntimeDesktopCommand::Start(ref a)
        | RuntimeDesktopCommand::Stop(ref a)
        | RuntimeDesktopCommand::Restart(ref a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let op = match &cmd {
                RuntimeDesktopCommand::Start(_) => "start",
                RuntimeDesktopCommand::Stop(_) => "stop",
                _ => "restart",
            };
            let c = ctx
                .api
                .runtime_display()
                .desktop_component(&a.id, &a.name, op)
                .await?;
            output::print_or_json(ctx.output, &c, || {
                println!("{}: {}/{}", c.name, c.state, c.sub_state);
            });
        }
        RuntimeDesktopCommand::Logs(a) => {
            super::validate_resource_id(&a.id, "runtime")?;
            let r = ctx
                .api
                .runtime_display()
                .desktop_logs(
                    &a.id,
                    &a.name,
                    a.cursor.as_deref().unwrap_or(""),
                    a.max.unwrap_or(0),
                    a.priority.as_deref().unwrap_or(""),
                )
                .await?;
            output::print_or_json(ctx.output, &r, || {
                for e in &r.entries {
                    println!("{} [{}] {}", e.timestamp, e.priority, e.message);
                }
                if r.truncated {
                    eprintln!("(truncated — resume with --cursor {})", r.next_cursor);
                }
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Storage mounts
// ---------------------------------------------------------------------------

async fn mounts(ctx: &AppContext, cmd: RuntimeMountsCommand) -> anyhow::Result<()> {
    match cmd {
        RuntimeMountsCommand::Attach(args) => {
            super::validate_resource_id(&args.runtime_id, "runtime")?;
            let spec: crate::api::types::MountSpec = serde_json::from_str(&args.mount)
                .map_err(|e| anyhow::anyhow!("invalid --mount JSON: {e}"))?;
            crate::api::types::validate_mount(&spec)
                .map_err(|e| anyhow::anyhow!("invalid --mount: {e}"))?;
            let mount = ctx
                .api
                .storage()
                .attach_mount(&args.runtime_id, &spec)
                .await?;
            output::print_or_json(ctx.output, &mount, || {
                println!("{}", table::mount_detail_table(&mount));
            });
        }
        RuntimeMountsCommand::List(args) => {
            super::validate_resource_id(&args.runtime_id, "runtime")?;
            let result = ctx.api.storage().list_mounts(&args.runtime_id).await?;
            output::print_or_json(ctx.output, &result, || {
                println!("{}", table::mount_table(&result.mounts));
            });
        }
        RuntimeMountsCommand::Detach(args) => {
            super::validate_resource_id(&args.runtime_id, "runtime")?;
            super::validate_resource_id(&args.attachment_id, "mount")?;
            ctx.api
                .storage()
                .detach_mount(&args.runtime_id, &args.attachment_id)
                .await?;
            output::print_or_json(ctx.output, &true, || {
                println!("Detached {}", args.attachment_id);
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        git_exit_code, git_failed, next_sse_chunk, repository_directory_name, scroll_deltas,
    };
    use crate::api::runtime_git::GitOperationResult;

    fn result(exit_code: Option<i32>, success: Option<bool>) -> GitOperationResult {
        GitOperationResult {
            success,
            stdout: None,
            stderr: None,
            error: None,
            exit_code,
        }
    }

    #[test]
    fn clone_destination_matches_what_git_would_choose() {
        for (url, expected) in [
            ("https://github.com/foo/bar.git", "bar"),
            ("https://github.com/foo/bar", "bar"),
            ("https://github.com/foo/bar/", "bar"),
            ("git@github.com:foo/bar.git", "bar"),
            ("ssh://git@github.com/foo/bar.git", "bar"),
            ("git://host/bar.git", "bar"),
        ] {
            assert_eq!(repository_directory_name(url), Some(expected), "{url}");
        }
    }

    #[test]
    fn clone_destination_declines_rather_than_guessing() {
        // Nothing left to name the directory after; the caller asks for
        // --target-dir instead of inventing one.
        for url in ["", "/", "https://host/.git"] {
            assert_eq!(repository_directory_name(url), None, "{url:?}");
        }
    }

    #[test]
    fn nonzero_exit_code_is_a_failure() {
        assert!(git_failed(&result(Some(1), Some(true))));
        assert!(git_failed(&result(Some(128), None)));
        assert!(!git_failed(&result(Some(0), None)));
    }

    #[test]
    fn success_flag_decides_when_no_exit_code() {
        assert!(git_failed(&result(None, Some(false))));
        assert!(!git_failed(&result(None, Some(true))));
    }

    #[test]
    fn a_response_with_neither_field_is_not_a_failure() {
        // A missing field must never fail a pipeline on its own.
        assert!(!git_failed(&result(None, None)));
    }

    #[test]
    fn git_exit_code_passes_through_and_normalizes_signals() {
        assert_eq!(git_exit_code(&result(Some(128), None)), 128);
        // A signalled process reports -1, which carries no usable status.
        assert_eq!(git_exit_code(&result(Some(-1), None)), 1);
        assert_eq!(git_exit_code(&result(None, Some(false))), 1);
    }

    #[test]
    fn fork_payload_matches_sdk_wire_fields() {
        // Every flag combination lands on the same wire keys the SDKs send.
        let minimal = super::fork_payload(1, None, None, None, &[], None).unwrap();
        assert_eq!(minimal, serde_json::json!({ "count": 1 }));

        let full = super::fork_payload(
            3,
            Some(600),
            Some(true),
            Some("keep".into()),
            &["A=B".to_string(), "K=".to_string()],
            Some(r#"{"team":"ml"}"#),
        )
        .unwrap();
        assert_eq!(full["count"], 3);
        assert_eq!(full["timeout_seconds"], 600);
        assert_eq!(full["persist_snapshot"], true);
        assert_eq!(full["name"], "keep");
        assert_eq!(full["env_vars"]["A"], "B");
        assert_eq!(full["env_vars"]["K"], "");
        assert_eq!(full["metadata"]["team"], "ml");
    }

    #[tokio::test]
    async fn next_sse_chunk_passes_chunks_through() {
        let data = Ok::<bytes::Bytes, reqwest::Error>(bytes::Bytes::from_static(b"x"));
        let mut stream = futures_util::stream::iter([data]);
        let chunk = next_sse_chunk(&mut stream).await.unwrap();
        assert_eq!(chunk.unwrap().unwrap(), bytes::Bytes::from_static(b"x"));
        // End of stream returns None rather than erroring.
        assert!(next_sse_chunk(&mut stream).await.unwrap().is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn next_sse_chunk_fails_on_a_stalled_stream() {
        // A stream that never yields trips the per-read grace — with paused
        // time the 45s elapses instantly in the test clock.
        let mut stream = futures_util::stream::pending::<Result<bytes::Bytes, reqwest::Error>>();
        let err = next_sse_chunk(&mut stream).await.unwrap_err();
        assert!(err.to_string().contains("stalled"), "{err}");
    }

    #[test]
    fn scroll_deltas_are_wire_integers() {
        assert_eq!(scroll_deltas("0,600").unwrap(), (0, 600));
        assert_eq!(scroll_deltas(" -24 , -480 ").unwrap(), (-24, -480));
        // The server reads integer deltas — a float would land as 0.
        let json = serde_json::json!({ "dy": scroll_deltas("0,600").unwrap().1 });
        assert!(json["dy"].is_i64());
    }

    #[test]
    fn scroll_deltas_reject_malformed_values() {
        for bad in ["600", "0.5,1", "a,b", "1,", ""] {
            assert!(scroll_deltas(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn window_ops_cover_the_server_surface() {
        // Every op the guest accepts — fullscreen was rejected here while
        // the server honored it.
        for op in [
            "focus",
            "close",
            "raise",
            "lower",
            "move",
            "resize",
            "maximize",
            "unmaximize",
            "minimize",
            "fullscreen",
            "unfullscreen",
        ] {
            assert!(super::WINDOW_OPS.contains(&op), "missing {op}");
        }
        assert!(!super::WINDOW_OPS.contains(&"tile"));
    }

    #[test]
    fn fork_payload_rejects_malformed_flags() {
        assert!(super::fork_payload(1, None, None, None, &["NOEQ".into()], None).is_err());
        assert!(super::fork_payload(1, None, None, None, &[], Some("not-json")).is_err());
        assert!(super::fork_payload(1, None, None, None, &[], Some("[1]")).is_err());
        assert!(
            super::fork_payload(1, None, None, None, &[], Some(r#"{"k":1}"#)).is_err(),
            "metadata values must be strings — the create convention"
        );
    }
}
