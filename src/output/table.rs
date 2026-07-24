// src/output/table.rs — comfy-table builders for each resource type.

use chrono::{DateTime, Utc};
use comfy_table::{
    presets::UTF8_FULL_CONDENSED, Attribute, Cell, Color, ContentArrangement, Table, TableComponent,
};

use crate::api::types::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn apply_style(t: &mut Table, headers: &[&str]) {
    t.load_preset(UTF8_FULL_CONDENSED)
        // ┆ (condensed inner separator) → │ so all vertical lines match
        .set_style(TableComponent::VerticalLines, '│')
        // Single-line header separator (replaces double-line ╞═╪═╡)
        .set_style(TableComponent::HeaderLines, '─')
        .set_style(TableComponent::LeftHeaderIntersection, '├')
        .set_style(TableComponent::MiddleHeaderIntersections, '┼')
        .set_style(TableComponent::RightHeaderIntersection, '┤')
        .set_header(
            headers
                .iter()
                .map(|h| Cell::new(h).add_attribute(Attribute::Bold).fg(Color::Cyan)),
        );
}

/// List tables: expand columns to fill the terminal width.
fn base_table(headers: &[&str]) -> Table {
    let mut t = Table::new();
    t.set_content_arrangement(ContentArrangement::Dynamic);
    apply_style(&mut t, headers);
    t
}

/// Detail / KV tables: size columns to content, never expand to fill terminal.
fn detail_table(headers: &[&str]) -> Table {
    let mut t = Table::new();
    t.set_content_arrangement(ContentArrangement::Disabled);
    apply_style(&mut t, headers);
    t
}

fn status_cell(status: &str) -> Cell {
    let cell = Cell::new(status);
    match status {
        "running" | "active" | "completed" | "healthy" => cell.fg(Color::Green),
        "paused" | "pending" | "starting" | "propagating" => cell.fg(Color::Yellow),
        "failed" | "terminated" | "error" | "deleted" | "unhealthy" => cell.fg(Color::Red),
        _ => cell,
    }
}

fn opt_str(o: &Option<String>) -> &str {
    o.as_deref().unwrap_or("—")
}

fn opt_u64(o: Option<u64>) -> String {
    o.map(|v| v.to_string()).unwrap_or_else(|| "—".into())
}

fn opt_u32(o: Option<u32>) -> String {
    o.map(|v| v.to_string()).unwrap_or_else(|| "—".into())
}

fn opt_u16(o: Option<u16>) -> String {
    o.map(|v| v.to_string()).unwrap_or_else(|| "—".into())
}

fn opt_i32(o: Option<i32>) -> String {
    o.map(|v| v.to_string()).unwrap_or_else(|| "—".into())
}

fn opt_i64(o: Option<i64>) -> String {
    o.map(|v| v.to_string()).unwrap_or_else(|| "—".into())
}

fn opt_f64_1dp(o: Option<f64>) -> String {
    o.map(|v| format!("{:.1}", v)).unwrap_or_else(|| "—".into())
}

/// Format an ISO-8601 timestamp as a human-readable relative time string.
/// Matches the frontend `formatRelativeTime` function.
fn format_relative(ts: Option<&str>) -> String {
    let ts = match ts {
        Some(s) if !s.is_empty() => s,
        _ => return "—".into(),
    };
    let dt = match ts.parse::<DateTime<Utc>>() {
        Ok(d) => d,
        Err(_) => return ts.into(),
    };
    let now = Utc::now();
    let diff = now.signed_duration_since(dt);
    let secs = diff.num_seconds();
    if secs < 0 {
        return "soon".into();
    }
    if secs < 60 {
        return "just now".into();
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{mins}m ago");
    }
    let hours = mins / 60;
    if hours < 24 {
        return format!("{hours}h ago");
    }
    let days = hours / 24;
    if days < 30 {
        return format!("{days}d ago");
    }
    let months = days / 30;
    if months < 12 {
        return format!("{months}mo ago");
    }
    format!("{}y ago", months / 12)
}

// ---------------------------------------------------------------------------
// Runtime
// ---------------------------------------------------------------------------

