//! Stream reading functions with delimiter and boundary-aware searching
//!
//! Provides optimized reading for both single-byte (stdio '\n') and
//! multi-byte (SSE '\n\n') delimiters with proper boundary handling.

use crate::bindings::exports::wasmcp::mcp_v20250618::server_io::IoError;
use crate::bindings::wasi::io::streams::InputStream;
use crate::stream_reader::{self, StreamConfig};
use std::cell::RefCell;

/// Thread-local buffer for storing data read beyond delimiter
///
/// When multiple newline-delimited messages arrive in one chunk,
/// this stores the remaining bytes for the next read call.
thread_local! {
    pub(crate) static READ_BUFFER: RefCell<Vec<u8>> = RefCell::new(Vec::new());
}

/// Read from stream until delimiter is found
///
/// Uses optimized fast path for single-byte delimiters (stdio),
/// and boundary-safe search for multi-byte delimiters (SSE).
pub fn read_until_delimiter(stream: &InputStream, delimiter: &[u8]) -> Result<Vec<u8>, IoError> {
    if delimiter.len() == 1 {
        // Fast path: single-byte delimiter (e.g., stdio '\n')
        read_until_byte(stream, delimiter[0])
    } else {
        // Generic path: multi-byte delimiter with boundary handling (e.g., SSE '\n\n')
        read_until_multibyte_delimiter(stream, delimiter)
    }
}

/// Fast path for single-byte delimiters
///
/// Iterates byte-by-byte, naturally handling chunk boundaries.
fn read_until_byte(stream: &InputStream, delimiter: u8) -> Result<Vec<u8>, IoError> {
    const MAX_SIZE: usize = 10 * 1024 * 1024; // 10MB max
    const CHUNK_SIZE: usize = 4096; // Read 4KB chunks
    let mut buffer = Vec::new();

    // FIRST: Check if we have buffered data from a previous read
    // This happens when multiple newline-delimited messages arrive in one chunk
    READ_BUFFER.with(|rb| {
        let mut read_buf = rb.borrow_mut();
        if !read_buf.is_empty() {
            buffer.extend_from_slice(&read_buf);
            read_buf.clear();
        }
    });

    // Check if buffered data already contains a complete message
    if let Some(pos) = buffer.iter().position(|&b| b == delimiter) {
        let message = buffer[..=pos].to_vec();
        let remaining = buffer[pos + 1..].to_vec();

        // Save remaining bytes for next call
        if !remaining.is_empty() {
            READ_BUFFER.with(|rb| *rb.borrow_mut() = remaining);
        }

        return Ok(message);
    }

    loop {
        if buffer.len() >= MAX_SIZE {
            return Err(IoError::Unexpected(format!(
                "Message exceeds maximum size of {} bytes",
                MAX_SIZE
            )));
        }

        let chunk = stream.blocking_read(CHUNK_SIZE as u64).map_err(|e| {
            eprintln!("[SERVER-IO] read_until_byte: blocking_read error: {:?}", e);
            IoError::Stream(e)
        })?;

        if chunk.is_empty() {
            // Empty chunk from blocking_read indicates EOF
            if buffer.is_empty() {
                return Err(IoError::Unexpected(
                    "Stream closed before delimiter".to_string(),
                ));
            }
            return Err(IoError::Unexpected("Stream closed mid-message".to_string()));
        }

        // Scan byte-by-byte for delimiter
        for (idx, &byte) in chunk.iter().enumerate() {
            buffer.push(byte);
            if byte == delimiter {
                // Found delimiter - save any remaining bytes from this chunk
                let remaining = &chunk[idx + 1..];
                if !remaining.is_empty() {
                    READ_BUFFER.with(|rb| rb.borrow_mut().extend_from_slice(remaining));
                }
                return Ok(buffer);
            }
        }
    }
}

