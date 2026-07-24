use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use a2a::{
    new_message_id, A2AError, AgentCapabilities, AgentCard, AgentInterface, AgentProvider,
    AgentSkill, Message, Part, PartContent, Role, StreamResponse, Task, TaskState, TaskStatus,
    TaskStatusUpdateEvent, TRANSPORT_PROTOCOL_HTTP_JSON, TRANSPORT_PROTOCOL_JSONRPC,
};
use a2a_server::{
    AgentExecutor, DefaultRequestHandler, ExecutorContext, InMemoryTaskStore, StaticAgentCard,
};
use anyhow::{bail, Context, Result};
use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use futures::stream::{self, BoxStream};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinHandle;
use tokio::time::{timeout, Duration};

use crate::cli::{AgentProtocolArg, AgentServeArgs};

const PYTHON_PROBE_TIMEOUT_SECS: u64 = 3;

const PYTHON_FRAMEWORK_RUNNER: &str = r##"
import asyncio
import importlib
import importlib.util
import inspect
import json
import os
import sys
import traceback
from pathlib import Path


_PROTOCOL_STDOUT = sys.stdout
sys.stdout = sys.stderr


def send_protocol(value):
    _PROTOCOL_STDOUT.write(json.dumps(value) + "\n")
    _PROTOCOL_STDOUT.flush()


def load_env(root):
    for env_path in (root / ".env", root / "gravixlayer" / ".env.local"):
        if not env_path.is_file():
            continue
        for raw in env_path.read_text(encoding="utf-8").splitlines():
            line = raw.strip()
            if not line or line.startswith("#") or "=" not in line:
                continue
            key, value = line.split("=", 1)
            key = key.strip()
            if not key or key in os.environ:
                continue
            os.environ[key] = value.strip().strip('"').strip("'")


def prepare_import_path(root):
    sys.path.insert(0, str(root))
    src = root / "src"
    if src.is_dir():
        sys.path.insert(0, str(src))


def resolve_langgraph_target(root, target):
    config_path = root / "langgraph.json"
    graphs = {}
    if config_path.is_file():
        graphs = json.loads(config_path.read_text(encoding="utf-8")).get("graphs") or {}
    if target and target in graphs:
        value = graphs[target]
        return value.get("path") if isinstance(value, dict) else value
    if target and ":" in target:
        return target
    if graphs:
        value = next(iter(graphs.values()))
        return value.get("path") if isinstance(value, dict) else value
    return find_target(root, ("agent.graph", "graph", "app.graph", "main"), ("graph", "app", "agent", "make_graph", "create_graph", "build_graph"), {"graph.py", "agent.py", "main.py"})


def module_names_for_file(root, file_path):
    relative = file_path.relative_to(root).with_suffix("")
    parts = [part for part in relative.parts if part != "__init__"]
    if not parts:
        return []
    names = [".".join(parts)]
    if parts[0] == "src" and len(parts) > 1:
        names.append(".".join(parts[1:]))
    return names


def iter_python_files(root):
    skip = {".git", ".venv", "venv", "env", "__pycache__", ".pytest_cache", "node_modules", "dist", "build"}
    for current, dirs, files in os.walk(root):
        dirs[:] = [name for name in dirs if name not in skip]
        for name in files:
            if name.endswith(".py"):
                yield Path(current) / name


def load_module_attr(module_name, attrs):
    try:
        module = importlib.import_module(module_name)
    except Exception:
        return None
    for attr in attrs:
        if hasattr(module, attr):
            return module, attr, getattr(module, attr)
    return None


def find_target(root, modules, attrs, filenames):
    for module_name in modules:
        loaded = load_module_attr(module_name, attrs)
        if loaded is not None:
            module, attr, _ = loaded
            return f"{module.__name__}:{attr}"
    for file_path in iter_python_files(root):
        if file_path.name not in filenames:
            continue
        for module_name in module_names_for_file(root, file_path):
            loaded = load_module_attr(module_name, attrs)
            if loaded is not None:
                module, attr, _ = loaded
                return f"{module.__name__}:{attr}"
    return None


def load_object(root, target):
    if not target or ":" not in target:
        raise RuntimeError("agent target must be module:attribute")
    module_name, attr = target.split(":", 1)
    if module_name.endswith(".py") or "/" in module_name or "\\" in module_name:
        file_path = (root / module_name).resolve()
        if not file_path.is_file():
            raise RuntimeError(f"target file does not exist: {file_path}")
        generated_name = "_gravix_target_" + "_".join(file_path.with_suffix("").parts[-4:]).replace("-", "_")
        spec = importlib.util.spec_from_file_location(generated_name, file_path)
        if spec is None or spec.loader is None:
            raise RuntimeError(f"could not import target file: {file_path}")
        module = importlib.util.module_from_spec(spec)
        sys.modules[generated_name] = module
        spec.loader.exec_module(module)
        return getattr(module, attr), generated_name
    module = importlib.import_module(module_name)
    return getattr(module, attr), module_name