pub fn runtime_row(rt: &Runtime) -> Vec<Cell> {
    vec![
        Cell::new(&rt.runtime_id),
        status_cell(rt.status.as_str()),
        Cell::new(opt_str(&rt.template)),
        Cell::new(opt_str(&rt.cloud)),
        Cell::new(opt_str(&rt.region)),
        Cell::new(
            rt.cpu_count
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into()),
        ),
        Cell::new(opt_u64(rt.memory_mb)),
        Cell::new(opt_u64(rt.disk_size_mb)),
        Cell::new(format_relative(rt.started_at.as_deref())),
    ]
}

pub fn runtime_table(runtimes: &[Runtime]) -> Table {
    let mut t = base_table(&[
        "RUNTIME ID",
        "STATUS",
        "TEMPLATE",
        "CLOUD",
        "REGION",
        "vCPU",
        "MEM (MB)",
        "DISK (MB)",
        "STARTED",
    ]);
    for rt in runtimes {
        t.add_row(runtime_row(rt));
    }
    t
}

pub fn runtime_detail_table(rt: &Runtime) -> Table {
    let mut t = detail_table(&["FIELD", "VALUE"]);
    t.add_row(vec![Cell::new("runtime_id"), Cell::new(&rt.runtime_id)]);
    t.add_row(vec![Cell::new("status"), status_cell(&rt.status)]);
    t.add_row(vec![
        Cell::new("template"),
        Cell::new(opt_str(&rt.template)),
    ]);
    t.add_row(vec![Cell::new("cloud"), Cell::new(opt_str(&rt.cloud))]);
    t.add_row(vec![Cell::new("region"), Cell::new(opt_str(&rt.region))]);
    t.add_row(vec![
        Cell::new("vcpu_count"),
        Cell::new(
            rt.cpu_count
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into()),
        ),
    ]);
    t.add_row(vec![
        Cell::new("memory_mb"),
        Cell::new(opt_u64(rt.memory_mb)),
    ]);
    t.add_row(vec![
        Cell::new("disk_size_mb"),
        Cell::new(opt_u64(rt.disk_size_mb)),
    ]);
    t.add_row(vec![
        Cell::new("ip_address"),
        Cell::new(opt_str(&rt.ip_address)),
    ]);
    t.add_row(vec![
        Cell::new("started_at"),
        Cell::new(format_relative(rt.started_at.as_deref())),
    ]);
    t.add_row(vec![
        Cell::new("timeout_at"),
        Cell::new(opt_str(&rt.timeout_at)),
    ]);
    t.add_row(vec![
        Cell::new("ended_at"),
        Cell::new(opt_str(&rt.ended_at)),
    ]);
    t.add_row(vec![
        Cell::new("internet_access"),
        Cell::new(
            rt.internet_access
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into()),
        ),
    ]);
    t.add_row(vec![
        Cell::new("ssh_enabled"),
        Cell::new(
            rt.ssh_enabled
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into()),
        ),
    ]);
    t
}

pub fn runtime_metrics_table(m: &RuntimeMetrics) -> Table {
    let mut t = detail_table(&["METRIC", "VALUE"]);
    t.add_row(vec![
        Cell::new("timestamp"),
        Cell::new(opt_str(&m.timestamp)),
    ]);
    t.add_row(vec![
        Cell::new("cpu_usage (%)"),
        Cell::new(opt_f64_1dp(m.cpu_usage)),
    ]);
    t.add_row(vec![
        Cell::new("memory_usage (MB)"),
        Cell::new(opt_f64_1dp(m.memory_usage)),
    ]);
    t.add_row(vec![
        Cell::new("memory_total (MB)"),
        Cell::new(opt_f64_1dp(m.memory_total)),
    ]);
    t.add_row(vec![
        Cell::new("disk_read (bytes)"),
        Cell::new(opt_i64(m.disk_read)),
    ]);
    t.add_row(vec![
        Cell::new("disk_write (bytes)"),
        Cell::new(opt_i64(m.disk_write)),
    ]);
    t.add_row(vec![
        Cell::new("network_rx (bytes)"),
        Cell::new(opt_i64(m.network_rx)),
    ]);
    t.add_row(vec![
        Cell::new("network_tx (bytes)"),
        Cell::new(opt_i64(m.network_tx)),
    ]);
    t.add_row(vec![
        Cell::new("load_avg_1m"),
        Cell::new(opt_f64_1dp(m.load_avg_1m)),
    ]);
    t.add_row(vec![
        Cell::new("load_avg_5m"),
        Cell::new(opt_f64_1dp(m.load_avg_5m)),
    ]);
    t.add_row(vec![
        Cell::new("load_avg_15m"),
        Cell::new(opt_f64_1dp(m.load_avg_15m)),
    ]);
    t.add_row(vec![
        Cell::new("uptime_seconds"),
        Cell::new(opt_i64(m.uptime_seconds)),
    ]);
    t.add_row(vec![
        Cell::new("process_count"),
        Cell::new(opt_i64(m.process_count)),
    ]);
    t.add_row(vec![
        Cell::new("iowait_percent (%)"),
        Cell::new(opt_f64_1dp(m.iowait_percent)),
    ]);
    t
}

