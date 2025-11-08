//! Capability discovery for MCP servers

use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    ClientMessage, ClientRequest, CompleteRequest, CompletionArgument, CompletionPromptReference,
    CompletionReference, ErrorCode, ListPromptsRequest, ListResourcesRequest, ListToolsRequest,
    ProtocolVersion, RequestId, ServerCapabilities, ServerLists, ServerResult,
};
use crate::bindings::wasmcp::mcp_v20250618::server_handler::handle;
use crate::bindings::wasmcp::mcp_v20250618::server_io::MessageFrame;
use crate::common::protocol::create_message_context;

/// Request ID for internal capability discovery probes
/// Uses -1 to avoid conflicts with real client request IDs (which are typically positive)
const CAPABILITY_PROBE_REQUEST_ID: i64 = -1;

/// Discover capabilities for initialize response
///
/// This is called during initialize to probe the downstream handler
pub fn discover_capabilities_for_init(
    protocol_version: ProtocolVersion,
    frame: &MessageFrame,
) -> ServerCapabilities {
    discover_capabilities(protocol_version, frame)
}

/// Discover server capabilities by probing downstream handler
///
/// This sends test requests to see what the middleware stack supports
fn discover_capabilities(
    protocol_version: ProtocolVersion,
    frame: &MessageFrame,
) -> ServerCapabilities {
    let mut list_changed_flags = ServerLists::empty();
    let mut has_completions = false;

    // Probe for tools support
    let tools_ctx = create_message_context(None, protocol_version, None, None, "", frame);
    let tools_request = ClientRequest::ToolsList(ListToolsRequest { cursor: None });
    let tools_message = ClientMessage::Request((
        RequestId::Number(CAPABILITY_PROBE_REQUEST_ID),
        tools_request,
    ));
    if let Some(Ok(_)) = handle(&tools_ctx, tools_message) {
        list_changed_flags |= ServerLists::TOOLS;
    }

    // Probe for resources support
    let resources_ctx = create_message_context(None, protocol_version, None, None, "", frame);
    let resources_request = ClientRequest::ResourcesList(ListResourcesRequest { cursor: None });
    let resources_message = ClientMessage::Request((
        RequestId::Number(CAPABILITY_PROBE_REQUEST_ID),
        resources_request,
    ));
    if let Some(Ok(_)) = handle(&resources_ctx, resources_message) {
        list_changed_flags |= ServerLists::RESOURCES;
    }

    // Probe for prompts support and use result to test completions
    let prompts_ctx = create_message_context(None, protocol_version, None, None, "", frame);
    let prompts_request = ClientRequest::PromptsList(ListPromptsRequest { cursor: None });
    let prompts_message = ClientMessage::Request((
        RequestId::Number(CAPABILITY_PROBE_REQUEST_ID),
        prompts_request,
    ));
    if let Some(Ok(ServerResult::PromptsList(prompts_result))) =
        handle(&prompts_ctx, prompts_message)
    {
        list_changed_flags |= ServerLists::PROMPTS;

        // Try to discover completions support using a real prompt
        if !prompts_result.prompts.is_empty() {
            let first_prompt = &prompts_result.prompts[0];

            // Check if prompt has arguments to complete
            if let Some(ref options) = first_prompt.options
                && let Some(ref args) = options.arguments
                && !args.is_empty()
            {
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
                let completion_ctx =
                    create_message_context(None, protocol_version, None, None, "", frame);
                let req = ClientRequest::CompletionComplete(completion_request);
                let completion_message =
                    ClientMessage::Request((RequestId::Number(CAPABILITY_PROBE_REQUEST_ID), req));
                match handle(&completion_ctx, completion_message) {
                    Some(Ok(_)) => has_completions = true,
                    Some(Err(ErrorCode::MethodNotFound(_))) => {
                        has_completions = false;
                    }
                    Some(Err(_)) => {
                        // Other errors (InvalidParams, etc.) suggest completions might be
                        // supported but our test failed - assume supported
                        has_completions = true;
                    }
                    None => has_completions = false,
                }
            }
        }
    }

    // Build capabilities based on what succeeded
    ServerCapabilities {
        completions: if has_completions {
            Some("{}".to_string())
        } else {
            None
        },
        experimental: None,
        logging: Some("{}".to_string()), // We support logging/setLevel
        list_changed: if list_changed_flags.is_empty() {
            None
        } else {
            Some(list_changed_flags)
        },
        subscriptions: None, // TODO: Probe for subscription support
    }
}
