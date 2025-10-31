//! JSON-RPC request parsing for MCP protocol
//!
//! This module handles parsing JSON-RPC requests into WIT types.
//! Serde handles validation automatically.

// Internal modules
mod content;
mod notifications;
mod requests;
mod responses;
mod types;

// Re-export public functions from submodules
pub use notifications::parse_client_notification;
pub use requests::parse_client_request;
pub use responses::parse_client_response;
pub use types::parse_request_id;

// Test module
#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::wasmcp::mcp_v20250618::mcp::{ClientRequest, ProtocolVersion, RequestId};
    use serde_json::json;

    #[test]
    fn test_parse_request_id() {
        let num_id = json!(42);
        let str_id = json!("test-123");

        let parsed_num = parse_request_id(&num_id).unwrap();
        let parsed_str = parse_request_id(&str_id).unwrap();

        assert!(matches!(parsed_num, RequestId::Number(42)));
        assert!(matches!(parsed_str, RequestId::String(s) if s == "test-123"));
    }

    #[test]
    fn test_parse_protocol_version() {
        assert!(matches!(
            types::parse_protocol_version("2025-06-18"),
            Ok(ProtocolVersion::V20250618)
        ));
        assert!(types::parse_protocol_version("invalid").is_err());
    }

    #[test]
    fn test_parse_initialize_request() {
        let request = json!({
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "roots": {
                        "listChanged": true
                    }
                },
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        });

        let parsed = parse_client_request(&request).unwrap();
        assert!(matches!(parsed, ClientRequest::Initialize(_)));
    }
}
