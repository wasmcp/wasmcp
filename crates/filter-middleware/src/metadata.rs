use crate::bindings::wasmcp::mcp_v20250618::mcp::Tool;
use crate::types::ToolMetadata;
use serde_json::Value;
use std::collections::HashMap;

/// Parse tool metadata from tool.options.meta JSON field.
///
/// Extracts component_id and tags object if present.
/// Returns empty metadata if parsing fails (graceful degradation).
#[must_use]
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

/// Check if tool passes whitelist criteria.
///
/// Tool passes if:
/// - Tool's component_id is in whitelist, OR
/// - Tool's name is in whitelist
///
/// Returns true if whitelist check passes.
#[must_use]
pub fn tool_passes_whitelist(tool: &Tool, metadata: &ToolMetadata, whitelist: &[String]) -> bool {
    // Check if component_id is whitelisted
    if let Some(comp_id) = &metadata.component_id
        && whitelist.contains(comp_id)
    {
        return true;
    }

    // Check if tool name is whitelisted
    whitelist.contains(&tool.name)
}

/// Check if tool is explicitly blacklisted by component_id OR name.
///
/// Tool is blacklisted if:
/// - Tool's component_id is in blacklist, OR
/// - Tool's name is in blacklist
///
/// Symmetric with whitelist logic to prevent bypass via component_id.
#[must_use]
pub fn tool_is_blacklisted(tool: &Tool, metadata: &ToolMetadata, blacklist: &[String]) -> bool {
    // Check if component_id is blacklisted
    if let Some(comp_id) = &metadata.component_id
        && blacklist.contains(comp_id)
    {
        return true;
    }

    // Check if tool name is blacklisted
    blacklist.contains(&tool.name)
}

