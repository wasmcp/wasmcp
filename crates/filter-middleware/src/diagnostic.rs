use crate::bindings::exports::wasmcp::mcp_v20250618::server_handler::MessageContext;
use crate::bindings::wasmcp::mcp_v20250618::mcp::*;
use crate::config::load_and_aggregate_configs;
use crate::helpers::{fetch_tools_from_downstream, to_downstream_ctx};
use crate::metadata::{parse_tool_metadata, tool_passes_whitelist};
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
            }))
        }
    };

    // Get all tools from downstream to check for conflicts
    let all_tools = match fetch_tools_from_downstream(
        ctx,
        RequestId::Number(0),
        ListToolsRequest { cursor: None },
    ) {
        Ok(tools) => tools,
        Err(_) => Vec::new(), // If we can't get tools, proceed without conflict detection
    };

    // Build diagnostic output with conflict detection
    let diagnostic = build_routing_diagnostic(&config, &all_tools);

    // Return as CallToolResult with pretty JSON
    Ok(ServerResult::ToolsCall(CallToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: TextData::Text(
                serde_json::to_string_pretty(&diagnostic)
                    .unwrap_or_else(|_| "Error serializing diagnostic".to_string()),
            ),
            options: None,
        })],
        is_error: None,
        meta: None,
        structured_content: None,
    }))
}

/// Build routing diagnostic output with conflict detection
pub fn build_routing_diagnostic(config: &AggregatedConfig, all_tools: &[Tool]) -> RoutingDiagnostic {
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

    // Parse metadata once for all tools
    let tools_with_meta: Vec<(Tool, ToolMetadata)> = all_tools
        .iter()
        .map(|t| (t.clone(), parse_tool_metadata(t)))
        .collect();

    for (path, rule) in &config.path_rules {
        // Skip if no whitelist or blacklist
        if rule.whitelist.is_empty() || rule.blacklist.is_empty() {
            continue;
        }

        // Check each tool to see if it would be whitelisted but also blacklisted
        for (tool, metadata) in &tools_with_meta {
            // Check if tool would pass whitelist check
            let passes_whitelist = tool_passes_whitelist(tool, metadata, &rule.whitelist);

            // Check if tool is blacklisted
            let is_blacklisted = rule.blacklist.contains(&tool.name);

            // If tool would pass whitelist but is blacklisted -> conflict
            if passes_whitelist && is_blacklisted {
                let whitelisted_via = if let Some(comp_id) = &metadata.component_id {
                    if rule.whitelist.contains(comp_id) {
                        format!("component '{}'", comp_id)
                    } else {
                        format!("tool name '{}'", tool.name)
                    }
                } else {
                    format!("tool name '{}'", tool.name)
                };

                conflict_reports.push(ConflictReport {
                    path: path.clone(),
                    tool_or_component: tool.name.clone(),
                    conflict: format!(
                        "Tool '{}' is whitelisted via {} (from {:?}) but blacklisted by {:?}",
                        tool.name, whitelisted_via, rule.sources.whitelist_from, rule.sources.blacklist_from
                    ),
                    resolution: "DENIED (blacklist wins per Deny Trumps Allow rule)".to_string(),
                });
            }
        }
    }

    conflict_reports
}