def maybe_compile_with_checkpointer(obj):
    if getattr(obj, "checkpointer", None) is not None:
        return obj
    builder = getattr(obj, "builder", None)
    compile_fn = getattr(builder, "compile", None)
    if not callable(compile_fn):
        return obj
    try:
        from langgraph.checkpoint.memory import MemorySaver
    except Exception:
        return obj
    try:
        signature = inspect.signature(compile_fn)
    except Exception:
        return compile_fn(checkpointer=MemorySaver())
    if "checkpointer" in signature.parameters or any(
        parameter.kind == inspect.Parameter.VAR_KEYWORD
        for parameter in signature.parameters.values()
    ):
        return compile_fn(checkpointer=MemorySaver())
    return obj


def to_jsonable(value):
    if value is None or isinstance(value, (str, int, float, bool)):
        return value
    if isinstance(value, dict):
        return {str(k): to_jsonable(v) for k, v in value.items()}
    if isinstance(value, (list, tuple)):
        return [to_jsonable(v) for v in value]
    if hasattr(value, "model_dump"):
        return to_jsonable(value.model_dump())
    if hasattr(value, "dict"):
        return to_jsonable(value.dict())
    if hasattr(value, "content"):
        return {"type": value.__class__.__name__, "content": to_jsonable(value.content)}
    return str(value)


def output_text(result):
    if isinstance(result, dict):
        messages = result.get("messages")
        if messages:
            last = messages[-1]
            content = getattr(last, "content", None)
            if content is not None:
                return content if isinstance(content, str) else json.dumps(to_jsonable(content))
            if isinstance(last, dict) and "content" in last:
                content = last["content"]
                return content if isinstance(content, str) else json.dumps(to_jsonable(content))
        if "output" in result:
            return str(result["output"])
        if "response" in result:
            return str(result["response"])
    return json.dumps(to_jsonable(result))


def extract_prompt(value):
    if isinstance(value, str):
        return value
    if isinstance(value, dict):
        for key in ("message", "prompt", "text", "input"):
            candidate = value.get(key)
            if isinstance(candidate, str):
                return candidate
        messages = value.get("messages")
        if isinstance(messages, list) and messages:
            last = messages[-1]
            if isinstance(last, dict) and isinstance(last.get("content"), str):
                return last["content"]
            content = getattr(last, "content", None)
            if isinstance(content, str):
                return content
    return str(value or "")


def graph_payload(request):
    input_value = request.get("input")
    if input_value is not None:
        if isinstance(input_value, dict):
            for key in ("message", "prompt", "text"):
                value = input_value.get(key)
                if isinstance(value, str):
                    return {"messages": [{"role": "user", "content": value}]}
        if isinstance(input_value, str):
            return {"messages": [{"role": "user", "content": input_value}]}
        return input_value
    return {"messages": [{"role": "user", "content": request.get("message") or ""}]}


async def invoke_graph(graph, payload, config):
    if hasattr(graph, "ainvoke"):
        return await graph.ainvoke(payload, config=config)
    if hasattr(graph, "invoke"):
        return graph.invoke(payload, config=config)
    raise RuntimeError("LangGraph target does not expose invoke or ainvoke")


def resolve_langchain_target(root, target):
    if target and ":" in target:
        return target
    return find_target(root, ("agent", "chain", "app.agent", "main"), ("agent", "chain", "runnable", "app", "graph"), {"agent.py", "chain.py", "main.py", "graph.py"})


def langchain_payloads(request):
    input_value = request.get("input")
    if isinstance(input_value, dict):
        for key in ("message", "prompt", "text"):
            value = input_value.get(key)
            if isinstance(value, str):
                return dedupe([input_value, {"messages": [{"role": "user", "content": value}]}, {"input": value}, {"question": value}])
        return [input_value]
    text = input_value if isinstance(input_value, str) else request.get("message") or ""
    return dedupe([{"messages": [{"role": "user", "content": text}]}, {"input": text}, {"question": text}])


def dedupe(values):
    out = []
    seen = set()
    for value in values:
        marker = repr(value)
        if marker not in seen:
            out.append(value)
            seen.add(marker)
    return out


def is_payload_shape_error(exc):
    if isinstance(exc, KeyError):
        return True
    message = str(exc).lower()
    return any(marker in message for marker in ("field required", "input key", "input_keys", "missing required", "missing some input keys", "validation error"))


