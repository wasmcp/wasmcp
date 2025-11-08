//! Message framing configuration for different transport types

use crate::bindings::wasmcp::mcp_v20250618::server_io::{MessageFrame, ReadLimit};

/// Maximum size for HTTP request bodies (10MB)
const HTTP_MAX_REQUEST_SIZE: u64 = 10 * 1024 * 1024;

/// Plain JSON framing configuration (no prefix/suffix)
///
/// Used for parsing incoming HTTP POST requests, which contain plain JSON
pub fn plain_json_frame() -> MessageFrame {
    MessageFrame {
        prefix: vec![],
        suffix: vec![],
    }
}

/// HTTP SSE framing configuration
///
/// Messages are framed as Server-Sent Events:
/// - Prefix: "data: "
/// - Suffix: "\n\n"
///
/// Used for writing SSE responses
pub fn http_sse_frame() -> MessageFrame {
    MessageFrame {
        prefix: b"data: ".to_vec(),
        suffix: b"\n\n".to_vec(),
    }
}

/// HTTP read limit configuration
///
/// For HTTP, we read the entire request body up to a maximum size
pub fn http_read_limit() -> ReadLimit {
    ReadLimit::MaxBytes(HTTP_MAX_REQUEST_SIZE)
}

/// Stdio newline-delimited JSON framing configuration
///
/// Messages are newline-delimited:
/// - Prefix: (none)
/// - Suffix: "\n"
pub fn stdio_frame() -> MessageFrame {
    MessageFrame {
        prefix: vec![],
        suffix: b"\n".to_vec(),
    }
}

/// Stdio read limit configuration
///
/// For stdio, we read until newline delimiter
pub fn stdio_read_limit() -> ReadLimit {
    ReadLimit::Delimiter(vec![b'\n'])
}
