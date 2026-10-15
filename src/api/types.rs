// src/api/types.rs — Serde response types for all GravixLayer API resources.
//
// All structs use `#[serde(deny_unknown_fields = false)]` (the default) so
// future API additions are silently ignored.  Fields that may be absent in
// some response variants are wrapped in `Option<T>`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Runtime
// ---------------------------------------------------------------------------

/// A single runtime returned by the API.
///
/// Mirrors `RuntimeAPIResponse` in the Go backend and `Runtime` dataclass in
/// the Python SDK (`gravixlayer/types/runtime.py`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runtime {
    /// Control plane always returns `runtime_id`; older/native paths may send `id`.
    #[serde(alias = "id")]
    pub runtime_id: String,
    pub status: String,
    pub template: Option<String>,
    pub template_id: Option<String>,
    /// Prefer `cloud`; accept deprecated `provider` / `compute_provider` aliases.
    #[serde(alias = "provider", alias = "compute_provider")]
    pub cloud: Option<String>,
    #[serde(alias = "compute_region")]
    pub region: Option<String>,
    pub started_at: Option<String>,
    pub timeout_at: Option<String>,
    pub cpu_count: Option<u32>,
    pub memory_mb: Option<u64>,
    pub disk_size_mb: Option<u64>,
    /// Prefer `metadata`; accept legacy `tags` alias (matches Python SDK normalizer).
    #[serde(alias = "tags")]
    pub metadata: Option<HashMap<String, String>>,
    pub ended_at: Option<String>,
    pub ip_address: Option<String>,
    pub ssh_enabled: Option<bool>,
    pub internet_access: Option<bool>,
    pub agent_id: Option<String>,
}

/// Paginated list of runtimes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeList {
    pub runtimes: Vec<Runtime>,
    pub total: Option<u64>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Response after a kill / terminate operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeKillResponse {
    pub runtime_id: Option<String>,
    pub message: Option<String>,
    pub status: Option<String>,
}

/// Response for pause / resume operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatusResponse {
    pub runtime_id: Option<String>,
    pub status: Option<String>,
    pub message: Option<String>,
}

/// Response for `POST /v1/agents/runtime/<id>/timeout`.
///
/// Matches the Python SDK's `RuntimeTimeoutResponse`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeTimeoutResponse {
    pub message: Option<String>,
    pub timeout: Option<i64>,
    pub timeout_at: Option<String>,
}

/// Result of `POST /v1/agents/runtime/<id>/fork` and
/// `POST /v1/agents/snapshots/<id>/fork`.
///
/// Fork is per-child: every position in `children` is a started runtime or an
/// error, and the request itself does not fail over a single child — the same
/// shape both SDKs expose (`children` is `Runtime | ForkError`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkResponse {
    #[serde(default)]
    pub children: Vec<ForkChild>,
    pub snapshot_id: Option<String>,
    pub snapshot_persisted: Option<bool>,
    /// Present on a live fork; absent on a snapshot fork.
    pub parent_runtime_id: Option<String>,
}

impl ForkResponse {
    /// Children that started, in request order.
    pub fn runtimes(&self) -> impl Iterator<Item = &Runtime> {
        self.children.iter().filter_map(ForkChild::runtime)
    }

    /// Per-child failures, in request order.
    pub fn errors(&self) -> impl Iterator<Item = (&str, &str)> {
        self.children.iter().filter_map(|c| match c {
            ForkChild::Error { error, code } => Some((error.as_str(), code.as_str())),
            _ => None,
        })
    }
}

/// One child entry in a fork batch: `{ "runtime": {...} }` on success or
/// `{ "error": ..., "code": ... }` on failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ForkChild {
    Runtime { runtime: Runtime },
    Error { error: String, code: String },
}

impl ForkChild {
    pub fn runtime(&self) -> Option<&Runtime> {
        match self {
            Self::Runtime { runtime } => Some(runtime),
            _ => None,
        }
    }
}

/// A command tracked inside a runtime (running or retained after exit).
///
/// Matches the API's `commands` payload and the SDKs' `CommandInfo`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInfo {
    pub pid: u32,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub working_dir: String,
    #[serde(default)]
    pub background: bool,
    pub status: String,
    pub exit_code: Option<i32>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub duration_ms: Option<i64>,
    #[serde(default)]
    pub timed_out: bool,
}