async def invoke_langchain(runnable, request):
    first_error = None
    payloads = langchain_payloads(request)
    for index, payload in enumerate(payloads):
        try:
            config = {"configurable": {"thread_id": request.get("session_id") or "default"}}
            if hasattr(runnable, "ainvoke"):
                return await runnable.ainvoke(payload, config=config)
            if hasattr(runnable, "invoke"):
                return runnable.invoke(payload, config=config)
            if callable(runnable):
                result = runnable(payload)
                if inspect.isawaitable(result):
                    return await result
                return result
            raise RuntimeError("LangChain target does not expose invoke, ainvoke, or callable behavior")
        except Exception as exc:
            if first_error is None:
                first_error = exc
            if index == len(payloads) - 1 or not is_payload_shape_error(exc):
                raise
    raise first_error or RuntimeError("LangChain invocation failed")


def resolve_google_adk_target(root, target):
    if target and ":" in target:
        return target
    return find_target(root, ("agent", "app.agent", "adk_agent", "main"), ("root_agent", "app", "agent"), {"agent.py", "adk_agent.py", "main.py"})


def adk_app_name(module_name, target):
    if module_name:
        parts = [part for part in module_name.split(".") if part]
        if len(parts) >= 2 and parts[-1] in {"agent", "adk_agent"}:
            return parts[-2]
    name = getattr(target, "name", None)
    return str(name) if name else "adk-agent"


async def maybe_call(value):
    if inspect.isawaitable(value):
        return await value
    return value


async def create_adk_session(service, app_name, user_id, session_id):
    kwargs = {"app_name": app_name, "user_id": user_id, "state": {}}
    if session_id:
        kwargs["session_id"] = session_id
    signature = inspect.signature(service.create_session)
    accepts_kwargs = any(param.kind == inspect.Parameter.VAR_KEYWORD for param in signature.parameters.values())
    filtered = {key: value for key, value in kwargs.items() if accepts_kwargs or key in signature.parameters}
    try:
        return await maybe_call(service.create_session(**filtered))
    except TypeError:
        filtered.pop("session_id", None)
        return await maybe_call(service.create_session(**filtered))


def text_part(part_cls, text):
    from_text = getattr(part_cls, "from_text", None)
    if callable(from_text):
        return from_text(text=text)
    return part_cls(text=text)


def build_runner_service_kwargs():
    kwargs = {}
    optional_services = (
        ("google.adk.artifacts.in_memory_artifact_service", "InMemoryArtifactService", "artifact_service"),
        ("google.adk.memory.in_memory_memory_service", "InMemoryMemoryService", "memory_service"),
        ("google.adk.auth.credential_service.in_memory_credential_service", "InMemoryCredentialService", "credential_service"),
    )
    for module_name, class_name, key in optional_services:
        try:
            cls = getattr(importlib.import_module(module_name), class_name)
            kwargs[key] = cls()
        except Exception:
            pass
    return kwargs


async def call_runner_async(runner, kwargs):
    run_async = runner.run_async
    signature = inspect.signature(run_async)
    accepts_kwargs = any(param.kind == inspect.Parameter.VAR_KEYWORD for param in signature.parameters.values())
    filtered = {key: value for key, value in kwargs.items() if accepts_kwargs or key in signature.parameters}
    async for event in run_async(**filtered):
        yield event


def iter_adk_text(event):
    parts = getattr(getattr(event, "content", None), "parts", None) or []
    for part in parts:
        text = getattr(part, "text", None)
        if isinstance(text, str) and text:
            yield text


async def invoke_google_adk(target, app_name, request):
    from google.adk.runners import Runner
    try:
        from google.adk.sessions import InMemorySessionService
    except ImportError:
        from google.adk.sessions.in_memory_session_service import InMemorySessionService
    from google.genai.types import Content, Part

    session_service = InMemorySessionService()
    root_agent = getattr(target, "root_agent", None)
    runner_kwargs = {"app_name": app_name, "session_service": session_service, **build_runner_service_kwargs()}
    runner_params = inspect.signature(Runner).parameters
    if root_agent is not None and "app" in runner_params:
        runner = Runner(app=target, **runner_kwargs)
    else:
        runner = Runner(agent=root_agent or target, **runner_kwargs)

    user_id = "gravix-user"
    session = await create_adk_session(session_service, app_name, user_id, request.get("session_id"))
    content = Content(parts=[text_part(Part, extract_prompt(request.get("input") if request.get("input") is not None else request.get("message")))], role="user")
    events = []
    text = []
    async for event in call_runner_async(runner, {"user_id": user_id, "session_id": session.id, "new_message": content}):
        events.append(to_jsonable(event))
        text.extend(iter_adk_text(event))
    return {"message": "".join(text), "raw": events}


