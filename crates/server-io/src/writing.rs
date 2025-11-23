//! Stream writing functions with buffering support
//!
//! Provides buffered and streaming write modes:
//! - Buffered mode (plain JSON): Accumulate all writes in memory, flush at end
//! - Streaming mode (SSE/stdio): Write immediately with async yielding

use crate::bindings::exports::wasmcp::mcp_v20250618::server_io::{IoError, MessageFrame};
use crate::bindings::wasi::io::streams::OutputStream;
use std::cell::RefCell;

/// Thread-local buffer for accumulating writes in buffered mode (plain JSON)
thread_local! {
    pub(crate) static BUFFER: RefCell<Vec<u8>> = RefCell::new(Vec::new());
}

/// Determine if frame indicates buffering mode
///
/// Plain JSON (empty prefix/suffix) requires buffering for atomic HTTP response.
/// SSE and stdio (with framing) use immediate writes.
fn should_buffer(frame: &MessageFrame) -> bool {
    frame.prefix.is_empty() && frame.suffix.is_empty()
}

/// Determine if frame indicates notification suppression
///
/// Per MCP spec: plain JSON mode only sends final response, SSE/stdio allow notifications.
pub fn should_suppress_notifications(frame: &MessageFrame) -> bool {
    frame.prefix.is_empty() && frame.suffix.is_empty()
}

/// Write bytes to output stream with async yielding pattern
///
/// Mimics Spin SDK's streaming pattern to avoid budget exhaustion:
/// 1. Write data incrementally based on check_write() capacity
/// 2. Flush after writing complete chunk
/// 3. Subscribe to pollable to yield to async executor
///
/// Frame determines buffering: plain JSON buffers, SSE/stdio stream immediately.
pub fn write_bytes(
    stream: &OutputStream,
    data: &[u8],
    frame: &MessageFrame,
) -> Result<(), IoError> {
    // Check if frame indicates buffering (plain JSON)
    if should_buffer(frame) {
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
