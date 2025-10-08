//! Lifecycle handler for the Model Context Protocol (MCP)
//!
//! This component is the **terminal handler** in the MCP request processing chain.
//! It handles core lifecycle methods that establish and maintain MCP connections:
//!
//! - `initialize` - First request to establish the MCP connection
//! - `ping` - Health check / keep-alive
//! - Unknown methods - Returns MethodNotFound error
//!
//! ## Capability Detection
//!
//! The lifecycle handler dynamically builds the server capabilities response
//! by scanning the context store for capability registrations from upstream middleware.
//!
//! Middleware components register capabilities using the system namespace:
//! ```rust
//! ctx.set("wasmcp:capability:tools", r#"{"listChanged":true}"#);
//! ```
//!
//! The initialize handler reads all `wasmcp:capability:*` keys and constructs
//! the `ServerCapabilities` structure automatically. This ensures the initialization
//! response always accurately reflects the actual capabilities of the composed server.

mod bindings {
    wit_bindgen::generate!({
        world: "lifecycle-handler",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp::incoming_handler::Guest;
use bindings::wasi::io::streams::OutputStream;
use bindings::wasmcp::mcp::context::Context;
use bindings::wasmcp::mcp::protocol::*;

// Import the writers
use bindings::wasmcp::mcp::empty_writer;
use bindings::wasmcp::mcp::error_writer;
use bindings::wasmcp::mcp::initialize_writer;

use serde_json::Value;
use std::collections::HashMap;

// System namespace constants
const WASMCP_CAPABILITY_PREFIX: &str = "wasmcp:capability:";

struct Component;

impl Guest for Component {
    fn handle(ctx: Context, out: OutputStream) {
        let message = ctx.data();

        match message {
            JsonrpcObject::Request(req) => handle_request(ctx, req, out),
            JsonrpcObject::Notification(_) => {
                // Lifecycle handler doesn't process notifications
                // They should have been handled by middleware or can be ignored
            }
            JsonrpcObject::Result(_) | JsonrpcObject::Error(_) => {
                // Results and errors shouldn't reach a handler
                // These are responses, not requests
            }
        }
    }
}

/// Route requests to the appropriate handler
fn handle_request(ctx: Context, request: Request, out: OutputStream) {
    match request.method {
        RequestMethod::Initialize(params) => {
            handle_initialize(&ctx, request.id, params, out)
        }
        RequestMethod::Ping => {
            handle_ping(request.id, out)
        }
        _ => {
            // All other methods are unknown to the lifecycle handler
            // This is the terminal handler, so return MethodNotFound
            handle_unknown_method(request.id, out)
        }
    }
}

/// Handle the initialize request
///
/// This builds the server capabilities by scanning the context store for
/// capability registrations from upstream middleware components.
fn handle_initialize(
    ctx: &Context,
    id: Id,
    params: InitializeParams,
    out: OutputStream,
) {
    // Build server info
    let server_info = Implementation {
        name: "wasmcp-server".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        title: Some("WASMCP MCP Server".to_string()),
    };

    // Dynamically detect capabilities from context
    let capabilities = build_server_capabilities(ctx);

    // Use the client's requested protocol version
    // We support all versions, so echo back what they requested
    let protocol_version = params.protocol_version;

    // Build the initialization result
    let result = InitializeResult {
        server_info,
        capabilities,
        protocol_version,
        options: None, // Could add instructions here in the future
    };

    // Send the response
    let _ = initialize_writer::send(&id, out, &result);
}

/// Handle the ping request
///
/// Ping is a simple health check that returns an empty result.
fn handle_ping(id: Id, out: OutputStream) {
    let _ = empty_writer::send(&id, out);
}

/// Handle unknown method requests
///
/// Since this is the terminal handler, any method we don't recognize
/// should return a MethodNotFound error.
fn handle_unknown_method(id: Id, out: OutputStream) {
    let error = Error {
        id: Some(id.clone()),
        code: ErrorCode::MethodNotFound,
        message: "Method not found".to_string(),
        data: None,
    };

    let _ = error_writer::send(&id, out, &error);
}

/// Build server capabilities by scanning context for capability registrations
///
/// Middleware components register their capabilities using keys like:
/// - `wasmcp:capability:tools`
/// - `wasmcp:capability:prompts`
/// - `wasmcp:capability:resources`
///
/// The values are JSON strings with capability options.
fn build_server_capabilities(ctx: &Context) -> ServerCapabilities {
    ServerCapabilities {
        tools: get_list_changed_capability(ctx, "tools"),
        prompts: get_list_changed_capability(ctx, "prompts"),
        resources: get_resources_capability(ctx),
        completions: get_simple_capability(ctx, "completions"),
        logging: get_simple_capability(ctx, "logging"),
        experimental: get_experimental_capability(ctx),
    }
}

/// Get a capability with listChanged support (tools, prompts)
fn get_list_changed_capability(
    ctx: &Context,
    name: &str,
) -> Option<ListChangedCapabilityOption> {
    let key = format!("{}{}", WASMCP_CAPABILITY_PREFIX, name);
    let json_str = ctx.get(&key)?;

    // Parse the JSON to extract listChanged option
    serde_json::from_str::<HashMap<String, Value>>(&json_str)
        .ok()
        .map(|map| ListChangedCapabilityOption {
            list_changed: map
                .get("listChanged")
                .and_then(|v| v.as_bool()),
        })
}

/// Get resources capability with subscribe and listChanged support
fn get_resources_capability(ctx: &Context) -> Option<ResourcesListChangedCapabilityOption> {
    let key = format!("{}resources", WASMCP_CAPABILITY_PREFIX);
    let json_str = ctx.get(&key)?;

    // Parse the JSON to extract subscribe and listChanged options
    serde_json::from_str::<HashMap<String, Value>>(&json_str)
        .ok()
        .map(|map| ResourcesListChangedCapabilityOption {
            subscribe: map.get("subscribe").and_then(|v| v.as_bool()),
            list_changed: map.get("listChanged").and_then(|v| v.as_bool()),
        })
}

/// Get a simple capability that just needs the JSON string (completions, logging)
fn get_simple_capability(ctx: &Context, name: &str) -> Option<String> {
    let key = format!("{}{}", WASMCP_CAPABILITY_PREFIX, name);
    ctx.get(&key)
}

/// Get experimental capabilities
fn get_experimental_capability(ctx: &Context) -> Option<Vec<(String, String)>> {
    let key = format!("{}experimental", WASMCP_CAPABILITY_PREFIX);
    let json_str = ctx.get(&key)?;

    // Parse as array of [name, json] tuples
    serde_json::from_str::<Vec<(String, Value)>>(&json_str)
        .ok()
        .map(|vec| {
            vec.into_iter()
                .map(|(name, value)| (name, value.to_string()))
                .collect()
        })
}

bindings::export!(Component with_types_in bindings);