class LangGraphRuntime:
    def __init__(self, root, target):
        resolved = resolve_langgraph_target(root, target)
        if not resolved:
            raise RuntimeError("could not resolve LangGraph target; add langgraph.json or pass --target module:attribute")
        graph, _ = load_object(root, resolved)
        self.graph = maybe_compile_with_checkpointer(graph)

    async def invoke(self, request):
        session_id = request.get("session_id") or "default"
        config = {"configurable": {"thread_id": session_id}}
        if "resume" in request and request["resume"] is not None:
            from langgraph.types import Command
            payload = Command(resume=request["resume"])
        else:
            payload = graph_payload(request)
        result = await invoke_graph(self.graph, payload, config)
        return {"message": output_text(result), "raw": to_jsonable(result)}


class LangChainRuntime:
    def __init__(self, root, target):
        resolved = resolve_langchain_target(root, target)
        if not resolved:
            raise RuntimeError("could not resolve LangChain target; expose agent, chain, runnable, app, or graph, or pass --target module:attribute")
        self.runnable, _ = load_object(root, resolved)

    async def invoke(self, request):
        result = await invoke_langchain(self.runnable, request)
        return {"message": output_text(result), "raw": to_jsonable(result)}


class GoogleADKRuntime:
    def __init__(self, root, target):
        resolved = resolve_google_adk_target(root, target)
        if not resolved:
            raise RuntimeError("could not resolve Google ADK target; expose root_agent, app, or agent, or pass --target module:attribute")
        adk_target, module_name = load_object(root, resolved)
        self.target = adk_target
        self.app_name = adk_app_name(module_name, adk_target)

        from google.adk.runners import Runner
        try:
            from google.adk.sessions import InMemorySessionService
        except ImportError:
            from google.adk.sessions.in_memory_session_service import InMemorySessionService
        from google.genai.types import Content, Part

        self.Content = Content
        self.Part = Part
        self.session_service = InMemorySessionService()
        root_agent = getattr(adk_target, "root_agent", None)
        runner_kwargs = {"app_name": self.app_name, "session_service": self.session_service, **build_runner_service_kwargs()}
        runner_params = inspect.signature(Runner).parameters
        if root_agent is not None and "app" in runner_params:
            self.runner = Runner(app=adk_target, **runner_kwargs)
        else:
            self.runner = Runner(agent=root_agent or adk_target, **runner_kwargs)

    async def invoke(self, request):
        user_id = "gravix-user"
        session = await create_adk_session(self.session_service, self.app_name, user_id, request.get("session_id"))
        prompt = extract_prompt(request.get("input") if request.get("input") is not None else request.get("message"))
        content = self.Content(parts=[text_part(self.Part, prompt)], role="user")
        events = []
        text = []
        async for event in call_runner_async(self.runner, {"user_id": user_id, "session_id": session.id, "new_message": content}):
            events.append(to_jsonable(event))
            text.extend(iter_adk_text(event))
        return {"message": "".join(text), "raw": events}


def build_runtime(root, framework, target):
    if framework == "langgraph":
        return LangGraphRuntime(root, target)
    if framework == "langchain":
        return LangChainRuntime(root, target)
    if framework == "google-adk":
        return GoogleADKRuntime(root, target)
    raise RuntimeError(f"unsupported framework: {framework}")


async def main():
    config_line = sys.stdin.readline()
    if not config_line:
        return
    try:
        config = json.loads(config_line)
        root = Path(config["root"]).resolve()
        load_env(root)
        prepare_import_path(root)
        framework = str(config.get("framework") or "").replace("_", "-")
        runtime = build_runtime(root, framework, config.get("target"))
        send_protocol({"ready": True})
    except Exception as exc:
        traceback.print_exc(file=sys.stderr)
        send_protocol({"ready": False, "error": str(exc), "error_type": exc.__class__.__name__})
        return

    for raw in sys.stdin:
        line = raw.strip()
        if not line:
            continue
        try:
            request = json.loads(line)
            response = await runtime.invoke(request)
            send_protocol({"ok": True, "response": response})
        except Exception as exc:
            traceback.print_exc(file=sys.stderr)
            send_protocol({"ok": False, "error": str(exc), "error_type": exc.__class__.__name__})


asyncio.run(main())
"##;

#[derive(Clone)]
pub struct PythonAgentBridge {
    pool: Arc<PythonWorkerPool>,
}

