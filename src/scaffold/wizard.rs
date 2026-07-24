//! Phase 4: Interactive 8-step agent project wizard.
//!
//! Invoked by `grx agent create`. Uses `dialoguer` for TTY prompts and
//! writes the full project scaffold (agent.py / main.py / pyproject.toml /
//! gravixlayer.json / .gitignore / .env.local) to disk without any external
//! template engine — just `str::replace("{{var}}", &val)`.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Select};

// ---------------------------------------------------------------------------
// Embedded template sources
// ---------------------------------------------------------------------------

mod tpl {
    // Common files
    pub const GITIGNORE: &str = include_str!("templates/common/gitignore.txt");
    pub const ENV_LOCAL: &str = include_str!("templates/common/env_local.txt");
    pub const CONFIG_JSON: &str = include_str!("templates/common/gravixlayer.json");

    // LangGraph
    pub const LG_AGENT: &str = include_str!("templates/langgraph/agent.py");
    pub const LG_PYPROJ: &str = include_str!("templates/langgraph/pyproject.toml");

    // OpenAI Agents SDK
    pub const OA_AGENT: &str = include_str!("templates/openai_agents/agent.py");
    pub const OA_MAIN: &str = include_str!("templates/openai_agents/main.py");
    pub const OA_PYPROJ: &str = include_str!("templates/openai_agents/pyproject.toml");

    // Google ADK
    pub const GK_AGENT: &str = include_str!("templates/google_adk/agent.py");
    pub const GK_PYPROJ: &str = include_str!("templates/google_adk/pyproject.toml");

    // CrewAI
    pub const CR_AGENT: &str = include_str!("templates/crewai/agent.py");
    pub const CR_MAIN: &str = include_str!("templates/crewai/main.py");
    pub const CR_PYPROJ: &str = include_str!("templates/crewai/pyproject.toml");

    // Strands Agents
    pub const ST_AGENT: &str = include_str!("templates/strands/agent.py");
    pub const ST_MAIN: &str = include_str!("templates/strands/main.py");
    pub const ST_PYPROJ: &str = include_str!("templates/strands/pyproject.toml");

    // Claude Agent SDK
    pub const CA_AGENT: &str = include_str!("templates/claude_agent/agent.py");
    pub const CA_MAIN: &str = include_str!("templates/claude_agent/main.py");
    pub const CA_PYPROJ: &str = include_str!("templates/claude_agent/pyproject.toml");

    // LangChain
    pub const LC_AGENT: &str = include_str!("templates/langchain/agent.py");
    pub const LC_PYPROJ: &str = include_str!("templates/langchain/pyproject.toml");

    // Custom (FastAPI)
    pub const CU_MAIN: &str = include_str!("templates/custom/main.py");
    pub const CU_PYPROJ: &str = include_str!("templates/custom/pyproject.toml");
}

// ---------------------------------------------------------------------------
// Framework / provider enumerations
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framework {
    LangGraph,
    LangChain,
    CrewAi,
    OpenAiAgents,
    GoogleAdk,
    Anthropic,
    Strands,
    Python,
}

