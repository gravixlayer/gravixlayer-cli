// src/scaffold/mod.rs — Agent project scaffolding.
//
// Generates a new agent project directory with a `gravixlayer/gravixlayer.json`
// manifest, framework-appropriate starter files, and a requirements.txt.

pub mod archive;
pub mod wizard;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::AgentFrameworkArg;
use crate::config::project::GravixlayerProject;

/// Create a new agent project scaffold at `output_dir/<name>`.
///
/// Returns the path of the created project directory.
pub fn init_agent_project(
    name: &str,
    framework: AgentFrameworkArg,
    output_dir: &Path,
    python_version: &str,
) -> anyhow::Result<PathBuf> {
    let project_dir = output_dir.join(name);
    if project_dir.exists() {
        anyhow::bail!("directory already exists: {}", project_dir.display());
    }

    // Create directory tree
    let gravixlayer_dir = project_dir.join("gravixlayer");
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&gravixlayer_dir)?;
    fs::create_dir_all(&src_dir)?;

    // Write gravixlayer.json
    let mut proj = GravixlayerProject::default();
    proj.name = Some(name.to_string());
    proj.framework = Some(framework.to_string());
    proj.python_version = Some(python_version.to_string());
    proj.port = Some(8000);
    proj.health_check_path = Some("/health".into());
    proj.start_command = Some(framework_start_command(framework));
    proj.requirements = Some("requirements.txt".into());
    proj.env = HashMap::new();
    proj.exclude = vec![];

    let json_path = gravixlayer_dir.join("gravixlayer.json");
    proj.save(&json_path)?;

    // Write framework-specific starter files
    write_framework_files(&project_dir, name, framework, python_version)?;

    // Write .gitignore
    fs::write(project_dir.join(".gitignore"), GITIGNORE_CONTENT)?;

    Ok(project_dir)
}

fn framework_start_command(fw: AgentFrameworkArg) -> Vec<String> {
    match fw {
        AgentFrameworkArg::Langgraph
        | AgentFrameworkArg::GoogleAdk
        | AgentFrameworkArg::Langchain => vec![
            "gravixlayer".into(),
            "agent".into(),
            "serve".into(),
            ".".into(),
            "--framework".into(),
            fw.to_string(),
            "--host".into(),
            "0.0.0.0".into(),
            "--port".into(),
            "8000".into(),
        ],
        AgentFrameworkArg::Crewai
        | AgentFrameworkArg::OpenaiAgents
        | AgentFrameworkArg::Anthropic
        | AgentFrameworkArg::Strands
        | AgentFrameworkArg::Python => vec!["python".into(), "src/main.py".into()],
    }
}

fn write_framework_files(
    project_dir: &Path,
    name: &str,
    framework: AgentFrameworkArg,
    _python_version: &str,
) -> anyhow::Result<()> {
    let reqs = framework_requirements(framework);
    fs::write(project_dir.join("requirements.txt"), reqs.join("\n") + "\n")?;

    if framework == AgentFrameworkArg::GoogleAdk {
        write_google_adk_files(project_dir, name)?;
        return Ok(());
    }

    if framework == AgentFrameworkArg::Langgraph {
        write_langgraph_files(project_dir)?;
        return Ok(());
    }

    if framework == AgentFrameworkArg::Langchain {
        write_langchain_files(project_dir)?;
        return Ok(());
    }

    let main_py = framework_main_py(name, framework);
    fs::write(project_dir.join("src").join("main.py"), main_py)?;

    // Minimal health server helper used by all frameworks
    let health_py = HEALTH_SERVER_PY;
    fs::write(project_dir.join("src").join("health.py"), health_py)?;

    Ok(())
}

fn framework_requirements(fw: AgentFrameworkArg) -> Vec<&'static str> {
    match fw {
        AgentFrameworkArg::Langgraph => vec![
            "langgraph>=1.0",
            "langgraph-checkpoint",
            "langchain>=1.0",
            "langchain-openai>=1.0",
            "langchain-anthropic>=1.0",
        ],
        AgentFrameworkArg::Langchain => vec![
            "langchain>=1.0",
            "langchain-openai>=1.0",
            "langchain-anthropic>=1.0",
        ],
        AgentFrameworkArg::Crewai => vec![
            "crewai>=0.177",
            "fastapi>=0.115",
            "uvicorn[standard]>=0.32",
            "pydantic>=2",
        ],
        AgentFrameworkArg::GoogleAdk => vec!["google-adk>=1.0.0"],
        AgentFrameworkArg::OpenaiAgents => vec![
            "openai-agents>=0.1",
            "fastapi>=0.115",
            "uvicorn[standard]>=0.32",
            "pydantic>=2",
        ],
        AgentFrameworkArg::Anthropic => vec![
            "claude-agent-sdk>=0.1.0",
            "anyio>=4",
            "fastapi>=0.115",
            "uvicorn[standard]>=0.32",
            "pydantic>=2",
        ],
        AgentFrameworkArg::Strands => vec![
            "strands-agents>=1.0",
            "strands-agents-tools>=0.2",
            "fastapi>=0.115",
            "uvicorn[standard]>=0.32",
            "pydantic>=2",
        ],
        AgentFrameworkArg::Python => {
            vec!["fastapi>=0.115", "uvicorn[standard]>=0.32", "pydantic>=2"]
        }
    }
}

