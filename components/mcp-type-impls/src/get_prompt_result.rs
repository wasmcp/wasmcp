use std::cell::RefCell;
use serde_json::{json, Value};
use crate::bindings::exports::wasmcp::mcp::types::{
    GuestGetPromptResult,
    GetPromptResult as GetPromptResultWrapper,
    ContentOptions,
    ResourceOptions,
    ResourceContentsOptions,
    Role,
};
use super::helpers::{role_to_string, apply_content_options, apply_resource_options, apply_resource_contents_options};

pub struct GetPromptResult {
    internal: RefCell<Value>,
}

impl GuestGetPromptResult for GetPromptResult {
    fn new() -> Self {
        Self {
            internal: RefCell::new(json!({
                "messages": [],
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

    fn set_description(&self, description: String) {
        self.internal.borrow_mut()["description"] = json!(description);
    }

    fn add_text_message(&self, role: Role, text: String, options: Option<ContentOptions>) {
        let mut content = json!({
            "type": "text",
            "text": text,
        });

        apply_content_options(&mut content, options);

        let message = json!({
            "role": role_to_string(role),
            "content": content,
        });

        self.internal.borrow_mut()["messages"]
            .as_array_mut()
            .expect("messages should be an array")
            .push(message);
    }

    fn add_image_message(&self, role: Role, mime_type: String, data: String, options: Option<ContentOptions>) {
        let mut content = json!({
            "type": "image",
            "mimeType": mime_type,
            "data": data,
        });

        apply_content_options(&mut content, options);

        let message = json!({
            "role": role_to_string(role),
            "content": content,
        });

        self.internal.borrow_mut()["messages"]
            .as_array_mut()
            .expect("messages should be an array")
            .push(message);
    }

    fn add_audio_message(&self, role: Role, mime_type: String, data: String, options: Option<ContentOptions>) {
        let mut content = json!({
            "type": "audio",
            "mimeType": mime_type,
            "data": data,
        });

        apply_content_options(&mut content, options);

        let message = json!({
            "role": role_to_string(role),
            "content": content,
        });

        self.internal.borrow_mut()["messages"]
            .as_array_mut()
            .expect("messages should be an array")
            .push(message);
    }

    fn add_resource_link_message(&self, role: Role, name: String, uri: String, options: Option<ResourceOptions>) {
        let mut content = json!({
            "type": "resource_link",
            "name": name,
            "uri": uri,
        });

        apply_resource_options(&mut content, options);

        let message = json!({
            "role": role_to_string(role),
            "content": content,
        });

        self.internal.borrow_mut()["messages"]
            .as_array_mut()
            .expect("messages should be an array")
            .push(message);
    }

    fn add_text_resource_message(&self, role: Role, uri: String, text: String, options: Option<ResourceContentsOptions>) {
        let mut resource = json!({
            "uri": uri,
            "text": text,
        });

        apply_resource_contents_options(&mut resource, options);

        let content = json!({
            "type": "resource",
            "resource": resource,
        });

        let message = json!({
            "role": role_to_string(role),
            "content": content,
        });

        self.internal.borrow_mut()["messages"]
            .as_array_mut()
            .expect("messages should be an array")
            .push(message);
    }

    fn add_blob_resource_message(&self, role: Role, uri: String, blob: String, options: Option<ResourceContentsOptions>) {
        let mut resource = json!({
            "uri": uri,
            "blob": blob,
        });

        apply_resource_contents_options(&mut resource, options);

        let content = json!({
            "type": "resource",
            "resource": resource,
        });

        let message = json!({
            "role": role_to_string(role),
            "content": content,
        });

        self.internal.borrow_mut()["messages"]
            .as_array_mut()
            .expect("messages should be an array")
            .push(message);
    }

    fn finish_json(this: GetPromptResultWrapper) -> Result<String, ()> {
        let inner = this.into_inner::<Self>();
        Ok(inner.internal.into_inner().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::exports::wasmcp::mcp::types::Annotations;
    use serde_json::json;

    #[test]
    fn test_get_prompt_result_empty() {
        let result = GetPromptResult::new();
        let wrapper = GetPromptResultWrapper::new(result);
        let json = GetPromptResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, json!({
            "messages": []
        }));
    }

    #[test]
    fn test_get_prompt_result_text_messages() {
        let result = GetPromptResult::new();

        result.set_description("A helpful prompt".to_string());

        result.add_text_message(Role::User, "Hello AI".to_string(), None);
        result.add_text_message(
            Role::Assistant,
            "Hello! How can I help?".to_string(),
            Some(ContentOptions {
                meta: None,
                annotations: Some(Annotations {
                    audience: Some(vec![Role::User, Role::Assistant]),
                    priority: None,
                    last_modified: None,
                }),
            })
        );

        let wrapper = GetPromptResultWrapper::new(result);
        let json = GetPromptResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["description"], "A helpful prompt");
        assert_eq!(parsed["messages"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["messages"][0]["role"], "user");
        assert_eq!(parsed["messages"][0]["content"]["text"], "Hello AI");
        assert_eq!(parsed["messages"][1]["role"], "assistant");
        assert_eq!(parsed["messages"][1]["content"]["annotations"]["audience"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_get_prompt_result_mixed_messages() {
        let result = GetPromptResult::new();

        result.add_text_message(Role::User, "Analyze this image".to_string(), None);

        result.add_image_message(
            Role::User,
            "image/png".to_string(),
            "imagedata".to_string(),
            None
        );

        result.add_audio_message(
            Role::Assistant,
            "audio/mp3".to_string(),
            "audiodata".to_string(),
            None
        );

        result.add_resource_link_message(
            Role::Assistant,
            "doc".to_string(),
            "file:///doc.txt".to_string(),
            Some(ResourceOptions {
                meta: None,
                annotations: None,
                description: Some("Reference document".to_string()),
                mime_type: Some("text/plain".to_string()),
                size: None,
                title: Some("Documentation".to_string()),
            })
        );

        result.add_text_resource_message(
            Role::User,
            "file:///data.txt".to_string(),
            "Data content".to_string(),
            None
        );

        result.add_blob_resource_message(
            Role::Assistant,
            "file:///result.bin".to_string(),
            "binarydata".to_string(),
            Some(ResourceContentsOptions {
                meta: Some(vec![("format".to_string(), "custom".to_string())]),
                mime_type: Some("application/octet-stream".to_string()),
            })
        );

        let wrapper = GetPromptResultWrapper::new(result);
        let json = GetPromptResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["messages"].as_array().unwrap().len(), 6);
        assert_eq!(parsed["messages"][0]["content"]["type"], "text");
        assert_eq!(parsed["messages"][1]["content"]["type"], "image");
        assert_eq!(parsed["messages"][2]["content"]["type"], "audio");
        assert_eq!(parsed["messages"][3]["content"]["type"], "resource_link");
        assert_eq!(parsed["messages"][3]["content"]["description"], "Reference document");
        assert_eq!(parsed["messages"][4]["content"]["type"], "resource");
        assert_eq!(parsed["messages"][4]["content"]["resource"]["text"], "Data content");
        assert_eq!(parsed["messages"][5]["content"]["resource"]["blob"], "binarydata");
        assert_eq!(parsed["messages"][5]["content"]["resource"]["_meta"]["format"], "custom");
    }

    #[test]
    fn test_get_prompt_result_with_meta() {
        let result = GetPromptResult::new();

        result.add_meta("prompt_version".to_string(), "2.0".to_string()).unwrap();
        result.add_meta("template_id".to_string(), "tmpl_123".to_string()).unwrap();

        result.add_text_message(Role::User, "Test".to_string(), None);

        let wrapper = GetPromptResultWrapper::new(result);
        let json = GetPromptResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["_meta"]["prompt_version"], "2.0");
        assert_eq!(parsed["_meta"]["template_id"], "tmpl_123");
    }
}