// ---------------------------------------------------------------------------
// Template
// ---------------------------------------------------------------------------

pub fn template_table(templates: &[Template]) -> Table {
    let mut t = base_table(&[
        "ID",
        "NAME",
        "KIND",
        "VCPU",
        "MEM",
        "DISK",
        "VISIBILITY",
        "CREATED",
    ]);
    for tmpl in templates {
        t.add_row(vec![
            Cell::new(tmpl.canonical_id()),
            Cell::new(opt_str(&tmpl.name)),
            Cell::new(opt_str(&tmpl.kind)),
            Cell::new(opt_u32(tmpl.vcpu_count)),
            Cell::new(opt_u64(tmpl.memory_mb)),
            Cell::new(opt_u64(tmpl.disk_size_mb)),
            Cell::new(opt_str(&tmpl.visibility)),
            Cell::new(format_relative(tmpl.created_at.as_deref())),
        ]);
    }
    t
}

pub fn template_detail_table(tmpl: &Template) -> Table {
    let mut t = detail_table(&["FIELD", "VALUE"]);
    t.add_row(vec![Cell::new("id"), Cell::new(tmpl.canonical_id())]);
    t.add_row(vec![Cell::new("name"), Cell::new(opt_str(&tmpl.name))]);
    t.add_row(vec![Cell::new("kind"), Cell::new(opt_str(&tmpl.kind))]);
    t.add_row(vec![
        Cell::new("description"),
        Cell::new(opt_str(&tmpl.description)),
    ]);
    t.add_row(vec![
        Cell::new("vcpu_count"),
        Cell::new(opt_u32(tmpl.vcpu_count)),
    ]);
    t.add_row(vec![
        Cell::new("memory_mb"),
        Cell::new(opt_u64(tmpl.memory_mb)),
    ]);
    t.add_row(vec![
        Cell::new("disk_size_mb"),
        Cell::new(opt_u64(tmpl.disk_size_mb)),
    ]);
    t.add_row(vec![
        Cell::new("visibility"),
        Cell::new(opt_str(&tmpl.visibility)),
    ]);
    t.add_row(vec![
        Cell::new("http_port"),
        Cell::new(opt_u16(tmpl.http_port)),
    ]);
    t.add_row(vec![
        Cell::new("status"),
        status_cell(tmpl.status.as_deref().unwrap_or("—")),
    ]);
    t.add_row(vec![
        Cell::new("framework"),
        Cell::new(opt_str(&tmpl.framework)),
    ]);
    t.add_row(vec![
        Cell::new("python_version"),
        Cell::new(opt_str(&tmpl.python_version)),
    ]);
    t.add_row(vec![
        Cell::new("node_version"),
        Cell::new(opt_str(&tmpl.node_version)),
    ]);
    t.add_row(vec![
        Cell::new("size_mb"),
        Cell::new(opt_f64_1dp(tmpl.size_mb)),
    ]);
    t.add_row(vec![
        Cell::new("created"),
        Cell::new(format_relative(tmpl.created_at.as_deref())),
    ]);
    t.add_row(vec![
        Cell::new("updated"),
        Cell::new(format_relative(tmpl.updated_at.as_deref())),
    ]);
    t.add_row(vec![Cell::new("cloud"), Cell::new(opt_str(&tmpl.cloud))]);
    t.add_row(vec![Cell::new("region"), Cell::new(opt_str(&tmpl.region))]);
    t.add_row(vec![
        Cell::new("is_active"),
        Cell::new(match tmpl.is_active {
            Some(true) => "true",
            Some(false) => "false",
            None => "—",
        }),
    ]);
    t
}

