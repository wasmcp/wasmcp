use crate::INTERNAL_REQUEST_ID_VALUE;
use crate::bindings::exports::wasmcp::mcp_v20250618::server_handler::MessageContext;
use crate::bindings::wasmcp::mcp_v20250618::mcp::*;
use crate::config::load_and_aggregate_configs;
use crate::helpers::fetch_tools_from_downstream;
use crate::metadata::{parse_tool_metadata, tool_is_blacklisted, tool_passes_whitelist};
use crate::types::*;
use std::collections::HashMap;

/// Create the inspect_routing diagnostic tool
pub fn create_inspect_routing_tool() -> Tool {
    Tool {
        name: "inspect_routing".to_string(),
        input_schema: r#"{
            "type": "object",
            "properties": {}
        }"#
        .to_string(),
        options: Some(ToolOptions {
            description: Some(
                "Inspect effective routing configuration and identify conflicts across multiple config sources"
                    .to_string(),
            ),
            title: Some("Inspect Routing Configuration".to_string()),
            meta: Some(
                r#"{"component_id":"filter-middleware","tags":{"category":"diagnostics","tool-level":"advanced"}}"#
                    .to_string(),
            ),
            annotations: None,
            output_schema: None,
        }),
    }
}

/// Handle inspect_routing tool call - return diagnostic information
pub fn handle_inspect_routing(ctx: &MessageContext) -> Result<ServerResult, ErrorCode> {
    // Load and aggregate configs (same as tools/list does)
    let config = match load_and_aggregate_configs(ctx) {
        Ok(c) => c,
        Err(e) => {
            return Ok(ServerResult::ToolsCall(CallToolResult {
                content: vec![ContentBlock::Text(TextContent {
                    text: TextData::Text(format!("Error loading routing configs: {}", e)),
                    options: None,
                })],
                is_error: Some(true),
                meta: None,
                structured_content: None,
            }));
        }
    };

    // Get all tools from downstream to check for conflicts
    let all_tools = fetch_tools_from_downstream(
        ctx,
        RequestId::Number(INTERNAL_REQUEST_ID_VALUE),
        ListToolsRequest { cursor: None },
    )
    .unwrap_or_default(); // If we can't get tools, proceed without conflict detection

    // Build diagnostic output with conflict detection
    let diagnostic = build_routing_diagnostic(&config, &all_tools);

    // Return as CallToolResult with pretty JSON
    // Use explicit error handling to prevent panics on serialization failure
    let diagnostic_text = match serde_json::to_string_pretty(&diagnostic) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("Warning: Diagnostic serialization failed: {}", e);
            // Return minimal valid JSON on failure
            r#"{"error": "Diagnostic serialization failed", "config_sources": []}"#.to_string()
        }
    };

    Ok(ServerResult::ToolsCall(CallToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: TextData::Text(diagnostic_text),
            options: None,
        })],
        is_error: None,
        meta: None,
        structured_content: None,
    }))
}

/// Build routing diagnostic output with conflict detection
pub fn build_routing_diagnostic(
    config: &AggregatedConfig,
    all_tools: &[Tool],
) -> RoutingDiagnostic {
    // Build effective rules map
    let mut effective_rules = HashMap::new();
    for (path, rule) in &config.path_rules {
        effective_rules.insert(
            path.clone(),
            EffectivePathRule {
                path: path.clone(),
                whitelist: rule.whitelist.clone(),
                blacklist: rule.blacklist.clone(),
                tag_filters: rule.tag_filters.clone(),
                sources: rule.sources.clone(),
            },
        );
    }

    // Detect conflicts with optimized metadata parsing
    let conflict_reports = detect_conflicts(config, all_tools);

    RoutingDiagnostic {
        config_sources: config.config_sources.clone(),
        effective_rules,
        conflict_reports,
    }
}

/// Detect conflicts by checking actual tools against rules
fn detect_conflicts(config: &AggregatedConfig, all_tools: &[Tool]) -> Vec<ConflictReport> {
    let mut conflict_reports = Vec::new();

    for (path, rule) in &config.path_rules {
        // Skip if no potential conflict (need both whitelist and blacklist)
        if rule.whitelist.is_empty() || rule.blacklist.is_empty() {
            continue;
        }

        // Check each tool directly without collecting
        for tool in all_tools {
            // Parse metadata on demand (only for relevant tools)
            let metadata = parse_tool_metadata(tool);

            // Check if tool would pass whitelist check
            let passes_whitelist = tool_passes_whitelist(tool, &metadata, &rule.whitelist);

            // Check if tool is blacklisted (must use same logic as filtering.rs)
            let is_blacklisted = tool_is_blacklisted(tool, &metadata, &rule.blacklist);

            // If tool would pass whitelist but is blacklisted -> conflict
            if passes_whitelist && is_blacklisted {
                let whitelisted_via = determine_whitelist_source(tool, &metadata, &rule.whitelist);

                conflict_reports.push(ConflictReport {
                    path: path.clone(),
                    tool_or_component: tool.name.clone(),
                    conflict: format!(
                        "Tool '{}' is whitelisted via {} (from {:?}) but blacklisted by {:?}",
                        tool.name,
                        whitelisted_via,
                        rule.sources.whitelist_from,
                        rule.sources.blacklist_from
                    ),
                    resolution: "DENIED (blacklist wins per Deny Trumps Allow rule)".to_string(),
                });
            }
        }
    }

    conflict_reports
}

