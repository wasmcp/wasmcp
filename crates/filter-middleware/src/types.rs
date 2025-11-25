use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Routing configuration loaded from routing://config resource
#[derive(Debug, Deserialize)]
pub struct RoutingConfig {
    pub version: String,
    #[serde(rename = "path-rules")]
    pub path_rules: HashMap<String, PathRule>,
    #[serde(rename = "tag-filters", default)]
    pub global_tag_filters: HashMap<String, TagFilterValue>,
}

/// Path-based filtering rule
#[derive(Debug, Deserialize)]
pub struct PathRule {
    pub whitelist: Option<Vec<String>>,
    pub blacklist: Option<Vec<String>>,
    #[serde(rename = "tag-filters", default)]
    pub tag_filters: HashMap<String, TagFilterValue>,
}

/// Tag filter value - can be single string or array of strings
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum TagFilterValue {
    Single(String),
    Multiple(Vec<String>),
}

/// Tool metadata extracted from tool.options.meta
#[derive(Debug, Clone)]
pub struct ToolMetadata {
    pub component_id: Option<String>,
    pub tags: HashMap<String, String>,
}

/// Aggregated config from multiple sources
#[derive(Debug)]
pub struct AggregatedConfig {
    pub path_rules: HashMap<String, AggregatedPathRule>,
    pub global_tag_filters: HashMap<String, Vec<String>>,
    pub config_sources: Vec<ConfigSource>,
}

/// Aggregated path rule with tracking of sources
#[derive(Debug)]
pub struct AggregatedPathRule {
    pub whitelist: Vec<String>,
    pub blacklist: Vec<String>,
    pub tag_filters: HashMap<String, Vec<String>>,
    pub sources: RuleSources,
}

/// Track which config URIs contributed to each rule component
#[derive(Debug, Clone, Serialize)]
pub struct RuleSources {
    pub whitelist_from: Vec<String>,
    pub blacklist_from: Vec<String>,
    pub tag_filters_from: Vec<String>,
}

/// Config source metadata
#[derive(Debug, Clone, Serialize)]
pub struct ConfigSource {
    pub uri: String,
    pub version: String,
}

/// Diagnostic output for inspect_routing tool
#[derive(Serialize)]
pub struct RoutingDiagnostic {
    pub config_sources: Vec<ConfigSource>,
    pub effective_rules: HashMap<String, EffectivePathRule>,
    pub conflict_reports: Vec<ConflictReport>,
}

/// Effective path rule for diagnostics
#[derive(Serialize)]
pub struct EffectivePathRule {
    pub path: String,
    pub whitelist: Vec<String>,
    pub blacklist: Vec<String>,
    pub tag_filters: HashMap<String, Vec<String>>,
    pub sources: RuleSources,
}

/// Conflict report entry
#[derive(Serialize)]
pub struct ConflictReport {
    pub path: String,
    pub tool_or_component: String,
    pub conflict: String,
    pub resolution: String,
}

/// Cached tool metadata for optimization
pub struct ToolWithMetadata<'a> {
    pub tool: &'a crate::bindings::wasmcp::mcp_v20250618::mcp::Tool,
    pub metadata: ToolMetadata,
}