pub fn build_status_table(s: &TemplateBuildStatus) -> Table {
    let mut t = detail_table(&["FIELD", "VALUE"]);
    t.add_row(vec![Cell::new("build_id"), Cell::new(&s.build_id)]);
    t.add_row(vec![
        Cell::new("template_id"),
        Cell::new(opt_str(&s.template_id)),
    ]);
    t.add_row(vec![Cell::new("status"), status_cell(&s.status)]);
    t.add_row(vec![Cell::new("phase"), Cell::new(opt_str(&s.phase))]);
    t.add_row(vec![
        Cell::new("progress_percent"),
        Cell::new(opt_i32(s.progress_percent)),
    ]);
    t.add_row(vec![Cell::new("error"), Cell::new(opt_str(&s.error))]);
    t.add_row(vec![Cell::new("message"), Cell::new(opt_str(&s.message))]);
    t.add_row(vec![
        Cell::new("started_at"),
        Cell::new(opt_str(&s.started_at)),
    ]);
    t.add_row(vec![
        Cell::new("completed_at"),
        Cell::new(opt_str(&s.completed_at)),
    ]);
    t
}

// ---------------------------------------------------------------------------
// Agent
// ---------------------------------------------------------------------------

pub fn agent_detail_table(a: &AgentEndpoint) -> Table {
    let mut t = detail_table(&["FIELD", "VALUE"]);
    t.add_row(vec![Cell::new("agent_id"), Cell::new(&a.agent_id)]);
    t.add_row(vec![Cell::new("name"), Cell::new(opt_str(&a.name))]);
    t.add_row(vec![
        Cell::new("status"),
        status_cell(a.status.as_deref().unwrap_or("—")),
    ]);
    t.add_row(vec![
        Cell::new("dns_status"),
        Cell::new(opt_str(&a.dns_status)),
    ]);
    t.add_row(vec![Cell::new("health"), Cell::new(opt_str(&a.health))]);
    t.add_row(vec![
        Cell::new("framework"),
        Cell::new(opt_str(&a.framework)),
    ]);
    t.add_row(vec![Cell::new("endpoint"), Cell::new(opt_str(&a.endpoint))]);
    t.add_row(vec![
        Cell::new("a2a_endpoint"),
        Cell::new(opt_str(&a.a2a_endpoint)),
    ]);
    t.add_row(vec![
        Cell::new("mcp_endpoint"),
        Cell::new(opt_str(&a.mcp_endpoint)),
    ]);
    t.add_row(vec![
        Cell::new("created"),
        Cell::new(format_relative(a.created_at.as_deref())),
    ]);
    t
}

pub fn agent_build_status_table(s: &AgentBuildStatusResponse) -> Table {
    let mut t = detail_table(&["FIELD", "VALUE"]);
    t.add_row(vec![Cell::new("build_id"), Cell::new(&s.build_id)]);
    t.add_row(vec![
        Cell::new("template_id"),
        Cell::new(opt_str(&s.template_id)),
    ]);
    t.add_row(vec![Cell::new("status"), status_cell(&s.status)]);
    t.add_row(vec![Cell::new("phase"), Cell::new(opt_str(&s.phase))]);
    t.add_row(vec![
        Cell::new("progress_percent"),
        Cell::new(opt_i32(s.progress_percent)),
    ]);
    t.add_row(vec![Cell::new("error"), Cell::new(opt_str(&s.error))]);
    t.add_row(vec![Cell::new("message"), Cell::new(opt_str(&s.message))]);
    t.add_row(vec![
        Cell::new("started_at"),
        Cell::new(opt_str(&s.started_at)),
    ]);
    t.add_row(vec![
        Cell::new("completed_at"),
        Cell::new(opt_str(&s.completed_at)),
    ]);
    t
}

// ---------------------------------------------------------------------------
// Billing
// ---------------------------------------------------------------------------

