//! Transport component for the Model Context Protocol (MCP)
//!
//! This transport component provides orchestration for MCP servers:
//! - Delegates I/O parsing/serialization to server-io component
//! - Manages session lifecycle via session-manager (HTTP only)
//! - Coordinates with middleware via server-handler
//! - Handles transport-level MCP methods (initialize, ping, logging/setLevel)
//!
//! Exports both HTTP and CLI interfaces - runtime imports what it needs

mod bindings {
    wit_bindgen::generate!({
        world: "transport",
        generate_all,
    });
}

mod common;
mod config;
mod error;
mod http;
mod stdio;

bindings::export!(Component with_types_in bindings);

struct Component;

// Export HTTP incoming-handler interface
impl bindings::exports::wasi::http::incoming_handler::Guest for Component {
    fn handle(
        request: bindings::wasi::http::types::IncomingRequest,
        response_out: bindings::wasi::http::types::ResponseOutparam,
    ) {
        http::HttpTransportGuest::handle(request, response_out)
    }
}

// Export CLI run interface
impl bindings::exports::wasi::cli::run::Guest for Component {
    fn run() -> Result<(), ()> {
        stdio::StdioTransportGuest::run()
    }
}