impl Framework {
    fn label(self) -> &'static str {
        match self {
            Self::LangGraph => "LangGraph",
            Self::LangChain => "LangChain",
            Self::CrewAi => "CrewAI",
            Self::OpenAiAgents => "OpenAI Agents SDK",
            Self::GoogleAdk => "Google ADK",
            Self::Anthropic => "Anthropic Claude Agent SDK",
            Self::Strands => "Strands Agents",
            Self::Python => "Python / FastAPI",
        }
    }

    fn framework_id(self) -> &'static str {
        match self {
            Self::LangGraph => "langgraph",
            Self::LangChain => "langchain",
            Self::CrewAi => "crewai",
            Self::OpenAiAgents => "openai-agents",
            Self::GoogleAdk => "google-adk",
            Self::Anthropic => "anthropic",
            Self::Strands => "strands",
            Self::Python => "python",
        }
    }

    fn available_providers(self) -> Vec<ModelProvider> {
        match self {
            Self::LangGraph => vec![ModelProvider::Anthropic, ModelProvider::OpenAi],
            Self::LangChain => vec![ModelProvider::Anthropic, ModelProvider::OpenAi],
            Self::CrewAi => vec![ModelProvider::OpenAi],
            Self::OpenAiAgents => vec![ModelProvider::OpenAi],
            Self::GoogleAdk => vec![ModelProvider::Google],
            Self::Anthropic => vec![ModelProvider::Anthropic],
            Self::Strands => vec![ModelProvider::Anthropic, ModelProvider::OpenAi],
            Self::Python => vec![ModelProvider::Any],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelProvider {
    Anthropic,
    OpenAi,
    Google,
    Any,
}

impl ModelProvider {
    fn label(self) -> &'static str {
        match self {
            Self::Anthropic => "Anthropic",
            Self::OpenAi => "OpenAI",
            Self::Google => "Google",
            Self::Any => "Custom / other",
        }
    }

    fn env_var(self) -> &'static str {
        match self {
            Self::Anthropic => "ANTHROPIC_API_KEY",
            Self::OpenAi => "OPENAI_API_KEY",
            Self::Google => "GOOGLE_API_KEY",
            Self::Any => "API_KEY",
        }
    }

    fn default_model(self, framework: Framework) -> &'static str {
        match (framework, self) {
            (Framework::LangGraph, Self::Anthropic) => "claude-sonnet-4-5-20250514",
            (Framework::LangGraph, Self::OpenAi) => "gpt-4.1",
            (Framework::LangChain, Self::Anthropic) => "claude-sonnet-4-5-20250514",
            (Framework::LangChain, Self::OpenAi) => "gpt-4.1",
            (Framework::CrewAi, Self::OpenAi) => "gpt-4.1",
            (Framework::OpenAiAgents, Self::OpenAi) => "gpt-4.1",
            (Framework::GoogleAdk, Self::Google) => "gemini-2.5-flash",
            (Framework::Anthropic, Self::Anthropic) => "claude-opus-4-7",
            (Framework::Strands, Self::Anthropic) => "claude-sonnet-4-5-20250514",
            (Framework::Strands, Self::OpenAi) => "gpt-4.1",
            _ => "gpt-4.1",
        }
    }

    fn model_options(self, framework: Framework) -> Vec<&'static str> {
        match (framework, self) {
            (_, Self::Anthropic) => vec![
                "claude-opus-4-7",
                "claude-sonnet-4-5-20250514",
                "claude-haiku-3-5",
            ],
            (_, Self::OpenAi) => vec!["gpt-4.1", "gpt-4o", "gpt-4o-mini", "o3"],
            (_, Self::Google) => vec!["gemini-2.5-flash", "gemini-2.5-pro"],
            _ => vec!["custom"],
        }
    }
}

// ---------------------------------------------------------------------------
// Resource preset
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub enum ResourcePreset {
    Small,
    Medium,
    Large,
}

impl ResourcePreset {
    fn label(self) -> &'static str {
        match self {
            Self::Small => "Small  (1 vCPU / 1 GB RAM / 10 GB disk)",
            Self::Medium => "Medium (2 vCPU / 2 GB RAM / 20 GB disk)",
            Self::Large => "Large  (4 vCPU / 4 GB RAM / 40 GB disk)",
        }
    }

    fn vcpu(self) -> u32 {
        match self {
            Self::Small => 1,
            Self::Medium => 2,
            Self::Large => 4,
        }
    }
    fn memory(self) -> u32 {
        match self {
            Self::Small => 1024,
            Self::Medium => 2048,
            Self::Large => 4096,
        }
    }
    fn disk(self) -> u32 {
        match self {
            Self::Small => 10240,
            Self::Medium => 20480,
            Self::Large => 40960,
        }
    }
}

// ---------------------------------------------------------------------------
// Wizard output
// ---------------------------------------------------------------------------

