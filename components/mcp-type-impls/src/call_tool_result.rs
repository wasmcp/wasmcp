use std::cell::RefCell;
use serde_json::{json, Value};
use crate::bindings::exports::wasmcp::mcp::types::{
    GuestCallToolResult,
    CallToolResult as CallToolResultWrapper,
    ContentOptions,
    ResourceOptions,
    ResourceContentsOptions,
};
use super::helpers::{apply_content_options, apply_resource_options, apply_resource_contents_options};

pub struct CallToolResult {
    internal: RefCell<Value>,
}

impl GuestCallToolResult for CallToolResult {
    fn new() -> Self {
        Self {
            internal: RefCell::new(json!({
                "content": [],
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

    fn add_text(&self, text: String, options: Option<ContentOptions>) {
        let mut content = json!({
            "type": "text",
            "text": text,
        });

        apply_content_options(&mut content, options);

        self.internal.borrow_mut()["content"]
            .as_array_mut()
            .expect("content should be an array")
            .push(content);
    }

    fn add_image(&self, mime_type: String, data: String, options: Option<ContentOptions>) {
        let mut content = json!({
            "type": "image",
            "mimeType": mime_type,
            "data": data,
        });

        apply_content_options(&mut content, options);

        self.internal.borrow_mut()["content"]
            .as_array_mut()
            .expect("content should be an array")
            .push(content);
    }

    fn add_audio(&self, mime_type: String, data: String, options: Option<ContentOptions>) {
        let mut content = json!({
            "type": "audio",
            "mimeType": mime_type,
            "data": data,
        });

        apply_content_options(&mut content, options);

        self.internal.borrow_mut()["content"]
            .as_array_mut()
            .expect("content should be an array")
            .push(content);
    }

    fn add_resource_link(&self, name: String, uri: String, options: Option<ResourceOptions>) {
        let mut content = json!({
            "type": "resource_link",
            "name": name,
            "uri": uri,
        });

        apply_resource_options(&mut content, options);

        self.internal.borrow_mut()["content"]
            .as_array_mut()
            .expect("content should be an array")
            .push(content);
    }

    fn add_text_resource(&self, uri: String, text: String, options: Option<ResourceContentsOptions>) {
        let mut resource = json!({
            "uri": uri,
            "text": text,
        });

        apply_resource_contents_options(&mut resource, options);

        let content = json!({
            "type": "resource",
            "resource": resource,
        });

        self.internal.borrow_mut()["content"]
            .as_array_mut()
            .expect("content should be an array")
            .push(content);
    }

    fn add_blob_resource(&self, uri: String, blob: String, options: Option<ResourceContentsOptions>) {
        let mut resource = json!({
            "uri": uri,
            "blob": blob,
        });

        apply_resource_contents_options(&mut resource, options);

        let content = json!({
            "type": "resource",
            "resource": resource,
        });

        self.internal.borrow_mut()["content"]
            .as_array_mut()
            .expect("content should be an array")
            .push(content);
    }

    fn set_error(&self) {
        self.internal.borrow_mut()["isError"] = json!(true);
    }

    fn set_structured_content(&self, content: String) {
        let parsed = serde_json::from_str::<Value>(&content)
            .unwrap_or_else(|_| json!(content));
        self.internal.borrow_mut()["structuredContent"] = parsed;
    }

    fn finish_json(this: CallToolResultWrapper) -> Result<String, ()> {
        let inner = this.into_inner::<Self>();
        Ok(inner.internal.into_inner().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::exports::wasmcp::mcp::types::{Annotations, Role};
    

    #[test]
    fn test_call_tool_result_text_content() {
        let result = CallToolResult::new();

        result.add_text("Hello, world!".to_string(), None);

        let content_options = ContentOptions {
            meta: Some(vec![("key".to_string(), "value".to_string())]),
            annotations: Some(Annotations {
                audience: Some(vec![Role::User]),
                priority: Some(0.8),
                last_modified: Some("2024-01-01T00:00:00Z".to_string()),
            }),
        };
        result.add_text("Annotated text".to_string(), Some(content_options));

        let wrapper = CallToolResultWrapper::new(result);
        let json = CallToolResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["content"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["content"][0]["type"], "text");
        assert_eq!(parsed["content"][0]["text"], "Hello, world!");
        assert_eq!(parsed["content"][1]["annotations"]["priority"], 0.8);
        assert_eq!(parsed["content"][1]["annotations"]["audience"][0], "user");
    }

    #[test]
    fn test_call_tool_result_mixed_content() {
        let result = CallToolResult::new();

        // Add different content types
        result.add_text("Some text".to_string(), None);
        result.add_image("image/png".to_string(), "base64data".to_string(), None);
        result.add_audio("audio/mp3".to_string(), "audiodata".to_string(), None);

        let resource_options = ResourceOptions {
            meta: None,
            annotations: None,
            description: Some("A helpful resource".to_string()),
            mime_type: Some("text/plain".to_string()),
            size: Some(1024),
            title: Some("Resource Title".to_string()),
        };
        result.add_resource_link("my-resource".to_string(), "file:///path/to/resource".to_string(), Some(resource_options));

        result.add_text_resource("file:///doc.txt".to_string(), "Document content".to_string(), None);
        result.add_blob_resource("file:///image.png".to_string(), "blobdata".to_string(), None);

        let wrapper = CallToolResultWrapper::new(result);
        let json = CallToolResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["content"].as_array().unwrap().len(), 6);
        assert_eq!(parsed["content"][1]["type"], "image");
        assert_eq!(parsed["content"][2]["type"], "audio");
        assert_eq!(parsed["content"][3]["type"], "resource_link");
        assert_eq!(parsed["content"][3]["description"], "A helpful resource");
        assert_eq!(parsed["content"][4]["type"], "resource");
        assert_eq!(parsed["content"][4]["resource"]["text"], "Document content");
    }

    #[test]
    fn test_call_tool_result_with_error() {
        let result = CallToolResult::new();

        result.add_text("Error occurred".to_string(), None);
        result.set_error();
        result.set_structured_content(r#"{"error": "Something went wrong"}"#.to_string());

        let wrapper = CallToolResultWrapper::new(result);
        let json = CallToolResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["isError"], true);
        assert_eq!(parsed["structuredContent"]["error"], "Something went wrong");
    }

    #[test]
    fn test_empty_optional_fields() {
        let result = CallToolResult::new();

        // Test with all None options
        result.add_text("text".to_string(), Some(ContentOptions {
            meta: None,
            annotations: None,
        }));

        result.add_resource_link("res".to_string(), "uri".to_string(), Some(ResourceOptions {
            meta: None,
            annotations: None,
            description: None,
            mime_type: None,
            size: None,
            title: None,
        }));

        let wrapper = CallToolResultWrapper::new(result);
        let json = CallToolResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        // Verify that empty options don't add unnecessary fields
        assert_eq!(parsed["content"][0].get("_meta"), None);
        assert_eq!(parsed["content"][0].get("annotations"), None);
        assert_eq!(parsed["content"][1].get("description"), None);
    }

    #[test]
    fn test_large_data_handling() {
        let result = CallToolResult::new();

        // Create a large string (1MB)
        let large_text = "a".repeat(1024 * 1024);
        result.add_text(large_text.clone(), None);

        let wrapper = CallToolResultWrapper::new(result);
        let json = CallToolResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["content"][0]["text"].as_str().unwrap().len(), 1024 * 1024);
    }

    #[test]
    fn test_structured_content_parsing() {
        let result = CallToolResult::new();

        // Valid JSON
        result.set_structured_content(r#"{"key": "value", "nested": {"inner": 123}}"#.to_string());

        let wrapper = CallToolResultWrapper::new(result);
        let json = CallToolResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["structuredContent"]["key"], "value");
        assert_eq!(parsed["structuredContent"]["nested"]["inner"], 123);
    }

    #[test]
    fn test_structured_content_invalid_json() {
        let result = CallToolResult::new();

        // Invalid JSON should be stored as string
        result.set_structured_content("not valid json".to_string());

        let wrapper = CallToolResultWrapper::new(result);
        let json = CallToolResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["structuredContent"], "not valid json");
    }
}