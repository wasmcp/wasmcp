//! Transport component for the Model Context Protocol (MCP)
//!
//! This transport component provides orchestration for MCP servers:
//! - Delegates I/O parsing/serialization to server-io component
//! - Manages session lifecycle via session-manager
//! - Coordinates with middleware via server-handler
//! - Handles transport-level MCP methods (initialize, ping, logging/setLevel)
//!
// =============================================================================
// HTTP Transport Variant (default)
// =============================================================================

#[cfg(not(feature = "stdio"))]
mod bindings {
    wit_bindgen::generate!({
        world: "transport-http",
        generate_all,
    });
}

#[cfg(not(feature = "stdio"))]
mod http;

#[cfg(not(feature = "stdio"))]
mod config;

// =============================================================================
// Stdio Transport Variant (--features stdio)
// =============================================================================

#[cfg(feature = "stdio")]
mod bindings {
    wit_bindgen::generate!({
        path: "wit-stdio",
        world: "transport-stdio",
        generate_all,
    });
}

#[cfg(feature = "stdio")]
mod stdio;

mod common;

bindings::export!(Component with_types_in bindings);

struct Component;

// HTTP variant: export incoming-handler
#[cfg(not(feature = "stdio"))]
impl bindings::exports::wasi::http::incoming_handler::Guest for Component {
    fn handle(
        request: bindings::wasi::http::types::IncomingRequest,
        response_out: bindings::wasi::http::types::ResponseOutparam,
    ) {
        http::HttpTransportGuest::handle(request, response_out)
    }
}

// Stdio variant: export run
#[cfg(feature = "stdio")]
impl bindings::exports::wasi::cli::run::Guest for Component {
    fn run() -> Result<(), ()> {
        stdio::StdioTransportGuest::run()
    }
}