/// Determine how a tool was whitelisted (by component ID or tool name).
fn determine_whitelist_source(
    tool: &Tool,
    metadata: &ToolMetadata,
    whitelist: &[String],
) -> String {
    if let Some(comp_id) = &metadata.component_id
        && whitelist.contains(comp_id)
    {
        return format!("component '{}'", comp_id);
    }
    format!("tool name '{}'", tool.name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AggregatedConfig, AggregatedPathRule, ConfigSource, RuleSources};

    #[test]
    fn test_detect_conflicts_component_id_blacklist() {
        // Critical regression test: Ensure conflict detection uses component_id blacklist logic
        // This test would FAIL if diagnostic.rs only checks tool.name in blacklist

        let mut config = AggregatedConfig {
            path_rules: std::collections::HashMap::new(),
            global_tag_filters: std::collections::HashMap::new(),
            config_sources: vec![ConfigSource {
                uri: "test://config".to_string(),
                version: "1.0".to_string(),
            }],
        };

        // Create rule that whitelists by component_id but blacklists by component_id too
        let rule = AggregatedPathRule {
            whitelist: vec!["evil-component".to_string()],
            blacklist: vec!["evil-component".to_string()], // Same component_id
            tag_filters: std::collections::HashMap::new(),
            sources: RuleSources {
                whitelist_from: vec!["config1".to_string()],
                blacklist_from: vec!["config2".to_string()],
                tag_filters_from: vec![],
            },
        };

        config.path_rules.insert("/mcp".to_string(), rule);

        // Tool with component_id that's both whitelisted AND blacklisted
        let tools = vec![Tool {
            name: "dangerous_tool".to_string(),
            input_schema: "{}".to_string(),
            options: Some(ToolOptions {
                description: None,
                title: None,
                meta: Some(r#"{"component_id":"evil-component"}"#.to_string()),
                annotations: None,
                output_schema: None,
            }),
        }];

        let conflicts = detect_conflicts(&config, &tools);

        // MUST detect conflict because component_id is both whitelisted AND blacklisted
        assert_eq!(
            conflicts.len(),
            1,
            "Should detect conflict when component_id is whitelisted and blacklisted"
        );
        assert!(conflicts[0].conflict.contains("evil-component"));
        assert!(conflicts[0].conflict.contains("whitelisted"));
        assert!(conflicts[0].conflict.contains("blacklisted"));
    }

    #[test]
    fn test_detect_conflicts_tool_name_and_component_id() {
        // Test mixed scenario: whitelisted by name, blacklisted by component_id

        let mut config = AggregatedConfig {
            path_rules: std::collections::HashMap::new(),
            global_tag_filters: std::collections::HashMap::new(),
            config_sources: vec![],
        };

        let rule = AggregatedPathRule {
            whitelist: vec!["useful_tool".to_string()], // Whitelist by tool name
            blacklist: vec!["bad-component".to_string()], // Blacklist by component_id
            tag_filters: std::collections::HashMap::new(),
            sources: RuleSources {
                whitelist_from: vec!["config1".to_string()],
                blacklist_from: vec!["config2".to_string()],
                tag_filters_from: vec![],
            },
        };

        config.path_rules.insert("/mcp".to_string(), rule);

        let tools = vec![Tool {
            name: "useful_tool".to_string(),
            input_schema: "{}".to_string(),
            options: Some(ToolOptions {
                description: None,
                title: None,
                meta: Some(r#"{"component_id":"bad-component"}"#.to_string()),
                annotations: None,
                output_schema: None,
            }),
        }];

        let conflicts = detect_conflicts(&config, &tools);

        // MUST detect this conflict
        assert_eq!(
            conflicts.len(),
            1,
            "Should detect conflict when tool name is whitelisted but component_id is blacklisted"
        );
    }

    #[test]
    fn test_detect_conflicts_no_conflict() {
        // Test that non-conflicting tools don't report conflicts

        let mut config = AggregatedConfig {
            path_rules: std::collections::HashMap::new(),
            global_tag_filters: std::collections::HashMap::new(),
            config_sources: vec![],
        };

        let rule = AggregatedPathRule {
            whitelist: vec!["safe-component".to_string()],
            blacklist: vec!["dangerous_tool".to_string()],
            tag_filters: std::collections::HashMap::new(),
            sources: RuleSources {
                whitelist_from: vec![],
                blacklist_from: vec![],
                tag_filters_from: vec![],
            },
        };

        config.path_rules.insert("/mcp".to_string(), rule);

        // Tool that passes whitelist and is NOT blacklisted
        let tools = vec![Tool {
            name: "safe_tool".to_string(),
            input_schema: "{}".to_string(),
            options: Some(ToolOptions {
                description: None,
                title: None,
                meta: Some(r#"{"component_id":"safe-component"}"#.to_string()),
                annotations: None,
                output_schema: None,
            }),
        }];

        let conflicts = detect_conflicts(&config, &tools);

        assert_eq!(
            conflicts.len(),
            0,
            "Should not detect conflict for safe tool"
        );
    }
}