struct PythonWorkerPool {
    workers: Vec<Mutex<PythonWorker>>,
    limiter: Semaphore,
    next_worker: AtomicUsize,
}

#[derive(Debug)]
struct PythonWorkerConfig {
    source: PathBuf,
    framework: String,
    target: Option<String>,
    python: String,
}

struct PythonWorker {
    id: usize,
    config: Arc<PythonWorkerConfig>,
    request_timeout: Duration,
    start_timeout: Duration,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
    stderr_task: Option<JoinHandle<()>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PythonAgentOutput {
    message: String,
    #[serde(default)]
    raw: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct InvokeRequest {
    message: Option<String>,
    input: Option<Value>,
    session_id: Option<String>,
    resume: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct WorkerReadyResponse {
    ready: bool,
    error: Option<String>,
    error_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WorkerInvokeResponse {
    ok: bool,
    response: Option<PythonAgentOutput>,
    error: Option<String>,
    error_type: Option<String>,
}

#[derive(Debug)]
struct AgentInvocationError(String);

impl std::fmt::Display for AgentInvocationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for AgentInvocationError {}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct InvokeResponse {
    message: String,
    output: String,
    raw: Value,
}

pub async fn serve(args: AgentServeArgs) -> Result<()> {
    let source = args
        .source
        .canonicalize()
        .with_context(|| format!("resolve source directory {}", args.source.display()))?;
    if !source.is_dir() {
        bail!("source is not a directory: {}", source.display());
    }

    let framework = match args.framework {
        Some(value) => value.to_string(),
        None => {
            let detected = crate::cmd::agent::infer_agent_project(&source)?
                .framework
                .ok_or_else(|| anyhow::anyhow!(
                    "could not auto-detect framework from {}; pass --framework <langgraph|langchain|google-adk>",
                    source.display()
                ))?;
            println!("Detected framework: {detected}");
            detected
        }
    };
    if !matches!(framework.as_str(), "langgraph" | "langchain" | "google-adk") {
        bail!("agent serve supports langgraph, langchain, and google-adk projects (got '{framework}')")
    }

    let protocols = normalize_protocols(&args.protocols, args.protocols_csv.as_deref());
    if protocols.iter().any(|protocol| protocol == "mcp") {
        bail!("agent serve currently supports http and a2a protocols; mcp requires a dedicated MCP server")
    }
    if args.workers == 0 || args.workers > 64 {
        bail!("--workers must be between 1 and 64")
    }
    if args.request_timeout_secs == 0 {
        bail!("--request-timeout-secs must be greater than 0")
    }
    if args.worker_start_timeout_secs == 0 {
        bail!("--worker-start-timeout-secs must be greater than 0")
    }
    let public_url = args
        .public_url
        .unwrap_or_else(|| default_public_url(&args.host, args.port));
    let python = resolve_python_executable(&args.python).await?;
    let bridge = PythonAgentBridge::start(
        source.clone(),
        python,
        framework.clone(),
        args.target.clone(),
        args.workers,
        Duration::from_secs(args.request_timeout_secs),
        Duration::from_secs(args.worker_start_timeout_secs),
    )
    .await?;

    let state = ServeState {
        bridge: bridge.clone(),
    };
    let mut app = Router::new()
        .route("/health", get(health))
        .route("/invoke", post(invoke))
        .with_state(state);

    if protocols.iter().any(|protocol| protocol == "a2a") {
        let handler = Arc::new(DefaultRequestHandler::new(bridge, InMemoryTaskStore::new()));
        let card = build_agent_card(&source, &framework, &public_url);
        let card_producer = Arc::new(StaticAgentCard::new(card));
        app = app
            .nest("/a2a", a2a_server::jsonrpc::jsonrpc_router(handler.clone()))
            .nest("/a2a/rest", a2a_server::rest::rest_router(handler))
            .merge(a2a_server::agent_card::agent_card_router(card_producer));
    }

    let addr: SocketAddr = format!("{}:{}", args.host, args.port)
        .parse()
        .with_context(|| format!("invalid listen address {}:{}", args.host, args.port))?;
    println!("Serving {} on http://{}", source.display(), addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Clone)]
struct ServeState {
    bridge: PythonAgentBridge,
}

async fn health() -> impl IntoResponse {
    Json(HealthResponse { status: "ok" })
}

async fn invoke(
    State(state): State<ServeState>,
    Json(request): Json<InvokeRequest>,
) -> impl IntoResponse {
    match state.bridge.invoke(request).await {
        Ok(output) => Json(InvokeResponse {
            message: output.message.clone(),
            output: output.message,
            raw: output.raw,
        })
        .into_response(),
        Err(error) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

impl PythonAgentBridge {
    async fn start(
        source: PathBuf,
        python: String,
        framework: String,
        target: Option<String>,
        worker_count: usize,
        request_timeout: Duration,
        start_timeout: Duration,
    ) -> Result<Self> {
        let config = Arc::new(PythonWorkerConfig {
            source,
            framework,
            target,
            python,
        });
        let pool = Arc::new(PythonWorkerPool::new(
            config,
            worker_count,
            request_timeout,
            start_timeout,
        ));
        pool.start_all().await?;
        Ok(Self { pool })
    }

    async fn invoke(&self, mut request: InvokeRequest) -> Result<PythonAgentOutput> {
        if request.message.is_none() {
            request.message = request.input.as_ref().and_then(extract_message);
        }
        self.pool.invoke(request).await
    }
}

impl PythonWorkerPool {
    fn new(
        config: Arc<PythonWorkerConfig>,
        worker_count: usize,
        request_timeout: Duration,
        start_timeout: Duration,
    ) -> Self {
        let workers = (0..worker_count)
            .map(|id| {
                Mutex::new(PythonWorker::new(
                    id,
                    config.clone(),
                    request_timeout,
                    start_timeout,
                ))
            })
            .collect();
        Self {
            workers,
            limiter: Semaphore::new(worker_count),
            next_worker: AtomicUsize::new(0),
        }
    }

    async fn start_all(&self) -> Result<()> {
        for worker in &self.workers {
            worker.lock().await.ensure_started().await?;
        }
        Ok(())
    }

    async fn invoke(&self, request: InvokeRequest) -> Result<PythonAgentOutput> {
        let _permit = self
            .limiter
            .acquire()
            .await
            .context("agent worker limiter closed")?;
        let index = self.select_worker(&request);
        let mut worker = self.workers[index].lock().await;
        worker.invoke(request).await
    }

    fn select_worker(&self, request: &InvokeRequest) -> usize {
        if self.workers.len() == 1 {
            return 0;
        }
        if let Some(session_id) = request
            .session_id
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            return sticky_worker_index(session_id, self.workers.len());
        }
        self.next_worker.fetch_add(1, Ordering::Relaxed) % self.workers.len()
    }
}

impl PythonWorker {
    fn new(
        id: usize,
        config: Arc<PythonWorkerConfig>,
        request_timeout: Duration,
        start_timeout: Duration,
    ) -> Self {
        Self {
            id,
            config,
            request_timeout,
            start_timeout,
            child: None,
            stdin: None,
            stdout: None,
            stderr_task: None,
        }
    }

    async fn invoke(&mut self, request: InvokeRequest) -> Result<PythonAgentOutput> {
        self.ensure_started().await?;
        let timeout_result = timeout(self.request_timeout, self.invoke_started(request)).await;
        match timeout_result {
            Ok(Ok(output)) => Ok(output),
            Ok(Err(error)) => {
                if error.downcast_ref::<AgentInvocationError>().is_none() {
                    self.stop().await;
                }
                Err(error)
            }
            Err(_) => {
                self.stop().await;
                bail!(
                    "agent worker {} request timed out after {}s",
                    self.id,
                    self.request_timeout.as_secs()
                )
            }
        }
    }

    async fn ensure_started(&mut self) -> Result<()> {
        if self.child.is_some() && self.stdin.is_some() && self.stdout.is_some() {
            return Ok(());
        }
        self.stop().await;

        let mut command = tokio::process::Command::new(&self.config.python);
        command
            .arg("-u")
            .arg("-c")
            .arg(PYTHON_FRAMEWORK_RUNNER)
            .current_dir(&self.config.source)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        let mut child = command
            .spawn()
            .with_context(|| format!("start Python executable '{}'", self.config.python))?;
        let mut stdin = child
            .stdin
            .take()
            .context("agent worker stdin unavailable")?;
        let stdout = child
            .stdout
            .take()
            .context("agent worker stdout unavailable")?;
        if let Some(stderr) = child.stderr.take() {
            self.stderr_task = Some(drain_worker_stderr(self.id, stderr));
        }

        let init = json!({
            "root": self.config.source,
            "framework": self.config.framework,
            "target": self.config.target,
        });
        write_json_line(&mut stdin, &init)
            .await
            .context("send agent worker init")?;
        let mut stdout = BufReader::new(stdout);
        let ready_line =
            read_worker_line(&mut stdout, self.start_timeout, "agent worker startup").await?;
        let ready: WorkerReadyResponse = serde_json::from_str(ready_line.trim())
            .with_context(|| format!("agent worker returned invalid startup JSON: {ready_line}"))?;
        if !ready.ready {
            self.stop_child(child).await;
            bail!(
                "agent worker startup failed: {}",
                format_worker_error(&ready.error_type, &ready.error)
            );
        }

        self.stdin = Some(stdin);
        self.stdout = Some(stdout);
        self.child = Some(child);
        Ok(())
    }

    async fn invoke_started(&mut self, request: InvokeRequest) -> Result<PythonAgentOutput> {
        let stdin = self
            .stdin
            .as_mut()
            .context("agent worker stdin unavailable")?;
        write_json_line(stdin, &request)
            .await
            .context("send agent worker request")?;
        let stdout = self
            .stdout
            .as_mut()
            .context("agent worker stdout unavailable")?;
        let line = read_worker_line(stdout, self.request_timeout, "agent worker response").await?;
        let response: WorkerInvokeResponse = serde_json::from_str(line.trim())
            .with_context(|| format!("agent worker returned invalid response JSON: {line}"))?;
        if !response.ok {
            return Err(anyhow::Error::new(AgentInvocationError(format!(
                "agent invocation failed: {}",
                format_worker_error(&response.error_type, &response.error)
            ))));
        }
        response
            .response
            .context("agent worker response did not include output")
    }

    async fn stop(&mut self) {
        self.stdin = None;
        self.stdout = None;
        if let Some(handle) = self.stderr_task.take() {
            handle.abort();
        }
        if let Some(child) = self.child.take() {
            self.stop_child(child).await;
        }
    }

    async fn stop_child(&self, mut child: Child) {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
}

fn sticky_worker_index(session_id: &str, worker_count: usize) -> usize {
    let mut hasher = DefaultHasher::new();
    session_id.hash(&mut hasher);
    (hasher.finish() as usize) % worker_count
}

async fn write_json_line<T: Serialize>(stdin: &mut ChildStdin, value: &T) -> Result<()> {
    let mut bytes = serde_json::to_vec(value).context("serialize agent worker payload")?;
    bytes.push(b'\n');
    stdin.write_all(&bytes).await?;
    stdin.flush().await?;
    Ok(())
}

async fn read_worker_line(
    stdout: &mut BufReader<ChildStdout>,
    duration: Duration,
    operation: &str,
) -> Result<String> {
    let mut line = String::new();
    let bytes = timeout(duration, stdout.read_line(&mut line))
        .await
        .with_context(|| format!("{operation} timed out after {}s", duration.as_secs()))??;
    if bytes == 0 {
        bail!("{operation} reached EOF")
    }
    Ok(line)
}

fn drain_worker_stderr(worker_id: usize, stderr: tokio::process::ChildStderr) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            tracing::debug!(worker_id, "agent worker stderr: {line}");
        }
    })
}

fn format_worker_error(error_type: &Option<String>, error: &Option<String>) -> String {
    match (error_type.as_deref(), error.as_deref()) {
        (Some(error_type), Some(error)) if !error.is_empty() => format!("{error_type}: {error}"),
        (_, Some(error)) if !error.is_empty() => error.to_string(),
        (Some(error_type), _) => error_type.to_string(),
        _ => "unknown error".to_string(),
    }
}

async fn resolve_python_executable(configured: &str) -> Result<String> {
    if configured != "python" || executable_works(configured).await {
        return Ok(configured.to_string());
    }
    if executable_works("python3").await {
        return Ok("python3".to_string());
    }
    bail!("could not find a Python executable; pass --python /path/to/python")
}

async fn executable_works(executable: &str) -> bool {
    let mut command = tokio::process::Command::new(executable);
    command
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    match timeout(
        Duration::from_secs(PYTHON_PROBE_TIMEOUT_SECS),
        command.status(),
    )
    .await
    {
        Ok(Ok(status)) => status.success(),
        _ => false,
    }
}

impl AgentExecutor for PythonAgentBridge {
    fn execute(
        &self,
        ctx: ExecutorContext,
    ) -> BoxStream<'static, Result<StreamResponse, A2AError>> {
        let bridge = self.clone();
        let task_id = ctx.task_id.clone();
        let context_id = ctx.context_id.clone();
        let request = InvokeRequest {
            message: ctx
                .message
                .as_ref()
                .and_then(Message::text)
                .map(ToOwned::to_owned),
            input: None,
            session_id: Some(context_id.clone()),
            resume: ctx.message.as_ref().and_then(extract_resume_part),
        };
        Box::pin(stream::once(async move {
            match bridge.invoke(request).await {
                Ok(output) => Ok(StreamResponse::Task(completed_task(
                    task_id,
                    context_id,
                    output.message,
                ))),
                Err(error) => Ok(StreamResponse::Task(failed_task(
                    task_id,
                    context_id,
                    error.to_string(),
                ))),
            }
        }))
    }

    fn cancel(&self, ctx: ExecutorContext) -> BoxStream<'static, Result<StreamResponse, A2AError>> {
        Box::pin(stream::once(async move {
            Ok(StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
                task_id: ctx.task_id,
                context_id: ctx.context_id,
                status: TaskStatus {
                    state: TaskState::Canceled,
                    message: None,
                    timestamp: Some(chrono::Utc::now()),
                },
                metadata: None,
            }))
        }))
    }
}

