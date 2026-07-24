// src/config/project.rs — gravixlayer.json project configuration.
//
// The project file lives at `<project_root>/gravixlayer/gravixlayer.json`.
// `GravixlayerProject::find()` walks up from the current directory until it
// locates this file, mirroring the behaviour of tools like `cargo` and `git`.
//
// Schema is intentionally permissive — unknown fields are silently ignored.

use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const PROJECT_DIR: &str = "gravixlayer";
const PROJECT_FILE: &str = "gravixlayer.json";

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GravixlayerProject {
    /// Human-readable agent name.
    pub name: Option<String>,

    /// Agent description shown in the dashboard.
    pub description: Option<String>,

    /// Framework used by this agent (langgraph, crewai, google-adk, etc.).
    pub framework: Option<String>,

    /// Python version to build against (e.g. "3.12").
    pub python_version: Option<String>,

    /// Node.js version to build against (e.g. "20").
    pub node_version: Option<String>,

    /// Path to pip requirements file (relative to project root).
    pub requirements: Option<String>,

    /// Application entrypoint command, e.g. "python -m simple_agent.app".
    #[serde(default, alias = "entry_point")]
    pub entrypoint: Option<String>,

    /// Framework-specific target to serve when a project exposes multiple agents/graphs/apps.
    #[serde(default, alias = "langgraph_target", alias = "langgraph_graph")]
    pub target: Option<String>,

    /// Entry-point command to start the agent (e.g. ["python", "main.py"]).
    pub start_command: Option<Vec<String>>,

    /// Health-check HTTP path (default: "/health").
    pub health_check_path: Option<String>,

    /// Exposed port for the agent HTTP server (default: 8000 for agent deploys).
    #[serde(default, alias = "http_port")]
    pub port: Option<u16>,

    /// Optional A2A protocol port. If omitted, the router falls back to HTTP port.
    pub a2a_port: Option<u16>,

    /// Optional MCP protocol port. If omitted, the router falls back to HTTP port.
    pub mcp_port: Option<u16>,

    /// Protocols to expose for deployed agents, e.g. ["http", "a2a"].
    #[serde(default)]
    pub protocols: Vec<String>,

    /// Whether the deployed agent endpoint should be public by default.
    pub is_public: Option<bool>,

    /// Runtime environment variables to inject at deploy time.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Files / globs to exclude from the source archive (in addition to the
    /// built-in list which mirrors the Python SDK's _ARCHIVE_EXCLUDE_PATTERNS).
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Template / base image to use for builds.
    pub template: Option<String>,

    /// Cloud for deployments.
    #[serde(alias = "provider")]
    pub cloud: Option<String>,

    /// Deployment region.
    pub region: Option<String>,

    /// A2A Agent Card metadata (optional).
    pub agent_card: Option<AgentCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentCard {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<AgentSkill>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<AgentCapabilities>,
    #[serde(default)]
    #[serde(alias = "default_input_modes")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub default_input_modes: Vec<String>,
    #[serde(default)]
    #[serde(alias = "default_output_modes")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub default_output_modes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentSkill {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub examples: Vec<String>,
    #[serde(default, alias = "input_modes", skip_serializing_if = "Vec::is_empty")]
    pub input_modes: Vec<String>,
    #[serde(default, alias = "output_modes", skip_serializing_if = "Vec::is_empty")]
    pub output_modes: Vec<String>,
    #[serde(
        default,
        alias = "security_requirements",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub security_requirements: Vec<HashMap<String, Vec<String>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapabilities {
    pub streaming: Option<bool>,
    #[serde(alias = "push_notifications")]
    pub push_notifications: Option<bool>,
    #[serde(alias = "state_transition_history")]
    pub state_transition_history: Option<bool>,
    #[serde(alias = "extended_agent_card")]
    pub extended_agent_card: Option<bool>,
}

// ---------------------------------------------------------------------------
// Discovery and load
// ---------------------------------------------------------------------------

impl GravixlayerProject {
    /// Walk up from `start_dir` looking for `gravixlayer/gravixlayer.json`.
    ///
    /// Returns `(project, project_root_dir)` where `project_root_dir` is the
    /// directory that contains the `gravixlayer/` sub-directory.
    pub fn find(start_dir: &Path) -> Option<(Self, PathBuf)> {
        let mut dir = start_dir.to_path_buf();
        loop {
            let candidate = dir.join(PROJECT_DIR).join(PROJECT_FILE);
            if candidate.is_file() {
                match std::fs::read_to_string(&candidate) {
                    Ok(raw) => match serde_json::from_str::<Self>(&raw) {
                        Ok(proj) => return Some((proj, dir)),
                        Err(e) => {
                            tracing::warn!(
                                "found {} but failed to parse it: {e}",
                                candidate.display()
                            );
                            return None;
                        }
                    },
                    Err(e) => {
                        tracing::warn!("cannot read {}: {e}", candidate.display());
                        return None;
                    }
                }
            }
            match dir.parent() {
                Some(parent) => dir = parent.to_path_buf(),
                None => return None,
            }
        }
    }

    /// Load from a specific path (rather than auto-discovering).
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        let proj = serde_json::from_str::<Self>(&raw)
            .map_err(|e| anyhow::anyhow!("failed to parse {}: {e}", path.display()))?;
        Ok(proj)
    }

    /// Save / write to a specific path (atomic: write to tmp then rename).
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, &json)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Convenience: find from the current working directory.
    pub fn find_from_cwd() -> Option<(Self, PathBuf)> {
        let cwd = env::current_dir().ok()?;
        Self::find(&cwd)
    }

    /// Path to `gravixlayer/gravixlayer.json` relative to a project root.
    #[allow(dead_code)]
    pub fn default_file_path(project_root: &Path) -> PathBuf {
        project_root.join(PROJECT_DIR).join(PROJECT_FILE)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn find_walks_up_to_project_file() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let sub = root.join("src").join("cmd");
        std::fs::create_dir_all(&sub).unwrap();

        // Create the project file under root/gravixlayer/gravixlayer.json
        let proj_dir = root.join(PROJECT_DIR);
        std::fs::create_dir_all(&proj_dir).unwrap();
        std::fs::write(
            proj_dir.join(PROJECT_FILE),
            r#"{"name":"test-agent","framework":"langgraph"}"#,
        )
        .unwrap();

        // Start search from a nested subdirectory
        let (proj, found_root) = GravixlayerProject::find(&sub).unwrap();
        assert_eq!(proj.name.as_deref(), Some("test-agent"));
        assert_eq!(found_root, root);
    }

    #[test]
    fn find_returns_none_when_no_project_file() {
        let dir = TempDir::new().unwrap();
        assert!(GravixlayerProject::find(dir.path()).is_none());
    }

    #[test]
    fn roundtrip_serialization() {
        let mut proj = GravixlayerProject::default();
        proj.name = Some("my-agent".into());
        proj.framework = Some("crewai".into());
        proj.port = Some(8080);

        let json = serde_json::to_string_pretty(&proj).unwrap();
        let restored: GravixlayerProject = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name.as_deref(), Some("my-agent"));
        assert_eq!(restored.port, Some(8080));
    }

    #[test]
    fn agent_card_accepts_snake_case_and_serializes_camel_case() {
        let project: GravixlayerProject = serde_json::from_str(
            r#"{
                "agent_card": {
                    "name": "Demo",
                    "description": "Demo agent",
                    "default_input_modes": ["text/plain"],
                    "default_output_modes": ["text/plain"],
                    "capabilities": { "push_notifications": true },
                    "skills": [{
                        "id": "demo",
                        "name": "Demo",
                        "description": "Demo skill",
                        "tags": ["demo"],
                        "input_modes": ["text/plain"],
                        "output_modes": ["text/plain"]
                    }]
                }
            }"#,
        )
        .unwrap();

        let value = serde_json::to_value(project.agent_card.unwrap()).unwrap();

        assert_eq!(value["defaultInputModes"][0], "text/plain");
        assert_eq!(value["capabilities"]["pushNotifications"], true);
        assert_eq!(value["skills"][0]["inputModes"][0], "text/plain");
        assert!(value.get("default_input_modes").is_none());
    }
}
