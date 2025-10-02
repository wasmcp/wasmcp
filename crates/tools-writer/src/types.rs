//! Type definitions for MCP result structures.

use serde::{Deserialize, Serialize};

/// Result structure for tools/list responses.
#[derive(Serialize, Deserialize)]
pub struct ListToolsResult {
    pub tools: Vec<serde_json::Value>,
    #[serde(rename = "nextCursor", skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub _meta: Option<serde_json::Value>,
}

/// Result structure for tools/call responses.
#[derive(Serialize, Deserialize)]
pub struct CallToolResult {
    pub content: Vec<serde_json::Value>,
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub _meta: Option<serde_json::Value>,
}

/// Internal state for streaming content blocks.
pub enum ContentBlockState {
    Text {
        text: String,
    },
    Image {
        data: Vec<u8>,
        mime_type: String,
    },
    Audio {
        data: Vec<u8>,
        mime_type: String,
    },
    Resource {
        uri: String,
        text: Option<String>,
        blob: Option<Vec<u8>>,
        mime_type: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_list_tools_result_serialization() {
        let result = ListToolsResult {
            tools: vec![json!({"name": "tool1"}), json!({"name": "tool2"})],
            next_cursor: Some("cursor_abc123".to_string()),
            _meta: Some(json!({"version": "1.0"})),
        };

        let json = serde_json::to_value(result).unwrap();
        assert_eq!(json["tools"].as_array().unwrap().len(), 2);
        assert_eq!(json["nextCursor"], "cursor_abc123");
        assert_eq!(json["_meta"]["version"], "1.0");
    }

    #[test]
    fn test_call_tool_result_serialization() {
        let result = CallToolResult {
            content: vec![
                json!({"type": "text", "text": "Hello"}),
                json!({"type": "text", "text": "World"}),
            ],
            is_error: Some(false),
            _meta: None,
        };

        let json = serde_json::to_value(result).unwrap();
        assert_eq!(json["content"].as_array().unwrap().len(), 2);
        assert_eq!(json["isError"], false);
        assert_eq!(json.get("_meta"), None); // Should be omitted when None
    }

    #[test]
    fn test_error_response_format() {
        let result = CallToolResult {
            content: vec![json!({"type": "text", "text": "An error occurred"})],
            is_error: Some(true),
            _meta: None,
        };

        let json = serde_json::to_value(result).unwrap();
        assert_eq!(json["isError"], true);
        assert_eq!(json["content"][0]["text"], "An error occurred");
    }

    #[test]
    fn test_pagination_cursor() {
        let result = ListToolsResult {
            tools: vec![
                json!({"name": "tool1", "inputSchema": {}}),
                json!({"name": "tool2", "inputSchema": {}}),
            ],
            next_cursor: Some("cursor_abc123".to_string()),
            _meta: None,
        };

        let json = serde_json::to_value(result).unwrap();
        assert_eq!(json["nextCursor"], "cursor_abc123");
        assert_eq!(json["tools"].as_array().unwrap().len(), 2);
    }
}