pub struct WizardResult {
    pub agent_name_kebab: String,
    pub agent_name_pascal: String,
    pub description: String,
    pub framework: Framework,
    pub provider: ModelProvider,
    pub model: String,
    pub protocols: Vec<String>,
    pub resources: ResourcePreset,
    pub python_version: String,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run the interactive 8-step agent creation wizard.
///
/// Returns `None` if the user chose not to confirm at step 8.
pub fn run_wizard(prefill_name: Option<&str>) -> Result<Option<WizardResult>> {
    let theme = ColorfulTheme::default();

    // ── Step 1: Agent name ──────────────────────────────────────────────────
    let agent_name_kebab: String = if let Some(n) = prefill_name {
        validate_agent_name(n)?;
        n.to_string()
    } else {
        Input::with_theme(&theme)
            .with_prompt("Agent name  (lowercase, hyphens, 2–63 chars)")
            .validate_with(|input: &String| -> Result<(), String> {
                validate_agent_name(input).map_err(|e| e.to_string())
            })
            .interact_text()
            .context("agent name prompt")?
    };

    // ── Step 2: Description ─────────────────────────────────────────────────
    let description: String = Input::with_theme(&theme)
        .with_prompt("Description  (optional — press Enter to skip)")
        .allow_empty(true)
        .interact_text()
        .context("description prompt")?;

    // ── Step 3: Framework ───────────────────────────────────────────────────
    let frameworks = [
        Framework::LangGraph,
        Framework::LangChain,
        Framework::CrewAi,
        Framework::OpenAiAgents,
        Framework::GoogleAdk,
        Framework::Anthropic,
        Framework::Strands,
        Framework::Python,
    ];
    let fw_labels: Vec<&str> = frameworks.iter().map(|f| f.label()).collect();
    let fw_idx = Select::with_theme(&theme)
        .with_prompt("Framework")
        .items(&fw_labels)
        .default(0)
        .interact()
        .context("framework prompt")?;
    let framework = frameworks[fw_idx];

    // ── Step 4: Model provider ──────────────────────────────────────────────
    let providers = framework.available_providers();
    let provider = if providers.len() == 1 {
        let p = providers[0];
        println!("  Provider   {}", p.label());
        p
    } else {
        let prov_labels: Vec<&str> = providers.iter().map(|p| p.label()).collect();
        let p_idx = Select::with_theme(&theme)
            .with_prompt("Model provider")
            .items(&prov_labels)
            .default(0)
            .interact()
            .context("provider prompt")?;
        providers[p_idx]
    };

    // ── Step 5: Model name ──────────────────────────────────────────────────
    let model = if provider == ModelProvider::Any {
        Input::with_theme(&theme)
            .with_prompt("Model name")
            .interact_text()
            .context("model name prompt")?
    } else {
        let model_options = provider.model_options(framework);
        let default_model = provider.default_model(framework);
        let default_idx = model_options
            .iter()
            .position(|&m| m == default_model)
            .unwrap_or(0);
        let m_idx = Select::with_theme(&theme)
            .with_prompt("Model")
            .items(&model_options)
            .default(default_idx)
            .interact()
            .context("model prompt")?;
        model_options[m_idx].to_string()
    };

    // ── Step 6: Protocols ───────────────────────────────────────────────────
    let all_protocols = ["HTTP", "A2A", "MCP"];
    let proto_selected = MultiSelect::with_theme(&theme)
        .with_prompt("Protocols  (Space to select, Enter to confirm)")
        .items(&all_protocols)
        .defaults(&[true, false, false])
        .interact()
        .context("protocols prompt")?;
    let protocols: Vec<String> = if proto_selected.is_empty() {
        vec!["http".to_string()]
    } else {
        proto_selected
            .iter()
            .map(|&i| all_protocols[i].to_lowercase())
            .collect()
    };

    // ── Step 7: Resources ───────────────────────────────────────────────────
    let presets = [
        ResourcePreset::Small,
        ResourcePreset::Medium,
        ResourcePreset::Large,
    ];
    let preset_labels: Vec<&str> = presets.iter().map(|p| p.label()).collect();
    let res_idx = Select::with_theme(&theme)
        .with_prompt("Resources")
        .items(&preset_labels)
        .default(1)
        .interact()
        .context("resources prompt")?;
    let resources = presets[res_idx];

    // ── Step 8: Confirm ─────────────────────────────────────────────────────
    println!();
    println!("  Summary");
    println!("  ─────────────────────────────────────────");
    println!("  Name        : {agent_name_kebab}");
    if !description.is_empty() {
        println!("  Description : {description}");
    }
    println!("  Framework   : {}", framework.label());
    println!("  Provider    : {}", provider.label());
    println!("  Model       : {model}");
    println!("  Protocols   : {}", protocols.join(", "));
    println!("  Resources   : {}", resources.label());
    println!("  Python      : 3.12");
    println!();

    let confirmed = Confirm::with_theme(&theme)
        .with_prompt("Create agent project?")
        .default(true)
        .interact()
        .context("confirm prompt")?;

    if !confirmed {
        return Ok(None);
    }

    Ok(Some(WizardResult {
        agent_name_pascal: to_pascal_case(&agent_name_kebab),
        agent_name_kebab,
        description,
        framework,
        provider,
        model,
        protocols,
        resources,
        python_version: "3.12".to_string(),
    }))
}

// ---------------------------------------------------------------------------
// Project scaffold
// ---------------------------------------------------------------------------

/// Write the agent project files based on the wizard result.
pub fn scaffold_project(result: &WizardResult, output_dir: Option<&Path>) -> Result<PathBuf> {
    let root = output_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(&result.agent_name_kebab));