fn completed_task(task_id: String, context_id: String, text: String) -> Task {
    Task {
        id: task_id.clone(),
        context_id: context_id.clone(),
        status: TaskStatus {
            state: TaskState::Completed,
            message: Some(Message {
                role: Role::Agent,
                message_id: new_message_id(),
                task_id: Some(task_id),
                context_id: Some(context_id),
                parts: vec![Part::text(text)],
                metadata: None,
                extensions: None,
                reference_task_ids: None,
            }),
            timestamp: Some(chrono::Utc::now()),
        },
        artifacts: None,
        history: None,
        metadata: None,
    }
}

fn failed_task(task_id: String, context_id: String, text: String) -> Task {
    let mut task = completed_task(task_id, context_id, text);
    task.status.state = TaskState::Failed;
    task
}

fn extract_message(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Object(object) => object
            .get("message")
            .or_else(|| object.get("input"))
            .and_then(extract_message),
        _ => None,
    }
}

fn extract_resume_part(message: &Message) -> Option<Value> {
    message.parts.iter().find_map(|part| match &part.content {
        PartContent::Data(value) => value.get("resume").cloned().or_else(|| Some(value.clone())),
        _ => None,
    })
}

fn normalize_protocols(values: &[AgentProtocolArg], csv: Option<&str>) -> Vec<String> {
    let mut protocols = Vec::new();
    for value in values
        .iter()
        .map(ToString::to_string)
        .chain(csv.into_iter().flat_map(|value| {
            value
                .split(',')
                .map(str::trim)
                .map(str::to_string)
                .collect::<Vec<_>>()
        }))
    {
        let value = value.trim().to_ascii_lowercase();
        if !value.is_empty() && !protocols.contains(&value) {
            protocols.push(value);
        }
    }
    if protocols.is_empty() {
        protocols.push("http".to_string());
        protocols.push("a2a".to_string());
    }
    protocols
}

