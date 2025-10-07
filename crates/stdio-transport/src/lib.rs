//! Stdio transport component for the Model Context Protocol (MCP)
//!
//! This component provides stdio-based transport for MCP, handling communication
//! over standard input and output streams with newline-delimited JSON-RPC messages.
//!
//! The component:
//! - Reads newline-delimited JSON-RPC messages from stdin
//! - Parses each message into a context object
//! - Adds an implicit session ID (stdio has a single persistent session)
//! - Forwards contexts to the handler chain
//! - Handlers write responses directly to stdout
//! - Continues processing until stdin is closed
//!
//! This implementation follows the stdio transport specification where:
//! - Messages are delimited by newlines and must not contain embedded newlines
//! - The server reads from stdin and writes to stdout
//! - stderr can be used for logging

mod bindings {
    wit_bindgen::generate!({
        world: "stdio-transport",
        generate_all,
    });
}

use bindings::exports::wasi::cli::run::Guest;
use bindings::wasi::cli::{stderr, stdin, stdout};
use bindings::wasi::io::streams::{InputStream, StreamError};
use bindings::wasmcp::mcp::context::Context;
use bindings::wasmcp::mcp::incoming_handler::handle;

struct Component;

impl Guest for Component {
    fn run() -> Result<(), ()> {
        // Generate a session ID for this stdio connection
        // For stdio, there's implicitly a single persistent session
        let session_id = generate_session_id();

        // Get the streams once
        let input_stream = stdin::get_stdin();
        let error_stream = stderr::get_stderr();

        // Log startup
        let startup_msg = format!("[stdio-transport] Starting MCP server (session: {})\n", session_id);
        let _ = error_stream.blocking_write_and_flush(startup_msg.as_bytes());

        // MCP over stdio is a persistent connection that handles multiple messages
        loop {
            // Read a line from stdin (newline-delimited JSON-RPC)
            let line = match read_line(&input_stream) {
                Ok(line) => line,
                Err(StreamError::Closed) => {
                    // stdin closed - this is normal shutdown
                    let shutdown_msg = "[stdio-transport] Stdin closed, shutting down\n";
                    let _ = error_stream.blocking_write_and_flush(shutdown_msg.as_bytes());
                    return Ok(());
                }
                Err(e) => {
                    // Other error - log and continue (might be temporary)
                    let error_msg = format!("[stdio-transport] Error reading from stdin: {:?}\n", e);
                    let _ = error_stream.blocking_write_and_flush(error_msg.as_bytes());
                    continue;
                }
            };

            // Skip empty lines
            if line.is_empty() {
                continue;
            }

            // Parse the line as a JSON-RPC message
            let context = match Context::from_bytes(&line) {
                Ok(ctx) => ctx,
                Err(e) => {
                    // Log parse error to stderr and send error response to stdout
                    let error_msg = format!("[stdio-transport] Failed to parse JSON-RPC: {}\n", e);
                    let _ = error_stream.blocking_write_and_flush(error_msg.as_bytes());

                    // Send JSON-RPC parse error to stdout
                    // Properly escape the error message for JSON
                    let escaped_error = e.replace('\\', "\\\\")
                        .replace('"', "\\\"")
                        .replace('\n', "\\n")
                        .replace('\r', "\\r")
                        .replace('\t', "\\t");

                    let output_stream = stdout::get_stdout();
                    let parse_error = format!(
                        "{{\"jsonrpc\":\"2.0\",\"error\":{{\"code\":-32700,\"message\":\"Parse error: {}\"}},\"id\":null}}\n",
                        escaped_error
                    );
                    let _ = output_stream.blocking_write_and_flush(parse_error.as_bytes());
                    continue;
                }
            };

            // Add session ID to the context
            // This allows handlers to use session features even in stdio mode
            context.set(&"session-id", &session_id);

            // Get a fresh stdout stream for this message
            // (Each message gets its own output stream to ensure proper flushing)
            let output_stream = stdout::get_stdout();

            // Forward to the handler chain
            // The handler will write the JSON-RPC response directly to stdout
            // Per MCP spec: handlers MUST include newline delimiters in their responses
            handle(context, output_stream);

            // IMPORTANT: We don't add a newline here - the handler is responsible
            // for including the newline delimiter as part of its JSON-RPC response
        }
    }
}

/// Read a line from the input stream (reads until newline or EOF)
fn read_line(stream: &InputStream) -> Result<Vec<u8>, StreamError> {
    let mut line = Vec::new();

    loop {
        // Read one byte at a time to detect newlines
        // Using blocking_read to wait for input
        let bytes = stream.blocking_read(1)?;

        if bytes.is_empty() {
            // End of stream
            if line.is_empty() {
                return Err(StreamError::Closed);
            } else {
                // Return what we have (line without newline at EOF)
                return Ok(line);
            }
        }

        let byte = bytes[0];

        if byte == b'\n' {
            // Found newline - return the line (without the newline)
            return Ok(line);
        }

        if byte == b'\r' {
            // Skip carriage returns (handle both \n and \r\n line endings)
            continue;
        }

        // Add byte to line
        line.push(byte);
    }
}

/// Generate a session ID for the stdio connection
fn generate_session_id() -> String {
    // For stdio, we use a fixed session ID since there's only one stdio connection
    // per process instance. The session persists for the lifetime of the process.
    "stdio-session".to_string()
}

bindings::export!(Component with_types_in bindings);