mod bindings {
    wit_bindgen::generate!({
        world: "filter-middleware",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp_v20250618::server_handler::{Guest, MessageContext};
use bindings::wasmcp::keyvalue::store::TypedValue;
use bindings::wasmcp::mcp_v20250618::mcp::*;
use bindings::wasmcp::mcp_v20250618::server_handler as downstream;
use bindings::wasmcp::mcp_v20250618::sessions;

use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

// Convert exported MessageContext to imported MessageContext
fn to_downstream_ctx<'a>(ctx: &'a MessageContext<'a>) -> downstream::MessageContext<'a> {
    downstream::MessageContext {
        client_stream: ctx.client_stream,
        protocol_version: ctx.protocol_version.clone(),
        session: ctx.session.as_ref().map(|s| downstream::Session {
            session_id: s.session_id.clone(),
            store_id: s.store_id.clone(),
        }),
        identity: ctx.identity.as_ref().map(|i| downstream::Identity {
            jwt: i.jwt.clone(),
            claims: i.claims.clone(),
        }),
        frame: ctx.frame.clone(),
        http_context: ctx.http_context.clone(),
    }
}

/// Routing configuration loaded from routing://config resource
#[derive(Debug, Deserialize)]
struct RoutingConfig {
    version: String,
    #[serde(rename = "path-rules")]
    path_rules: HashMap<String, PathRule>,
    #[serde(rename = "tag-filters", default)]
    global_tag_filters: HashMap<String, TagFilterValue>,
}

/// Path-based filtering rule
#[derive(Debug, Deserialize)]
struct PathRule {
    whitelist: Option<Vec<String>>,
    blacklist: Option<Vec<String>>,
    #[serde(rename = "tag-filters", default)]
    tag_filters: HashMap<String, TagFilterValue>,
}

/// Tag filter value - can be single string or array of strings
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
enum TagFilterValue {
    Single(String),
    Multiple(Vec<String>),
}

/// Tool metadata extracted from tool.options.meta
#[derive(Debug)]
struct ToolMetadata {
    component_id: Option<String>,
    tags: HashMap<String, String>,
}

struct FilterMiddleware;

impl Guest for FilterMiddleware {
    fn handle(
        ctx: MessageContext,
        message: ClientMessage,
    ) -> Option<Result<ServerResult, ErrorCode>> {
        match message {
            ClientMessage::Request((request_id, request)) => {
                // Handle requests - match on request type
                let result = match &request {
                    ClientRequest::ToolsList(list_req) => {
                        handle_tools_list(request_id.clone(), list_req.clone(), &ctx)
                    }
                    ClientRequest::ToolsCall(call_req) => {
                        handle_tools_call(request_id.clone(), call_req.clone(), &ctx)
                    }
                    _ => {
                        // Delegate all other requests to downstream handler
                        let downstream_msg = ClientMessage::Request((request_id.clone(), request));
                        return downstream::handle(&to_downstream_ctx(&ctx), downstream_msg);
                    }
                };
                Some(result)
            }
            _ => {
                // Forward notifications, results, errors to downstream
                downstream::handle(&to_downstream_ctx(&ctx), message)
            }
        }
    }
}

/// Handle tools/list - apply path and tag filtering
fn handle_tools_list(
    request_id: RequestId,
    req: ListToolsRequest,
    ctx: &MessageContext,
) -> Result<ServerResult, ErrorCode> {
    // Call downstream to get all tools
    let downstream_req = ClientRequest::ToolsList(req.clone());
    let downstream_msg = ClientMessage::Request((request_id.clone(), downstream_req));

    let all_tools = match downstream::handle(&to_downstream_ctx(ctx), downstream_msg) {
        Some(Ok(ServerResult::ToolsList(result))) => result.tools,
        Some(Ok(_)) => {
            return Err(ErrorCode::InternalError(Error {
                code: -32603,
                message: "Unexpected result type from downstream".to_string(),
                data: None,
            }))
        }
        Some(Err(e)) => return Err(e),
        None => {
            return Err(ErrorCode::MethodNotFound(Error {
                code: -32601,
                message: "Method not found: tools/list".to_string(),
                data: None,
            }))
        }
    };

    // Load routing config
    let config = match load_routing_config(ctx) {
        Ok(c) => c,
        Err(e) => {
            return Err(ErrorCode::InternalError(Error {
                code: -32603,
                message: format!("Failed to load routing config: {}", e),
                data: None,
            }));
        }
    };

    let current_path = extract_path(ctx);

    // Stage 1: Apply path-based whitelist/blacklist filtering
    let path_filtered = apply_path_filter(&all_tools, &current_path, &config);

    // Stage 2: Apply tag-based filtering (global + path-specific)
    let tag_filtered = apply_tag_filters(&path_filtered, &current_path, &config);

    // Store filtered tool names in session for validation in tools/call
    if let Err(e) = store_tool_registry(ctx, &tag_filtered) {
        return Err(ErrorCode::InternalError(Error {
            code: -32603,
            message: format!("Failed to store tool registry: {}", e),
            data: None,
        }));
    }

    // Return filtered list
    Ok(ServerResult::ToolsList(ListToolsResult {
        tools: tag_filtered,
        next_cursor: None,
        meta: None,
    }))
}

/// Handle tools/call - validate tool is in allowed list
fn handle_tools_call(
    request_id: RequestId,
    req: CallToolRequest,
    ctx: &MessageContext,
) -> Result<ServerResult, ErrorCode> {
    // Load allowed tools from session
    let allowed_tools = match load_tool_registry(ctx) {
        Ok(tools) => tools,
        Err(_) => {
            // If no registry in session, allow call (tools/list may not have been called)
            let downstream_req = ClientRequest::ToolsCall(req);
            let downstream_msg = ClientMessage::Request((request_id, downstream_req));
            return match downstream::handle(&to_downstream_ctx(ctx), downstream_msg) {
                Some(result) => result,
                None => Err(ErrorCode::MethodNotFound(Error {
                    code: -32601,
                    message: "Method not found".to_string(),
                    data: None,
                })),
            };
        }
    };

    // Validate tool is allowed
    if !allowed_tools.contains(&req.name) {
        return Err(ErrorCode::InvalidParams(Error {
            code: -32602,
            message: format!("Tool '{}' not available at this path", req.name),
            data: None,
        }));
    }

    // Tool is allowed, delegate to downstream
    let downstream_req = ClientRequest::ToolsCall(req);
    let downstream_msg = ClientMessage::Request((request_id, downstream_req));
    match downstream::handle(&to_downstream_ctx(ctx), downstream_msg) {
        Some(result) => result,
        None => Err(ErrorCode::MethodNotFound(Error {
            code: -32601,
            message: "Method not found".to_string(),
            data: None,
        })),
    }
}

/// Load routing configuration from routing://config resource
fn load_routing_config(ctx: &MessageContext) -> Result<RoutingConfig, String> {
    // Read routing://config resource by sending request through downstream handler
    let request = ReadResourceRequest {
        uri: "routing://config".to_string(),
    };

    // Send resources/read request through server-handler chain
    let downstream_req = ClientRequest::ResourcesRead(request);
    let downstream_msg = ClientMessage::Request((RequestId::Number(0), downstream_req));

    let result = match downstream::handle(&to_downstream_ctx(ctx), downstream_msg) {
        Some(Ok(ServerResult::ResourcesRead(result))) => result,
        Some(Ok(_)) => return Err("Unexpected result type from resources/read".to_string()),
        Some(Err(e)) => return Err(format!("Resource read failed: {:?}", e)),
        None => return Err("routing://config resource not found".to_string()),
    };

    let contents = result
        .contents
        .first()
        .ok_or_else(|| "routing://config has no contents".to_string())?;

    // Extract text from resource-contents variant
    let text_contents = match contents {
        ResourceContents::Text(t) => t,
        ResourceContents::Blob(_) => {
            return Err("routing://config is binary, expected text".to_string())
        }
    };

    // Extract string from text-data variant
    let config_text = match &text_contents.text {
        TextData::Text(s) => s,
        TextData::TextStream(_) => {
            return Err("routing://config is streamed, expected inline text".to_string())
        }
    };

    // Parse TOML
    toml::from_str(config_text).map_err(|e| format!("TOML parse error: {}", e))
}

/// Extract HTTP path from message context
fn extract_path(ctx: &MessageContext) -> String {
    ctx.http_context
        .as_ref()
        .map(|h| h.path.clone())
        .unwrap_or_else(|| "/mcp".to_string())
}

/// Apply path-based filtering with hierarchical matching
fn apply_path_filter(tools: &[Tool], path: &str, config: &RoutingConfig) -> Vec<Tool> {
    // Find most specific (longest) matching path rule
    let rule = find_most_specific_path_rule(path, config);

    // If no matching rule, allow all tools (tag filtering only)
    let rule = match rule {
        Some(r) => r,
        None => return tools.to_vec(),
    };

    // If rule exists but has no whitelist/blacklist, allow all
    if rule.whitelist.is_none() && rule.blacklist.is_none() {
        return tools.to_vec();
    }

    let mut filtered = tools.to_vec();

    // Apply whitelist (if present)
    if let Some(whitelist) = &rule.whitelist {
        filtered.retain(|tool| {
            let meta = parse_tool_metadata(tool);

            // Check if component_id is whitelisted
            if let Some(comp_id) = &meta.component_id {
                if whitelist.contains(comp_id) {
                    return true;
                }
            }

            // Check if tool name is whitelisted
            whitelist.contains(&tool.name)
        });
    }

    // Apply blacklist (if present)
    if let Some(blacklist) = &rule.blacklist {
        filtered.retain(|tool| !blacklist.contains(&tool.name));
    }

    filtered
}

/// Find the most specific (longest matching) path rule
fn find_most_specific_path_rule<'a>(path: &str, config: &'a RoutingConfig) -> Option<&'a PathRule> {
    // Find all matching path rules (path starts with rule path)
    let mut matches: Vec<(&str, &PathRule)> = config
        .path_rules
        .iter()
        .filter(|(rule_path, _)| path.starts_with(rule_path.as_str()))
        .map(|(k, v)| (k.as_str(), v))
        .collect();

    // Sort by path length (descending) to get most specific first
    matches.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    // Return most specific match
    matches.first().map(|(_, rule)| *rule)
}

/// Apply tag-based filtering (global + path-specific)
/// Global filters apply to all paths, path-specific filters apply only at matching paths
/// Both use AND logic - tool must match all active filters
fn apply_tag_filters(tools: &[Tool], path: &str, config: &RoutingConfig) -> Vec<Tool> {
    // Collect all active tag filters (global + path-specific)
    let mut active_filters: HashMap<String, Vec<String>> = HashMap::new();

    // Add global tag filters
    for (key, value) in &config.global_tag_filters {
        active_filters.insert(key.clone(), tag_filter_value_to_vec(value));
    }

    // Add path-specific tag filters (if path rule exists)
    if let Some(rule) = find_most_specific_path_rule(path, config) {
        for (key, value) in &rule.tag_filters {
            active_filters.insert(key.clone(), tag_filter_value_to_vec(value));
        }
    }

    // If no filters, return all tools
    if active_filters.is_empty() {
        return tools.to_vec();
    }

    // Filter tools - must match ALL active filters (AND behavior)
    tools
        .iter()
        .filter(|tool| {
            let meta = parse_tool_metadata(tool);

            // Tool must match ALL active tag filters
            for (tag_name, allowed_values) in &active_filters {
                let tool_tag_value = meta.tags.get(tag_name);

                match tool_tag_value {
                    Some(value) if allowed_values.contains(value) => continue,
                    _ => return false, // Tag missing or doesn't match
                }
            }

            true
        })
        .cloned()
        .collect()
}

/// Convert TagFilterValue enum to Vec<String>
fn tag_filter_value_to_vec(value: &TagFilterValue) -> Vec<String> {
    match value {
        TagFilterValue::Single(s) => vec![s.clone()],
        TagFilterValue::Multiple(v) => v.clone(),
    }
}

/// Parse tool metadata from tool.options.meta JSON
fn parse_tool_metadata(tool: &Tool) -> ToolMetadata {
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

/// Store filtered tool names in session for validation
fn store_tool_registry(ctx: &MessageContext, tools: &[Tool]) -> Result<(), String> {
    let session = match &ctx.session {
        Some(s) => s,
        None => return Ok(()), // No session, skip storage
    };

    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
    let registry_json = serde_json::to_string(&tool_names)
        .map_err(|e| format!("Failed to serialize tool registry: {}", e))?;

    let session_obj = sessions::Session::open(&session.session_id, &session.store_id)
        .map_err(|e| format!("Failed to open session: {:?}", e))?;

    session_obj
        .set("filter:tool_registry", &TypedValue::AsString(registry_json))
        .map_err(|e| format!("Failed to set tool registry: {:?}", e))?;

    Ok(())
}

/// Load filtered tool names from session
fn load_tool_registry(ctx: &MessageContext) -> Result<Vec<String>, String> {
    let session = match &ctx.session {
        Some(s) => s,
        None => return Err("No session".to_string()),
    };

    let session_obj = sessions::Session::open(&session.session_id, &session.store_id)
        .map_err(|e| format!("Failed to open session: {:?}", e))?;

    let value = session_obj
        .get("filter:tool_registry")
        .map_err(|e| format!("Failed to get tool registry: {:?}", e))?;

    match value {
        Some(TypedValue::AsString(json)) => {
            serde_json::from_str(&json).map_err(|e| format!("Failed to parse tool registry: {}", e))
        }
        _ => Err("Tool registry not found in session".to_string()),
    }
}

bindings::export!(FilterMiddleware with_types_in bindings);
