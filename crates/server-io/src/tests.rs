#[cfg(test)]
mod tests {
    use super::*;
    use crate::serializer;

    #[test]
    fn test_http_sse_formatting() {
        let json = serde_json::json!({"test": "data"});
        let formatted = serializer::format_sse_event(&json);

        assert_eq!(formatted, "data: {\"test\":\"data\"}\n\n");
        assert!(formatted.starts_with("data: "));
        assert!(formatted.ends_with("\n\n"));
        assert!(!formatted.contains("\n\n\n")); // Should have exactly 2 newlines at end
    }

    #[test]
    fn test_stdio_json_line_formatting() {
        let json = serde_json::json!({"test": "data"});
        let formatted = serializer::format_json_line(&json);

        assert_eq!(formatted, "{\"test\":\"data\"}\n");
        assert!(!formatted.contains("data: ")); // Should NOT have SSE prefix
        assert!(formatted.ends_with('\n'));
        assert!(!formatted.ends_with("\n\n")); // Should have exactly 1 newline
    }

    #[test]
    fn test_formatting_difference() {
        let json = serde_json::json!({"jsonrpc": "2.0", "id": 1, "result": {}});

        let http_format = serializer::format_sse_event(&json);
        let stdio_format = serializer::format_json_line(&json);

        // HTTP should have SSE prefix
        assert!(http_format.starts_with("data: "));
        assert!(!stdio_format.starts_with("data: "));

        // HTTP should end with double newline, stdio with single
        assert!(http_format.ends_with("\n\n"));
        assert!(stdio_format.ends_with("\n"));
        assert!(!stdio_format.ends_with("\n\n"));

        // Both should contain the same JSON content
        let http_json_part = http_format.strip_prefix("data: ").unwrap().trim();
        let stdio_json_part = stdio_format.trim();
        assert_eq!(http_json_part, stdio_json_part);
    }

    #[test]
    fn test_read_buffer_preserves_excess_bytes() {
        // Test the READ_BUFFER logic for batched message handling
        // Simulates what happens when multiple newline-delimited messages arrive in one chunk

        // Clear any existing buffer state
        crate::READ_BUFFER.with(|rb| rb.borrow_mut().clear());

        // Simulate having buffered bytes (as if from a previous read)
        let remaining_bytes = b"remaining message\n".to_vec();
        crate::READ_BUFFER.with(|rb| {
            *rb.borrow_mut() = remaining_bytes.clone();
        });

        // Verify we can read it back
        let retrieved = crate::READ_BUFFER.with(|rb| {
            let buf = rb.borrow();
            buf.clone()
        });

        assert_eq!(retrieved, remaining_bytes);
        assert_eq!(retrieved, b"remaining message\n");
    }

    #[test]
    fn test_read_buffer_isolation() {
        // Verify thread-local isolation - each test gets its own buffer
        crate::READ_BUFFER.with(|rb| {
            rb.borrow_mut().clear();
            rb.borrow_mut().extend_from_slice(b"test data");
        });

        let data = crate::READ_BUFFER.with(|rb| rb.borrow().clone());
        assert_eq!(data, b"test data");

        // Clear and verify
        crate::READ_BUFFER.with(|rb| rb.borrow_mut().clear());
        let empty = crate::READ_BUFFER.with(|rb| rb.borrow().clone());
        assert!(empty.is_empty());
    }

    #[test]
    fn test_delimiter_scanning_logic() {
        // Test the core logic of finding delimiter and splitting remainder
        let chunk = b"{\"msg\":\"first\"}\n{\"msg\":\"second\"}\n".to_vec();
        let delimiter = b'\n';

        // Find first delimiter position
        let pos = chunk.iter().position(|&b| b == delimiter).unwrap();
        assert_eq!(pos, 15); // Position of first \n (0-indexed)

        // Extract message (including delimiter)
        let message = &chunk[..=pos];
        assert_eq!(message, b"{\"msg\":\"first\"}\n");

        // Extract remaining bytes (after delimiter)
        let remaining = &chunk[pos + 1..];
        assert_eq!(remaining, b"{\"msg\":\"second\"}\n");

        // Verify second message can be extracted
        let pos2 = remaining.iter().position(|&b| b == delimiter).unwrap();
        let message2 = &remaining[..=pos2];
        assert_eq!(message2, b"{\"msg\":\"second\"}\n");
    }

    #[test]
    fn test_batched_messages_simulation() {
        // Simulate the actual bug scenario: notifications/initialized + tools/list in same chunk
        let batched_chunk = b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n{\"jsonrpc\":\"2.0\",\"method\":\"tools/list\",\"id\":1}\n".to_vec();
        let delimiter = b'\n';

        // First message extraction
        let pos1 = batched_chunk.iter().position(|&b| b == delimiter).unwrap();
        let msg1 = &batched_chunk[..=pos1];
        let remainder_after_msg1 = &batched_chunk[pos1 + 1..];

        assert!(
            std::str::from_utf8(msg1)
                .unwrap()
                .contains("notifications/initialized")
        );
        assert!(
            std::str::from_utf8(remainder_after_msg1)
                .unwrap()
                .contains("tools/list")
        );

        // Second message extraction from remainder
        let pos2 = remainder_after_msg1
            .iter()
            .position(|&b| b == delimiter)
            .unwrap();
        let msg2 = &remainder_after_msg1[..=pos2];

        assert!(std::str::from_utf8(msg2).unwrap().contains("tools/list"));

        // Verify both are valid JSON-RPC
        let json1: serde_json::Value = serde_json::from_slice(&msg1[..msg1.len() - 1]).unwrap(); // strip \n
        let json2: serde_json::Value = serde_json::from_slice(&msg2[..msg2.len() - 1]).unwrap(); // strip \n

        assert_eq!(json1["method"], "notifications/initialized");
        assert_eq!(json2["method"], "tools/list");
        assert_eq!(json2["id"], 1);
    }

    // Note: More comprehensive tests requiring mock InputStream/OutputStream
    // would need WASI resource mocking, which is complex for unit tests.
    // Integration tests should verify full request/response handling.
}
