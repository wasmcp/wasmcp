//! Common transport logic shared between HTTP and stdio implementations

use crate::bindings::wasi::io::streams::{InputStream, OutputStream};
use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    ClientNotification, ClientRequest, ErrorCode, ProtocolVersion, RequestId, ServerCapabilities,
    ServerResult,
};
use crate::bindings::wasmcp::mcp_v20250618::server_handler::{
    NotificationCtx, RequestCtx, Session, handle_notification, handle_request,
};
use crate::bindings::wasmcp::mcp_v20250618::server_io::{
    self, IoError,
};

/// Parsed MCP message from the wire
#[derive(Debug)]
pub enum McpMessage {
    Request(RequestId, ClientRequest),
    Notification(ClientNotification),
    Result(RequestId, crate::bindings::wasmcp::mcp_v20250618::mcp::ClientResult),
    Error(Option<RequestId>, ErrorCode),
}

/// Parse incoming MCP message using server-io
///
/// Tries to parse as request, notification, result, or error in that order
pub fn parse_mcp_message(
    input: &InputStream,
) -> Result<McpMessage, String> {
    // Try to parse as request
    if let Ok((request_id, client_request)) = server_io::parse_request(input) {
        return Ok(McpMessage::Request(request_id, client_request));
    }

    // Try to parse as notification
    if let Ok(client_notification) = server_io::parse_notification(input) {
        return Ok(McpMessage::Notification(client_notification));
    }

    // Try to parse as result
    if let Ok((result_id, client_result)) = server_io::parse_result(input) {
        return Ok(McpMessage::Result(result_id, client_result));
    }

    // Try to parse as error
    if let Ok((error_id, error_code)) = server_io::parse_error(input) {
        return Ok(McpMessage::Error(error_id, error_code));
    }

    Err("Failed to parse message as request, notification, result, or error".to_string())
}

/// Write MCP result using server-io
pub fn write_mcp_result(
    output: &OutputStream,
    id: &RequestId,
    result: ServerResult,
) -> Result<(), IoError> {
    server_io::write_result(output, id, result)
}

/// Discover capabilities for initialize response
///
/// This is called during initialize to probe the downstream handler
pub fn discover_capabilities_for_init(_protocol_version: ProtocolVersion) -> ServerCapabilities {
    discover_capabilities()
}

/// Handle transport-level MCP method: ping
///
/// Simple health check that returns empty success (no specific result variant)
pub fn handle_ping() -> Result<(), ErrorCode> {
    Ok(())
}

/// Handle transport-level MCP method: logging/setLevel
///
/// Transport-level logging configuration (returns empty success)
pub fn handle_set_log_level(_level: String) -> Result<(), ErrorCode> {
    // No-op for now - could be implemented with env_logger or similar
    Ok(())
}

/// Discover server capabilities by probing downstream handler
///
/// This sends test requests to see what the middleware stack supports
fn discover_capabilities() -> ServerCapabilities {
    use crate::bindings::wasmcp::mcp_v20250618::mcp::{
        ClientRequest, CompleteRequest, CompletionArgument, CompletionPromptReference,
        CompletionReference, ListPromptsRequest, ListResourcesRequest, ListToolsRequest,
        ServerLists,
    };

    let mut list_changed_flags = ServerLists::empty();
    let mut has_completions = false;

    // Probe for tools support
    let tools_ctx = RequestCtx {
        id: RequestId::Number(0),
        protocol_version: "2025-06-18".to_string(),
        messages: None,
        session: None,
        user: None,
    };
    let tools_request = ClientRequest::ToolsList(ListToolsRequest { cursor: None });
    if handle_request(&tools_ctx, &tools_request).is_ok() {
        list_changed_flags |= ServerLists::TOOLS;
    }

    // Probe for resources support
    let resources_ctx = RequestCtx {
        id: RequestId::Number(1),
        protocol_version: "2025-06-18".to_string(),
        messages: None,
        session: None,
        user: None,
    };
    let resources_request = ClientRequest::ResourcesList(ListResourcesRequest { cursor: None });
    if handle_request(&resources_ctx, &resources_request).is_ok() {
        list_changed_flags |= ServerLists::RESOURCES;
    }

    // Probe for prompts support and use result to test completions
    let prompts_ctx = RequestCtx {
        id: RequestId::Number(2),
        protocol_version: "2025-06-18".to_string(),
        messages: None,
        session: None,
        user: None,
    };
    let prompts_request = ClientRequest::PromptsList(ListPromptsRequest { cursor: None });
    if let Ok(ServerResult::PromptsList(prompts_result)) =
        handle_request(&prompts_ctx, &prompts_request)
    {
        list_changed_flags |= ServerLists::PROMPTS;

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
                        let completion_ctx = RequestCtx {
                            id: RequestId::Number(3),
                            protocol_version: "2025-06-18".to_string(),
                            messages: None,
                            session: None,
                            user: None,
                        };
                        let req = ClientRequest::CompletionComplete(completion_request);
                        match handle_request(&completion_ctx, &req) {
                            Ok(_) => has_completions = true,
                            Err(ErrorCode::MethodNotFound(_)) => {
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

/// Delegate non-transport methods to middleware via server-handler
pub fn delegate_to_middleware(
    request_id: RequestId,
    client_request: ClientRequest,
    protocol_version: ProtocolVersion,
    session_id: Option<&str>,
    bucket_name: String,
    output_stream: &OutputStream,
) -> Result<ServerResult, ErrorCode> {
    // Create session if provided
    let session = session_id.map(|id| Session {
        session_id: id.to_string(),
        store_id: bucket_name.clone(),
    });

    // Create request context
    let ctx = RequestCtx {
        id: request_id,
        protocol_version: protocol_version_to_string(protocol_version),
        messages: Some(output_stream),
        session,    // Not .as_ref() - Option<Session> not Option<&Session>
        user: None, // TODO: Add user identity support
    };

    // Delegate to imported server-handler
    handle_request(&ctx, &client_request)
}

/// Delegate notification to middleware via server-handler
pub fn delegate_notification(
    client_notification: ClientNotification,
    protocol_version: ProtocolVersion,
    session_id: Option<&str>,
    bucket_name: String,
) -> Result<(), ErrorCode> {
    // Create session if provided
    let session = session_id.map(|id| Session {
        session_id: id.to_string(),
        store_id: bucket_name.clone(),
    });

    // Create notification context (no messages or id - notifications are one-way)
    let ctx = NotificationCtx {
        protocol_version: protocol_version_to_string(protocol_version),
        session,
        user: None,
    };

    // Delegate to imported server-handler (returns unit, not Result)
    handle_notification(&ctx, &client_notification);
    Ok(())
}

/// Convert ProtocolVersion enum to string format
fn protocol_version_to_string(version: ProtocolVersion) -> String {
    match version {
        ProtocolVersion::V20250618 => "2025-06-18".to_string(),
        ProtocolVersion::V20250326 => "2025-03-26".to_string(),
        ProtocolVersion::V20241105 => "2024-11-05".to_string(),
    }
}