/// `GET /v1/agents/runtime/<id>/commands` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandList {
    #[serde(default)]
    pub commands: Vec<CommandInfo>,
}

/// Resource metrics snapshot.
///
/// Runtime metrics returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeMetrics {
    pub runtime_id: Option<String>,
    pub timestamp: Option<String>,
    /// CPU usage percentage (0-100)
    pub cpu_usage: Option<f64>,
    /// Memory used in MB
    pub memory_usage: Option<f64>,
    /// Total memory in MB
    pub memory_total: Option<f64>,
    /// Disk bytes read
    pub disk_read: Option<i64>,
    /// Disk bytes written
    pub disk_write: Option<i64>,
    /// Network bytes received
    pub network_rx: Option<i64>,
    /// Network bytes sent
    pub network_tx: Option<i64>,
    pub load_avg_1m: Option<f64>,
    pub load_avg_5m: Option<f64>,
    pub load_avg_15m: Option<f64>,
    pub uptime_seconds: Option<i64>,
    pub process_count: Option<i64>,
    pub iowait_percent: Option<f64>,
}

/// Payload for `POST /v1/agents/runtime`.
#[derive(Debug, Clone, Serialize)]
pub struct CreateRuntimeRequest {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub template: String,
    #[serde(rename = "cloud")]
    pub cloud: String,
    pub region: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internet_access: Option<bool>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub env_vars: HashMap<String, String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub providers: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub network_policy_ids: Vec<String>,
    /// Named snapshot id or name. Mutually exclusive with `template`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
    /// Storage mounts to attach at boot (≤8; paths absolute, non-nesting).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mounts: Vec<MountSpec>,
}

/// Payload for `POST /v1/agents/runtime/<id>/timeout`.
#[derive(Debug, Clone, Serialize)]
pub struct SetTimeoutRequest {
    pub timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConnectResponse {
    pub runtime_id: String,
    pub status: Option<String>,
    pub domain: Option<String>,
    pub message: Option<String>,
}

/// Persistent code context. The API emits the id twice — `id` and
/// `context_id`, same value — so serde's `alias` (which rejects a field
/// seen twice) cannot express the shape. Both keys are read and
/// `context_id` wins, matching how the SDKs normalize.
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeCodeContext {
    pub context_id: String,
    pub language: Option<String>,
    pub cwd: Option<String>,
    pub vm_id: Option<String>,
}

impl<'de> Deserialize<'de> for RuntimeCodeContext {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            context_id: Option<String>,
            id: Option<String>,
            language: Option<String>,
            cwd: Option<String>,
            vm_id: Option<String>,
        }
        let raw = Raw::deserialize(deserializer)?;
        Ok(Self {
            context_id: raw.context_id.or(raw.id).unwrap_or_default(),
            language: raw.language,
            cwd: raw.cwd,
            vm_id: raw.vm_id,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeCodeContextDeleteResponse {
    pub message: Option<String>,
}

/// Response from `POST /v1/agents/runtime/<id>/ssh/enable`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshEnableResponse {
    pub runtime_id: String,
    pub enabled: bool,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub connect_cmd: Option<String>,
    pub private_key: Option<String>,
    pub public_key: Option<String>,
    pub ssh_config: Option<String>,
    pub message: Option<String>,
}

/// Response from `POST /v1/agents/runtime/<id>/ssh/disable`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshDisableResponse {
    pub runtime_id: String,
    pub disabled: Option<bool>,
    pub message: Option<String>,
}

/// Response from `GET /v1/agents/runtime/<id>/ssh/status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshStatusResponse {
    pub runtime_id: String,
    pub enabled: bool,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub daemon_running: Option<bool>,
}

// ---------------------------------------------------------------------------
// Template (container image / environment)
// ---------------------------------------------------------------------------

/// A single template entry returned by the API.
///
/// Mirrors the Python SDK's `TemplateInfo`. The API's primary key field is
/// `id`; some older responses used `template_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub template_id: Option<String>,
    pub id: Option<String>,
    pub name: Option<String>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub framework: Option<String>,
    pub python_version: Option<String>,
    pub node_version: Option<String>,
    pub size_mb: Option<f64>,
    pub vcpu_count: Option<u32>,
    pub memory_mb: Option<u64>,
    pub disk_size_mb: Option<u64>,
    pub visibility: Option<String>,
    /// Product split: `sandbox` | `agent` (set by build path, not by clients).
    pub kind: Option<String>,
    pub http_port: Option<u16>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub is_public: Option<bool>,
    #[serde(alias = "provider")]
    pub cloud: Option<String>,
    pub region: Option<String>,
    pub is_active: Option<bool>,
}

