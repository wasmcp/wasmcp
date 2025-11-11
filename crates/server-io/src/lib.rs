//! Universal Server I/O Implementation
//!
//! Implements the server-io interface with transport-agnostic framing support.
//! Handles bidirectional JSON-RPC message exchange with runtime-specified framing.
//!
//! Architecture:
//! - Transport passes frame on each parse/send call (explicit, no state)
//! - Single parse_message() function replaces 4 parse_* functions
//! - Single send_message() function replaces 4 write_* functions
//! - No compile-time feature flags for transport selection
//!
//! This component provides full spec-compliant MCP 2025-06-18 message handling
//! for any transport (HTTP SSE, stdio, custom) via runtime framing parameters.

#![allow(warnings)]

mod bindings {
    wit_bindgen::generate!({
        world: "server-io",
        generate_all,
    });
}

mod framing;
mod message_dispatch;
mod parser;
mod reading;
mod serialization;
mod serializer;
mod stream_reader;
mod writing;

#[cfg(test)]
mod tests;

use bindings::exports::wasmcp::mcp_v20250618::server_io::{
    Guest, IoError, MessageFrame, ReadLimit,
};
use bindings::wasi::io::streams::{InputStream, OutputStream};
use bindings::wasmcp::mcp_v20250618::mcp::*;

struct ServerIo;

impl Guest for ServerIo {
    /// Parse an incoming message from the client
    ///
    /// Reads from the input stream according to the read limit, strips framing,
    /// and parses the JSON-RPC message into a client-message variant.
    fn parse_message(
        input: &InputStream,
        limit: ReadLimit,
        frame: MessageFrame,
    ) -> Result<ClientMessage, IoError> {
        // Read raw bytes based on limit
        let raw_bytes = match limit {
            ReadLimit::Delimiter(delim) => reading::read_until_delimiter(input, &delim).map_err(|e| {
                // Clear READ_BUFFER on error to prevent stale data
                reading::READ_BUFFER.with(|rb| rb.borrow_mut().clear());
                e
            })?,
            ReadLimit::MaxBytes(max) => reading::read_max_bytes(input, max)?,
        };

        // Strip framing prefix and suffix
        let json_bytes = framing::strip_framing(&raw_bytes, &frame)?;

        // Convert to UTF-8 string
        let json_str = String::from_utf8(json_bytes)
            .map_err(|e| IoError::InvalidJsonrpc(format!("Invalid UTF-8: {}", e)))?;

        // Parse JSON
        let json: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| IoError::InvalidJsonrpc(format!("Invalid JSON: {}", e)))?;

        // Determine message type and parse
        message_dispatch::parse_client_message(&json)
    }

    /// Send a message to the client
    ///
    /// Serializes the server-message variant to JSON-RPC format, applies framing,
    /// and writes to the output stream.
    fn send_message(
        output: &OutputStream,
        message: ServerMessage,
        frame: MessageFrame,
    ) -> Result<(), IoError> {
        // In JSON mode, suppress notifications (only final response is sent)
        // Per MCP spec 2025-06-18: SSE allows multiple messages, JSON mode only sends final response
        if writing::is_json_mode() {
            if let ServerMessage::Notification(_) = message {
                return Ok(());
            }
        }

        // Get framed bytes
        let framed = framing::serialize_message_to_bytes(message, &frame)?;

        // Write to stream
        writing::write_bytes(output, &framed)?;
        Ok(())
    }

    /// Flush buffered data to stream (for buffered mode)
    ///
    /// In buffered mode (MCP_SERVER_MODE=json), all writes accumulate in memory.
    /// This function writes the entire buffer to the stream in one blocking operation.
    fn flush_buffer(output: &OutputStream) -> Result<(), IoError> {
        if !writing::is_buffer_mode() {
            return Ok(());
        }

        let data = writing::BUFFER.with(|buf| {
            let borrowed = buf.borrow();
            borrowed.clone()
        });

        output.blocking_write_and_flush(&data).map_err(|e| {
            eprintln!("[SERVER-IO] blocking_write_and_flush failed: {:?}", e);
            IoError::Stream(e)
        })?;

        // Clear buffer after successful write
        writing::BUFFER.with(|buf| {
            buf.borrow_mut().clear();
        });

        Ok(())
    }
}

bindings::export!(ServerIo with_types_in bindings);
