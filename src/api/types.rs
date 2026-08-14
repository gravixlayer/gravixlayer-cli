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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeCodeContext {
    #[serde(alias = "id")]
    pub context_id: String,
    pub language: Option<String>,
    pub cwd: Option<String>,
    pub vm_id: Option<String>,
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