impl Template {
    /// Return the canonical ID regardless of which field the server populated.
    pub fn canonical_id(&self) -> &str {
        self.template_id
            .as_deref()
            .or(self.id.as_deref())
            .unwrap_or("")
    }
}

/// Paginated list of templates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateList {
    pub templates: Vec<Template>,
    pub total: Option<u64>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Response after starting a template build.
///
/// Mirrors the Python SDK's `TemplateBuildResponse`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateBuildResponse {
    pub build_id: String,
    pub template_id: Option<String>,
    pub status: Option<String>,
    pub message: Option<String>,
}

/// Build status polling response.
///
/// Mirrors the Python SDK's `TemplateBuildStatus`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateBuildStatus {
    pub build_id: String,
    pub template_id: Option<String>,
    pub status: String,
    pub phase: Option<String>,
    #[serde(default)]
    pub progress_percent: Option<i32>,
    pub error: Option<String>,
    pub message: Option<String>,
    pub logs: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Template snapshot info from `GET /v1/agents/template/<id>/snapshot`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSnapshot {
    pub template_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub has_snapshot: bool,
    pub vcpu_count: Option<u32>,
    pub memory_mb: Option<u64>,
    pub created_at: Option<String>,
    pub cellcore_version: Option<String>,
    pub snapshot_size_bytes: Option<i64>,
}

// ---------------------------------------------------------------------------
// Named user snapshots (catalog)
// ---------------------------------------------------------------------------

/// A project-scoped named snapshot from `/v1/agents/snapshots`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub is_active: Option<bool>,
    #[serde(default)]
    pub distribution_status: Option<String>,
    #[serde(alias = "provider")]
    pub cloud: Option<String>,
    pub region: Option<String>,
    pub vcpu_count: Option<u32>,
    pub memory_mb: Option<u64>,
    pub disk_size_mb: Option<u64>,
    pub visibility: Option<String>,
    pub size_bytes: Option<i64>,
    pub source_runtime_id: Option<String>,
    pub source_template_id: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotList {
    #[serde(default)]
    pub snapshots: Vec<Snapshot>,
    pub total: Option<u64>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateSnapshotRequest {
    pub name: String,
    pub runtime_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

// ---------------------------------------------------------------------------
// Agent (deployed endpoint)
// ---------------------------------------------------------------------------

/// Response after starting an agent build.
///
/// Mirrors `AgentBuildResponse` in `gravixlayer/types/agents.py`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBuildResponse {
    pub build_id: String,
    pub template_id: Option<String>,
    pub status: Option<String>,
    pub message: Option<String>,
}

/// Agent build status polling response.
///
/// Mirrors `AgentBuildStatusResponse` in `gravixlayer/types/agents.py`
/// (same wire shape as template build status).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBuildStatusResponse {
    pub build_id: String,
    pub template_id: Option<String>,
    pub status: String,
    pub phase: Option<String>,
    #[serde(default)]
    pub progress_percent: Option<i32>,
    pub error: Option<String>,
    pub message: Option<String>,
    pub logs: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

/// Response after deploying an agent.
///
/// Mirrors `AgentDeployResponse` in the Python SDK (`gravixlayer/types/agents.py`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDeployResponse {
    pub agent_id: String,
    pub runtime_id: Option<String>,
    pub name: Option<String>,
    pub framework: Option<String>,
    pub status: Option<String>,
    pub endpoint: Option<String>,
    pub a2a_endpoint: Option<String>,
    pub mcp_endpoint: Option<String>,
    pub agent_card_url: Option<String>,
    pub internal_endpoint: Option<String>,
    pub dns_status: Option<String>,
    pub created_at: Option<String>,
    pub message: Option<String>,
}

