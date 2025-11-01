//! Stream reading utilities for handling input-stream data
//!
//! This module handles reading text-stream and blob-stream variants from WIT types
//! with bounded memory usage for edge worker deployments.

use crate::bindings::wasi::io::streams::{InputStream, StreamError};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

/// Configuration for stream reading behavior
pub struct StreamConfig {
    /// Maximum total bytes to read (prevents OOM)
    pub max_size: u64,
    /// Chunk size for reading (balances memory vs syscalls)
    pub chunk_size: u64,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            // 50MB limit - allows 1MB worker to handle large resources with headroom
            max_size: 50 * 1024 * 1024,
            // 64KB chunks - good balance for edge workers
            chunk_size: 64 * 1024,
        }
    }
}

/// Read an input stream to a string (for text-stream variant)
///
/// Reads in chunks to avoid buffering entire stream at once.
/// Returns error if stream exceeds max_size.
pub fn read_text_stream(stream: &InputStream, config: &StreamConfig) -> Result<String, String> {
    let bytes = read_bytes_chunked(stream, config)?;
    String::from_utf8(bytes).map_err(|e| format!("Invalid UTF-8 in text stream: {}", e))
}

/// Read an input stream to base64-encoded string (for blob-stream variant)
///
/// Reads stream in chunks then encodes to base64.
/// Returns error if stream exceeds max_size.
pub fn read_blob_stream(stream: &InputStream, config: &StreamConfig) -> Result<String, String> {
    let bytes = read_bytes_chunked(stream, config)?;
    Ok(BASE64.encode(&bytes))
}

/// Read an input stream in chunks with size limit
pub fn read_bytes_chunked(stream: &InputStream, config: &StreamConfig) -> Result<Vec<u8>, String> {
    let mut buffer = Vec::new();
    let mut total_read = 0u64;

    loop {
        // Calculate how much we can read in this chunk
        let remaining = config.max_size.saturating_sub(total_read);
        if remaining == 0 {
            return Err(format!(
                "Stream exceeds maximum size of {} bytes",
                config.max_size
            ));
        }

        let to_read = remaining.min(config.chunk_size);

        // Read chunk (blocking to ensure we get data or EOF)
        match stream.blocking_read(to_read) {
            Ok(chunk) => {
                if chunk.is_empty() {
                    // EOF reached
                    break;
                }
                total_read += chunk.len() as u64;
                buffer.extend_from_slice(&chunk);
            }
            Err(StreamError::Closed) => {
                // Stream closed, treat as EOF
                break;
            }
            Err(e) => {
                return Err(format!("Stream read error: {:?}", e));
            }
        }
    }

    Ok(buffer)
}
