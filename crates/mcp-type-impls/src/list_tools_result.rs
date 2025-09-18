use std::cell::RefCell;
use serde_json::{json, Value};
use crate::bindings::exports::wasmcp::mcp::types::{
    GuestListToolsResult,
    ListToolsResult as ListToolsResultWrapper,
    ToolOptions,
};
use super::helpers::build_tool_annotations;

pub struct ListToolsResult {
    internal: RefCell<Value>,
}

impl GuestListToolsResult for ListToolsResult {
    fn new() -> Self {
        Self {
            internal: RefCell::new(json!({
                "tools": [],
            })),
        }
    }

    fn add_meta(&self, key: String, value: String) -> Result<(), ()> {
        let mut internal = self.internal.borrow_mut();
        let meta = internal
            .as_object_mut()
            .ok_or(())?
            .entry("_meta")
            .or_insert_with(|| json!({}));

        meta[key] = json!(value);
        Ok(())
    }

    fn set_next_cursor(&self, cursor: String) {
        self.internal.borrow_mut()["nextCursor"] = json!(cursor);
    }

    fn add_tool(&self, name: String, input_schema: String, options: Option<ToolOptions>) {
        // Parse inputSchema as JSON - it should be a valid JSON Schema object
        let parsed_schema = serde_json::from_str::<Value>(&input_schema)
            .unwrap_or_else(|_| json!({ "type": "object" }));

        let mut tool = json!({
            "name": name,
            "inputSchema": parsed_schema,
        });

        if let Some(opts) = options {
            // Process optional fields according to MCP spec
            if let Some(desc) = opts.description {
                tool["description"] = json!(desc);
            }
            if let Some(title) = opts.title {
                tool["title"] = json!(title);
            }
            if let Some(meta) = opts.meta {
                let mut meta_obj = json!({});
                for (key, value) in meta {
                    meta_obj[key] = json!(value);
                }
                tool["_meta"] = meta_obj;
            }
            if let Some(output_schema) = opts.output_schema {
                tool["outputSchema"] = serde_json::from_str::<Value>(&output_schema)
                    .unwrap_or_else(|_| json!({ "type": "object" }));
            }
            if let Some(annotations) = opts.annotations {
                tool["annotations"] = build_tool_annotations(annotations);
            }
        }

        self.internal.borrow_mut()["tools"]
            .as_array_mut()
            .expect("tools should be an array")
            .push(tool);
    }

    fn finish_json(this: ListToolsResultWrapper) -> Result<String, ()> {
        let inner = this.into_inner::<Self>();
        Ok(inner.internal.into_inner().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::exports::wasmcp::mcp::types::{ToolAnnotations, ToolHints};
    use serde_json::json;

    #[test]
    fn test_list_tools_result_empty() {
        let result = ListToolsResult::new();
        let wrapper = ListToolsResultWrapper::new(result);
        let json = ListToolsResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, json!({
            "tools": []
        }));
    }

    #[test]
    fn test_list_tools_result_with_tools() {
        let result = ListToolsResult::new();

        // Add a simple tool
        result.add_tool(
            "get_weather".to_string(),
            r#"{"type": "object", "properties": {"city": {"type": "string"}}, "required": ["city"]}"#.to_string(),
            None
        );

        // Add a tool with all options
        let options = ToolOptions {
            meta: Some(vec![("version".to_string(), "1.0".to_string())]),
            annotations: Some(ToolAnnotations {
                hints: ToolHints::READ_ONLY | ToolHints::IDEMPOTENT,
                title: Some("Calculator Tool".to_string()),
            }),
            description: Some("Performs calculations".to_string()),
            output_schema: Some(r#"{"type": "number"}"#.to_string()),
            title: Some("Calculator".to_string()),
        };

        result.add_tool(
            "calculate".to_string(),
            r#"{"type": "object"}"#.to_string(),
            Some(options)
        );

        result.set_next_cursor("cursor123".to_string());
        result.add_meta("request_id".to_string(), "req123".to_string()).unwrap();

        let wrapper = ListToolsResultWrapper::new(result);
        let json = ListToolsResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["tools"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["tools"][0]["name"], "get_weather");
        assert_eq!(parsed["tools"][1]["name"], "calculate");
        assert_eq!(parsed["tools"][1]["description"], "Performs calculations");
        assert_eq!(parsed["tools"][1]["annotations"]["readOnlyHint"], true);
        assert_eq!(parsed["tools"][1]["annotations"]["idempotentHint"], true);
        assert_eq!(parsed["tools"][1]["annotations"]["destructiveHint"], false);
        assert_eq!(parsed["nextCursor"], "cursor123");
        assert_eq!(parsed["_meta"]["request_id"], "req123");
    }

    #[test]
    fn test_special_characters_in_tool() {
        let result = ListToolsResult::new();

        // Test with special characters and unicode
        result.add_tool(
            "test_tool_😀".to_string(),
            r#"{"type": "object", "description": "Tool with \"quotes\" and \n newlines"}"#.to_string(),
            Some(ToolOptions {
                meta: None,
                annotations: None,
                description: Some("Description with special chars: <>&\"'".to_string()),
                output_schema: None,
                title: Some("Title with emoji 🚀".to_string()),
            })
        );

        let wrapper = ListToolsResultWrapper::new(result);
        let json = ListToolsResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["tools"][0]["name"], "test_tool_😀");
        assert_eq!(parsed["tools"][0]["title"], "Title with emoji 🚀");
        assert_eq!(parsed["tools"][0]["description"], "Description with special chars: <>&\"'");
    }

    #[test]
    fn test_json_schema_validation() {
        let result = ListToolsResult::new();

        // Valid JSON schema
        let valid_schema = r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "number", "minimum": 0}
            },
            "required": ["name"]
        }"#;

        result.add_tool("valid_tool".to_string(), valid_schema.to_string(), None);

        // Invalid JSON (will fallback to default)
        result.add_tool("invalid_tool".to_string(), "not valid json".to_string(), None);

        let wrapper = ListToolsResultWrapper::new(result);
        let json = ListToolsResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        // Valid schema should be parsed as object
        assert!(parsed["tools"][0]["inputSchema"].is_object());
        assert_eq!(parsed["tools"][0]["inputSchema"]["type"], "object");

        // Invalid JSON should fallback to default object
        assert_eq!(parsed["tools"][1]["inputSchema"]["type"], "object");
    }
}