// src/framework.rs — Canonical agent framework and protocol vocabulary.

use anyhow::{bail, Result};

pub const CANONICAL_FRAMEWORKS: &[&str] = &[
    "langgraph",
    "langchain",
    "crewai",
    "google-adk",
    "openai-agents",
    "anthropic",
    "strands",
    "python",
];

pub const CANONICAL_PROTOCOLS: &[&str] = &["http", "a2a", "mcp"];

pub fn canonical_framework(value: &str) -> Option<&'static str> {
    match normalize_key(value).as_str() {
        "langgraph" => Some("langgraph"),
        "langchain" => Some("langchain"),
        "crewai" | "crew" => Some("crewai"),
        "google-adk" | "google_adk" | "adk" => Some("google-adk"),
        "openai-agents" | "openai_agents" | "openai" => Some("openai-agents"),
        "anthropic" | "claude" | "claude-agent" | "claude_agent" | "claude-agent-sdk" => {
            Some("anthropic")
        }
        "strands" | "strands-agents" | "strands_agents" => Some("strands"),
        "python" | "generic" | "custom" | "fastapi" => Some("python"),
        "a2a" | "a2a-native" | "a2a_native" => None,
        _ => None,
    }
}

pub fn canonical_protocol(value: &str) -> Option<&'static str> {
    match normalize_key(value).as_str() {
        "http" | "https" => Some("http"),
        "a2a" => Some("a2a"),
        "mcp" => Some("mcp"),
        _ => None,
    }
}

pub fn normalize_framework(value: &str) -> Result<String> {
    canonical_framework(value)
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!(unknown_framework_message(value)))
}

pub fn normalize_protocols(values: &[String]) -> Result<Vec<String>> {
    let mut protocols = Vec::new();
    for value in values {
        let Some(protocol) = canonical_protocol(value) else {
            bail!(
                "unknown protocol '{}'; expected one of: {}",
                value,
                CANONICAL_PROTOCOLS.join(", ")
            );
        };
        if !protocols.iter().any(|item| item == protocol) {
            protocols.push(protocol.to_string());
        }
    }
    if protocols.is_empty() {
        protocols.push("http".to_string());
    }
    Ok(protocols)
}

pub fn validate_protocol_compatibility(
    framework: Option<&str>,
    protocols: &[String],
    inferred_protocols: &[String],
) -> Result<()> {
    if !protocols.iter().any(|protocol| protocol == "a2a") {
        return Ok(());
    }

    if inferred_protocols.iter().any(|protocol| protocol == "a2a") {
        return Ok(());
    }

    let Some(framework) = framework.and_then(canonical_framework) else {
        return Ok(());
    };

    if matches!(
        framework,
        "langgraph" | "langchain" | "google-adk" | "strands" | "python"
    ) {
        return Ok(());
    }

    bail!(
        "protocol 'a2a' is not enabled automatically for framework '{}'. A2A is a protocol; add an A2A server/adapter to the project or use a framework with a supported A2A adapter (langgraph, langchain, google-adk, strands, python).",
        framework
    )
}

pub fn unknown_framework_message(value: &str) -> String {
    if normalize_key(value).starts_with("a2a") {
        return format!(
            "'{}' is a protocol, not an agent framework. Use --framework with one of: {} and add --protocol a2a when the project exposes A2A.",
            value,
            CANONICAL_FRAMEWORKS.join(", ")
        );
    }
    format!(
        "unknown framework '{}'; expected one of: {}",
        value,
        CANONICAL_FRAMEWORKS.join(", ")
    )
}

fn normalize_key(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace('_', "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_framework_aliases() {
        assert_eq!(canonical_framework("openai"), Some("openai-agents"));
        assert_eq!(canonical_framework("claude-agent-sdk"), Some("anthropic"));
        assert_eq!(canonical_framework("strands_agents"), Some("strands"));
        assert_eq!(canonical_framework("a2a"), None);
    }

    #[test]
    fn dedupes_protocols() {
        let protocols =
            normalize_protocols(&["http".into(), "https".into(), "a2a".into()]).unwrap();
        assert_eq!(protocols, vec!["http", "a2a"]);
    }
}