/// Generic path for multi-byte delimiters with boundary handling
///
/// Searches across chunk boundaries by checking overlapping regions.
/// This prevents missing delimiters split across two read chunks.
///
/// Example: For delimiter "\n\n" with chunks ending at boundary:
///   Chunk 1: "data: {...}\n"
///   Chunk 2: "\ndata: {...}"
///
/// Without boundary handling, each chunk search would fail (only single \n).
/// With boundary handling, we search the overlapping region and find the delimiter.
fn read_until_multibyte_delimiter(
    stream: &InputStream,
    delimiter: &[u8],
) -> Result<Vec<u8>, IoError> {
    const MAX_SIZE: usize = 10 * 1024 * 1024; // 10MB max
    const CHUNK_SIZE: usize = 4096; // Read 4KB chunks
    let mut buffer = Vec::new();

    loop {
        if buffer.len() >= MAX_SIZE {
            return Err(IoError::Unexpected(format!(
                "Message exceeds maximum size of {} bytes",
                MAX_SIZE
            )));
        }

        let chunk = stream
            .blocking_read(CHUNK_SIZE as u64)
            .map_err(|e| IoError::Stream(e))?;

        if chunk.is_empty() {
            if buffer.is_empty() {
                return Err(IoError::Unexpected(
                    "Stream closed before delimiter".to_string(),
                ));
            } else {
                break; // EOF reached
            }
        }

        let chunk_len = chunk.len();
        buffer.extend_from_slice(&chunk);

        // Use boundary-aware search (tested in unit tests)
        if let Some((pos, _found)) = search_with_boundary(&buffer, chunk_len, delimiter) {
            buffer.truncate(pos);
            return Ok(buffer);
        }
    }

    Ok(buffer)
}

/// Read up to max_bytes from stream
pub fn read_max_bytes(stream: &InputStream, max_bytes: u64) -> Result<Vec<u8>, IoError> {
    let config = StreamConfig {
        max_size: max_bytes,
        chunk_size: 4096,
    };

    stream_reader::read_bytes_chunked(stream, &config).map_err(|e| IoError::Unexpected(e))
}

/// Find position of needle in haystack
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }

    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Search for delimiter in buffer with boundary-aware logic
///
/// This function handles the case where a delimiter might be split across
/// chunk boundaries by searching an overlapping region.
///
/// Returns Some((position, found)) where:
/// - position: byte offset of delimiter start in buffer
/// - found: true if delimiter found, false if need more data
///
/// # Arguments
/// * `buffer` - Complete buffer accumulated so far
/// * `new_chunk_len` - Length of the most recently added chunk
/// * `delimiter` - Delimiter bytes to search for
///
/// # Example
/// ```ignore
/// // Buffer: "data: {...}\n" + "\nmore data"
/// //                      ^~^  delimiter "\n\n" spans boundary
/// let buffer = b"data: {...}\n\nmore data";
/// let result = search_with_boundary(buffer, 10, b"\n\n");
/// assert_eq!(result, Some((12, true))); // Found at position 12
/// ```
fn search_with_boundary(
    buffer: &[u8],
    new_chunk_len: usize,
    delimiter: &[u8],
) -> Option<(usize, bool)> {
    if buffer.is_empty() || delimiter.is_empty() {
        return None;
    }

    let buffer_len = buffer.len();
    let delimiter_len = delimiter.len();

    // Calculate overlapping search region
    // We need to check the last (delimiter_len - 1) bytes of previous data
    // plus all of the new chunk, to catch delimiters spanning the boundary
    let search_start = buffer_len.saturating_sub(new_chunk_len + delimiter_len - 1);

    // Search in the overlapping region
    if let Some(relative_pos) = find_subsequence(&buffer[search_start..], delimiter) {
        let absolute_pos = search_start + relative_pos;
        Some((absolute_pos, true))
    } else {
        None
    }
}

// =============================================================================
// TESTS - Boundary-aware delimiter search
// =============================================================================

#[cfg(all(test, not(target_family = "wasm")))]
mod boundary_tests {
    use super::*;

    #[test]
    fn test_find_subsequence_basic() {
        assert_eq!(find_subsequence(b"hello world", b"world"), Some(6));
        assert_eq!(find_subsequence(b"hello world", b"xyz"), None);
        assert_eq!(find_subsequence(b"", b"test"), None);
        assert_eq!(find_subsequence(b"test", b""), Some(0));
    }

