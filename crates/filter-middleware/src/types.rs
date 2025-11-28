use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Routing configuration loaded from config:// resources.
/// Supports both global and path-specific filtering rules with tag-based filtering.
#[derive(Debug, Deserialize)]
pub struct RoutingConfig {
    /// Configuration format version
    pub version: String,
    /// Path-based filtering rules (e.g., "/mcp/calculator" -> PathRule)
    #[serde(rename = "path-rules")]
    pub path_rules: HashMap<String, PathRule>,
    /// Global tag filters that apply to all paths
    #[serde(rename = "tag-filters", default)]
    pub global_tag_filters: HashMap<String, TagFilterValue>,
}

/// Path-based filtering rule with whitelist, blacklist, and tag filters.
#[derive(Debug, Deserialize)]
pub struct PathRule {
    /// Allowed component IDs or tool names (if present, only these are allowed)
    pub whitelist: Option<Vec<String>>,
    /// Denied tool names (always takes precedence - "Deny Trumps Allow")
    pub blacklist: Option<Vec<String>>,
    /// Additional tag-based filtering requirements for this path
    #[serde(rename = "tag-filters", default)]
    pub tag_filters: HashMap<String, TagFilterValue>,
}

/// Tag filter value supporting single string or array of strings.
///
/// Allows flexible TOML syntax:
/// - Single value: `category = "math"`
/// - Multiple values: `category = ["math", "science"]`
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum TagFilterValue {
    /// Single allowed value for tag
    Single(String),
    /// Multiple allowed values for tag
    Multiple(Vec<String>),
}

/// Tool metadata extracted from tool.options.meta JSON field.
#[derive(Debug, Clone)]
pub struct ToolMetadata {
    /// Component ID if present in metadata.component_id
    pub component_id: Option<String>,
    /// Tag key-value pairs from metadata.tags object
    pub tags: HashMap<String, String>,
}

/// Aggregated routing configuration from multiple config sources.
/// Implements "Deny Trumps Allow" semantics across configs.
#[derive(Debug)]
pub struct AggregatedConfig {
    /// Merged path rules with union of whitelists and blacklists
    pub path_rules: HashMap<String, AggregatedPathRule>,
    /// Merged global tag filters (union of all config values)
    pub global_tag_filters: HashMap<String, Vec<String>>,
    /// Tracking info for which configs contributed to aggregation
    pub config_sources: Vec<ConfigSource>,
}

/// Aggregated path rule with source tracking for debugging.
#[derive(Debug)]
pub struct AggregatedPathRule {
    /// Union of all whitelists for this path
    pub whitelist: Vec<String>,
    /// Union of all blacklists for this path (takes precedence)
    pub blacklist: Vec<String>,
    /// Merged tag filters for this path
    pub tag_filters: HashMap<String, Vec<String>>,
    /// Source URIs that contributed to each list
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

/// Tool paired with pre-parsed metadata for filtering optimization.
/// Caching metadata avoids re-parsing during multi-stage filtering.
pub struct ToolWithMetadata<'a> {
    /// Reference to original tool from downstream handler
    pub tool: &'a crate::bindings::wasmcp::mcp_v20250618::mcp::Tool,
    /// Cached metadata extracted from tool.options.meta JSON
    pub metadata: ToolMetadata,
}