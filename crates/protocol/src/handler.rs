//! Lifecycle handler for MCP protocol
//!
//! This module implements the terminal handler for lifecycle-related MCP methods:
//! - `initialize` - Establishes the MCP connection and negotiates capabilities
//! - `ping` - Health check and keep-alive mechanism
//! - Unknown methods - Returns MethodNotFound errors for unrecognized methods
//!
//! ## Architecture
//!
//! The lifecycle handler serves as the final handler in a middleware chain.
//! It processes fundamental protocol operations that don't require feature-specific
//! logic. Feature handlers (tools, resources, prompts) sit earlier in the chain
//! and forward unrecognized methods to this lifecycle handler.
//!
//! ## Capability Detection
//!
//! Server capabilities are dynamically discovered by scanning the context store
//! for registrations made by upstream middleware components. Each feature handler
//! registers its capabilities using the `context::register-capability()` function,
//! and this handler aggregates them into the initialization response.

// Context functions will be used when implementing capability aggregation
// use crate::bindings::wasmcp::mcp::context::{get, register_capability};
use crate::bindings::wasmcp::mcp::output::IoError;
use crate::bindings::wasmcp::mcp::protocol::{
    ErrorCode, Implementation, InitializeParams, InitializeResult, InitializeResultOptions,
    McpError, McpMessage, McpRequest, RequestMethod, ServerCapabilities,
};

// Import response writers from our own implementation
use crate::responses::error::write_error;
use crate::responses::lifecycle::{write_initialization, write_pong};

/// Handle an incoming MCP message.
///
/// This is the entry point for all messages reaching the lifecycle handler.
/// It dispatches to specific handlers based on the message type and method.
pub fn handle_message(msg: &McpMessage) -> Result<(), IoError> {
    match msg {
        McpMessage::Request(req) => handle_request(req),
        // Notifications don't require responses
        McpMessage::Notification(_) => Ok(()),
        // Results and errors are responses, not handled by a server handler
        McpMessage::Result(_) | McpMessage::Error(_) => Ok(()),
    }
}

/// Handle an MCP request.
///
/// Routes requests to specific method handlers or returns MethodNotFound
/// for unrecognized methods.
fn handle_request(req: &McpRequest) -> Result<(), IoError> {
    match &req.method {
        RequestMethod::Initialize(params) => handle_initialize(&req.id, params),
        RequestMethod::Ping => handle_ping(&req.id),
        _ => {
            // All other methods are unknown to the lifecycle handler
            // This is the terminal handler, so return MethodNotFound
            let error = McpError {
                id: Some(req.id.clone()),
                code: ErrorCode::MethodNotFound,
                message: "Method not found".to_string(),
                data: None,
            };
            write_error(error)
        }
    }
}

/// Handle the initialize request.
///
/// Performs the MCP handshake by:
/// 1. Building server information
/// 2. Discovering registered capabilities from the context
/// 3. Echoing the client's protocol version
/// 4. Sending the initialization response
fn handle_initialize(
    id: &crate::bindings::wasmcp::mcp::protocol::Id,
    params: &InitializeParams,
) -> Result<(), IoError> {
    // Build server info
    let server_info = Implementation {
        name: "wasmcp-server".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        title: Some("WebAssembly Component Model Context Protocol Server".to_string()),
    };

    // Dynamically detect capabilities from context
    // Capabilities are registered by feature handlers using context::register_capability()
    let capabilities = build_server_capabilities();

    // Echo back the client's requested protocol version
    // We support all versions defined in the protocol enum
    let protocol_version = params.protocol_version;

    // Build the initialization result
    let result = InitializeResult {
        server_info,
        capabilities,
        protocol_version,
        options: Some(InitializeResultOptions {
            instructions: Some(
                "MCP server implemented using WebAssembly Component Model".to_string(),
            ),
            meta: None,
        }),
    };

    // Send the response using the lifecycle-response interface
    write_initialization(id.clone(), result)
}

/// Handle the ping request.
///
/// Responds with an empty successful result to indicate the server is alive.
fn handle_ping(id: &crate::bindings::wasmcp::mcp::protocol::Id) -> Result<(), IoError> {
    write_pong(id.clone())
}

/// Build server capabilities by discovering registered capabilities.
///
/// This function scans for capabilities that were registered by middleware
/// components in the handler chain. Each feature handler registers its
/// capabilities, and this function aggregates them into a complete
/// ServerCapabilities structure.
///
/// ## Implementation Note
///
/// Currently, we're transitioning from the old string-based context storage
/// to the new ServerCapability enum-based registration. For now, we return
/// None for all capabilities, but feature handlers can register them via
/// `context::register_capability()` and they will be properly included
/// once we implement the aggregation logic.
fn build_server_capabilities() -> ServerCapabilities {
    // TODO: Implement capability aggregation from registered ServerCapability values
    // For now, return empty capabilities
    // Future implementation will scan context for registered capabilities

    ServerCapabilities {
        tools: None,
        prompts: None,
        resources: None,
        completions: None,
        logging: None,
        experimental: None,
    }
}
