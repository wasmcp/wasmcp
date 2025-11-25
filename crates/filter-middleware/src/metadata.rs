use crate::bindings::wasmcp::mcp_v20250618::mcp::Tool;
use crate::types::{AggregatedPathRule, ToolMetadata};
use serde_json::Value;
use std::collections::HashMap;

/// Parse tool metadata from tool.options.meta JSON
pub fn parse_tool_metadata(tool: &Tool) -> ToolMetadata {
    let meta_json = tool
        .options
        .as_ref()
        .and_then(|opts| opts.meta.as_ref())
        .and_then(|m| serde_json::from_str::<Value>(m).ok());

    let component_id = meta_json
        .as_ref()
        .and_then(|m| m.get("component_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let tags = meta_json
        .as_ref()
        .and_then(|m| m.get("tags"))
        .and_then(|t| t.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    ToolMetadata { component_id, tags }
}

/// Check if a tool passes the whitelist criteria
pub fn tool_passes_whitelist(tool: &Tool, metadata: &ToolMetadata, whitelist: &[String]) -> bool {
    // Check if component_id is whitelisted
    if let Some(comp_id) = &metadata.component_id {
        if whitelist.contains(comp_id) {
            return true;
        }
    }

    // Check if tool name is whitelisted
    whitelist.contains(&tool.name)
}

/// Check if a tool is blacklisted
pub fn tool_is_blacklisted(tool: &Tool, blacklist: &[String]) -> bool {
    blacklist.contains(&tool.name)
}

/// Check if tool matches all active tag filters
pub fn tool_matches_tag_filters(metadata: &ToolMetadata, active_filters: &HashMap<String, Vec<String>>) -> bool {
    // Tool must match ALL active tag filters
    for (tag_name, allowed_values) in active_filters {
        let tool_tag_value = metadata.tags.get(tag_name);

        match tool_tag_value {
            Some(value) if allowed_values.contains(value) => continue,
            _ => return false, // Tag missing or doesn't match
        }
    }

    true
}