fn write_google_adk_files(project_dir: &Path, name: &str) -> anyhow::Result<()> {
    let package = name.replace('-', "_");
    let package_dir = project_dir.join(&package);
    fs::create_dir_all(&package_dir)?;
    fs::write(
        package_dir.join("__init__.py"),
        "from .agent import root_agent\n\n__all__ = [\"root_agent\"]\n",
    )?;
    fs::write(
        package_dir.join("agent.py"),
        format!(
            r#"from __future__ import annotations

from google.adk.agents import Agent

root_agent = Agent(
    name="{package}",
    model="gemini-2.5-flash",
    description="A native Google ADK agent deployed on GravixLayer.",
    instruction="You are a reliable production assistant.",
)
"#
        ),
    )?;
    Ok(())
}

fn write_langgraph_files(project_dir: &Path) -> anyhow::Result<()> {
    fs::write(
        project_dir.join("langgraph.json"),
        "{\n  \"graphs\": {\n    \"agent\": \"./src/agent.py:graph\"\n  }\n}\n",
    )?;
    fs::write(project_dir.join("src").join("__init__.py"), "")?;
    fs::write(project_dir.join("src").join("agent.py"), LANGGRAPH_AGENT)?;
    Ok(())
}

fn write_langchain_files(project_dir: &Path) -> anyhow::Result<()> {
    fs::write(project_dir.join("src").join("__init__.py"), "")?;
    fs::write(project_dir.join("src").join("agent.py"), LANGCHAIN_AGENT)?;
    Ok(())
}

fn framework_main_py(name: &str, fw: AgentFrameworkArg) -> String {
    let sanitized_name = name.replace('-', "_");
    let runner = match fw {
        AgentFrameworkArg::Langgraph => LANGGRAPH_RUNNER,
        AgentFrameworkArg::Langchain => LANGCHAIN_RUNNER,
        AgentFrameworkArg::Crewai => CREWAI_RUNNER,
        AgentFrameworkArg::GoogleAdk => GOOGLE_ADK_RUNNER,
        AgentFrameworkArg::OpenaiAgents => OPENAI_AGENTS_RUNNER,
        AgentFrameworkArg::Anthropic => ANTHROPIC_RUNNER,
        AgentFrameworkArg::Strands => STRANDS_RUNNER,
        AgentFrameworkArg::Python => PYTHON_RUNNER,
    };

    format!(
        r#"from __future__ import annotations

import os

import uvicorn
from fastapi import FastAPI
from pydantic import BaseModel

from health import health_router

{runner}

app = FastAPI(title="{name}", version="0.1.0")
app.include_router(health_router)


class InvokeRequest(BaseModel):
    message: str


class InvokeResponse(BaseModel):
    response: str


@app.post("/invoke", response_model=InvokeResponse)
async def invoke(req: InvokeRequest) -> InvokeResponse:
    return InvokeResponse(response=await run_agent(req.message))


if __name__ == "__main__":
    uvicorn.run(
        "main:app",
        host=os.getenv("HOST", "0.0.0.0"),
        port=int(os.getenv("PORT", "8000")),
        reload=os.getenv("RELOAD", "0") == "1",
    )
"#,
        name = sanitized_name,
        runner = runner,
    )
}

static LANGGRAPH_RUNNER: &str = r#"from langchain.chat_models import init_chat_model
from langgraph.graph import END, START, MessagesState, StateGraph

model = init_chat_model("openai:gpt-4.1", temperature=0)


async def call_model(state: MessagesState) -> dict:
    response = await model.ainvoke(state["messages"])
    return {"messages": [response]}


graph_builder = StateGraph(MessagesState)
graph_builder.add_node("agent", call_model)
graph_builder.add_edge(START, "agent")
graph_builder.add_edge("agent", END)
agent_graph = graph_builder.compile()


