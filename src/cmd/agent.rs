// src/cmd/agent.rs — Agent command handlers.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use futures_util::StreamExt;

use crate::api::types::{AgentBuildMetadata, DeployAgentRequest};
use crate::cli::{
    AgentBuildArgs, AgentBuildStatusArgs, AgentCommand, AgentCreateArgs, AgentDeployArgs,
    AgentDestroyArgs, AgentDevArgs, AgentDockerfileArgs, AgentGetArgs, AgentInitArgs,
    AgentInvokeArgs, AgentPackageArgs, AgentStreamArgs, AgentUpArgs,
};
use crate::config::project::GravixlayerProject;
use crate::ctx::AppContext;
use crate::framework::{self, validate_protocol_compatibility};
use crate::output::{self, table};
use crate::scaffold::{self, archive, wizard};

const AGENT_RUNTIME_BINARY: &str = "python -m gravixlayer.runtime.autoserve";

pub async fn handle(ctx: &AppContext, cmd: AgentCommand) -> Result<()> {
    match cmd {
        AgentCommand::Init(args) => init(ctx, args).await,
        AgentCommand::Create(args) => create(args).await,
        AgentCommand::Build(args) => {
            ctx.require_api_key()?;
            build(ctx, args).await
        }
        AgentCommand::Status(args) => {
            ctx.require_api_key()?;
            status(ctx, args).await
        }
        AgentCommand::Deploy(args) => {
            ctx.require_api_key()?;
            deploy(ctx, args).await
        }
        AgentCommand::Get(args) => {
            ctx.require_api_key()?;
            get(ctx, args).await
        }
        AgentCommand::Invoke(args) => {
            ctx.require_api_key()?;
            invoke(ctx, args).await
        }
        AgentCommand::Stream(args) => {
            ctx.require_api_key()?;
            stream(ctx, args).await
        }
        AgentCommand::Dev(args) => dev(ctx, args).await,
        AgentCommand::Up(args) => up(ctx, args).await,
        AgentCommand::Package(args) => package(ctx, args).await,
        AgentCommand::Dockerfile(args) => dockerfile(ctx, args).await,
        AgentCommand::Destroy(args) => {
            ctx.require_api_key()?;
            destroy(ctx, args).await
        }
        AgentCommand::Serve(args) => crate::cmd::agent_serve::serve(args).await,
    }
}

// ---------------------------------------------------------------------------
// Init (non-interactive)
// ---------------------------------------------------------------------------

async fn init(ctx: &AppContext, args: AgentInitArgs) -> Result<()> {
    let output_dir = args.output.clone().unwrap_or_else(|| PathBuf::from("."));
    let project_dir = scaffold::init_agent_project(
        &args.name,
        args.framework,
        &output_dir,
        &args.python_version,
    )?;
    output::success(
        ctx.output,
        format!(
            "Agent project '{}' created at {}",
            args.name,
            project_dir.display()
        ),
    );
    output::info(ctx.output, format!("  cd {}", project_dir.display()));
    output::info(ctx.output, "  gravixlayer agent build");
    Ok(())
}

// ---------------------------------------------------------------------------
// Create (interactive 8-step wizard — Phase 4)
// ---------------------------------------------------------------------------

async fn create(args: AgentCreateArgs) -> Result<()> {
    let result = wizard::run_wizard(args.name.as_deref()).context("agent create wizard")?;

    match result {
        None => {
            println!("Cancelled.");
            return Ok(());
        }
        Some(r) => {
            let output_dir = args.output.as_deref();
            let project_dir =
                wizard::scaffold_project(&r, output_dir).context("scaffold project files")?;

            println!();
            println!(
                "  Agent '{}' created at {}",
                r.agent_name_kebab,
                project_dir.display()
            );
            println!();
            println!("  Next steps:");
            println!("    cd {}", project_dir.display());
            println!("    # Edit gravixlayer/.env.local with your API keys");
            println!("    gravixlayer agent dev        # start live dev session");
            println!("    gravixlayer agent build      # build for deployment");
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Build
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct AgentSourceContext {
    root: PathBuf,
    project: Option<GravixlayerProject>,
    inferred: InferredAgentProject,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct InferredAgentProject {
    pub(crate) framework: Option<String>,
    pub(crate) python_version: Option<String>,
    pub(crate) entrypoint: Option<String>,
    pub(crate) target: Option<String>,
    pub(crate) ports: Vec<u16>,
    pub(crate) protocols: Vec<String>,
}

impl AgentSourceContext {
    fn discover(ctx: &AppContext, source: &Path) -> Result<Self> {
        let canonical = source
            .canonicalize()
            .with_context(|| format!("source directory not found: {}", source.display()))?;
        if !canonical.is_dir() {
            bail!("source is not a directory: {}", canonical.display());
        }

        let root = discover_agent_project_root(&canonical)?;
        let project = GravixlayerProject::find(&root)
            .map(|(project, _)| project)
            .or_else(|| ctx.project.clone());
        let inferred = infer_agent_project(&root)?;

        Ok(Self {
            root,
            project,
            inferred,
        })
    }

    fn default_name(&self) -> Option<String> {
        self.root
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned)
    }
}

fn discover_agent_project_root(source: &Path) -> Result<PathBuf> {
    const MAX_DISCOVERY_DEPTH: usize = 6;
    const MAX_DISCOVERY_DIRS: usize = 512;

    #[derive(Debug)]
    struct Candidate {
        path: PathBuf,
        score: i32,
        depth: usize,
    }

    let mut candidates = Vec::new();
    let mut stack = vec![(source.to_path_buf(), 0usize)];
    let mut visited = 0usize;

    while let Some((dir, depth)) = stack.pop() {
        visited += 1;
        if visited > MAX_DISCOVERY_DIRS {
            bail!(
                "agent source contains too many directories while discovering project root (>{MAX_DISCOVERY_DIRS})"
            );
        }

        let score = score_agent_project_root(&dir);
        if score > 0 {
            candidates.push(Candidate {
                path: dir.clone(),
                score,
                depth,
            });
        }

        if depth >= MAX_DISCOVERY_DEPTH || should_skip_agent_discovery_dir(&dir) {
            continue;
        }

        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let path = entry.path();
                if !should_skip_agent_discovery_dir(&path) {
                    stack.push((path, depth + 1));
                }
            }
        }
    }

    candidates.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.depth.cmp(&b.depth))
            .then_with(|| a.path.cmp(&b.path))
    });

    let Some(best) = candidates.first() else {
        bail!(
            "source is not a buildable agent project: expected Dockerfile, requirements.txt, pyproject.toml, setup.py, or langgraph.json in {} or a child directory",
            source.display()
        );
    };

    let ambiguous: Vec<&Candidate> = candidates
        .iter()
        .filter(|candidate| candidate.score == best.score && candidate.depth == best.depth)
        .collect();
    if ambiguous.len() > 1 {
        let roots = ambiguous
            .iter()
            .take(5)
            .map(|candidate| candidate.path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        bail!("multiple agent project roots detected; pass the specific project directory. Candidates: {roots}");
    }

    Ok(best.path.clone())
}

fn score_agent_project_root(path: &Path) -> i32 {
    let mut score = 0;
    if path.join("Dockerfile").is_file() {
        score += 120;
    }
    if path.join("pyproject.toml").is_file() {
        score += 100;
    }
    if path.join("requirements.txt").is_file() {
        score += 90;
    }
    if path.join("setup.py").is_file() {
        score += 80;
    }
    if path.join("langgraph.json").is_file() {
        score += 60;
    }
    score
}

fn should_skip_agent_discovery_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    matches!(
        name,
        ".git"
            | ".hg"
            | ".svn"
            | ".venv"
            | "venv"
            | "env"
            | "node_modules"
            | "__pycache__"
            | ".mypy_cache"
            | ".pytest_cache"
            | ".ruff_cache"
            | ".tox"
            | "dist"
            | "build"
    )
}

pub(crate) fn infer_agent_project(root: &Path) -> Result<InferredAgentProject> {
    let mut inferred = InferredAgentProject::default();

    if let Some((python_version, graph_target)) = read_langgraph_config(root)? {
        inferred.framework = Some("langgraph".to_string());
        inferred.python_version = python_version;
        inferred.target = graph_target.clone();
        if graph_target.is_some() {
            inferred.ports = vec![8000];
        }
    }

    let deps = read_dependency_names(root)?;
    if inferred.framework.is_none() {
        inferred.framework = infer_framework_from_dependencies(&deps);
    }
    if inferred.entrypoint.is_none() {
        inferred.entrypoint = infer_a2a_entrypoint(root)?;
    }
    if inferred.ports.is_empty() && inferred.entrypoint.is_some() {
        inferred.ports = vec![8000];
    }

    let has_a2a = inferred.entrypoint.is_some();
    if has_a2a {
        inferred.protocols = vec!["http".to_string(), "a2a".to_string()];
    }

    Ok(inferred)
}

