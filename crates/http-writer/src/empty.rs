//! Empty writer implementation for HTTP/SSE transport.
//!
//! Used for responses that have no content, such as ping responses.

use crate::bindings::exports::wasmcp::mcp::empty_writer::Guest;
use crate::bindings::wasi::io::streams::{OutputStream, StreamError};
use crate::bindings::wasmcp::mcp::protocol::Id;
use crate::utils::{build_jsonrpc_response, write_sse_message};

pub struct EmptyWriter;

impl Guest for EmptyWriter {
    fn send(id: Id, out: OutputStream) -> Result<(), StreamError> {
        // Empty result is just an empty JSON object
        let response = build_jsonrpc_response(&id, "{}");
        write_sse_message(&out, &response)
    }
}