/// A deployed agent endpoint.
///
/// Mirrors `AgentEndpoint` in the Python SDK (`gravixlayer/types/agents.py`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEndpoint {
    pub agent_id: String,
    pub endpoint: Option<String>,
    pub internal_endpoint: Option<String>,
    pub a2a_endpoint: Option<String>,
    pub mcp_endpoint: Option<String>,
    pub agent_card_url: Option<String>,
    pub name: Option<String>,
    pub status: Option<String>,
    pub health: Option<String>,
    pub dns_status: Option<String>,
    pub framework: Option<String>,
    pub protocols: Option<serde_json::Value>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Response after destroying an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDestroyResponse {
    pub agent_id: Option<String>,
    pub message: Option<String>,
    pub status: Option<String>,
}

/// Payload for `POST /v1/agents/deploy`.
///
/// Field names and types mirror `AgentDeployRequest` in the Python SDK
/// (`gravixlayer/types/agents.py`).  Only non-zero / non-empty fields are
/// serialised (matching the SDK's `to_dict()` implementation).
#[derive(Debug, Clone, Serialize)]
pub struct DeployAgentRequest {
    pub template_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub framework: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_point: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub a2a_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_port: Option<u16>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub protocols: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_public: Option<bool>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub environment: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_card: Option<serde_json::Value>,
}

/// Metadata attached to an agent multipart build request.
///
/// Field names match the agent build API contract.
#[derive(Debug, Clone, Serialize)]
pub struct AgentBuildMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entrypoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub python_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub framework: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub ports: Vec<u16>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub environment: HashMap<String, String>,
    /// Number of vCPUs for the template VM (matches backend field `vcpu_count`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vcpu_count: Option<u32>,
    /// Memory in MB for the template VM (matches backend field `memory_mb`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_mb: Option<u32>,
    /// Disk size in MB for the template VM (matches backend field `disk_mb`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_mb: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_cmd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ready_cmd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ready_timeout_secs: Option<u32>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub tags: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Billing
// ---------------------------------------------------------------------------

/// Billing summary for a period.
///
/// Billing summary returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingSummary {
    pub account_id: Option<String>,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub total_cpu_hours: Option<f64>,
    pub total_ram_gb_hours: Option<f64>,
    pub total_storage_gb_hours: Option<f64>,
    pub cpu_cost: Option<f64>,
    pub ram_cost: Option<f64>,
    pub storage_cost: Option<f64>,
    pub total_cost: Option<f64>,
    pub total_runtimes: Option<i64>,
    pub active_runtimes: Option<i64>,
    pub provider_breakdown: Option<HashMap<String, f64>>,
    pub region_breakdown: Option<HashMap<String, f64>>,
    pub template_breakdown: Option<HashMap<String, f64>>,
}

/// A single runtime billing record returned by `GET /v1/agents/billing/history`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingItem {
    pub billing_id: Option<String>,
    pub runtime_id: Option<String>,
    pub session_start: Option<String>,
    pub session_end: Option<String>,
    pub duration_seconds: Option<f64>,
    pub cpu_cost: Option<f64>,
    pub ram_cost: Option<f64>,
    pub storage_cost: Option<f64>,
    pub total_cost: Option<f64>,
    pub billing_status: Option<String>,
}

/// Paginated billing history.
///
/// Mirrors the response from `GET /v1/agents/billing/history`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingHistory {
    #[serde(alias = "records", default)]
    pub items: Vec<BillingItem>,
    pub total: Option<u64>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub pages: Option<u64>,
}

/// Quota and limit information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingQuota {
    pub account_id: Option<String>,
    pub tier_id: Option<String>,
    pub tier_name: Option<String>,
    pub tier_display_name: Option<String>,
    pub vcpu_limit: Option<u32>,
    pub ram_gb_limit: Option<u32>,
    pub disk_gb_limit: Option<u32>,
    pub api_requests_per_min: Option<u32>,
    pub runtime_creation_per_min: Option<u32>,
    pub runtime_lifecycle_per_min: Option<u32>,
    pub vcpu_used: Option<u32>,
    pub ram_mb_used: Option<u32>,
    pub disk_mb_used: Option<u32>,
    #[serde(rename = "is_custom_override")]
    pub is_custom_override: Option<bool>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Secret Providers (/v1/identity/providers)
// ---------------------------------------------------------------------------

