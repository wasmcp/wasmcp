//! Stream writing functions with buffering support
//!
//! Provides buffered and streaming write modes:
//! - Buffered mode (JSON): Accumulate all writes in memory, flush at end
//! - Streaming mode (SSE/stdio): Write immediately with async yielding

use crate::bindings::exports::wasmcp::mcp_v20250618::server_io::IoError;
use crate::bindings::wasi::io::streams::OutputStream;
use std::cell::RefCell;

/// Thread-local buffer for accumulating writes in buffered mode (JSON)
thread_local! {
    pub(crate) static BUFFER: RefCell<Vec<u8>> = RefCell::new(Vec::new());
}

/// Server mode - mirrors transport crate's ServerMode enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ServerMode {
    Sse,
    Json,
    Stdio,
}

/// Get the current server mode from MCP_SERVER_MODE env var
/// Parsing logic matches transport/src/config.rs
fn get_server_mode() -> ServerMode {
    std::env::var("MCP_SERVER_MODE")
        .ok()
        .and_then(|v| match v.to_lowercase().as_str() {
            "sse" => Some(ServerMode::Sse),
            "json" => Some(ServerMode::Json),
            "stdio" => Some(ServerMode::Stdio),
            _ => None,
        })
        .unwrap_or(ServerMode::Sse) // Default to SSE mode (immediate writes, allows notifications)
}

/// Check if we're in buffer mode
pub fn is_buffer_mode() -> bool {
    // Buffer for: json mode
    // Don't buffer for: sse and stdio modes (immediate writes)
    matches!(get_server_mode(), ServerMode::Json)
}

/// Check if we're in JSON mode (suppresses notifications)
pub fn is_json_mode() -> bool {
    matches!(get_server_mode(), ServerMode::Json)
}

/// Write bytes to output stream with async yielding pattern
///
/// Mimics Spin SDK's streaming pattern to avoid budget exhaustion:
/// 1. Write data incrementally based on check_write() capacity
/// 2. Flush after writing complete chunk
/// 3. Subscribe to pollable to yield to async executor
pub fn write_bytes(stream: &OutputStream, data: &[u8]) -> Result<(), IoError> {
    // Check if we're in buffer mode
    if is_buffer_mode() {
        BUFFER.with(|buf| {
            buf.borrow_mut().extend_from_slice(data);
        });
        return Ok(());
    }

    // Streaming mode with async yielding pattern

    let mut offset = 0;

    // Write loop: incrementally write based on available capacity
    while offset < data.len() {
        match stream.check_write() {
            Ok(0) => {
                // No capacity - subscribe to pollable to yield to async executor
                let pollable = stream.subscribe();
                pollable.block(); // This yields to async executor until writable
                continue;
            }
            Ok(count) => {
                // Write up to available capacity
                let chunk_size = (data.len() - offset).min(count as usize);

                stream
                    .write(&data[offset..offset + chunk_size])
                    .map_err(|e| {
                        eprintln!("[SERVER-IO] ⚠️  Write failed: {:?}", e);
                        IoError::Stream(e)
                    })?;

                offset += chunk_size;
            }
            Err(e) => {
                eprintln!("[SERVER-IO] ⚠️  check_write failed: {:?}", e);
                return Err(IoError::Stream(e));
            }
        }
    }

    // Flush after writing complete message (like Spin SDK does)
    stream.flush().map_err(|e| {
        eprintln!("[SERVER-IO] ⚠️  Flush failed: {:?}", e);
        IoError::Stream(e)
    })?;

    Ok(())
}