fn read_langgraph_config(root: &Path) -> Result<Option<(Option<String>, Option<String>)>> {
    let path = root.join("langgraph.json");
    if !path.is_file() {
        return Ok(None);
    }

    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    let python_version = value
        .get("python_version")
        .and_then(|value| value.as_str())
        .map(normalize_python_version)
        .filter(|version| !version.is_empty());
    let graph_target = value
        .get("graphs")
        .and_then(|graphs| graphs.as_object())
        .and_then(|graphs| graphs.get("agent").or_else(|| graphs.values().next()))
        .and_then(langgraph_graph_target)
        .map(ToOwned::to_owned);

    Ok(Some((python_version, graph_target)))
}

fn langgraph_graph_target(value: &serde_json::Value) -> Option<&str> {
    value
        .as_str()
        .or_else(|| value.get("path").and_then(|path| path.as_str()))
        .map(str::trim)
        .filter(|target| !target.is_empty())
}

fn normalize_python_version(version: &str) -> String {
    let parts: Vec<&str> = version.trim().split('.').collect();
    match parts.as_slice() {
        [major, minor, ..] => format!("{major}.{minor}"),
        _ => version.trim().to_string(),
    }
}

fn read_dependency_names(root: &Path) -> Result<Vec<String>> {
    let mut deps = Vec::new();
    let requirements = root.join("requirements.txt");
    if requirements.is_file() {
        let raw = fs::read_to_string(&requirements)
            .with_context(|| format!("failed to read {}", requirements.display()))?;
        for line in raw.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') || line.starts_with('-') {
                continue;
            }
            deps.push(normalize_dependency_name(line));
        }
    }

    let pyproject = root.join("pyproject.toml");
    if pyproject.is_file() {
        let raw = fs::read_to_string(&pyproject)
            .with_context(|| format!("failed to read {}", pyproject.display()))?;
        for dep in [
            "langgraph",
            "langchain",
            "langchain-core",
            "crewai",
            "google-adk",
            "openai-agents",
            "anthropic",
            "claude-agent-sdk",
            "strands-agents",
            "a2a-sdk",
            "a2a-python",
            "mcp",
        ] {
            if raw.to_ascii_lowercase().contains(dep) {
                deps.push(dep.to_string());
            }
        }
    }

    deps.sort();
    deps.dedup();
    Ok(deps)
}

fn normalize_dependency_name(spec: &str) -> String {
    spec.trim()
        .split(&['>', '<', '=', '!', '~', ';', ' '][..])
        .next()
        .unwrap_or(spec)
        .split('[')
        .next()
        .unwrap_or(spec)
        .to_ascii_lowercase()
        .replace('_', "-")
}

fn infer_framework_from_dependencies(deps: &[String]) -> Option<String> {
    if deps.iter().any(|dep| dep == "langgraph") {
        Some("langgraph".to_string())
    } else if deps.iter().any(|dep| dep == "crewai") {
        Some("crewai".to_string())
    } else if deps.iter().any(|dep| dep == "google-adk") {
        Some("google-adk".to_string())
    } else if deps.iter().any(|dep| dep == "openai-agents") {
        Some("openai-agents".to_string())
    } else if deps.iter().any(|dep| dep == "strands-agents") {
        Some("strands".to_string())
    } else if deps
        .iter()
        .any(|dep| dep == "claude-agent-sdk" || dep == "anthropic")
    {
        Some("anthropic".to_string())
    } else if deps
        .iter()
        .any(|dep| dep == "langchain" || dep == "langchain-core")
    {
        Some("langchain".to_string())
    } else {
        Some("python".to_string())
    }
}

fn infer_a2a_entrypoint(root: &Path) -> Result<Option<String>> {
    const MAX_A2A_DISCOVERY_DEPTH: usize = 8;
    const MAX_A2A_DISCOVERY_DIRS: usize = 512;
    const MAX_A2A_DISCOVERY_FILES: usize = 1024;

    let mut stack = vec![(root.to_path_buf(), 0usize)];
    let mut visited_dirs = 0usize;
    let mut visited_files = 0usize;

    while let Some((dir, depth)) = stack.pop() {
        visited_dirs += 1;
        if visited_dirs > MAX_A2A_DISCOVERY_DIRS {
            bail!(
                "A2A entrypoint discovery visited more than {MAX_A2A_DISCOVERY_DIRS} directories; add entrypoint or start_command to gravixlayer/gravixlayer.json"
            );
        }
        if should_skip_agent_discovery_dir(&dir) {
            continue;
        }
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                if depth < MAX_A2A_DISCOVERY_DEPTH && !path.is_symlink() {
                    stack.push((path, depth + 1));
                }
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("py") {
                continue;
            }
            visited_files += 1;
            if visited_files > MAX_A2A_DISCOVERY_FILES {
                bail!(
                    "A2A entrypoint discovery inspected more than {MAX_A2A_DISCOVERY_FILES} Python files; add entrypoint or start_command to gravixlayer/gravixlayer.json"
                );
            }
            let raw = fs::read_to_string(&path).unwrap_or_default();
            if raw.contains("run_a2a(")
                || raw.contains("create_a2a_app(")
                || raw.contains("A2AServer(")
            {
                if let Some(module) = python_module_for_file(root, &path) {
                    return Ok(Some(format!("python -m {module}")));
                }
            }
        }
    }
    Ok(None)
}

fn python_module_for_file(root: &Path, file: &Path) -> Option<String> {
    let rel = file.strip_prefix(root).ok()?;
    let mut parts: Vec<String> = rel
        .with_extension("")
        .components()
        .filter_map(|component| component.as_os_str().to_str().map(ToOwned::to_owned))
        .collect();
    if parts.last().map(String::as_str) == Some("__init__") {
        parts.pop();
    }
    if parts.is_empty() || !parts.iter().all(|part| is_python_identifier(part)) {
        return None;
    }
    Some(parts.join("."))
}

fn is_python_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(c) if c == '_' || c.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

fn configured_entrypoint(project: Option<&GravixlayerProject>) -> Option<String> {
    project
        .and_then(|project| project.entrypoint.clone())
        .or_else(|| {
            project
                .and_then(|project| project.start_command.clone())
                .filter(|parts| !parts.is_empty())
                .map(|parts| parts.join(" "))
        })
}

fn resolve_ports(
    cli_ports: Vec<u16>,
    project: Option<&GravixlayerProject>,
    inferred: &InferredAgentProject,
) -> Vec<u16> {
    if !cli_ports.is_empty() {
        return cli_ports;
    }
    if let Some(port) = project.and_then(|project| project.port) {
        return vec![port];
    }
    if !inferred.ports.is_empty() {
        return inferred.ports.clone();
    }
    vec![8000]
}

#[cfg(test)]
fn resolve_http_port(
    cli_http_port: Option<u16>,
    project: Option<&GravixlayerProject>,
    ports: &[u16],
) -> u16 {
    cli_http_port
        .or_else(|| project.and_then(|project| project.port))
        .or_else(|| ports.first().copied())
        .unwrap_or(8000)
}

/// Deploy-time HTTP port for the API request.
///
/// - Explicit `--http-port` always wins (intentional override).
/// - Build+deploy (from source): project `port` / ports[0] / 8000 so RegisterAgent
///   matches the entrypoint baked in this build.
/// - Template-only: omit (`None`) so CP uses `template.http_port`. Never invent a
///   port from cwd `gravixlayer.json` — that would override template SSOT.
fn resolve_deploy_http_port(
    from_source: bool,
    cli_http_port: Option<u16>,
    project: Option<&GravixlayerProject>,
    ports: &[u16],
) -> Option<u16> {
    if let Some(port) = cli_http_port {
        return Some(port);
    }
    if !from_source {
        return None;
    }
    if let Some(port) = project.and_then(|project| project.port) {
        return Some(port);
    }
    Some(ports.first().copied().unwrap_or(8000))
}

fn resolve_protocols(
    cli_protocols: &[crate::cli::AgentProtocolArg],
    project: Option<&GravixlayerProject>,
    inferred: &InferredAgentProject,
) -> Result<Vec<String>> {
    if !cli_protocols.is_empty() {
        let protocols: Vec<String> = cli_protocols.iter().map(ToString::to_string).collect();
        return framework::normalize_protocols(&protocols);
    }
    if let Some(project) = project {
        if !project.protocols.is_empty() {
            return framework::normalize_protocols(&project.protocols);
        }
    }
    framework::normalize_protocols(&inferred.protocols)
}

fn load_dotenv(root: &Path) -> Result<HashMap<String, String>> {
    let mut env = HashMap::new();
    for path in [
        root.join(".env"),
        root.join("gravixlayer").join(".env.local"),
    ] {
        if !path.is_file() {
            continue;
        }
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        for line in raw.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            if key.is_empty() || !key.chars().all(|c| c == '_' || c.is_ascii_alphanumeric()) {
                continue;
            }
            let value = value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            env.insert(key.to_string(), value);
        }
    }
    Ok(env)
}

fn merged_agent_environment(
    source: Option<&AgentSourceContext>,
    project: Option<&GravixlayerProject>,
    explicit: HashMap<String, String>,
) -> Result<HashMap<String, String>> {
    let mut env = HashMap::new();
    if let Some(source) = source {
        env.extend(load_dotenv(&source.root)?);
    }
    if let Some(project) = project {
        env.extend(project.env.clone());
    }
    env.extend(explicit);
    Ok(env)
}