/// Masked secret pair (values are write-only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretInfo {
    pub id: String,
    pub key: String,
    #[serde(default)]
    pub value_set: bool,
    #[serde(default)]
    pub masked: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Secret provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretProvider {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub account_id: Option<String>,
    pub project_id: Option<String>,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(default)]
    pub is_system: bool,
    #[serde(default)]
    pub secret_count: i64,
    #[serde(default)]
    pub secrets: Vec<SecretInfo>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretProviderEnvelope {
    pub provider: SecretProvider,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretProviderList {
    #[serde(default)]
    pub providers: Vec<SecretProvider>,
    #[serde(default)]
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretEnvelope {
    pub secret: SecretInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretList {
    #[serde(default)]
    pub secrets: Vec<SecretInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessEnvelope {
    #[serde(default = "default_true")]
    pub success: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateSecretProviderRequest {
    pub name: String,
    pub provider_type: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub secrets: Vec<SecretPairRequest>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SecretPairRequest {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateSecretProviderRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateSecretRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttachProviderRequest {
    pub runtime_id: String,
}

// ---------------------------------------------------------------------------
// Network Policies (/v1/network-policies)
// ---------------------------------------------------------------------------

/// Single destination/port/protocol egress rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicyRule {
    pub id: String,
    pub policy_id: String,
    pub destination: String,
    #[serde(default)]
    pub port: i64,
    #[serde(default = "default_tcp")]
    pub protocol: String,
    pub account_id: Option<String>,
    pub description: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

fn default_tcp() -> String {
    "tcp".to_string()
}

/// Network policy (egress firewall).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    pub id: String,
    pub name: String,
    pub egress_mode: String,
    pub account_id: Option<String>,
    pub project_id: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub is_system: bool,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(default)]
    pub rule_count: i64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicyEnvelope {
    pub policy: NetworkPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicyList {
    #[serde(default)]
    pub policies: Vec<NetworkPolicy>,
    #[serde(default)]
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicyRuleEnvelope {
    pub rule: NetworkPolicyRule,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicyRuleList {
    #[serde(default)]
    pub rules: Vec<NetworkPolicyRule>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateNetworkPolicyRequest {
    pub name: String,
    pub egress_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateNetworkPolicyRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub egress_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_default: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddNetworkPolicyRuleRequest {
    pub destination: String,
    pub port: i64,
    pub protocol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateNetworkPolicyRuleRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttachNetworkPolicyRequest {
    pub runtime_id: String,
}

// ---------------------------------------------------------------------------
// Storage mounts (/v1/agents/buckets, /v1/agents/bucket-links, runtime mounts)
// ---------------------------------------------------------------------------

/// A mount request: bucket, link, or git repository at a guest path.
///
/// `bucket` and `link` are mutually exclusive. Git mounts are read-only and
/// take an https clone URL; credentials are secret-provider references.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MountSpec {
    pub kind: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_seconds: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_secret: Option<String>,
}

/// Validate a mount before it leaves the process — the API validates again,
/// but a local check turns a confusing 400 into an immediate, precise error.
pub fn validate_mount(m: &MountSpec) -> Result<(), String> {
    match m.kind.as_str() {
        "bucket" | "git" => {}
        other => {
            return Err(format!(
                "mount kind must be 'bucket' or 'git', got '{other}'"
            ))
        }
    }
    if !m.path.starts_with('/')
        || m.path == "/"
        || m.path.ends_with('/')
        || m.path.contains("//")
        || m.path.split('/').any(|s| s == "..")
    {
        return Err(format!(
            "mount path must be a normalized absolute path below the guest root, got '{}'",
            m.path
        ));
    }
    if let Some(mode) = &m.mode {
        if !matches!(mode.as_str(), "ro" | "rw") {
            return Err(format!("mount mode must be 'ro' or 'rw', got '{mode}'"));
        }
        if m.kind == "git" && mode == "rw" {
            return Err("git mounts are read-only".to_string());
        }
    }
    if m.kind == "bucket" {
        match (m.bucket.as_deref(), m.link.as_deref()) {
            (Some(_), Some(_)) => {
                return Err("a bucket mount takes bucket or link, not both".to_string())
            }
            (None, None) => {
                return Err("a bucket mount needs bucket or link (id or name)".to_string())
            }
            _ => {}
        }
    } else {
        match &m.url {
            Some(u) if u.to_ascii_lowercase().starts_with("https://") => {}
            _ => return Err("a git mount needs an https:// url".to_string()),
        }
        if m.credential_provider.is_some() != m.credential_secret.is_some() {
            return Err(
                "git credentials need credential_provider and credential_secret".to_string(),
            );
        }
        for (name, v) in [("depth", m.depth), ("refresh_seconds", m.refresh_seconds)] {
            if let Some(n) = v {
                if !(0..=u32::MAX as i64).contains(&n) {
                    return Err(format!("git {name} must be between 0 and {}", u32::MAX));
                }
            }
        }
    }
    // Stray fields for the other kind are ambiguous input — refuse them
    // rather than silently dropping.
    match m.kind.as_str() {
        "bucket" => {
            if m.url.is_some()
                || m.git_ref.is_some()
                || m.depth.is_some()
                || m.refresh_seconds.is_some()
            {
                return Err("url/git_ref/depth/refresh_seconds are for git mounts".into());
            }
        }
        "git" if m.bucket.is_some() || m.link.is_some() => {
            return Err("bucket/link are for bucket mounts, not git".into());
        }
        "git" => {}
        _ => {}
    }
    Ok(())
}

/// Payload for `POST /v1/agents/buckets`.
#[derive(Debug, Clone, Serialize)]
pub struct CreateBucketRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

/// Payload for `POST /v1/agents/bucket-links`.
#[derive(Debug, Clone, Serialize)]
pub struct CreateBucketLinkRequest {
    pub name: String,
    pub provider: String,
    pub bucket: String,
    pub auth_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_style: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_key_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_token_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_arn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

/// Response for `GET /v1/agents/buckets` and `/bucket-links`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageBucket {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub bucket: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketLink {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub bucket: String,
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub path_style: bool,
    #[serde(default)]
    pub auth_mode: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub verify_error: String,
    #[serde(default)]
    pub last_verified_at: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketEnvelope {
    pub bucket: StorageBucket,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketListEnvelope {
    #[serde(default)]
    pub buckets: Vec<StorageBucket>,
    #[serde(default)]
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkEnvelope {
    pub link: BucketLink,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkListEnvelope {
    #[serde(default)]
    pub links: Vec<BucketLink>,
    #[serde(default)]
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeMount {
    pub id: String,
    #[serde(default)]
    pub runtime_id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default)]
    pub mount_path: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub provenance: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub detached_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountEnvelope {
    pub attachment: RuntimeMount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountListEnvelope {
    #[serde(default)]
    pub mounts: Vec<RuntimeMount>,
}

// ---------------------------------------------------------------------------
// Auth / whoami
// ---------------------------------------------------------------------------

/// Response for whoami / auth status checks.
// TODO: used once `auth whoami` is re-enabled (requires JWT-compatible endpoint).
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhoAmI {
    pub account_id: Option<String>,
    pub email: Option<String>,
    pub plan: Option<String>,
    pub created_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mount(kind: &str, path: &str) -> MountSpec {
        MountSpec {
            kind: kind.to_string(),
            path: path.to_string(),
            mode: None,
            required: None,
            bucket: None,
            link: None,
            url: None,
            git_ref: None,
            depth: None,
            refresh_seconds: None,
            credential_provider: None,
            credential_secret: None,
        }
    }

    #[test]
    fn mount_bucket_by_id_validates() {
        let m = MountSpec {
            bucket: Some("bkt-1".into()),
            mode: Some("rw".into()),
            ..mount("bucket", "/mnt/data")
        };
        validate_mount(&m).unwrap();
    }

    #[test]
    fn mount_git_https_validates() {
        let m = MountSpec {
            url: Some("https://example.com/r.git".into()),
            git_ref: Some("main".into()),
            depth: Some(1),
            ..mount("git", "/mnt/repo")
        };
        validate_mount(&m).unwrap();
    }

    #[test]
    fn mount_rejects_bad_combinations() {
        let cases: Vec<MountSpec> = vec![
            MountSpec {
                bucket: Some("a".into()),
                link: Some("b".into()),
                ..mount("bucket", "/m")
            },
            mount("bucket", "/m"), // no target
            MountSpec {
                url: Some("https://x/y.git".into()),
                mode: Some("rw".into()),
                ..mount("git", "/m")
            },
            MountSpec {
                url: Some("git@x:y".into()),
                ..mount("git", "/m")
            },
            mount("git", "/m"), // no url
            MountSpec {
                bucket: Some("a".into()),
                ..mount("bucket", "rel")
            },
            MountSpec {
                bucket: Some("a".into()),
                ..mount("bucket", "/")
            },
            MountSpec {
                url: Some("https://x/y.git".into()),
                credential_provider: Some("p".into()),
                ..mount("git", "/m")
            },
            MountSpec {
                bucket: Some("a".into()),
                mode: Some("exec".into()),
                ..mount("bucket", "/m")
            },
        ];
        for (i, m) in cases.iter().enumerate() {
            assert!(validate_mount(m).is_err(), "case {i} should fail: {m:?}");
        }
    }

    #[test]
    fn fork_response_parses_runtime_and_error_children() {
        let body = serde_json::json!({
            "children": [
                { "runtime": { "runtime_id": "rt-1", "status": "running" } },
                { "error": "capacity exhausted", "code": "capacity_exhausted" },
                { "runtime": { "runtime_id": "rt-3", "status": "running" } }
            ],
            "snapshot_id": "snap-1",
            "snapshot_persisted": false,
            "parent_runtime_id": "rt-parent"
        });
        let resp: ForkResponse = serde_json::from_value(body).unwrap();
        assert_eq!(resp.children.len(), 3);
        assert_eq!(resp.snapshot_id.as_deref(), Some("snap-1"));
        assert_eq!(resp.snapshot_persisted, Some(false));
        assert_eq!(resp.parent_runtime_id.as_deref(), Some("rt-parent"));

        let ids: Vec<&str> = resp.runtimes().map(|r| r.runtime_id.as_str()).collect();
        assert_eq!(ids, ["rt-1", "rt-3"], "errors are skipped, order kept");
        let errs: Vec<(&str, &str)> = resp.errors().collect();
        assert_eq!(errs, [("capacity exhausted", "capacity_exhausted")]);
    }

    #[test]
    fn fork_response_tolerates_missing_optional_fields() {
        // Snapshot fork omits parent_runtime_id; a wholly-empty body must
        // still parse rather than panic.
        let resp: ForkResponse = serde_json::from_str("{}").unwrap();
        assert!(resp.children.is_empty());
        assert!(resp.snapshot_id.is_none());
        assert_eq!(resp.runtimes().count(), 0);
        assert_eq!(resp.errors().count(), 0);
    }

    #[test]
    fn command_list_parses_server_envelope() {
        let body = serde_json::json!({
            "commands": [
                {
                    "pid": 12,
                    "command": "sleep",
                    "args": ["60"],
                    "working_dir": "/",
                    "background": true,
                    "status": "running",
                    "exit_code": null,
                    "started_at": "2026-03-03T10:00:00Z",
                    "ended_at": null,
                    "duration_ms": null,
                    "timed_out": false
                }
            ]
        });
        let list: CommandList = serde_json::from_value(body).unwrap();
        assert_eq!(list.commands.len(), 1);
        let cmd = &list.commands[0];
        assert_eq!(cmd.pid, 12);
        assert_eq!(cmd.command, "sleep");
        assert_eq!(cmd.args, ["60"]);
        assert!(cmd.background);
        assert_eq!(cmd.status, "running");
        assert!(cmd.exit_code.is_none());
    }

    #[test]
    fn code_context_accepts_the_server_dual_id_shape() {
        // The API emits `id` and `context_id` together (same value) — a
        // serde alias would fail "duplicate field". Both must parse.
        let dual: RuntimeCodeContext = serde_json::from_str(
            r#"{"context_id":"c-1","id":"c-1","language":"python","cwd":"/workspace"}"#,
        )
        .unwrap();
        assert_eq!(dual.context_id, "c-1");
        assert_eq!(dual.language.as_deref(), Some("python"));

        // Either key alone still produces the record.
        let id_only: RuntimeCodeContext = serde_json::from_str(r#"{"id":"c-2"}"#).unwrap();
        assert_eq!(id_only.context_id, "c-2");
        let ctx_only: RuntimeCodeContext = serde_json::from_str(r#"{"context_id":"c-3"}"#).unwrap();
        assert_eq!(ctx_only.context_id, "c-3");
    }

    #[test]
    fn command_info_parses_minimal_record() {
        let cmd: CommandInfo = serde_json::from_str(r#"{"pid": 1, "status": "exited"}"#).unwrap();
        assert_eq!(cmd.pid, 1);
        assert_eq!(cmd.status, "exited");
        assert!(cmd.args.is_empty());
        assert!(!cmd.background);
        assert!(!cmd.timed_out);
    }
}