    if root.exists() {
        bail!("directory already exists: {}", root.display());
    }

    let name = &result.agent_name_kebab;
    let pascal = &result.agent_name_pascal;
    let agent_package = result.agent_name_kebab.replace('-', "_");

    // Substitution map for template variables
    let protocols_json = serde_json::to_string(&result.protocols)?;
    let resources_json = serde_json::json!({
        "vcpu":      result.resources.vcpu(),
        "memory_mb": result.resources.memory(),
        "disk_mb":   result.resources.disk(),
    })
    .to_string();
    let api_key_env = result.provider.env_var();
    let protocols_csv = result.protocols.join(",");
    let default_module = format!("app.{pascal}.main");
    let (entrypoint, start_command): (String, Vec<String>) = match result.framework {
        Framework::LangGraph | Framework::GoogleAdk | Framework::LangChain => (
            format!(
                "python -m gravixlayer.runtime.autoserve --framework {} --root /app --host 0.0.0.0 --port 8000 --protocols {protocols_csv}",
                result.framework.framework_id()
            ),
            vec![
                "gravixlayer".to_string(),
                "agent".to_string(),
                "serve".to_string(),
                ".".to_string(),
                "--framework".to_string(),
                result.framework.framework_id().to_string(),
                "--host".to_string(),
                "0.0.0.0".to_string(),
                "--port".to_string(),
                "8000".to_string(),
                "--protocols".to_string(),
                protocols_csv.clone(),
            ],
        ),
        _ => (
            format!("python -m {default_module}"),
            vec!["python".to_string(), "-m".to_string(), default_module],
        ),
    };
    let entrypoint_json = serde_json::to_string(&entrypoint)?;
    let start_command_json = serde_json::to_string(&start_command)?;
    let langchain_model_id = match result.provider {
        ModelProvider::Anthropic => format!("anthropic:{}", result.model),
        ModelProvider::OpenAi => format!("openai:{}", result.model),
        ModelProvider::Google => format!("google_genai:{}", result.model),
        ModelProvider::Any => result.model.clone(),
    };

