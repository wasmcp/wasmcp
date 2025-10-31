//! Server capability discovery for MCP transport
//!
//! Discovers what the downstream handler supports by probing with list requests.
//! This allows the transport to advertise accurate capabilities to clients.

use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    ClientRequest, CompleteRequest, CompletionArgument, CompletionPromptReference,
    CompletionReference, ListPromptsRequest, ListPromptsResult, ListResourcesRequest,
    ListToolsRequest, Prompt, ServerCapabilities, ServerLists, ServerResult,
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

/// Find a prompt with arguments suitable for completion testing
///
/// Searches the prompts list for the first prompt that has arguments,
/// which we can use to test completion support.
///
/// # Arguments
/// * `prompts_result` - Result from list-prompts request
///
/// # Returns
/// Reference to first prompt with arguments, or None if no suitable prompt found
fn find_prompt_with_arguments(prompts_result: &ListPromptsResult) -> Option<&Prompt> {
    prompts_result.prompts.iter().find(|prompt| {
        prompt
            .options
            .as_ref()
            .and_then(|opts| opts.arguments.as_ref())
            .map(|args| !args.is_empty())
            .unwrap_or(false)
    })
}

/// Test if completion endpoint is supported by downstream handler
///
/// Makes a test completion request using a real prompt and argument.
/// Per MCP spec, if the method is not implemented, we get MethodNotFound error.
/// Other errors (InvalidParams, etc.) suggest the method exists but our test failed.
///
/// # Arguments
/// * `prompt` - Prompt to use for testing (must have arguments)
/// * `protocol_version` - Negotiated protocol version string
///
/// # Returns
/// true if completions are supported, false otherwise
fn test_completion_support(prompt: &Prompt, protocol_version: &str) -> bool {
    // Get first argument from prompt (we know it exists from find_prompt_with_arguments)
    let first_arg = prompt
        .options
        .as_ref()
        .and_then(|opts| opts.arguments.as_ref())
        .and_then(|args| args.first())
        .expect("Prompt should have arguments");

    let completion_request = CompleteRequest {
        argument: CompletionArgument {
            name: first_arg.name.clone(),
            value: "".to_string(),
        },
        ref_: CompletionReference::Prompt(CompletionPromptReference {
            name: prompt.name.clone(),
            title: None,
        }),
        context: None,
    };

    let req = ClientRequest::CompletionComplete(completion_request);
    let ctx = create_discovery_ctx(3, protocol_version);

    match handle_request(&ctx, &req) {
        Ok(_) => true,
        Err(crate::bindings::wasmcp::mcp_v20250618::mcp::ErrorCode::MethodNotFound(_)) => false,
        Err(_) => {
            // Other errors (InvalidParams, etc.) suggest completions might be
            // supported but our test failed - assume supported
            true
        }
    }
}

/// Discover prompts support and optionally test completions
///
/// Makes list-prompts request to check if prompts are supported.
/// If prompts are supported and contain arguments, also tests completion support.
///
/// # Arguments
/// * `protocol_version` - Negotiated protocol version string
///
/// # Returns
/// Tuple of (prompts_supported, completions_supported)
fn discover_prompts_and_completions(protocol_version: &str) -> (bool, bool) {
    let req = ClientRequest::PromptsList(ListPromptsRequest { cursor: None });
    let ctx = create_discovery_ctx(2, protocol_version);

    match handle_request(&ctx, &req) {
        Ok(ServerResult::PromptsList(prompts_result)) => {
            let prompts_supported = true;

            // Try to discover completions support using a real prompt
            let completions_supported = find_prompt_with_arguments(&prompts_result)
                .map(|prompt| test_completion_support(prompt, protocol_version))
                .unwrap_or(false);

            (prompts_supported, completions_supported)
        }
        _ => (false, false),
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
    let (prompts_supported, has_completions) = discover_prompts_and_completions(protocol_version);
    if prompts_supported {
        list_flags |= ServerLists::PROMPTS;
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
