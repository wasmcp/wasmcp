//! Initialize handler component for the Model Context Protocol (MCP).
//!
//! This is the terminal handler in the MCP chain that handles initialize requests.
//! All MCP servers must have an initialize handler as it's required by the protocol.

#[rustfmt::skip]
#[allow(clippy::all)]
#[allow(dead_code)]
#[allow(unused_imports)]
#[allow(non_snake_case)]
mod bindings;

use bindings::exports::wasmcp::mcp::incoming_handler::{Guest, OutputStream, Request};
use bindings::wasmcp::mcp::initialize_result::{
    InitializeResult, InitializeResultOptions, ServerCapabilities,
};
use bindings::wasmcp::mcp::request::Feature;
use bindings::wasmcp::mcp::types::{Implementation, ProtocolVersion};

pub struct Component;

/// Handles incoming requests - processes initialize requests and returns
/// method not found error for all other requests.
impl Guest for Component {
    fn handle(request: Request, output: OutputStream) {
        // Get the feature from the request to check if it's an initialize request
        let feature = request.feature();

        // Check if this is an initialize request
        if matches!(feature, Feature::Initialize) {
            // Handle initialize request directly
            handle_initialize_request(request, output);
        } else {
            // Return method not found error for unhandled requests
            write_method_not_found_error(request, output, feature);
        }
    }
}

/// Handles the initialize request by writing the appropriate response.
fn handle_initialize_request(request: Request, output: OutputStream) {
    // Get request ID for the response
    let id = request.id();

    // Extract protocol version from params if available
    let protocol_version = match request.params() {
        Ok(bindings::wasmcp::mcp::request::Params::Initialize(init_params)) => {
            init_params.protocol_version
        }
        _ => ProtocolVersion::V20250618,
    };

    // Create server info
    let server_info = Implementation {
        name: "MCP Server".to_string(),
        title: Some(
            "MCP server implemented with [wasmcp](https://github.com/wasmcp/wasmcp)".to_string(),
        ),
        version: "0.1.0".to_string(),
    };

    // Get server capabilities that were registered by middleware
    let capabilities = request
        .get_capabilities()
        .unwrap_or(None)
        .unwrap_or_else(ServerCapabilities::empty);

    // Create optional result info
    let options = Some(InitializeResultOptions {
        instructions: Some(
            "This is the base MCP handler that handles initialize requests.".to_string(),
        ),
        meta: None,
    });

    // Create the initialize result
    let result = InitializeResult {
        server_info,
        capabilities,
        protocol_version,
        options,
    };

    // Use the initialize-writer to send the complete response with ID
    let _ = bindings::wasmcp::mcp::initialize_result::write(&id, output, &result);
}

/// Writes a JSON-RPC error response for method not found
fn write_method_not_found_error(request: Request, output: OutputStream, feature: Feature) {
    let id = request.id();

    // Convert feature to method name for error message
    let method = match feature {
        Feature::Initialize => "initialize",
        Feature::Tools => "tools/*",
        Feature::Resources => "resources/*",
        Feature::Prompts => "prompts/*",
        Feature::Completion => "completion/complete",
    };

    // Write JSON-RPC error response
    let error_response = format!(
        r#"{{"jsonrpc":"2.0","id":{},"error":{{"code":-32601,"message":"Method not found: {}"}}}}"#,
        match id {
            bindings::wasmcp::mcp::request::Id::Number(n) => n.to_string(),
            bindings::wasmcp::mcp::request::Id::String(s) =>
                serde_json::to_string(&s).unwrap_or_else(|_| r#""""#.to_string()),
        },
        method
    );

    let _ = output.blocking_write_and_flush(error_response.as_bytes());
}

bindings::export!(Component with_types_in bindings);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_component_exists() {
        // Basic test to ensure the component compiles
        let _component = Component;
    }
}