    #[test]
    fn test_search_with_boundary_no_split() {
        // Delimiter entirely within new chunk
        let buffer = b"data: {\"test\":\"value\"}\n\n";
        let result = search_with_boundary(buffer, 10, b"\n\n");
        assert!(result.is_some());
        let (pos, found) = result.unwrap();
        assert!(found);
        assert_eq!(&buffer[pos..pos + 2], b"\n\n");
    }

    #[test]
    fn test_search_with_boundary_split_delimiter() {
        // Delimiter split across boundary: first chunk ends with \n, second starts with \n
        let mut buffer = Vec::new();
        buffer.extend_from_slice(b"data: {\"test\":\"value\"}\n"); // First chunk
        let chunk1_len = buffer.len();
        buffer.extend_from_slice(b"\nmore data"); // Second chunk adds the second \n

        let result = search_with_boundary(&buffer, b"\nmore data".len(), b"\n\n");
        assert!(result.is_some());
        let (pos, found) = result.unwrap();
        assert!(found);
        assert_eq!(pos, chunk1_len - 1); // Delimiter starts at last byte of first chunk
        assert_eq!(&buffer[pos..pos + 2], b"\n\n");
    }

    #[test]
    fn test_search_with_boundary_single_byte_delimiter() {
        let buffer = b"hello\nworld";
        let result = search_with_boundary(buffer, 6, b"\n");
        assert!(result.is_some());
        let (pos, found) = result.unwrap();
        assert!(found);
        assert_eq!(pos, 5);
    }

    #[test]
    fn test_search_with_boundary_delimiter_at_exact_boundary() {
        // Chunk 1: "test"
        // Chunk 2: "\n\ndata"
        // Delimiter "\n\n" starts exactly at boundary
        let buffer = b"test\n\ndata";
        let result = search_with_boundary(buffer, 6, b"\n\n"); // "\n\ndata" is new chunk
        assert!(result.is_some());
        let (pos, found) = result.unwrap();
        assert!(found);
        assert_eq!(pos, 4);
        assert_eq!(&buffer[pos..pos + 2], b"\n\n");
    }

    #[test]
    fn test_search_with_boundary_no_delimiter_found() {
        let buffer = b"data without delimiter";
        let result = search_with_boundary(buffer, 10, b"\n\n");
        assert!(result.is_none());
    }

    #[test]
    fn test_search_with_boundary_three_byte_delimiter() {
        // Test with unusual multi-byte delimiter
        let mut buffer = Vec::new();
        buffer.extend_from_slice(b"prefix-AB"); // First chunk ends with "AB"
        buffer.extend_from_slice(b"C-suffix"); // Second chunk starts with "C"

        let result = search_with_boundary(&buffer, 8, b"ABC");
        assert!(result.is_some());
        let (pos, found) = result.unwrap();
        assert!(found);
        assert_eq!(&buffer[pos..pos + 3], b"ABC");
    }

    #[test]
    fn test_search_with_boundary_delimiter_split_three_ways() {
        // Pathological case: delimiter "ABC" split as "A" | "B" | "C" across three chunks
        // This function is called incrementally, so we test two-chunk boundaries
        let mut buffer = Vec::new();
        buffer.extend_from_slice(b"prefix-A"); // First chunk
        buffer.extend_from_slice(b"BC-suffix"); // Second chunk

        let result = search_with_boundary(&buffer, 9, b"ABC");
        assert!(result.is_some());
        let (pos, found) = result.unwrap();
        assert!(found);
        assert_eq!(&buffer[pos..pos + 3], b"ABC");
    }

    #[test]
    fn test_search_with_boundary_empty_buffer() {
        let buffer = b"";
        let result = search_with_boundary(buffer, 0, b"\n\n");
        assert!(result.is_none());
    }

    #[test]
    fn test_search_with_boundary_overlapping_search_region() {
        // Test that we search far enough back
        // Buffer: "xxxx" + "x\n\n" - delimiter at boundary
        let buffer = b"xxxxx\n\n";
        let result = search_with_boundary(buffer, 3, b"\n\n"); // Last 3 bytes: "x\n\n"
        assert!(result.is_some());
        let (pos, found) = result.unwrap();
        assert!(found);
        assert_eq!(pos, 5);
    }
}