    let vars: Vec<(&str, &str)> = vec![
        ("{{agent_name}}", pascal),
        ("{{agent_name_kebab}}", name),
        ("{{agent_package}}", &agent_package),
        ("{{model_name}}", &result.model),
        ("{{langchain_model_id}}", &langchain_model_id),
        ("{{model_provider}}", result.provider.label()),
        ("{{api_key_env}}", api_key_env),
        ("{{python_version}}", &result.python_version),
        ("{{http_port}}", "8000"),
        ("{{description}}", &result.description),
        ("{{framework}}", result.framework.framework_id()),
        ("{{protocols_json}}", &protocols_json),
        ("{{entrypoint_json}}", &entrypoint_json),
        ("{{start_command_json}}", &start_command_json),
        ("{{resources_json}}", &resources_json),
    ];

    let render = |src: &str| -> String {
        let mut out = src.to_string();
        for (k, v) in &vars {
            out = out.replace(k, v);
        }
        out
    };

    // Determine which template files to write
    let (agent_py, main_py, pyproject_toml) = match result.framework {
        Framework::LangGraph => (Some(tpl::LG_AGENT), None, tpl::LG_PYPROJ),
        Framework::LangChain => (Some(tpl::LC_AGENT), None, tpl::LC_PYPROJ),
        Framework::CrewAi => (Some(tpl::CR_AGENT), Some(tpl::CR_MAIN), tpl::CR_PYPROJ),
        Framework::OpenAiAgents => (Some(tpl::OA_AGENT), Some(tpl::OA_MAIN), tpl::OA_PYPROJ),
        Framework::GoogleAdk => (Some(tpl::GK_AGENT), None, tpl::GK_PYPROJ),
        Framework::Anthropic => (Some(tpl::CA_AGENT), Some(tpl::CA_MAIN), tpl::CA_PYPROJ),
        Framework::Strands => (Some(tpl::ST_AGENT), Some(tpl::ST_MAIN), tpl::ST_PYPROJ),
        Framework::Python => (None, Some(tpl::CU_MAIN), tpl::CU_PYPROJ),
    };

    // Directory structure
    //  <root>/
    //  ├── gravixlayer/
    //  │   ├── gravixlayer.json
    //  │   └── .env.local
    //  ├── app/
    //  │   └── <AgentName>/
    //  │       ├── __init__.py
    //  │       ├── agent.py  (if applicable)
    //  │       ├── main.py
    //  │       └── pyproject.toml
    //  └── .gitignore

    let gravixlayer_dir = root.join("gravixlayer");
    let app_dir = if result.framework == Framework::GoogleAdk {
        root.join(&agent_package)
    } else {
        root.join("app").join(pascal)
    };

    std::fs::create_dir_all(&gravixlayer_dir).context("create gravixlayer dir")?;
    std::fs::create_dir_all(&app_dir).context("create app dir")?;

    // gravixlayer.json
    write_file(
        &gravixlayer_dir.join("gravixlayer.json"),
        &render(tpl::CONFIG_JSON),
    )?;
    if result.framework == Framework::LangGraph {
        write_file(
            &root.join("langgraph.json"),
            &format!(
                "{{\n  \"python_version\": {:?},\n  \"graphs\": {{\n    \"agent\": {:?}\n  }}\n}}\n",
                result.python_version,
                format!("./app/{pascal}/agent.py:graph")
            ),
        )?;
    }

    // .env.local
    write_file(&gravixlayer_dir.join(".env.local"), &render(tpl::ENV_LOCAL))?;

    // .gitignore
    write_file(&root.join(".gitignore"), tpl::GITIGNORE)?;

    if result.framework == Framework::GoogleAdk {
        write_file(
            &app_dir.join("__init__.py"),
            "from .agent import root_agent\n\n__all__ = [\"root_agent\"]\n",
        )?;
    } else {
        write_file(&root.join("app").join("__init__.py"), "")?;
        write_file(&app_dir.join("__init__.py"), "")?;
    }

    // agent.py (only for frameworks that have one)
    if let Some(agent_src) = agent_py {
        write_file(&app_dir.join("agent.py"), &render(agent_src))?;
    }