/// Check if tool matches ALL active tag filters (AND logic).
///
/// For each required tag, tool must:
/// - Have the tag present in metadata.tags
/// - Tag value must be in the allowed values list
///
/// Empty filter set matches all tools.
#[must_use]
pub fn tool_matches_tag_filters(
    metadata: &ToolMetadata,
    active_filters: &HashMap<String, Vec<String>>,
) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_matches_tag_filters_all_match() {
        let metadata = ToolMetadata {
            component_id: None,
            tags: {
                let mut map = HashMap::new();
                map.insert("category".to_string(), "math".to_string());
                map.insert("level".to_string(), "basic".to_string());
                map
            },
        };

        let mut filters = HashMap::new();
        filters.insert(
            "category".to_string(),
            vec!["math".to_string(), "science".to_string()],
        );
        filters.insert("level".to_string(), vec!["basic".to_string()]);

        assert!(tool_matches_tag_filters(&metadata, &filters));
    }

    #[test]
    fn test_tool_matches_tag_filters_missing_tag() {
        let metadata = ToolMetadata {
            component_id: None,
            tags: {
                let mut map = HashMap::new();
                map.insert("category".to_string(), "math".to_string());
                // Missing "level" tag
                map
            },
        };

        let mut filters = HashMap::new();
        filters.insert("category".to_string(), vec!["math".to_string()]);
        filters.insert("level".to_string(), vec!["basic".to_string()]);

        // Should fail because "level" tag is missing
        assert!(!tool_matches_tag_filters(&metadata, &filters));
    }

    #[test]
    fn test_tool_matches_tag_filters_wrong_value() {
        let metadata = ToolMetadata {
            component_id: None,
            tags: {
                let mut map = HashMap::new();
                map.insert("category".to_string(), "history".to_string());
                map
            },
        };

        let mut filters = HashMap::new();
        filters.insert(
            "category".to_string(),
            vec!["math".to_string(), "science".to_string()],
        );

        // Should fail because "history" is not in allowed values
        assert!(!tool_matches_tag_filters(&metadata, &filters));
    }

    #[test]
    fn test_tool_matches_tag_filters_empty_filters() {
        let metadata = ToolMetadata {
            component_id: None,
            tags: HashMap::new(),
        };

        let filters = HashMap::new();

        // Empty filters should match all tools
        assert!(tool_matches_tag_filters(&metadata, &filters));
    }

    #[test]
    fn test_tool_matches_tag_filters_and_logic() {
        // Test that ALL filters must match (AND logic, not OR)
        let metadata = ToolMetadata {
            component_id: None,
            tags: {
                let mut map = HashMap::new();
                map.insert("category".to_string(), "math".to_string());
                map.insert("level".to_string(), "advanced".to_string()); // Wrong level
                map
            },
        };

        let mut filters = HashMap::new();
        filters.insert("category".to_string(), vec!["math".to_string()]);
        filters.insert("level".to_string(), vec!["basic".to_string()]);

        // Should fail because level doesn't match (even though category matches)
        assert!(!tool_matches_tag_filters(&metadata, &filters));
    }

    #[test]
    fn test_tool_passes_whitelist_by_component_id() {
        use crate::bindings::wasmcp::mcp_v20250618::mcp::Tool;

        let tool = Tool {
            name: "multiply".to_string(),
            input_schema: "{}".to_string(),
            options: None,
        };

        let metadata = ToolMetadata {
            component_id: Some("calculator-rs".to_string()),
            tags: HashMap::new(),
        };

        let whitelist = vec!["calculator-rs".to_string()];

        assert!(tool_passes_whitelist(&tool, &metadata, &whitelist));
    }

    #[test]
    fn test_tool_passes_whitelist_by_tool_name() {
        use crate::bindings::wasmcp::mcp_v20250618::mcp::Tool;

        let tool = Tool {
            name: "multiply".to_string(),
            input_schema: "{}".to_string(),
            options: None,
        };

        let metadata = ToolMetadata {
            component_id: None,
            tags: HashMap::new(),
        };

        let whitelist = vec!["multiply".to_string()];

        assert!(tool_passes_whitelist(&tool, &metadata, &whitelist));
    }

    #[test]
    fn test_tool_passes_whitelist_not_in_list() {
        use crate::bindings::wasmcp::mcp_v20250618::mcp::Tool;

        let tool = Tool {
            name: "divide".to_string(),
            input_schema: "{}".to_string(),
            options: None,
        };

        let metadata = ToolMetadata {
            component_id: Some("calculator-rs".to_string()),
            tags: HashMap::new(),
        };

        let whitelist = vec!["weather-ts".to_string()];

        assert!(!tool_passes_whitelist(&tool, &metadata, &whitelist));
    }

    #[test]
    fn test_tool_is_blacklisted_by_name() {
        use crate::bindings::wasmcp::mcp_v20250618::mcp::Tool;

        let tool = Tool {
            name: "dangerous_tool".to_string(),
            input_schema: "{}".to_string(),
            options: None,
        };

        let metadata = ToolMetadata {
            component_id: None,
            tags: HashMap::new(),
        };

        let blacklist = vec!["dangerous_tool".to_string()];

        assert!(tool_is_blacklisted(&tool, &metadata, &blacklist));
    }

    #[test]
    fn test_tool_is_blacklisted_by_component_id() {
        use crate::bindings::wasmcp::mcp_v20250618::mcp::Tool;

        let tool = Tool {
            name: "some_tool".to_string(),
            input_schema: "{}".to_string(),
            options: None,
        };

        let metadata = ToolMetadata {
            component_id: Some("evil-component".to_string()),
            tags: HashMap::new(),
        };

        let blacklist = vec!["evil-component".to_string()];

        assert!(tool_is_blacklisted(&tool, &metadata, &blacklist));
    }

    #[test]
    fn test_tool_is_blacklisted_false() {
        use crate::bindings::wasmcp::mcp_v20250618::mcp::Tool;

        let tool = Tool {
            name: "safe_tool".to_string(),
            input_schema: "{}".to_string(),
            options: None,
        };

        let metadata = ToolMetadata {
            component_id: Some("safe-component".to_string()),
            tags: HashMap::new(),
        };

        let blacklist = vec!["dangerous_tool".to_string()];

        assert!(!tool_is_blacklisted(&tool, &metadata, &blacklist));
    }

    #[test]
    fn test_blacklist_bypass_prevention() {
        // Critical security test: Tool whitelisted by component_id but blacklisted by name
        use crate::bindings::wasmcp::mcp_v20250618::mcp::Tool;

        let tool = Tool {
            name: "dangerous".to_string(),
            input_schema: "{}".to_string(),
            options: None,
        };

        let metadata = ToolMetadata {
            component_id: Some("trusted-component".to_string()),
            tags: HashMap::new(),
        };

        let whitelist = vec!["trusted-component".to_string()];
        let blacklist = vec!["dangerous".to_string()];

        // Tool passes whitelist via component_id
        assert!(tool_passes_whitelist(&tool, &metadata, &whitelist));

        // But MUST still be blocked by blacklist via name
        assert!(tool_is_blacklisted(&tool, &metadata, &blacklist));
    }

    #[test]
    fn test_blacklist_component_id_bypass_prevention() {
        // Critical security test: Tool whitelisted by name but blacklisted by component_id
        use crate::bindings::wasmcp::mcp_v20250618::mcp::Tool;

        let tool = Tool {
            name: "useful_tool".to_string(),
            input_schema: "{}".to_string(),
            options: None,
        };

        let metadata = ToolMetadata {
            component_id: Some("malicious-component".to_string()),
            tags: HashMap::new(),
        };

        let whitelist = vec!["useful_tool".to_string()];
        let blacklist = vec!["malicious-component".to_string()];

        // Tool passes whitelist via name
        assert!(tool_passes_whitelist(&tool, &metadata, &whitelist));

        // But MUST still be blocked by blacklist via component_id
        assert!(tool_is_blacklisted(&tool, &metadata, &blacklist));
    }

    #[test]
    fn test_parse_tool_metadata_with_tags() {
        use crate::bindings::wasmcp::mcp_v20250618::mcp::{Tool, ToolOptions};

        let tool = Tool {
            name: "add".to_string(),
            input_schema: "{}".to_string(),
            options: Some(ToolOptions {
                description: None,
                title: None,
                meta: Some(
                    r#"{"component_id":"calculator","tags":{"category":"math","level":"basic"}}"#
                        .to_string(),
                ),
                annotations: None,
                output_schema: None,
            }),
        };

        let metadata = parse_tool_metadata(&tool);

        assert_eq!(metadata.component_id, Some("calculator".to_string()));
        assert_eq!(metadata.tags.get("category"), Some(&"math".to_string()));
        assert_eq!(metadata.tags.get("level"), Some(&"basic".to_string()));
    }

    #[test]
    fn test_parse_tool_metadata_no_meta() {
        use crate::bindings::wasmcp::mcp_v20250618::mcp::Tool;

        let tool = Tool {
            name: "add".to_string(),
            input_schema: "{}".to_string(),
            options: None,
        };

        let metadata = parse_tool_metadata(&tool);

        assert_eq!(metadata.component_id, None);
        assert!(metadata.tags.is_empty());
    }

    #[test]
    fn test_parse_tool_metadata_invalid_json() {
        use crate::bindings::wasmcp::mcp_v20250618::mcp::{Tool, ToolOptions};

        let tool = Tool {
            name: "add".to_string(),
            input_schema: "{}".to_string(),
            options: Some(ToolOptions {
                description: None,
                title: None,
                meta: Some("not valid json".to_string()),
                annotations: None,
                output_schema: None,
            }),
        };

        let metadata = parse_tool_metadata(&tool);

        // Should handle gracefully with empty metadata
        assert_eq!(metadata.component_id, None);
        assert!(metadata.tags.is_empty());
    }
}