fn resolve_framework(
    cli_framework: Option<crate::cli::AgentFrameworkArg>,
    project: Option<&GravixlayerProject>,
    inferred: &InferredAgentProject,
) -> Result<Option<String>> {
    let raw = cli_framework
        .map(|framework| framework.to_string())
        .or_else(|| project.and_then(|project| project.framework.clone()))
        .or_else(|| inferred.framework.clone());

    raw.map(|value| framework::normalize_framework(&value))
        .transpose()
}

async fn build(ctx: &AppContext, args: AgentBuildArgs) -> Result<()> {
    let source = AgentSourceContext::discover(ctx, &args.source)?;
    let project = source.project.as_ref();

    let extra_excludes = ctx.project.as_ref();
    let extra_excludes = project
        .or(extra_excludes)
        .map(|p| p.exclude.clone())
        .unwrap_or_default();

    let env_vars =
        merged_agent_environment(Some(&source), project, parse_env_vars(&args.env_vars)?)?;
    let tags = parse_env_vars(&args.tags)?;
    let ports = resolve_ports(args.ports, project, &source.inferred);
    let framework = resolve_framework(args.framework, project, &source.inferred)?;
    let protocols = resolve_protocols(&[], project, &source.inferred)?;
    let entrypoint = args
        .entrypoint
        .or_else(|| configured_entrypoint(project))
        .or_else(|| {
            native_runtime_entrypoint(
                &framework,
                resolve_agent_target(args.target, project, &source.inferred).as_deref(),
                &ports,
                &protocols,
            )
        })
        .or_else(|| source.inferred.entrypoint.clone());

    // Client-side preflight: the control plane only accepts entrypoints that
    // invoke the canonical runtime (`python -m gravixlayer.runtime.autoserve`).
    // Catch bad input here so users get an immediate, actionable error rather
    // than a server-side 500 after upload.
    if let Some(ref ep) = entrypoint {
        let trimmed = ep.trim_start_matches("exec ").trim();
        if !trimmed.starts_with(AGENT_RUNTIME_BINARY) {
            bail!(
                "agent entrypoint must invoke `{AGENT_RUNTIME_BINARY}`; got `{ep}`.\n\
                 Hint: omit --entrypoint and let the CLI generate it from --framework / --target / \
                 --port / --protocol, or remove `entrypoint`/`start_command` from gravixlayer/gravixlayer.json."
            );
        }
    }

    let spinner = output::Spinner::new("Packaging source archive…");
    let archive_bytes = archive::create_source_archive(&source.root, &extra_excludes)?;
    drop(spinner);

    // Reject archives that exceed the platform limit. The cap is consistent across
    // the CLI, HTTP API, and gRPC transport so a build never passes one layer only
    // to fail at the next. Real agents should be far smaller.
    const MAX_ARCHIVE_BYTES: usize = 104_857_600; // 100 MB
    if archive_bytes.len() > MAX_ARCHIVE_BYTES {
        bail!(
            "archive is too large ({:.1} MB); maximum is 100 MB. \
             Add patterns to .gravixlayerignore to reduce the size.",
            archive_bytes.len() as f64 / 1_048_576.0
        );
    }

    output::info(
        ctx.output,
        format!("Archive: {:.1} KB", archive_bytes.len() as f64 / 1024.0),
    );

    let metadata = AgentBuildMetadata {
        name: args
            .name
            .or_else(|| project.and_then(|p| p.name.clone()))
            .or_else(|| source.default_name()),
        description: args
            .description
            .or_else(|| project.and_then(|p| p.description.clone())),
        framework,
        python_version: args
            .python_version
            .or_else(|| project.and_then(|p| p.python_version.clone()))
            .or_else(|| source.inferred.python_version.clone()),
        entrypoint,
        ports,
        environment: env_vars,
        vcpu_count: args.vcpu_count,
        memory_mb: args.memory_mb,
        disk_mb: args.disk_mb,
        start_cmd: args.start_cmd,
        ready_cmd: args.ready_cmd,
        ready_timeout_secs: args.ready_timeout_secs,
        tags,
    };

    let spinner = output::Spinner::new("Submitting build…");
    let build_resp = ctx.api.agent().build(archive_bytes, &metadata).await?;
    spinner.finish_ok(format!("Build submitted: {}", build_resp.build_id));

    if args.wait {
        let sp = output::Spinner::new(format!("Building {}…", build_resp.build_id));
        let deadline = Instant::now() + Duration::from_secs(args.build_timeout);
        let final_status = ctx
            .api
            .agent()
            .wait_for_build(&build_resp.build_id, deadline)
            .await?;
        sp.finish_ok(format!(
            "Build {} completed (template: {})",
            final_status.build_id,
            final_status.template_id.as_deref().unwrap_or("—")
        ));
        output::print_or_json(ctx.output, &final_status, || {
            println!("{}", table::agent_build_status_table(&final_status));
        });
    } else {
        output::info(
            ctx.output,
            format!(
                "Track progress with: gravixlayer agent status {}",
                build_resp.build_id
            ),
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Build status
// ---------------------------------------------------------------------------

async fn status(ctx: &AppContext, args: AgentBuildStatusArgs) -> Result<()> {
    let s = ctx.api.agent().build_status(&args.build_id).await?;
    output::print_or_json(ctx.output, &s, || {
        println!("{}", table::agent_build_status_table(&s));
    });
    Ok(())
}

// ---------------------------------------------------------------------------
// Deploy
// ---------------------------------------------------------------------------

async fn deploy(ctx: &AppContext, args: AgentDeployArgs) -> Result<()> {
    let source = if args.template_id.is_none() {
        Some(AgentSourceContext::discover(ctx, &args.source)?)
    } else {
        None
    };
    let project = source
        .as_ref()
        .and_then(|source| source.project.as_ref())
        .or(ctx.project.as_ref());

    let environment =
        merged_agent_environment(source.as_ref(), project, parse_env_vars(&args.env_vars)?)?;
    let deploy_environment = parse_env_vars(&args.deploy_env_vars)?;
    let tags = parse_env_vars(&args.tags)?;
    let inferred = source.as_ref().map(|source| &source.inferred);
    let empty_inferred = InferredAgentProject::default();
    let inferred = inferred.unwrap_or(&empty_inferred);
    let framework_str = args
        .framework
        .map(|f| f.to_string())
        .or_else(|| project.and_then(|project| project.framework.clone()))
        .or_else(|| inferred.framework.clone())
        .map(|value| framework::normalize_framework(&value))
        .transpose()?;
    let build_ports = resolve_ports(args.ports.clone(), project, inferred);
    let protocols = resolve_protocols(&args.protocols, project, inferred)?;
    let entry_point = args
        .entrypoint
        .clone()
        .or_else(|| configured_entrypoint(project))
        .or_else(|| {
            native_runtime_entrypoint(
                &framework_str,
                resolve_agent_target(args.target.clone(), project, inferred).as_deref(),
                &build_ports,
                &protocols,
            )
        })
        .or_else(|| inferred.entrypoint.clone());
    let http_port = resolve_deploy_http_port(
        args.template_id.is_none(),
        args.http_port,
        project,
        &build_ports,
    );
    let native_single_port = framework_str
        .as_deref()
        .is_some_and(is_native_cli_serve_framework);
    let a2a_port = args.a2a_port.or_else(|| {
        if native_single_port {
            None
        } else {
            project.and_then(|project| project.a2a_port)
        }
    });
    let mcp_port = args
        .mcp_port
        .or_else(|| project.and_then(|project| project.mcp_port));
    validate_protocol_compatibility(framework_str.as_deref(), &protocols, &inferred.protocols)?;
    let is_public = args
        .is_public
        .or_else(|| project.and_then(|project| project.is_public));
    let timeout = args.timeout;
    let wait = args.wait;
    let wait_timeout = args.wait_timeout;
    let agent_card = project
        .and_then(|project| project.agent_card.as_ref())
        .map(serde_json::to_value)
        .transpose()?;

    // Determine the template_id: either provided directly or built from source.
    let template_id = if let Some(tid) = args.template_id.clone() {
        // Deploy directly from existing template — skip build.
        tid
    } else {
        let source = source
            .as_ref()
            .expect("source context exists when template_id is absent");

        let extra_excludes = ctx.project.as_ref();
        let extra_excludes = project
            .or(extra_excludes)
            .map(|p| p.exclude.clone())
            .unwrap_or_default();

        let spinner = output::Spinner::new("Packaging source archive…");
        let archive_bytes = archive::create_source_archive(&source.root, &extra_excludes)?;
        drop(spinner);

        const MAX_ARCHIVE_BYTES: usize = 104_857_600; // 100 MB
        if archive_bytes.len() > MAX_ARCHIVE_BYTES {
            bail!(
                "archive is too large ({:.1} MB); maximum is 100 MB.",
                archive_bytes.len() as f64 / 1_048_576.0
            );
        }

        output::info(
            ctx.output,
            format!("Archive: {:.1} KB", archive_bytes.len() as f64 / 1024.0),
        );

        let metadata = AgentBuildMetadata {
            name: args
                .name
                .or_else(|| project.and_then(|p| p.name.clone()))
                .or_else(|| source.default_name()),
            description: args
                .description
                .or_else(|| project.and_then(|p| p.description.clone())),
            framework: framework_str.clone(),
            python_version: args
                .python_version
                .or_else(|| project.and_then(|p| p.python_version.clone()))
                .or_else(|| inferred.python_version.clone()),
            entrypoint: entry_point.clone(),
            ports: build_ports.clone(),
            environment: environment.clone(),
            vcpu_count: args.vcpu_count,
            memory_mb: args.memory_mb,
            disk_mb: args.disk_mb,
            start_cmd: args.start_cmd.clone(),
            ready_cmd: args.ready_cmd.clone(),
            ready_timeout_secs: args.ready_timeout_secs,
            tags: tags.clone(),
        };

        let spinner = output::Spinner::new("Submitting build…");
        let build_resp = ctx.api.agent().build(archive_bytes, &metadata).await?;
        spinner.finish_ok(format!("Build submitted: {}", build_resp.build_id));

        let sp = output::Spinner::new(format!("Building {}…", build_resp.build_id));
        let build_deadline = Instant::now() + Duration::from_secs(args.build_timeout);
        let final_status = ctx
            .api
            .agent()
            .wait_for_build(&build_resp.build_id, build_deadline)
            .await?;

        let tid = final_status.template_id.ok_or_else(|| {
            anyhow::anyhow!(
                "build {} completed but no template_id returned",
                final_status.build_id
            )
        })?;
        sp.finish_ok(format!("Build complete — template: {}", tid));
        tid
    };

    let req = DeployAgentRequest {
        template_id,
        framework: framework_str,
        entry_point,
        http_port,
        a2a_port,
        mcp_port,
        protocols,
        is_public,
        environment: if deploy_environment.is_empty() {
            environment
        } else {
            deploy_environment
        },
        timeout,
        agent_card,
    };

    let spinner = output::Spinner::new("Deploying agent…");
    let resp = ctx.api.agent().deploy(&req).await?;
    spinner.finish_ok(format!("Agent {} deploying…", resp.agent_id));

    if wait {
        let sp = output::Spinner::new(format!(
            "Waiting for agent {} to become active…",
            resp.agent_id
        ));
        let deadline = Instant::now() + Duration::from_secs(wait_timeout);
        let mut ep = ctx
            .api
            .agent()
            .wait_until_active(&resp.agent_id, deadline)
            .await?;
        // Deploy response derives a2a/mcp from the request protocols; GET used to
        // drop them when TEXT[] protocols failed to scan. Prefer GET, fall back
        // to deploy so the table is complete even across rolling upgrades.
        if ep.a2a_endpoint.as_deref().unwrap_or("").is_empty() {
            if let Some(ref a2a) = resp.a2a_endpoint {
                if !a2a.is_empty() {
                    ep.a2a_endpoint = Some(a2a.clone());
                }
            }
        }
        if ep.mcp_endpoint.as_deref().unwrap_or("").is_empty() {
            if let Some(ref mcp) = resp.mcp_endpoint {
                if !mcp.is_empty() {
                    ep.mcp_endpoint = Some(mcp.clone());
                }
            }
        }
        if ep.name.as_deref().unwrap_or("").is_empty() {
            if let Some(ref name) = resp.name {
                if !name.is_empty() {
                    ep.name = Some(name.clone());
                }
            }
        }
        if ep.framework.as_deref().unwrap_or("").is_empty() {
            if let Some(ref framework) = resp.framework {
                if !framework.is_empty() {
                    ep.framework = Some(framework.clone());
                }
            }
        }
        if ep.created_at.as_deref().unwrap_or("").is_empty() {
            if let Some(ref created) = resp.created_at {
                if !created.is_empty() {
                    ep.created_at = Some(created.clone());
                }
            }
        }
        sp.finish_ok(format!(
            "Agent {} is active: {}",
            ep.agent_id,
            ep.endpoint.as_deref().unwrap_or("(no endpoint yet)")
        ));
        output::print_or_json(ctx.output, &ep, || {
            println!("{}", table::agent_detail_table(&ep));
        });
    } else {
        output::print_or_json(ctx.output, &resp, || {
            output::kv(ctx.output, "agent_id", &resp.agent_id);
            if let Some(ref name) = resp.name {
                output::kv(ctx.output, "name", name);
            }
            output::kv(ctx.output, "status", resp.status.as_deref().unwrap_or("—"));
            if let Some(ref framework) = resp.framework {
                output::kv(ctx.output, "framework", framework);
            }
            if let Some(ref ep) = resp.endpoint {
                output::kv(ctx.output, "endpoint", ep);
            }
            if let Some(ref a2a) = resp.a2a_endpoint {
                output::kv(ctx.output, "a2a_endpoint", a2a);
            }
            if let Some(ref mcp) = resp.mcp_endpoint {
                output::kv(ctx.output, "mcp_endpoint", mcp);
            }
            if let Some(ref created) = resp.created_at {
                output::kv(ctx.output, "created", created);
            }
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Get
// ---------------------------------------------------------------------------

async fn get(ctx: &AppContext, args: AgentGetArgs) -> Result<()> {
    super::validate_resource_id(&args.id, "agent")?;
    let ep = ctx.api.agent().get(&args.id).await?;
    output::print_or_json(ctx.output, &ep, || {
        println!("{}", table::agent_detail_table(&ep));
    });
    Ok(())
}

// ---------------------------------------------------------------------------
// Invoke
// ---------------------------------------------------------------------------

async fn invoke(ctx: &AppContext, args: AgentInvokeArgs) -> Result<()> {
    super::validate_resource_id(&args.id, "agent")?;
    let payload = build_agent_invocation_payload(
        args.input.as_deref(),
        args.message.as_deref(),
        args.session_id.as_deref(),
        args.resume.as_deref(),
        args.metadata.as_deref(),
    )?;

    let resp = ctx.api.agent().invoke(&args.id, &payload).await?;
    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(())
}

// ---------------------------------------------------------------------------
// Stream  (SSE output from a deployed agent)
// ---------------------------------------------------------------------------

async fn stream(ctx: &AppContext, args: AgentStreamArgs) -> Result<()> {
    super::validate_resource_id(&args.id, "agent")?;
    let payload = build_agent_invocation_payload(
        args.input.as_deref(),
        args.message.as_deref(),
        args.session_id.as_deref(),
        args.resume.as_deref(),
        args.metadata.as_deref(),
    )?;

    let resp = ctx.api.agent().stream(&args.id, &payload).await?;

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    const SSE_BUF_MAX: usize = 1024 * 1024; // 1 MiB
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.context("stream read")?;
        buf.push_str(&String::from_utf8_lossy(&bytes));
        if buf.len() > SSE_BUF_MAX && !buf.contains("\n\n") {
            bail!(
                "SSE event exceeded buffer limit ({}B); stream may be malformed",
                SSE_BUF_MAX
            );
        }
        while let Some(pos) = buf.find("\n\n") {
            let event = buf[..pos].to_string();
            buf = buf[pos + 2..].to_string();
            for line in event.lines() {
                if let Some(data) = line.strip_prefix("data:") {
                    let data = data.trim();
                    if data == "[DONE]" {
                        return Ok(());
                    }
                    // Try to parse as JSON for pretty output; fall back to raw.
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                        // Extract text content if nested
                        if let Some(text) = v.get("text").and_then(|t| t.as_str()) {
                            print!("{text}");
                        } else if let Some(content) = v.get("content").and_then(|t| t.as_str()) {
                            print!("{content}");
                        } else {
                            print!("{}", serde_json::to_string(&v)?);
                        }
                        let _ = std::io::Write::flush(&mut std::io::stdout());
                    } else {
                        println!("{data}");
                    }
                }
            }
        }
    }
    println!(); // trailing newline
    Ok(())
}

// ---------------------------------------------------------------------------
// Dev
// ---------------------------------------------------------------------------

async fn dev(ctx: &AppContext, args: AgentDevArgs) -> Result<()> {
    if args.runtime_sync {
        ctx.require_api_key()?;
        return remote_dev(ctx, args).await;
    }

    local_dev(ctx, args).await
}

async fn local_dev(ctx: &AppContext, args: AgentDevArgs) -> Result<()> {
    let source = AgentSourceContext::discover(ctx, &args.source)?;
    let protocols = resolve_protocols(&[], source.project.as_ref(), &source.inferred)?;
    let command = resolve_local_dev_command(&source, &args, &protocols)?;
    let mut environment =
        merged_agent_environment(Some(&source), source.project.as_ref(), HashMap::new())?;
    environment
        .entry("GRAVIXLAYER_PROTOCOLS".to_string())
        .or_insert_with(|| protocols.join(","));
    let endpoint = format!("http://{}:{}", args.host, args.port);

    output::info(
        ctx.output,
        format!("Starting local agent: {}", format_command(&command)),
    );
    output::info(ctx.output, format!("Endpoint: {endpoint}"));

    let mut child = tokio::process::Command::new(&command[0]);
    child.args(&command[1..]);
    child.current_dir(&source.root);
    child.envs(environment);
    child.env("HOST", &args.host);
    child.env("PORT", args.port.to_string());

    if args.message.is_some() {
        child.stdout(Stdio::null()).stderr(Stdio::null());
    } else {
        child.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    }

    let mut child = child.spawn().with_context(|| {
        format!(
            "failed to start local agent command: {}",
            format_command(&command)
        )
    })?;

    if let Some(message) = args.message {
        let result = async {
            wait_for_local_health(&endpoint).await?;
            invoke_local_agent(&endpoint, &message).await
        }
        .await;
        let _ = child.kill().await;
        let response = result?;
        println!("{}", serde_json::to_string_pretty(&response)?);
        return Ok(());
    }

    let status = child.wait().await.context("local agent process")?;
    if !status.success() {
        bail!("local agent exited with status {status}");
    }
    Ok(())
}

async fn up(ctx: &AppContext, args: AgentUpArgs) -> Result<()> {
    let docker_status = tokio::process::Command::new("docker")
        .arg("info")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;
    match docker_status {
        Ok(status) if status.success() => {}
        _ => bail!("Docker is required for `gravixlayer agent up`; start Docker and try again"),
    }

    let source = AgentSourceContext::discover(ctx, &args.source)?;
    let project = source.project.as_ref();
    let framework = resolve_framework(args.framework, project, &source.inferred)?;
    let python_version = args
        .python_version
        .or_else(|| project.and_then(|project| project.python_version.clone()))
        .or_else(|| source.inferred.python_version.clone())
        .unwrap_or_else(|| "3.12".to_string());
    let ports = vec![args.port];
    let target = resolve_agent_target(args.target.clone(), project, &source.inferred);
    let protocols = resolve_protocols(&[], project, &source.inferred)?;
    let native_entrypoint = framework
        .as_deref()
        .filter(|framework| is_native_cli_serve_framework(framework))
        .map(|framework| {
            native_runtime_command(
                AGENT_RUNTIME_BINARY,
                framework,
                "/app",
                "0.0.0.0",
                args.port,
                target.as_deref(),
                &protocols,
            )
            .join(" ")
        });
    let entrypoint = configured_entrypoint(project)
        .or(native_entrypoint)
        .or_else(|| source.inferred.entrypoint.clone())
        .unwrap_or_else(|| "python -m main".to_string());
    let mut environment =
        merged_agent_environment(Some(&source), project, parse_env_vars(&args.env_vars)?)?;
    environment
        .entry("GRAVIXLAYER_PROTOCOLS".to_string())
        .or_insert_with(|| protocols.join(","));
    let image_name = format!(
        "gravixlayer-agent-dev-{}-{}",
        image_name_component(&source.default_name().unwrap_or_else(|| "agent".to_string())),
        local_image_suffix()
    );

    let generated_dockerfile = if source.root.join("Dockerfile").is_file() {
        None
    } else {
        Some(generate_local_dockerfile(
            framework.as_deref(),
            &python_version,
            &entrypoint,
            &ports,
            &protocols,
            source.root.join("requirements.txt").is_file(),
            source.root.join("pyproject.toml").is_file(),
        ))
    };

    let spinner = output::Spinner::new(format!("Building local Docker image {image_name}…"));
    let mut build_cmd = tokio::process::Command::new("docker");
    build_cmd.arg("build").arg("-t").arg(&image_name);
    if generated_dockerfile.is_some() {
        build_cmd.arg("-f").arg("-").stdin(Stdio::piped());
    }
    build_cmd.arg(".");
    build_cmd.current_dir(&source.root);
    build_cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    let mut build_child = build_cmd.spawn().context("start docker build")?;
    if let Some(dockerfile) = generated_dockerfile {
        use tokio::io::AsyncWriteExt;

        let mut stdin = build_child
            .stdin
            .take()
            .context("open docker build stdin")?;
        stdin.write_all(dockerfile.as_bytes()).await?;
    }
    let build_status = build_child.wait().await.context("docker build")?;
    if !build_status.success() {
        bail!("docker build exited with status {build_status}");
    }
    spinner.finish_ok(format!("Built local Docker image {image_name}"));

    let endpoint = format!("http://{}:{}", args.host, args.port);
    output::info(
        ctx.output,
        format!("Starting local Docker agent: {endpoint}"),
    );

    let mut run_cmd = tokio::process::Command::new("docker");
    run_cmd
        .arg("run")
        .arg("--rm")
        .arg("-p")
        .arg(format!("{}:{}:{}", args.host, args.port, args.port));
    for (key, value) in environment {
        run_cmd.arg("-e").arg(format!("{key}={value}"));
    }
    run_cmd.arg(&image_name);
    run_cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    let status = run_cmd.status().await.context("docker run")?;
    if !status.success() {
        bail!("docker run exited with status {status}");
    }
    Ok(())
}

async fn remote_dev(ctx: &AppContext, args: AgentDevArgs) -> Result<()> {
    let source = args.source.canonicalize().context("resolve source dir")?;
    if !source.is_dir() {
        bail!("source is not a directory: {}", source.display());
    }

    // Load project configuration — find_from_cwd searches up from CWD.
    // Change into the source directory first so it finds the project file there.
    let orig_dir = std::env::current_dir().ok();
    let _ = std::env::set_current_dir(&source);
    let project_opt = crate::config::project::GravixlayerProject::find_from_cwd();
    if let Some(ref d) = orig_dir {
        let _ = std::env::set_current_dir(d);
    }
    let (project, _) =
        project_opt.context("load gravixlayer.json (run from project root or pass source path)")?;

    let agent_name = project
        .name
        .clone()
        .unwrap_or_else(|| "unknown-agent".to_string());

    let template = project
        .template
        .as_deref()
        .unwrap_or("base-small")
        .to_string();

    // 1. Create an ephemeral runtime
    use crate::api::types::CreateRuntimeRequest;
    let spinner = output::Spinner::new(format!("Creating runtime for {agent_name}…"));
    let runtime = ctx
        .api
        .runtime()
        .create(CreateRuntimeRequest {
            template: template.clone(),
            cloud: args.cloud.clone(),
            region: args.region.clone(),
            timeout: Some(3600),
            internet_access: Some(true),
            env_vars: HashMap::new(),
            metadata: HashMap::new(),
            agent_id: None,
            providers: Vec::new(),
            network_policy_ids: Vec::new(),
        })
        .await?;
    spinner.finish_ok(format!("Runtime {} created", runtime.runtime_id));

    let runtime_id = runtime.runtime_id.clone();

    // Run the rest of the dev session; ensure runtime is killed on any exit path
    // (error return, panic, or clean Ctrl+C).
    let result = dev_session(ctx, &runtime_id, &source, &args).await;
    // Best-effort cleanup — ignore kill errors (runtime may have already terminated).
    let _ = ctx.api.runtime().kill(&runtime_id).await;
    if result.is_ok() {
        output::success(ctx.output, format!("Runtime {runtime_id} terminated."));
    }
    result
}

fn resolve_local_dev_command(
    source: &AgentSourceContext,
    args: &AgentDevArgs,
    protocols: &[String],
) -> Result<Vec<String>> {
    if let Some(parts) = source
        .project
        .as_ref()
        .and_then(|project| project.start_command.clone())
        .filter(|parts| !parts.is_empty())
    {
        return Ok(parts);
    }

    if let Some(entrypoint) = configured_entrypoint(source.project.as_ref()) {
        return shell_words::split(&entrypoint)
            .map_err(|err| anyhow::anyhow!("invalid entrypoint command: {err}"));
    }

    if let Some(framework) = source.inferred.framework.as_deref() {
        if is_native_cli_serve_framework(framework) {
            let target = resolve_agent_target(
                args.target.clone(),
                source.project.as_ref(),
                &source.inferred,
            );
            // Local native frameworks use the same autoserve stack as production
            // (HITL interrupt/resume + A2A InputRequired), not the Rust agent serve bridge.
            return Ok(native_autoserve_dev_command(
                framework,
                ".",
                &args.host,
                args.port,
                target.as_deref(),
                protocols,
            ));
        }
    }

    let main_py = source.root.join("src").join("main.py");
    if main_py.is_file() {
        return Ok(vec!["python".into(), "src/main.py".into()]);
    }

    let main_py = source.root.join("main.py");
    if main_py.is_file() {
        return Ok(vec!["python".into(), "main.py".into()]);
    }

    if source.root.join("langgraph.json").is_file() {
        let mut command = vec![
            "langgraph".into(),
            "dev".into(),
            "--host".into(),
            args.host.clone(),
            "--port".into(),
            args.port.to_string(),
        ];
        if args.no_reload {
            command.push("--no-reload".into());
        }
        return Ok(command);
    }

    bail!(
        "could not determine a local dev command. Add start_command or entrypoint to gravixlayer/gravixlayer.json"
    )
}

async fn wait_for_local_health(endpoint: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let health_url = format!("{endpoint}/health");
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if Instant::now() > deadline {
            bail!("local agent did not become healthy at {health_url} within 30 seconds");
        }
        match client.get(&health_url).send().await {
            Ok(resp) if resp.status().is_success() => return Ok(()),
            _ => tokio::time::sleep(Duration::from_millis(250)).await,
        }
    }
}

async fn invoke_local_agent(endpoint: &str, message: &str) -> Result<serde_json::Value> {
    let client = reqwest::Client::new();
    let url = format!("{endpoint}/invoke");
    let response = client
        .post(&url)
        .json(&serde_json::json!({ "message": message, "input": { "message": message } }))
        .send()
        .await
        .with_context(|| format!("POST {url}"))?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        bail!("local invoke failed with HTTP {status}: {body}");
    }
    serde_json::from_str(&body)
        .with_context(|| format!("local invoke returned non-JSON body: {body}"))
}

fn format_command(parts: &[String]) -> String {
    parts
        .iter()
        .map(|part| {
            if part
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || "-_/.:=".contains(ch))
            {
                part.clone()
            } else {
                format!("'{}'", part.replace('\'', "'\\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn image_name_component(value: &str) -> String {
    let normalized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if normalized.is_empty() {
        "agent".to_string()
    } else {
        normalized
    }
}

fn local_image_suffix() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{}-{}", std::process::id(), nanos)
}

async fn package(ctx: &AppContext, args: AgentPackageArgs) -> Result<()> {
    let source = AgentSourceContext::discover(ctx, &args.source)?;
    let extra_excludes = source
        .project
        .as_ref()
        .or(ctx.project.as_ref())
        .map(|project| project.exclude.clone())
        .unwrap_or_default();

    if args.dry_run {
        let size = archive::estimate_archive_size(&source.root, &extra_excludes)?;
        output::info(
            ctx.output,
            format!(
                "Estimated uncompressed archive size: {:.1} KB ({size} bytes)",
                size as f64 / 1024.0
            ),
        );
        return Ok(());
    }

    let bytes = archive::create_source_archive(&source.root, &extra_excludes)?;
    let out_path = args.output.unwrap_or_else(|| {
        source
            .root
            .file_name()
            .map(|name| PathBuf::from(format!("{}.tar.gz", name.to_string_lossy())))
            .unwrap_or_else(|| PathBuf::from("agent.tar.gz"))
    });
    fs::write(&out_path, &bytes)?;
    output::success(
        ctx.output,
        format!(
            "Agent archive written to {} ({:.1} KB)",
            out_path.display(),
            bytes.len() as f64 / 1024.0
        ),
    );
    Ok(())
}

async fn dockerfile(ctx: &AppContext, args: AgentDockerfileArgs) -> Result<()> {
    let source = AgentSourceContext::discover(ctx, &args.source)?;
    let project = source.project.as_ref();
    let framework = resolve_framework(args.framework, project, &source.inferred)?;
    let python_version = args
        .python_version
        .or_else(|| project.and_then(|project| project.python_version.clone()))
        .or_else(|| source.inferred.python_version.clone())
        .unwrap_or_else(|| "3.12".to_string());
    let ports = resolve_ports(Vec::new(), project, &source.inferred);
    let target = resolve_agent_target(args.target.clone(), project, &source.inferred);
    let protocols = resolve_protocols(&[], project, &source.inferred)?;
    let native_entrypoint = framework
        .as_deref()
        .filter(|framework| is_native_cli_serve_framework(framework))
        .map(|framework| {
            let port = ports.first().copied().unwrap_or(8000);
            native_runtime_command(
                AGENT_RUNTIME_BINARY,
                framework,
                "/app",
                "0.0.0.0",
                port,
                target.as_deref(),
                &protocols,
            )
            .join(" ")
        });
    let entrypoint = configured_entrypoint(project)
        .or(native_entrypoint)
        .or_else(|| source.inferred.entrypoint.clone())
        .unwrap_or_else(|| "python -m main".to_string());
    let dockerfile = generate_local_dockerfile(
        framework.as_deref(),
        &python_version,
        &entrypoint,
        &ports,
        &protocols,
        source.root.join("requirements.txt").is_file(),
        source.root.join("pyproject.toml").is_file(),
    );

    if let Some(path) = args.output {
        fs::write(&path, dockerfile)?;
        output::success(
            ctx.output,
            format!("Dockerfile written to {}", path.display()),
        );
    } else {
        print!("{dockerfile}");
    }
    Ok(())
}

fn is_native_cli_serve_framework(framework: &str) -> bool {
    matches!(
        framework::canonical_framework(framework),
        Some("langgraph" | "langchain" | "google-adk")
    )
}

/// Local `agent dev` command for native frameworks — same process as production deploy.
fn native_autoserve_dev_command(
    framework: &str,
    root: &str,
    host: &str,
    port: u16,
    target: Option<&str>,
    protocols: &[String],
) -> Vec<String> {
    let canonical_framework = framework::canonical_framework(framework).unwrap_or(framework);
    let mut command = vec![
        "python".to_string(),
        "-m".to_string(),
        "gravixlayer.runtime.autoserve".to_string(),
        "--framework".to_string(),
        canonical_framework.to_string(),
        "--root".to_string(),
        root.to_string(),
        "--host".to_string(),
        host.to_string(),
        "--port".to_string(),
        port.to_string(),
    ];
    if !protocols.is_empty() {
        command.push("--protocols".to_string());
        command.push(protocols.join(","));
    }
    if let Some(target) = target.filter(|target| !target.trim().is_empty()) {
        command.push("--target".to_string());
        command.push(target.to_string());
    }
    command
}

#[cfg(test)]
fn native_serve_command(
    binary: &str,
    framework: &str,
    source: &str,
    host: &str,
    port: u16,
    target: Option<&str>,
    protocols: &[String],
) -> Vec<String> {
    let canonical_framework = framework::canonical_framework(framework).unwrap_or(framework);
    let mut command = vec![
        binary.to_string(),
        "agent".to_string(),
        "serve".to_string(),
        source.to_string(),
        "--framework".to_string(),
        canonical_framework.to_string(),
        "--host".to_string(),
        host.to_string(),
        "--port".to_string(),
        port.to_string(),
    ];
    if !protocols.is_empty() {
        command.push("--protocols".to_string());
        command.push(protocols.join(","));
    }
    if let Some(target) = target.filter(|target| !target.trim().is_empty()) {
        command.push("--target".to_string());
        command.push(target.to_string());
    }
    command
}

fn native_runtime_command(
    binary: &str,
    framework: &str,
    root: &str,
    host: &str,
    port: u16,
    target: Option<&str>,
    protocols: &[String],
) -> Vec<String> {
    let canonical_framework = framework::canonical_framework(framework).unwrap_or(framework);
    let mut command = vec![
        binary.to_string(),
        "--framework".to_string(),
        canonical_framework.to_string(),
        "--root".to_string(),
        root.to_string(),
        "--host".to_string(),
        host.to_string(),
        "--port".to_string(),
        port.to_string(),
    ];
    if !protocols.is_empty() {
        command.push("--protocols".to_string());
        command.push(protocols.join(","));
    }
    if let Some(target) = target.filter(|target| !target.trim().is_empty()) {
        command.push("--target".to_string());
        command.push(target.to_string());
    }
    command
}

#[allow(dead_code)] // retained for direct `gravixlayer agent serve` callers/tests
fn current_cli_binary() -> String {
    std::env::current_exe()
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| "gravixlayer".to_string())
}

fn resolve_agent_target(
    cli_target: Option<String>,
    project: Option<&GravixlayerProject>,
    inferred: &InferredAgentProject,
) -> Option<String> {
    cli_target
        .or_else(|| project.and_then(|project| project.target.clone()))
        .or_else(|| inferred.target.clone())
}

fn native_runtime_entrypoint(
    framework: &Option<String>,
    target: Option<&str>,
    ports: &[u16],
    protocols: &[String],
) -> Option<String> {
    let Some(framework) = framework.as_deref() else {
        return None;
    };
    if !is_native_cli_serve_framework(framework) {
        return None;
    }
    let port = ports.first().copied().unwrap_or(8000);
    Some(
        native_runtime_command(
            AGENT_RUNTIME_BINARY,
            framework,
            "/app",
            "0.0.0.0",
            port,
            target,
            protocols,
        )
        .join(" "),
    )
}

fn generate_local_dockerfile(
    framework: Option<&str>,
    python_version: &str,
    entrypoint: &str,
    ports: &[u16],
    protocols: &[String],
    has_requirements: bool,
    has_pyproject: bool,
) -> String {
    let mut lines = vec![
        format!(
            "FROM ghcr.io/astral-sh/uv:python{}-bookworm-slim",
            python_version
        ),
        "WORKDIR /app".to_string(),
        "ENV UV_SYSTEM_PYTHON=1 UV_COMPILE_BYTECODE=1 PYTHONUNBUFFERED=1 PYTHONDONTWRITEBYTECODE=1"
            .to_string(),
        String::new(),
    ];

    if has_requirements {
        lines.push("COPY requirements.txt requirements.txt".to_string());
        lines.push("RUN uv pip install -r requirements.txt".to_string());
    } else if has_pyproject {
        lines.push("COPY pyproject.toml pyproject.toml".to_string());
        lines.push("COPY uv.lock* ./".to_string());
        lines.push("RUN uv pip compile pyproject.toml -q -o /tmp/requirements.txt && uv pip install -r /tmp/requirements.txt".to_string());
    }

    lines.push("RUN uv pip install 'gravixlayer[runtime]>=0.1.51'".to_string());
    lines.push("RUN python -m gravixlayer.runtime.autoserve --help >/dev/null".to_string());
    match framework {
        Some("langgraph") => {
            lines.push("RUN uv pip install langgraph langgraph-checkpoint".to_string())
        }
        Some("langchain") => lines.push("RUN uv pip install langchain".to_string()),
        Some("google-adk") => {
            lines.push("RUN uv pip install 'google-adk[a2a]' 'protobuf<7'".to_string())
        }
        _ => {}
    }
    if protocols.iter().any(|protocol| protocol == "a2a")
        || matches!(framework, Some("langgraph" | "langchain" | "google-adk"))
    {
        lines.push("RUN uv pip install 'a2a-sdk[http-server]>=1.0.0' 'protobuf<7'".to_string());
    }
    lines.push(String::new());
    lines.push("COPY . .".to_string());
    lines.push("RUN uv pip install --no-deps . 2>/dev/null || true".to_string());
    lines.push(String::new());
    let exposed = if ports.is_empty() {
        "8000".to_string()
    } else {
        ports
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    };
    lines.push(format!("EXPOSE {exposed}"));
    lines.push("RUN useradd -m -u 1000 agent && chown -R agent:agent /app".to_string());
    lines.push("USER agent".to_string());
    let cmd_parts = shell_words::split(entrypoint)
        .unwrap_or_else(|_| entrypoint.split_whitespace().map(str::to_string).collect());
    let cmd_json = cmd_parts
        .iter()
        .map(|part| serde_json::to_string(part).unwrap_or_else(|_| "\"\"".to_string()))
        .collect::<Vec<_>>()
        .join(", ");
    lines.push(format!("CMD [{cmd_json}]"));
    lines.push(String::new());
    lines.join("\n")
}

/// Inner dev session — separated so the outer function can always run cleanup.
async fn dev_session(
    ctx: &AppContext,
    runtime_id: &str,
    source: &std::path::Path,
    args: &AgentDevArgs,
) -> Result<()> {
    use notify::{Config as NotifyConfig, Event, RecommendedWatcher, RecursiveMode, Watcher};
    use std::sync::mpsc;

    // 2. Wait until running
    {
        let sp = output::Spinner::new("Waiting for runtime to start…");
        let deadline = Instant::now() + Duration::from_secs(120);
        ctx.api
            .runtime()
            .wait_until_running(runtime_id, deadline)
            .await?;
        sp.finish_ok("Runtime is running");
    }

    // 3. Upload source files
    {
        let spinner = output::Spinner::new("Uploading source…");
        upload_project_files(ctx, runtime_id, source).await?;
        spinner.finish_ok("Source uploaded");
    }

    // 4. Set up file watcher
    let watch_path = source.join(&args.watch_dir);
    if !watch_path.exists() {
        output::warn(&format!(
            "Watch directory not found: {} — watching project root instead",
            watch_path.display()
        ));
    }
    let watch_target = if watch_path.exists() {
        watch_path.clone()
    } else {
        source.to_path_buf()
    };

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher =
        RecommendedWatcher::new(tx, NotifyConfig::default()).context("create file watcher")?;
    watcher
        .watch(&watch_target, RecursiveMode::Recursive)
        .context("start watching")?;

    output::info(ctx.output, format!("  Runtime ID  : {runtime_id}"));
    output::info(ctx.output, "  Watching for changes… (Ctrl+C to stop)");
    println!();

    // 5. Main event loop — exits on Ctrl+C only; cleanup is done by the caller.
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);

    loop {
        tokio::select! {
            _ = &mut ctrl_c => {
                println!();
                output::info(ctx.output, "Shutting down dev session…");
                return Ok(());
            }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                // Drain all pending FS events
                let mut changed_files: Vec<PathBuf> = Vec::new();
                while let Ok(event_res) = rx.try_recv() {
                    if let Ok(event) = event_res {
                        for path in event.paths {
                            if path.is_file() {
                                changed_files.push(path);
                            }
                        }
                    }
                }

                if !changed_files.is_empty() {
                    // Deduplicate
                    changed_files.sort();
                    changed_files.dedup();

                    for path in &changed_files {
                        let rel = path.strip_prefix(source).unwrap_or(path);
                        let remote = format!("/{}", rel.to_string_lossy().replace('\\', "/"));
                        match ctx
                            .api
                            .runtime_files()
                            .upload(runtime_id, path, &remote, None, None)
                            .await
                        {
                            Ok(_) => output::info(ctx.output, format!("  ↑ {}", rel.display())),
                            Err(e) => output::warn(&format!("  upload failed ({}): {e}", rel.display())),
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Destroy
// ---------------------------------------------------------------------------

async fn destroy(ctx: &AppContext, args: AgentDestroyArgs) -> Result<()> {
    super::validate_resource_id(&args.id, "agent")?;
    if !args.yes {
        eprint!("Destroy agent {}? [y/N] ", args.id);
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }
    let resp = ctx.api.agent().destroy(&args.id).await?;
    output::print_or_json(ctx.output, &resp, || {
        output::success(
            ctx.output,
            format!(
                "Agent {} destroyed: {}",
                args.id,
                resp.message.as_deref().unwrap_or("ok")
            ),
        );
    });
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_env_vars(pairs: &[String]) -> Result<HashMap<String, String>> {
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

fn build_agent_invocation_payload(
    raw_input: Option<&str>,
    message: Option<&str>,
    session_id: Option<&str>,
    resume: Option<&str>,
    raw_metadata: Option<&str>,
) -> Result<serde_json::Value> {
    let input = if let Some(input) = raw_input {
        Some(
            serde_json::from_str::<serde_json::Value>(input)
                .map_err(|e| anyhow::anyhow!("--input is not valid JSON: {e}"))?,
        )
    } else {
        message.map(|msg| serde_json::json!({ "message": msg }))
    };

    if input.is_none() && resume.is_none() {
        bail!("provide --input JSON, --message text, or --resume value");
    }

    let mut payload = serde_json::Map::new();
    if let Some(input) = input {
        payload.insert("input".into(), input);
    }
    if let Some(session_id) = session_id {
        payload.insert("session_id".into(), session_id.into());
    }
    if let Some(resume) = resume {
        let resume_value = serde_json::from_str::<serde_json::Value>(resume)
            .unwrap_or_else(|_| serde_json::Value::String(resume.to_string()));
        payload.insert("resume".into(), resume_value);
    }

    if let Some(metadata) = raw_metadata {
        let metadata_value: serde_json::Value = serde_json::from_str(metadata)
            .map_err(|e| anyhow::anyhow!("--metadata is not valid JSON: {e}"))?;
        if !metadata_value.is_object() {
            bail!("--metadata must be a JSON object");
        }
        payload.insert("metadata".into(), metadata_value);
    }
    Ok(serde_json::Value::Object(payload))
}

/// Upload all non-excluded files from a project source dir into the runtime.
async fn upload_project_files(
    ctx: &AppContext,
    runtime_id: &str,
    source: &std::path::Path,
) -> Result<()> {
    use walkdir::WalkDir;

    let excluded = archive::BUILTIN_EXCLUDES;
    for entry in WalkDir::new(source).into_iter().filter_entry(|e| {
        let name = e.file_name().to_string_lossy();
        !excluded.iter().any(|pat| {
            if pat.starts_with('*') {
                name.ends_with(pat.trim_start_matches('*'))
            } else {
                name == *pat
            }
        })
    }) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let rel = path.strip_prefix(source).unwrap_or(path);
        let remote = format!("/{}", rel.to_string_lossy().replace('\\', "/"));
        ctx.api
            .runtime_files()
            .upload(runtime_id, path, &remote, None, None)
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        build_agent_invocation_payload, discover_agent_project_root, generate_local_dockerfile,
        infer_agent_project, native_runtime_entrypoint, native_serve_command,
        python_module_for_file, resolve_deploy_http_port, resolve_http_port,
        resolve_local_dev_command, resolve_ports, AgentSourceContext, InferredAgentProject,
    };
    use crate::cli::AgentDevArgs;
    use tempfile::TempDir;

    #[test]
    fn accepts_root_project_marker() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("requirements.txt"), "").unwrap();

        let root = discover_agent_project_root(dir.path()).unwrap();
        assert_eq!(root, dir.path());
    }

    #[test]
    fn discovers_nested_project_marker() {
        let dir = TempDir::new().unwrap();
        let app_dir = dir.path().join("examples").join("agent-app");
        std::fs::create_dir_all(&app_dir).unwrap();
        std::fs::write(
            app_dir.join("pyproject.toml"),
            "[project]\nname = \"agent-app\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let root = discover_agent_project_root(dir.path()).unwrap();
        assert_eq!(root, app_dir);
    }

    #[test]
    fn rejects_source_without_project_marker() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("app.py"), "print('ok')\n").unwrap();

        let err = discover_agent_project_root(dir.path()).unwrap_err();
        assert!(err.to_string().contains(
            "expected Dockerfile, requirements.txt, pyproject.toml, setup.py, or langgraph.json"
        ));
    }

    #[test]
    fn infers_a2a_langgraph_project_defaults() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("langgraph.json"),
            r#"{"python_version":"3.13","graphs":{"agent":"simple_agent.graph:graph"}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("requirements.txt"),
            "langgraph>=1\na2a-sdk>=1.0.0\n",
        )
        .unwrap();
        let app_dir = dir.path().join("simple_agent");
        std::fs::create_dir(&app_dir).unwrap();
        std::fs::write(
            app_dir.join("app.py"),
            "from gravixlayer.a2a import run_a2a\nrun_a2a(executor=None, agent_card=None)\n",
        )
        .unwrap();

        let inferred = infer_agent_project(dir.path()).unwrap();
        assert_eq!(inferred.framework.as_deref(), Some("langgraph"));
        assert_eq!(inferred.python_version.as_deref(), Some("3.13"));
        assert_eq!(
            inferred.entrypoint.as_deref(),
            Some("python -m simple_agent.app")
        );
        assert_eq!(inferred.protocols, vec!["http", "a2a"]);
    }

    #[test]
    fn does_not_infer_a2a_from_dependency_without_server_entrypoint() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("langgraph.json"),
            r#"{"python_version":"3.13","graphs":{"agent":"simple_agent.graph:graph"}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("requirements.txt"),
            "langgraph>=1\na2a-sdk>=1.0.0\n",
        )
        .unwrap();

        let inferred = infer_agent_project(dir.path()).unwrap();
        assert_eq!(inferred.framework.as_deref(), Some("langgraph"));
        assert!(inferred.entrypoint.is_none());
        assert!(inferred.protocols.is_empty());
    }

    #[test]
    fn converts_python_file_to_module() {
        let dir = TempDir::new().unwrap();
        let app_dir = dir.path().join("simple_agent");
        std::fs::create_dir(&app_dir).unwrap();
        let app_py = app_dir.join("app.py");
        std::fs::write(&app_py, "").unwrap();

        assert_eq!(
            python_module_for_file(dir.path(), &app_py).as_deref(),
            Some("simple_agent.app")
        );
    }

    #[test]
    fn native_framework_local_dev_uses_autoserve_before_main_py() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("requirements.txt"), "langchain>=1\n").unwrap();
        std::fs::write(dir.path().join("main.py"), "print('sample cli')\n").unwrap();

        let source = AgentSourceContext {
            root: dir.path().to_path_buf(),
            project: None,
            inferred: infer_agent_project(dir.path()).unwrap(),
        };
        let args = AgentDevArgs {
            source: dir.path().to_path_buf(),
            message: None,
            host: "127.0.0.1".to_string(),
            port: 8080,
            no_reload: false,
            target: None,
            runtime_sync: false,
            cloud: "azure".to_string(),
            region: "eastus2".to_string(),
            watch_dir: "app".to_string(),
        };

        let protocols = vec!["http".to_string()];
        let command = resolve_local_dev_command(&source, &args, &protocols).unwrap();

        assert_eq!(command[0], "python");
        assert_eq!(command[1], "-m");
        assert_eq!(command[2], "gravixlayer.runtime.autoserve");
        assert!(command.contains(&"langchain".to_string()));
        assert!(command.contains(&"--protocols".to_string()));
        assert!(command.contains(&"http".to_string()));
    }

    #[test]
    fn native_framework_dockerfile_uses_sdk_runtime_entrypoint() {
        let protocols = vec!["http".to_string(), "a2a".to_string()];
        let entrypoint =
            native_runtime_entrypoint(&Some("langgraph".to_string()), None, &[8080], &protocols)
                .unwrap();
        let dockerfile = generate_local_dockerfile(
            Some("langgraph"),
            "3.12",
            &entrypoint,
            &[8080],
            &protocols,
            true,
            true,
        );

        assert!(dockerfile.contains("CMD [\"python\", \"-m\", \"gravixlayer.runtime.autoserve\""));
        assert!(dockerfile.contains("gravixlayer[runtime]>=0.1.51"));
        assert!(dockerfile.contains("python -m gravixlayer.runtime.autoserve --help"));
        assert!(dockerfile.contains("a2a-sdk[http-server]"));
        assert!(!dockerfile.contains("gravixlayer-cli/main/scripts/install.sh"));
        assert!(!dockerfile.contains("\"agent\", \"serve\""));
    }

    #[test]
    fn resolve_ports_defaults_to_8000_when_unset() {
        let inferred = InferredAgentProject::default();
        assert_eq!(resolve_ports(Vec::new(), None, &inferred), vec![8000]);
    }

    #[test]
    fn resolve_http_port_prefers_cli_then_ports_then_default() {
        assert_eq!(resolve_http_port(Some(9000), None, &[8000]), 9000);
        assert_eq!(resolve_http_port(None, None, &[9000]), 9000);
        assert_eq!(resolve_http_port(None, None, &[]), 8000);
    }

    #[test]
    fn resolve_deploy_http_port_omits_on_template_only() {
        // Template-only must not invent a port — CP owns template.http_port.
        assert_eq!(resolve_deploy_http_port(false, None, None, &[]), None);
        assert_eq!(resolve_deploy_http_port(false, None, None, &[9000]), None);
        assert_eq!(
            resolve_deploy_http_port(false, Some(9000), None, &[]),
            Some(9000)
        );
        // Build+deploy always declares the port baked into this build.
        assert_eq!(
            resolve_deploy_http_port(true, None, None, &[9000]),
            Some(9000)
        );
        assert_eq!(resolve_deploy_http_port(true, None, None, &[]), Some(8000));
    }

    #[test]
    fn resolve_deploy_http_port_ignores_project_port_on_template_only() {
        let mut project = crate::config::project::GravixlayerProject::default();
        project.port = Some(8000);
        // cwd gravixlayer.json must not override template.http_port.
        assert_eq!(
            resolve_deploy_http_port(false, None, Some(&project), &[9000]),
            None
        );
        project.port = Some(9000);
        assert_eq!(
            resolve_deploy_http_port(false, None, Some(&project), &[]),
            None
        );
        // Explicit flag still wins on template-only.
        assert_eq!(
            resolve_deploy_http_port(false, Some(7000), Some(&project), &[]),
            Some(7000)
        );
        // From-source may use project.port.
        assert_eq!(
            resolve_deploy_http_port(true, None, Some(&project), &[8000]),
            Some(9000)
        );
    }

    #[test]
    fn native_runtime_entrypoint_supports_langchain_without_target() {
        let entrypoint =
            native_runtime_entrypoint(&Some("langchain".to_string()), None, &[8000], &[]);

        assert_eq!(
            entrypoint.as_deref(),
            Some("python -m gravixlayer.runtime.autoserve --framework langchain --root /app --host 0.0.0.0 --port 8000")
        );
    }

    #[test]
    fn native_serve_command_forwards_target_for_langchain() {
        let command = native_serve_command(
            "gravixlayer",
            "langchain",
            ".",
            "0.0.0.0",
            8000,
            Some("app.chain:chain"),
            &[],
        );

        assert!(command.contains(&"--target".to_string()));
        assert!(command.contains(&"app.chain:chain".to_string()));
    }

    #[test]
    fn native_serve_command_forwards_target_for_google_adk() {
        let command = native_serve_command(
            "gravixlayer",
            "google-adk",
            ".",
            "0.0.0.0",
            8000,
            Some("agent.root_agent"),
            &[],
        );

        assert!(command.contains(&"--target".to_string()));
        assert!(command.contains(&"agent.root_agent".to_string()));
    }

    #[test]
    fn invocation_payload_resume_accepts_json_objects() {
        let payload = build_agent_invocation_payload(
            None,
            None,
            Some("thread-1"),
            Some(r#"{"decisions":[{"type":"approve"}]}"#),
            None,
        )
        .unwrap();

        assert_eq!(payload["session_id"], "thread-1");
        assert_eq!(payload["resume"]["decisions"][0]["type"], "approve");
    }

    #[test]
    fn native_langgraph_dev_can_select_named_graph_target() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("langgraph.json"),
            r#"{"graphs":{"agent":"./src/agent.py:agent","deep_agent":"./src/deep_agent/agent.py:make_graph"}}"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("requirements.txt"), "langgraph>=1.0\n").unwrap();

        let source = AgentSourceContext {
            root: dir.path().to_path_buf(),
            project: None,
            inferred: infer_agent_project(dir.path()).unwrap(),
        };
        let args = AgentDevArgs {
            source: dir.path().to_path_buf(),
            message: None,
            host: "127.0.0.1".to_string(),
            port: 8080,
            no_reload: false,
            target: Some("deep_agent".to_string()),
            runtime_sync: false,
            cloud: "azure".to_string(),
            region: "eastus2".to_string(),
            watch_dir: "app".to_string(),
        };

        let protocols = vec!["http".to_string(), "a2a".to_string()];
        let command = resolve_local_dev_command(&source, &args, &protocols).unwrap();

        assert!(command.contains(&"gravixlayer.runtime.autoserve".to_string()));
        assert!(command.contains(&"--target".to_string()));
        assert!(command.contains(&"deep_agent".to_string()));
        assert!(command.contains(&"--protocols".to_string()));
        assert!(command.contains(&"http,a2a".to_string()));
    }
}