    // main.py
    if let Some(main_src) = main_py {
        write_file(&app_dir.join("main.py"), &render(main_src))?;
    }

    // pyproject.toml
    let pyproject_path = if matches!(
        result.framework,
        Framework::GoogleAdk | Framework::LangGraph | Framework::LangChain
    ) {
        root.join("pyproject.toml")
    } else {
        app_dir.join("pyproject.toml")
    };
    write_file(&pyproject_path, &render(pyproject_toml))?;

    Ok(root)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn write_file(path: &Path, content: &str) -> Result<()> {
    std::fs::write(path, content).with_context(|| format!("write {}", path.display()))
}

fn validate_agent_name(name: &str) -> Result<()> {
    if name.len() < 2 || name.len() > 63 {
        bail!("name must be 2–63 characters");
    }
    let first = name.chars().next().unwrap();
    if !first.is_ascii_lowercase() {
        bail!("name must start with a lowercase ASCII letter");
    }
    for ch in name.chars() {
        if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '-' {
            bail!("name may only contain lowercase letters, digits, and hyphens");
        }
    }
    Ok(())
}

fn to_pascal_case(kebab: &str) -> String {
    kebab
        .split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    upper + chars.as_str()
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pascal_case_conversion() {
        assert_eq!(to_pascal_case("my-agent"), "MyAgent");
        assert_eq!(to_pascal_case("hello-world-demo"), "HelloWorldDemo");
        assert_eq!(to_pascal_case("single"), "Single");
    }

    #[test]
    fn name_validation_pass() {
        assert!(validate_agent_name("my-agent").is_ok());
        assert!(validate_agent_name("ab").is_ok());
        assert!(validate_agent_name("agent123").is_ok());
    }

    #[test]
    fn name_validation_fail() {
        assert!(validate_agent_name("A").is_err()); // uppercase
        assert!(validate_agent_name("a").is_err()); // too short
        assert!(validate_agent_name("1agent").is_err()); // starts with digit
        assert!(validate_agent_name("my_agent").is_err()); // underscore
        let long = "a".repeat(64);
        assert!(validate_agent_name(&long).is_err());
    }

    #[test]
    fn google_adk_scaffold_uses_native_adk_layout() {
        let temp = tempfile::TempDir::new().unwrap();
        let project_dir = temp.path().join("time-agent");
        let result = WizardResult {
            agent_name_kebab: "time-agent".to_string(),
            agent_name_pascal: "TimeAgent".to_string(),
            description: "Answers time questions".to_string(),
            framework: Framework::GoogleAdk,
            provider: ModelProvider::Google,
            model: "gemini-2.5-flash".to_string(),
            protocols: vec!["http".to_string(), "a2a".to_string()],
            resources: ResourcePreset::Small,
            python_version: "3.12".to_string(),
        };

        let root = scaffold_project(&result, Some(&project_dir)).unwrap();

        assert!(root.join("time_agent").join("__init__.py").is_file());
        assert!(root.join("time_agent").join("agent.py").is_file());
        assert!(root.join("pyproject.toml").is_file());
        assert!(!root.join("time_agent").join("main.py").exists());
        assert!(!root.join("app").join("TimeAgent").join("agent.py").exists());

        let agent_py = std::fs::read_to_string(root.join("time_agent").join("agent.py")).unwrap();
        assert!(agent_py.contains("from google.adk.agents import Agent"));
        assert!(agent_py.contains("name=\"time_agent\""));

        let config =
            std::fs::read_to_string(root.join("gravixlayer").join("gravixlayer.json")).unwrap();
        assert!(config.contains("python -m gravixlayer.runtime.autoserve"));
        assert!(config.contains("start_command"));
        assert!(config.contains("\"agent\""));
        assert!(config.contains("\"serve\""));
        assert!(config.contains("google-adk"));
        assert!(config.contains("--protocols"));
        assert!(!config.contains("a2a_port"));
        assert!(!config.contains("mcp_port"));
    }

    #[test]
    fn langgraph_scaffold_uses_native_langgraph_layout() {
        let temp = tempfile::TempDir::new().unwrap();
        let project_dir = temp.path().join("research-agent");
        let result = WizardResult {
            agent_name_kebab: "research-agent".to_string(),
            agent_name_pascal: "ResearchAgent".to_string(),
            description: "Researches user questions".to_string(),
            framework: Framework::LangGraph,
            provider: ModelProvider::OpenAi,
            model: "gpt-4.1".to_string(),
            protocols: vec!["http".to_string(), "a2a".to_string()],
            resources: ResourcePreset::Small,
            python_version: "3.12".to_string(),
        };

        let root = scaffold_project(&result, Some(&project_dir)).unwrap();

        assert!(root.join("langgraph.json").is_file());
        assert!(root.join("pyproject.toml").is_file());
        assert!(root
            .join("app")
            .join("ResearchAgent")
            .join("agent.py")
            .is_file());
        assert!(!root
            .join("app")
            .join("ResearchAgent")
            .join("main.py")
            .exists());

        let langgraph_config = std::fs::read_to_string(root.join("langgraph.json")).unwrap();
        assert!(langgraph_config.contains("./app/ResearchAgent/agent.py:graph"));

        let pyproject = std::fs::read_to_string(root.join("pyproject.toml")).unwrap();
        assert!(pyproject.contains("langgraph>=1.0"));
        assert!(pyproject.contains("langgraph-checkpoint"));
        assert!(!pyproject.contains("fastapi"));

        let config =
            std::fs::read_to_string(root.join("gravixlayer").join("gravixlayer.json")).unwrap();
        assert!(config.contains("python -m gravixlayer.runtime.autoserve"));
        assert!(config.contains("start_command"));
        assert!(config.contains("\"agent\""));
        assert!(config.contains("\"serve\""));
        assert!(config.contains("langgraph"));
        assert!(config.contains("--protocols"));
        assert!(config.contains("http,a2a"));
    }

    #[test]
    fn langchain_scaffold_uses_native_langchain_layout() {
        let temp = tempfile::TempDir::new().unwrap();
        let project_dir = temp.path().join("assistant-agent");
        let result = WizardResult {
            agent_name_kebab: "assistant-agent".to_string(),
            agent_name_pascal: "AssistantAgent".to_string(),
            description: "Answers user questions".to_string(),
            framework: Framework::LangChain,
            provider: ModelProvider::OpenAi,
            model: "gpt-4.1".to_string(),
            protocols: vec!["http".to_string(), "a2a".to_string()],
            resources: ResourcePreset::Small,
            python_version: "3.12".to_string(),
        };

        let root = scaffold_project(&result, Some(&project_dir)).unwrap();

        assert!(root.join("pyproject.toml").is_file());
        assert!(root
            .join("app")
            .join("AssistantAgent")
            .join("agent.py")
            .is_file());
        assert!(!root
            .join("app")
            .join("AssistantAgent")
            .join("main.py")
            .exists());

        let agent_py =
            std::fs::read_to_string(root.join("app").join("AssistantAgent").join("agent.py"))
                .unwrap();
        assert!(agent_py.contains("from langchain.agents import create_agent"));
        assert!(agent_py.contains("agent = create_agent"));

        let pyproject = std::fs::read_to_string(root.join("pyproject.toml")).unwrap();
        assert!(pyproject.contains("langchain>=1.0"));
        assert!(!pyproject.contains("fastapi"));

        let config =
            std::fs::read_to_string(root.join("gravixlayer").join("gravixlayer.json")).unwrap();
        assert!(config.contains("python -m gravixlayer.runtime.autoserve"));
        assert!(config.contains("start_command"));
        assert!(config.contains("\"agent\""));
        assert!(config.contains("\"serve\""));
        assert!(config.contains("langchain"));
        assert!(config.contains("--protocols"));
        assert!(config.contains("http,a2a"));
    }
}
