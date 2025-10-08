//! Error writer implementation for HTTP/SSE transport.
//!
//! Handles JSON-RPC error responses with proper error codes and messages.

use crate::bindings::exports::wasmcp::mcp::error_writer::Guest;
use crate::bindings::wasi::io::streams::{OutputStream, StreamError};
use crate::bindings::wasmcp::mcp::protocol::{Error, Id, ErrorCode};
use crate::utils::{build_jsonrpc_error, write_message};

pub struct ErrorWriter;

impl Guest for ErrorWriter {
    fn send(id: Id, out: OutputStream, error: Error) -> Result<(), StreamError> {
        // Convert error code enum to JSON-RPC error code integer
        let error_code = match error.code {
            ErrorCode::ParseError => -32700,
            ErrorCode::InvalidRequest => -32600,
            ErrorCode::MethodNotFound => -32601,
            ErrorCode::InvalidParams => -32602,
            ErrorCode::InternalError => -32603,
        };

        // Format the error data field if present (it's already a JSON string)
        let data_json = error.data.as_deref();

        // Build the complete error response
        let response = build_jsonrpc_error(&id, error_code, &error.message, data_json);

        // Write as SSE message
        write_message(&out, &response)
    }
}