pub fn billing_summary_table(s: &BillingSummary) -> Table {
    let mut t = detail_table(&["FIELD", "VALUE"]);
    t.add_row(vec![
        Cell::new("period_start"),
        Cell::new(opt_str(&s.period_start)),
    ]);
    t.add_row(vec![
        Cell::new("period_end"),
        Cell::new(opt_str(&s.period_end)),
    ]);
    t.add_row(vec![
        Cell::new("total_runtimes"),
        Cell::new(
            s.total_runtimes
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into()),
        ),
    ]);
    t.add_row(vec![
        Cell::new("active_runtimes"),
        Cell::new(
            s.active_runtimes
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into()),
        ),
    ]);
    t.add_row(vec![
        Cell::new("total_cpu_hours"),
        Cell::new(opt_f64_1dp(s.total_cpu_hours)),
    ]);
    t.add_row(vec![
        Cell::new("total_ram_gb_hours"),
        Cell::new(opt_f64_1dp(s.total_ram_gb_hours)),
    ]);
    t.add_row(vec![
        Cell::new("total_storage_gb_hours"),
        Cell::new(opt_f64_1dp(s.total_storage_gb_hours)),
    ]);
    t.add_row(vec![
        Cell::new("cpu_cost"),
        Cell::new(
            s.cpu_cost
                .map(|v| format!("${:.4}", v))
                .unwrap_or_else(|| "—".into()),
        ),
    ]);
    t.add_row(vec![
        Cell::new("ram_cost"),
        Cell::new(
            s.ram_cost
                .map(|v| format!("${:.4}", v))
                .unwrap_or_else(|| "—".into()),
        ),
    ]);
    t.add_row(vec![
        Cell::new("storage_cost"),
        Cell::new(
            s.storage_cost
                .map(|v| format!("${:.4}", v))
                .unwrap_or_else(|| "—".into()),
        ),
    ]);
    t.add_row(vec![
        Cell::new("total_cost"),
        Cell::new(
            s.total_cost
                .map(|v| format!("${:.4}", v))
                .unwrap_or_else(|| "—".into()),
        ),
    ]);
    t
}

pub fn billing_history_table(items: &[BillingItem]) -> Table {
    let mut t = base_table(&[
        "RUNTIME ID",
        "SESSION START",
        "DURATION (s)",
        "TOTAL COST",
        "STATUS",
    ]);
    for item in items {
        t.add_row(vec![
            Cell::new(opt_str(&item.runtime_id)),
            Cell::new(opt_str(&item.session_start)),
            Cell::new(
                item.duration_seconds
                    .map(|v| format!("{:.0}", v))
                    .unwrap_or_else(|| "—".into()),
            ),
            Cell::new(
                item.total_cost
                    .map(|v| format!("${:.4}", v))
                    .unwrap_or_else(|| "—".into()),
            ),
            status_cell(item.billing_status.as_deref().unwrap_or("—")),
        ]);
    }
    t
}

pub fn billing_quota_table(q: &BillingQuota) -> Table {
    let mut t = detail_table(&["QUOTA", "CURRENT", "MAX"]);
    t.add_row(vec![
        Cell::new("tier"),
        Cell::new(
            q.tier_display_name
                .as_deref()
                .or(q.tier_name.as_deref())
                .unwrap_or("—"),
        ),
        Cell::new("—"),
    ]);
    t.add_row(vec![
        Cell::new("vcpu"),
        Cell::new(
            q.vcpu_used
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into()),
        ),
        Cell::new(
            q.vcpu_limit
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into()),
        ),
    ]);
    t.add_row(vec![
        Cell::new("ram_gb"),
        Cell::new(
            q.ram_mb_used
                .map(|v| format!("{:.1}", v as f64 / 1024.0))
                .unwrap_or_else(|| "—".into()),
        ),
        Cell::new(
            q.ram_gb_limit
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into()),
        ),
    ]);
    t.add_row(vec![
        Cell::new("disk_gb"),
        Cell::new(
            q.disk_mb_used
                .map(|v| format!("{:.1}", v as f64 / 1024.0))
                .unwrap_or_else(|| "—".into()),
        ),
        Cell::new(
            q.disk_gb_limit
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into()),
        ),
    ]);
    t.add_row(vec![
        Cell::new("api_requests_per_min"),
        Cell::new("—"),
        Cell::new(
            q.api_requests_per_min
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into()),
        ),
    ]);
    t.add_row(vec![
        Cell::new("runtime_creation_per_min"),
        Cell::new("—"),
        Cell::new(
            q.runtime_creation_per_min
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into()),
        ),
    ]);
    t.add_row(vec![
        Cell::new("runtime_lifecycle_per_min"),
        Cell::new("—"),
        Cell::new(
            q.runtime_lifecycle_per_min
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into()),
        ),
    ]);
    t
}

// ---------------------------------------------------------------------------
// Secret Providers
// ---------------------------------------------------------------------------

pub fn provider_table(providers: &[SecretProvider]) -> Table {
    let mut t = base_table(&["ID", "NAME", "TYPE", "SECRETS", "STATUS", "CREATED"]);
    for p in providers {
        let status = if p.is_active { "active" } else { "disabled" };
        t.add_row(vec![
            Cell::new(&p.id),
            Cell::new(&p.name),
            Cell::new(&p.provider_type),
            Cell::new(p.secret_count.to_string()),
            status_cell(status),
            Cell::new(format_relative(p.created_at.as_deref())),
        ]);
    }
    t
}

