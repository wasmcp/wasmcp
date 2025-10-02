//! HTTP utilities for MCP transport
//!
//! This module contains pure functions for HTTP response handling,
//! error formatting, and content type validation.

/// Determines appropriate HTTP status code for MCP error types
pub fn status_code_for_error(error_type: &str) -> u16 {
    match error_type {
        "parse_error" | "invalid_request" => 400,
        "method_not_found" => 404,
        "internal_error" => 500,
        _ => 400, // Default to bad request
    }
}

/// Formats error messages for HTTP responses, ensuring non-empty output
pub fn format_error_response(error_type: &str, details: Option<&str>) -> Vec<u8> {
    let base_msg = if error_type.is_empty() {
        "Error".to_string()
    } else {
        error_type.to_string()
    };

    match details {
        Some(d) => format!("{}: {}", base_msg, d).into_bytes(),
        None => base_msg.into_bytes(),
    }
}

/// Validates if a content-type header indicates JSON
pub fn is_json_content_type(content_type: &str) -> bool {
    content_type == "application/json" || content_type.starts_with("application/json;")
}

/// Returns the appropriate content-type header value for responses
pub fn content_type_for_response(is_error: bool) -> &'static str {
    if is_error {
        "text/plain"
    } else {
        "application/json"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(status_code_for_error("parse_error"), 400);
        assert_eq!(status_code_for_error("invalid_request"), 400);
        assert_eq!(status_code_for_error("method_not_found"), 404);
        assert_eq!(status_code_for_error("internal_error"), 500);
        assert_eq!(status_code_for_error("unknown"), 400);
    }

    #[test]
    fn test_error_message_formatting() {
        let msg = format_error_response("parse_error", Some("Invalid JSON"));
        assert_eq!(msg, b"parse_error: Invalid JSON");

        let msg = format_error_response("internal_error", None);
        assert_eq!(msg, b"internal_error");

        let msg = format_error_response("method_not_found", Some(""));
        assert_eq!(msg, b"method_not_found: ");

        // Empty error type gets default
        let msg = format_error_response("", None);
        assert_eq!(msg, b"Error");
    }

    #[test]
    fn test_content_type_validation() {
        assert!(is_json_content_type("application/json"));
        assert!(is_json_content_type("application/json; charset=utf-8"));
        assert!(is_json_content_type("application/json;charset=UTF-8"));

        assert!(!is_json_content_type("text/plain"));
        assert!(!is_json_content_type("application/xml"));
        assert!(!is_json_content_type(""));
    }

    #[test]
    fn test_response_content_type() {
        assert_eq!(content_type_for_response(false), "application/json");
        assert_eq!(content_type_for_response(true), "text/plain");
    }

    #[test]
    fn test_error_message_properties() {
        // Property: error messages should never be empty
        let test_cases = vec![
            ("", None),
            ("", Some("")),
            ("error", None),
            ("error", Some("details")),
        ];

        for (error_type, details) in test_cases {
            let msg = format_error_response(error_type, details);
            assert!(!msg.is_empty(), "Error message should never be empty");
        }

        // Property: error messages should be valid UTF-8
        for error_type in &["parse_error", "テスト", "🔥"] {
            let msg = format_error_response(error_type, Some("test"));
            assert!(
                String::from_utf8(msg).is_ok(),
                "Error messages should be valid UTF-8"
            );
        }
    }
}