fn default_public_url(host: &str, port: u16) -> String {
    let host = if host == "0.0.0.0" { "localhost" } else { host };
    format!("http://{host}:{port}")
}

fn build_agent_card(source: &Path, framework: &str, public_url: &str) -> AgentCard {
    let name = source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("agent")
        .to_string();
    AgentCard {
        name: name.clone(),
        description: format!("Native {framework} agent served by GravixLayer CLI: {name}"),
        version: a2a::VERSION.to_string(),
        supported_interfaces: vec![
            AgentInterface::new(format!("{public_url}/a2a"), TRANSPORT_PROTOCOL_JSONRPC),
            AgentInterface::new(
                format!("{public_url}/a2a/rest"),
                TRANSPORT_PROTOCOL_HTTP_JSON,
            ),
        ],
        capabilities: AgentCapabilities {
            streaming: Some(true),
            push_notifications: Some(false),
            extensions: None,
            extended_agent_card: None,
        },
        default_input_modes: vec!["text/plain".to_string(), "application/json".to_string()],
        default_output_modes: vec!["text/plain".to_string(), "application/json".to_string()],
        skills: vec![AgentSkill {
            id: "default".to_string(),
            name: "Agent".to_string(),
            description: format!("Invoke the {framework} agent"),
            tags: vec![framework.to_string(), "agent".to_string()],
            examples: None,
            input_modes: None,
            output_modes: None,
            security_requirements: None,
        }],
        provider: Some(AgentProvider {
            organization: "GravixLayer".to_string(),
            url: "https://gravixlayer.ai".to_string(),
        }),
        documentation_url: None,
        icon_url: None,
        security_schemes: None,
        security_requirements: None,
        signatures: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sticky_worker_index_is_stable_for_session() {
        let first = sticky_worker_index("session-123", 8);
        let second = sticky_worker_index("session-123", 8);

        assert_eq!(first, second);
        assert!(first < 8);
    }

    #[test]
    fn worker_error_prefers_type_and_message() {
        let message =
            format_worker_error(&Some("RuntimeError".to_string()), &Some("boom".to_string()));

        assert_eq!(message, "RuntimeError: boom");
    }
}
