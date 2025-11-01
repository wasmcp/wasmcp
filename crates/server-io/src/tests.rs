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

    // Note: More comprehensive tests requiring mock InputStream/OutputStream
    // would need WASI resource mocking, which is complex for unit tests.
    // Integration tests should verify full request/response handling.
}
