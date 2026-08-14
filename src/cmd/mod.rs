// src/cmd/mod.rs — Command handler module declarations.

pub mod agent;
pub mod agent_serve;
pub mod auth;
pub mod billing;
pub mod config;
pub mod doctor;
pub mod network_policy;
pub mod package;
pub mod provider;
pub mod runtime;
pub mod snapshot;
pub mod template;
pub mod update;
pub mod validate;

// ---------------------------------------------------------------------------
// Shared validation utilities
// ---------------------------------------------------------------------------

/// Reject user-supplied resource IDs that could inject path segments.
///
/// Valid IDs from the API are alphanumeric with hyphens (e.g. `ag-abc123`,
/// `rt-xyz789`).  Reject anything containing `/`, `\`, `..`, or `%`.
pub fn validate_resource_id(id: &str, kind: &str) -> anyhow::Result<()> {
    if id.is_empty() {
        anyhow::bail!("{kind} ID must not be empty");
    }
    if id.contains('/') || id.contains('\\') || id.contains("..") || id.contains('%') {
        anyhow::bail!("invalid {kind} ID '{}': contains disallowed characters", id);
    }
    Ok(())
}