async def run_agent(message: str) -> str:
    result = await agent_graph.ainvoke({"messages": [{"role": "user", "content": message}]})
    return result["messages"][-1].content
"#;

static LANGGRAPH_AGENT: &str = r#"from __future__ import annotations

from langchain.chat_models import init_chat_model
from langgraph.graph import END, START, MessagesState, StateGraph

model = init_chat_model("openai:gpt-4.1", temperature=0)


async def call_model(state: MessagesState) -> dict:
    response = await model.ainvoke(state["messages"])
    return {"messages": [response]}


builder = StateGraph(MessagesState)
builder.add_node("agent", call_model)
builder.add_edge(START, "agent")
builder.add_edge("agent", END)

graph = builder.compile()
"#;

static LANGCHAIN_RUNNER: &str = r#"from datetime import datetime, timezone

from langchain.agents import create_agent


def current_time() -> str:
    """Return the current UTC time in ISO 8601 format."""
    return datetime.now(timezone.utc).isoformat()


agent = create_agent(
    model="openai:gpt-4.1",
    tools=[current_time],
    system_prompt="You are a reliable production assistant.",
)


async def run_agent(message: str) -> str:
    result = await agent.ainvoke({"messages": [{"role": "user", "content": message}]})
    return result["messages"][-1].content
"#;

static LANGCHAIN_AGENT: &str = r#"from __future__ import annotations

from datetime import datetime, timezone

from langchain.agents import create_agent


def current_time() -> str:
    """Return the current UTC time in ISO 8601 format."""
    return datetime.now(timezone.utc).isoformat()


agent = create_agent(
    model="openai:gpt-4.1",
    tools=[current_time],
    system_prompt="You are a reliable production assistant.",
)
"#;

static CREWAI_RUNNER: &str = r#"from crewai import Agent

agent = Agent(
    role="Production assistant",
    goal="Answer user requests clearly and act within the provided instructions.",
    backstory="You are a reliable production assistant.",
    llm="gpt-4.1",
    verbose=False,
)


async def run_agent(message: str) -> str:
    result = await agent.kickoff_async(message)
    if hasattr(result, "raw") and result.raw:
        return result.raw
    return str(result)
"#;

static GOOGLE_ADK_RUNNER: &str = r#"from google.adk.agents import Agent
from google.adk.runners import Runner
from google.adk.sessions import InMemorySessionService
from google.genai.types import Content, Part

root_agent = Agent(
    name="agent",
    model="gemini-2.5-flash",
    instruction="You are a reliable production assistant.",
)
session_service = InMemorySessionService()
runner = Runner(
    agent=root_agent,
    app_name="gravixlayer-agent",
    session_service=session_service,
)


async def run_agent(message: str) -> str:
    session = await session_service.create_session(
        app_name="gravixlayer-agent",
        user_id="gravix-user",
    )
    content = Content(parts=[Part(text=message)], role="user")
    parts: list[str] = []
    async for event in runner.run_async(
        user_id="gravix-user",
        session_id=session.id,
        new_message=content,
    ):
        if event.content and event.content.parts:
            for part in event.content.parts:
                if getattr(part, "text", None):
                    parts.append(part.text)
    return "".join(parts)
"#;

static OPENAI_AGENTS_RUNNER: &str = r#"from agents import Agent, Runner

agent = Agent(
    name="assistant",
    instructions="You are a reliable production assistant.",
    model="gpt-4.1",
)


async def run_agent(message: str) -> str:
    result = await Runner.run(agent, message)
    return result.final_output
"#;

static ANTHROPIC_RUNNER: &str = r#"from claude_agent_sdk import ClaudeAgentOptions, query

options = ClaudeAgentOptions(model="claude-opus-4-7")


async def run_agent(message: str) -> str:
    chunks: list[str] = []
    async for event in query(prompt=message, options=options):
        if hasattr(event, "result") and event.result:
            chunks.append(event.result)
    return "".join(chunks)
"#;

static STRANDS_RUNNER: &str = r#"from datetime import datetime, timezone

from strands import Agent, tool


@tool
def current_time() -> str:
    """Return the current UTC time in ISO 8601 format."""
    return datetime.now(timezone.utc).isoformat()


agent = Agent(
    name="assistant",
    system_prompt="You are a reliable production assistant.",
    tools=[current_time],
)


async def run_agent(message: str) -> str:
    result = await agent.invoke_async(message)
    return str(result)
"#;

static PYTHON_RUNNER: &str = r#"from datetime import datetime, timezone


