//! {{ handler_type_capitalized }} middleware for MCP.

#[allow(warnings)]
mod bindings;

use bindings::exports::wasmcp::mcp::incoming_handler::{Guest, OutputStream, Request};
use bindings::wasmcp::mcp::incoming_handler as next_handler;

pub struct Component;

impl Guest for Component {
    fn handle(request: Request, output: OutputStream) {
        // Add your middleware logic here
        // Example: logging, authentication, rate limiting, etc.

        // Forward to next handler
        next_handler::handle(request, output);
    }
}

bindings::export!(Component with_types_in bindings);
