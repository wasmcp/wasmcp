//! Server capability discovery for MCP transport
//!
//! Discovers what the downstream handler supports by probing with list requests.
//! This allows the transport to advertise accurate capabilities to clients.

use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    ClientRequest, CompleteRequest, CompletionArgument, CompletionPromptReference,
    CompletionReference, ListPromptsRequest, ListResourcesRequest, ListToolsRequest,
    ServerCapabilities, ServerLists, ServerResult,
};
use crate::bindings::wasmcp::mcp_v20250618::server_handler::{
    handle_request, RequestCtx, RequestId,
};

/// Helper to create a RequestCtx for capability discovery probes
///
/// All discovery requests use the same basic context with no session,
/// no user, and no message stream.
///
/// # Arguments
/// * `id` - Request ID number for this probe
/// * `protocol_version` - Negotiated protocol version string
///
/// # Returns
/// RequestCtx configured for discovery
fn create_discovery_ctx<'a>(id: i64, protocol_version: &'a str) -> RequestCtx<'a> {
    RequestCtx {
        id: RequestId::Number(id),
        protocol_version: protocol_version.to_string(),
        messages: None,
        session: None,
        user: None,
    }
}

/// Discover capabilities by probing downstream handler
///
/// Makes test calls to list-tools, list-resources, list-prompts, and completion/complete
/// to determine what the downstream handler supports. This allows the transport to
/// accurately advertise capabilities during initialization.
///
/// # Arguments
/// * `protocol_version` - The negotiated protocol version string (e.g., "2025-06-18")
///
/// # Returns
/// ServerCapabilities structure with discovered capabilities:
/// - `completions`: If completion/complete is supported
/// - `logging`: Always supported (transport provides logging)
/// - `list_changed`: Which list types are supported (tools, resources, prompts)
/// - `subscriptions`: None (stateless transport)
pub fn discover_capabilities(protocol_version: &str) -> ServerCapabilities {
    // Try to discover what the downstream handler supports by calling list methods
    // With optional output stream, we can pass None for discovery calls
    let mut list_flags = ServerLists::empty();

    // Try list-tools
    let req = ClientRequest::ToolsList(ListToolsRequest { cursor: None });
    let ctx = create_discovery_ctx(0, protocol_version);
    if handle_request(&ctx, &req).is_ok() {
        list_flags |= ServerLists::TOOLS;
    }

    // Try list-resources
    let req = ClientRequest::ResourcesList(ListResourcesRequest { cursor: None });
    let ctx = create_discovery_ctx(1, protocol_version);
    if handle_request(&ctx, &req).is_ok() {
        list_flags |= ServerLists::RESOURCES;
    }

    // Try list-prompts and use result to test completions
    let mut has_completions = false;
    let req = ClientRequest::PromptsList(ListPromptsRequest { cursor: None });
    let ctx = create_discovery_ctx(2, protocol_version);
    if let Ok(ServerResult::PromptsList(prompts_result)) = handle_request(&ctx, &req) {
        list_flags |= ServerLists::PROMPTS;

        // Try to discover completions support using a real prompt
        if !prompts_result.prompts.is_empty() {
            let first_prompt = &prompts_result.prompts[0];

            // Check if prompt has arguments to complete
            if let Some(ref options) = first_prompt.options {
                if let Some(ref args) = options.arguments {
                    if !args.is_empty() {
                        // Try completion with real prompt name and first argument
                        let completion_request = CompleteRequest {
                            argument: CompletionArgument {
                                name: args[0].name.clone(),
                                value: "".to_string(),
                            },
                            ref_: CompletionReference::Prompt(CompletionPromptReference {
                                name: first_prompt.name.clone(),
                                title: None,
                            }),
                            context: None,
                        };

                        // Test if completions are supported
                        let req = ClientRequest::CompletionComplete(completion_request);
                        let ctx = create_discovery_ctx(3, protocol_version);
                        match handle_request(&ctx, &req) {
                            Ok(_) => has_completions = true,
                            Err(
                                crate::bindings::wasmcp::mcp_v20250618::mcp::ErrorCode::MethodNotFound(_),
                            ) => {
                                has_completions = false;
                            }
                            Err(_) => {
                                // Other errors (InvalidParams, etc.) suggest completions might be
                                // supported but our test failed - assume supported
                                has_completions = true;
                            }
                        }
                    }
                }
            }
        }
    }

    ServerCapabilities {
        completions: if has_completions {
            Some("{}".to_string())
        } else {
            None
        },
        experimental: None,
        logging: Some("{}".to_string()), // We provide logging mechanism
        list_changed: if !list_flags.is_empty() {
            Some(list_flags)
        } else {
            None
        },
        subscriptions: None, // No subscribe support in stateless transport
    }
}

/// Serialize server capabilities to JSON
///
/// Converts ServerCapabilities structure to the JSON format expected by MCP clients.
/// Per MCP spec, capabilities are nested objects with specific flags:
/// - `completions`: {}
/// - `logging`: {}
/// - `tools`: { "listChanged": true }
/// - `resources`: { "listChanged": true, "subscribe": true }
/// - `prompts`: { "listChanged": true }
///
/// # Arguments
/// * `caps` - Server capabilities to serialize
///
/// # Returns
/// JSON object with capability structure
pub fn serialize_capabilities(caps: &ServerCapabilities) -> serde_json::Value {
    let mut result = serde_json::Map::new();

    if let Some(ref _completions) = caps.completions {
        result.insert("completions".to_string(), serde_json::json!({}));
    }

    if let Some(ref _logging) = caps.logging {
        result.insert("logging".to_string(), serde_json::json!({}));
    }

    // Serialize list_changed capabilities - each capability type gets its own nested object
    if let Some(flags) = caps.list_changed {
        if flags.contains(ServerLists::TOOLS) {
            let mut tools_caps = serde_json::Map::new();
            tools_caps.insert("listChanged".to_string(), serde_json::json!(true));
            result.insert("tools".to_string(), serde_json::Value::Object(tools_caps));
        }
        if flags.contains(ServerLists::RESOURCES) {
            let mut resources_caps = serde_json::Map::new();
            resources_caps.insert("listChanged".to_string(), serde_json::json!(true));
            result.insert(
                "resources".to_string(),
                serde_json::Value::Object(resources_caps),
            );
        }
        if flags.contains(ServerLists::PROMPTS) {
            let mut prompts_caps = serde_json::Map::new();
            prompts_caps.insert("listChanged".to_string(), serde_json::json!(true));
            result.insert(
                "prompts".to_string(),
                serde_json::Value::Object(prompts_caps),
            );
        }
    }

    if let Some(ref _subscriptions) = caps.subscriptions {
        // Handle subscriptions if present
        let mut resources_caps = result
            .get_mut("resources")
            .and_then(|v| v.as_object_mut())
            .map(|o| o.clone())
            .unwrap_or_default();
        resources_caps.insert("subscribe".to_string(), serde_json::json!(true));
        result.insert(
            "resources".to_string(),
            serde_json::Value::Object(resources_caps),
        );
    }

    serde_json::Value::Object(result)
}
