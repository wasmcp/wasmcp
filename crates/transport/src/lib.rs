//! Universal transport component for the Model Context Protocol (MCP)
//!
//! This transport component provides a unified orchestration layer that:
//! - Delegates I/O parsing/serialization to server-io components
//! - Manages session lifecycle via session-manager
//! - Coordinates with middleware via server-handler
//! - Handles transport-level MCP methods (initialize, ping, logging/setLevel)
//!
//! Architecture:
//! - Exports both wasi:http/incoming-handler AND wasi:cli/run
//! - Composition determines which interface is actually used
//! - Works with http-server-io OR stdio-server-io (via server-io interface)
//! - Session support is automatic when session-manager is available

mod bindings {
    wit_bindgen::generate!({
        world: "transport",
        generate_all,
    });
}

// HTTP transport implementation
mod http;

// Stdio transport implementation
mod stdio;

// Common transport logic
mod common;

// Configuration
mod config;

bindings::export!(Component with_types_in bindings);

struct Component;

// Implement HTTP incoming-handler
impl bindings::exports::wasi::http::incoming_handler::Guest for Component {
    fn handle(
        request: bindings::wasi::http::types::IncomingRequest,
        response_out: bindings::wasi::http::types::ResponseOutparam,
    ) {
        http::HttpTransportGuest::handle(request, response_out)
    }
}

// Implement stdio run
impl bindings::exports::wasi::cli::run::Guest for Component {
    fn run() -> Result<(), ()> {
        stdio::StdioTransportGuest::run()
    }
}