async def run_agent(message: str) -> str:
    return (
        f"Request accepted at {datetime.now(timezone.utc).isoformat()} "
        f"with {len(message)} input characters."
    )
"#;

static HEALTH_SERVER_PY: &str = r#"""Health check router — imported by main.py."""
from fastapi import APIRouter

health_router = APIRouter()


@health_router.get("/health")
async def health() -> dict:
    return {"status": "ok"}
"#;

static GITIGNORE_CONTENT: &str = r#"__pycache__/
*.py[cod]
.venv/
venv/
env/
.env
*.egg-info/
dist/
build/
.pytest_cache/
.mypy_cache/
.ruff_cache/
node_modules/
.DS_Store
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_google_adk_project_uses_native_layout() {
        let temp = tempfile::TempDir::new().unwrap();

        let root = init_agent_project(
            "travel-agent",
            AgentFrameworkArg::GoogleAdk,
            temp.path(),
            "3.12",
        )
        .unwrap();

        assert!(root.join("travel_agent").join("__init__.py").is_file());
        assert!(root.join("travel_agent").join("agent.py").is_file());
        assert!(!root.join("src").join("main.py").exists());

        let requirements = fs::read_to_string(root.join("requirements.txt")).unwrap();
        assert!(requirements.contains("google-adk>=1.0.0"));
        assert!(!requirements.contains("a2a-sdk"));
        assert!(!requirements.contains("protobuf<7"));

        let config =
            GravixlayerProject::load(&root.join("gravixlayer").join("gravixlayer.json")).unwrap();
        assert_eq!(config.framework.as_deref(), Some("google-adk"));
        assert_eq!(
            config.start_command.unwrap(),
            vec![
                "gravixlayer",
                "agent",
                "serve",
                ".",
                "--framework",
                "google-adk",
                "--host",
                "0.0.0.0",
                "--port",
                "8000",
            ]
        );
    }

    #[test]
    fn init_langgraph_project_uses_native_layout() {
        let temp = tempfile::TempDir::new().unwrap();

        let root = init_agent_project(
            "research-agent",
            AgentFrameworkArg::Langgraph,
            temp.path(),
            "3.12",
        )
        .unwrap();

        assert!(root.join("langgraph.json").is_file());
        assert!(root.join("src").join("agent.py").is_file());
        assert!(!root.join("src").join("main.py").exists());
        assert!(!root.join("src").join("health.py").exists());

        let langgraph_config = fs::read_to_string(root.join("langgraph.json")).unwrap();
        assert!(langgraph_config.contains("./src/agent.py:graph"));

        let requirements = fs::read_to_string(root.join("requirements.txt")).unwrap();
        assert!(requirements.contains("langgraph>=1.0"));
        assert!(requirements.contains("langgraph-checkpoint"));
        assert!(!requirements.contains("fastapi"));

        let config =
            GravixlayerProject::load(&root.join("gravixlayer").join("gravixlayer.json")).unwrap();
        assert_eq!(config.framework.as_deref(), Some("langgraph"));
        assert_eq!(
            config.start_command.unwrap(),
            vec![
                "gravixlayer",
                "agent",
                "serve",
                ".",
                "--framework",
                "langgraph",
                "--host",
                "0.0.0.0",
                "--port",
                "8000",
            ]
        );
    }

    #[test]
    fn init_langchain_project_uses_native_layout() {
        let temp = tempfile::TempDir::new().unwrap();

        let root = init_agent_project(
            "assistant-agent",
            AgentFrameworkArg::Langchain,
            temp.path(),
            "3.12",
        )
        .unwrap();

        assert!(root.join("src").join("agent.py").is_file());
        assert!(!root.join("src").join("main.py").exists());
        assert!(!root.join("src").join("health.py").exists());

        let agent_py = fs::read_to_string(root.join("src").join("agent.py")).unwrap();
        assert!(agent_py.contains("from langchain.agents import create_agent"));
        assert!(agent_py.contains("agent = create_agent"));

        let requirements = fs::read_to_string(root.join("requirements.txt")).unwrap();
        assert!(requirements.contains("langchain>=1.0"));
        assert!(!requirements.contains("fastapi"));

        let config =
            GravixlayerProject::load(&root.join("gravixlayer").join("gravixlayer.json")).unwrap();
        assert_eq!(config.framework.as_deref(), Some("langchain"));
        assert_eq!(
            config.start_command.unwrap(),
            vec![
                "gravixlayer",
                "agent",
                "serve",
                ".",
                "--framework",
                "langchain",
                "--host",
                "0.0.0.0",
                "--port",
                "8000",
            ]
        );
    }
}
