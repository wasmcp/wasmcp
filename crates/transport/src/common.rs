//! Common transport logic shared between HTTP and stdio implementations

use crate::bindings::wasi::io::streams::{InputStream, OutputStream};
use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    ClientMessage, ClientNotification, ClientRequest, ErrorCode, ProtocolVersion, RequestId,
    ServerCapabilities, ServerMessage, ServerResult,
};
use crate::bindings::wasmcp::mcp_v20250618::server_handler::{MessageContext, Session, handle};
use crate::bindings::wasmcp::mcp_v20250618::server_io::{self, IoError, MessageFrame, ReadLimit};

/// Parsed MCP message from the wire
#[derive(Debug)]
pub enum McpMessage {
    Request(RequestId, ClientRequest),
    Notification(ClientNotification),
    Result(
        RequestId,
        crate::bindings::wasmcp::mcp_v20250618::mcp::ClientResult,
    ),
    Error(
        Option<RequestId>,
        crate::bindings::wasmcp::mcp_v20250618::mcp::ErrorCode,
    ),
}

// =============================================================================
// FRAMING CONFIGURATION HELPERS
// =============================================================================

/// Plain JSON framing configuration (no prefix/suffix)
///
/// Used for parsing incoming HTTP POST requests, which contain plain JSON
pub fn plain_json_frame() -> MessageFrame {
    MessageFrame {
        prefix: vec![],
        suffix: vec![],
    }
}

/// HTTP SSE framing configuration
///
/// Messages are framed as Server-Sent Events:
/// - Prefix: "data: "
/// - Suffix: "\n\n"
///
/// Used for writing SSE responses
pub fn http_sse_frame() -> MessageFrame {
    MessageFrame {
        prefix: b"data: ".to_vec(),
        suffix: b"\n\n".to_vec(),
    }
}

/// HTTP read limit configuration
///
/// For HTTP, we read the entire request body up to a maximum size
pub fn http_read_limit() -> ReadLimit {
    ReadLimit::MaxBytes(10 * 1024 * 1024) // 10MB max
}

/// Stdio newline-delimited JSON framing configuration
///
/// Messages are newline-delimited:
/// - Prefix: (none)
/// - Suffix: "\n"
pub fn stdio_frame() -> MessageFrame {
    MessageFrame {
        prefix: vec![],
        suffix: b"\n".to_vec(),
    }
}

/// Stdio read limit configuration
///
/// For stdio, we read until newline delimiter
pub fn stdio_read_limit() -> ReadLimit {
    ReadLimit::Delimiter(vec![b'\n'])
}

// =============================================================================
// MESSAGE PARSING
// =============================================================================

/// Parse incoming MCP message using server-io
///
/// Uses the new unified parse_message() interface.
/// Note: set_frame() must be called before using this function.
pub fn parse_mcp_message(input: &InputStream, limit: ReadLimit) -> Result<McpMessage, String> {
    let client_message = server_io::parse_message(input, &limit)
        .map_err(|e| format!("Failed to parse message: {:?}", e))?;

    match client_message {
        ClientMessage::Request((request_id, client_request)) => {
            Ok(McpMessage::Request(request_id, client_request))
        }
        ClientMessage::Notification(client_notification) => {
            Ok(McpMessage::Notification(client_notification))
        }
        ClientMessage::Result((result_id, client_result)) => {
            Ok(McpMessage::Result(result_id, client_result))
        }
        ClientMessage::Error((error_id, error_code)) => Ok(McpMessage::Error(error_id, error_code)),
    }
}

// =============================================================================
// MESSAGE WRITING
// =============================================================================

/// Write MCP result using server-io
///
/// Uses the new unified send_message() interface.
/// Note: set_frame() must be called before using this function.
pub fn write_mcp_result(
    output: &OutputStream,
    id: RequestId,
    result: ServerResult,
) -> Result<(), IoError> {
    let message = ServerMessage::Result((id, result));
    server_io::send_message(output, message)
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
    let tools_ctx = MessageContext {
        client_stream: None,
        protocol_version: "2025-06-18".to_string(),
        session: None,
        identity: None,
    };
    let tools_request = ClientRequest::ToolsList(ListToolsRequest { cursor: None });
    let tools_message = ClientMessage::Request((RequestId::Number(0), tools_request));
    if let Some(Ok(_)) = handle(&tools_ctx, tools_message) {
        list_changed_flags |= ServerLists::TOOLS;
    }

    // Probe for resources support
    let resources_ctx = MessageContext {
        client_stream: None,
        protocol_version: "2025-06-18".to_string(),
        session: None,
        identity: None,
    };
    let resources_request = ClientRequest::ResourcesList(ListResourcesRequest { cursor: None });
    let resources_message = ClientMessage::Request((RequestId::Number(1), resources_request));
    if let Some(Ok(_)) = handle(&resources_ctx, resources_message) {
        list_changed_flags |= ServerLists::RESOURCES;
    }

    // Probe for prompts support and use result to test completions
    let prompts_ctx = MessageContext {
        client_stream: None,
        protocol_version: "2025-06-18".to_string(),
        session: None,
        identity: None,
    };
    let prompts_request = ClientRequest::PromptsList(ListPromptsRequest { cursor: None });
    let prompts_message = ClientMessage::Request((RequestId::Number(2), prompts_request));
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
                let completion_ctx = MessageContext {
                    client_stream: None,
                    protocol_version: "2025-06-18".to_string(),
                    session: None,
                    identity: None,
                };
                let req = ClientRequest::CompletionComplete(completion_request);
                let completion_message = ClientMessage::Request((RequestId::Number(3), req));
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

    // Create message context
    let ctx = MessageContext {
        client_stream: Some(output_stream),
        protocol_version: protocol_version_to_string(protocol_version),
        session,
        identity: None, // TODO: Add user identity support
    };

    // Create client message
    let message = ClientMessage::Request((request_id, client_request));

    // Delegate to imported server-handler
    match handle(&ctx, message) {
        Some(Ok(result)) => Ok(result),
        Some(Err(e)) => Err(e),
        None => Err(ErrorCode::InternalError(
            crate::bindings::wasmcp::mcp_v20250618::mcp::Error {
                code: -32603,
                message: "Handler returned None for request".to_string(),
                data: None,
            },
        )),
    }
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

    // Create message context (no client-stream for notifications - they're one-way)
    let ctx = MessageContext {
        client_stream: None,
        protocol_version: protocol_version_to_string(protocol_version),
        session,
        identity: None,
    };

    // Create client message
    let message = ClientMessage::Notification(client_notification);

    // Delegate to imported server-handler (should return None for notifications)
    handle(&ctx, message);
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