pub fn provider_detail_table(p: &SecretProvider) -> Table {
    let mut t = detail_table(&["FIELD", "VALUE"]);
    let status = if p.is_active { "active" } else { "disabled" };
    t.add_row(vec![Cell::new("id"), Cell::new(&p.id)]);
    t.add_row(vec![Cell::new("name"), Cell::new(&p.name)]);
    t.add_row(vec![Cell::new("type"), Cell::new(&p.provider_type)]);
    t.add_row(vec![Cell::new("status"), status_cell(status)]);
    t.add_row(vec![
        Cell::new("secrets"),
        Cell::new(p.secret_count.to_string()),
    ]);
    t.add_row(vec![
        Cell::new("created"),
        Cell::new(format_relative(p.created_at.as_deref())),
    ]);
    t.add_row(vec![
        Cell::new("updated"),
        Cell::new(format_relative(p.updated_at.as_deref())),
    ]);
    t
}

pub fn secret_table(secrets: &[SecretInfo]) -> Table {
    let mut t = base_table(&["ID", "KEY", "VALUE", "UPDATED"]);
    for s in secrets {
        t.add_row(vec![
            Cell::new(&s.id),
            Cell::new(&s.key),
            Cell::new(s.masked.as_deref().unwrap_or("••••••••")),
            Cell::new(format_relative(s.updated_at.as_deref())),
        ]);
    }
    t
}

// ---------------------------------------------------------------------------
// Network Policies
// ---------------------------------------------------------------------------

pub fn network_policy_table(policies: &[NetworkPolicy]) -> Table {
    let mut t = base_table(&[
        "ID", "NAME", "MODE", "RULES", "DEFAULT", "STATUS", "CREATED",
    ]);
    for p in policies {
        let status = if p.is_active { "active" } else { "disabled" };
        t.add_row(vec![
            Cell::new(&p.id),
            Cell::new(&p.name),
            Cell::new(&p.egress_mode),
            Cell::new(p.rule_count.to_string()),
            Cell::new(if p.is_default { "yes" } else { "no" }),
            status_cell(status),
            Cell::new(format_relative(p.created_at.as_deref())),
        ]);
    }
    t
}

pub fn network_policy_detail_table(p: &NetworkPolicy) -> Table {
    let mut t = detail_table(&["FIELD", "VALUE"]);
    let status = if p.is_active { "active" } else { "disabled" };
    t.add_row(vec![Cell::new("id"), Cell::new(&p.id)]);
    t.add_row(vec![Cell::new("name"), Cell::new(&p.name)]);
    t.add_row(vec![Cell::new("egress_mode"), Cell::new(&p.egress_mode)]);
    t.add_row(vec![Cell::new("status"), status_cell(status)]);
    t.add_row(vec![
        Cell::new("rules"),
        Cell::new(p.rule_count.to_string()),
    ]);
    t.add_row(vec![
        Cell::new("is_default"),
        Cell::new(if p.is_default { "true" } else { "false" }),
    ]);
    t.add_row(vec![
        Cell::new("is_system"),
        Cell::new(if p.is_system { "true" } else { "false" }),
    ]);
    if let Some(ref desc) = p.description {
        t.add_row(vec![Cell::new("description"), Cell::new(desc)]);
    }
    t.add_row(vec![
        Cell::new("created"),
        Cell::new(format_relative(p.created_at.as_deref())),
    ]);
    t.add_row(vec![
        Cell::new("updated"),
        Cell::new(format_relative(p.updated_at.as_deref())),
    ]);
    t
}

pub fn network_policy_rule_table(rules: &[NetworkPolicyRule]) -> Table {
    let mut t = base_table(&["ID", "DESTINATION", "PORT", "PROTOCOL", "UPDATED"]);
    for r in rules {
        let port = if r.port == 0 {
            "any".to_string()
        } else {
            r.port.to_string()
        };
        t.add_row(vec![
            Cell::new(&r.id),
            Cell::new(&r.destination),
            Cell::new(port),
            Cell::new(&r.protocol),
            Cell::new(format_relative(r.updated_at.as_deref())),
        ]);
    }
    t
}
