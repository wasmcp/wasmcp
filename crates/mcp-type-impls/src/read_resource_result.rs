use std::cell::RefCell;
use serde_json::{json, Value};
use crate::bindings::exports::wasmcp::mcp::types::{
    GuestReadResourceResult,
    ReadResourceResult as ReadResourceResultWrapper,
    ResourceContentsOptions,
};
use super::helpers::apply_resource_contents_options;

pub struct ReadResourceResult {
    internal: RefCell<Value>,
}

impl GuestReadResourceResult for ReadResourceResult {
    fn new() -> Self {
        Self {
            internal: RefCell::new(json!({
                "contents": [],
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

    fn add_text_resource(&self, uri: String, text: String, options: Option<ResourceContentsOptions>) {
        let mut contents = json!({
            "uri": uri,
            "text": text,
        });

        apply_resource_contents_options(&mut contents, options);

        self.internal.borrow_mut()["contents"]
            .as_array_mut()
            .expect("contents should be an array")
            .push(contents);
    }

    fn add_blob_resource(&self, uri: String, blob: String, options: Option<ResourceContentsOptions>) {
        let mut contents = json!({
            "uri": uri,
            "blob": blob,
        });

        apply_resource_contents_options(&mut contents, options);

        self.internal.borrow_mut()["contents"]
            .as_array_mut()
            .expect("contents should be an array")
            .push(contents);
    }

    fn finish_json(this: ReadResourceResultWrapper) -> Result<String, ()> {
        let inner = this.into_inner::<Self>();
        Ok(inner.internal.into_inner().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_read_resource_result_empty() {
        let result = ReadResourceResult::new();
        let wrapper = ReadResourceResultWrapper::new(result);
        let json = ReadResourceResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, json!({
            "contents": []
        }));
    }

    #[test]
    fn test_read_resource_result_text() {
        let result = ReadResourceResult::new();

        result.add_text_resource(
            "file:///readme.txt".to_string(),
            "This is the readme content".to_string(),
            Some(ResourceContentsOptions {
                meta: None,
                mime_type: Some("text/plain".to_string()),
            })
        );

        result.add_text_resource(
            "file:///data.json".to_string(),
            r#"{"key": "value"}"#.to_string(),
            Some(ResourceContentsOptions {
                meta: Some(vec![("encoding".to_string(), "utf-8".to_string())]),
                mime_type: Some("application/json".to_string()),
            })
        );

        let wrapper = ReadResourceResultWrapper::new(result);
        let json = ReadResourceResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["contents"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["contents"][0]["text"], "This is the readme content");
        assert_eq!(parsed["contents"][0]["mimeType"], "text/plain");
        assert_eq!(parsed["contents"][1]["text"], r#"{"key": "value"}"#);
        assert_eq!(parsed["contents"][1]["_meta"]["encoding"], "utf-8");
    }

    #[test]
    fn test_read_resource_result_blob() {
        let result = ReadResourceResult::new();

        result.add_blob_resource(
            "file:///image.jpg".to_string(),
            "base64imagedata".to_string(),
            Some(ResourceContentsOptions {
                meta: Some(vec![("size".to_string(), "1024".to_string())]),
                mime_type: Some("image/jpeg".to_string()),
            })
        );

        result.add_blob_resource(
            "file:///binary.dat".to_string(),
            "binarydata".to_string(),
            None
        );

        let wrapper = ReadResourceResultWrapper::new(result);
        let json = ReadResourceResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["contents"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["contents"][0]["blob"], "base64imagedata");
        assert_eq!(parsed["contents"][0]["mimeType"], "image/jpeg");
        assert_eq!(parsed["contents"][0]["_meta"]["size"], "1024");
        assert_eq!(parsed["contents"][1]["blob"], "binarydata");
    }

    #[test]
    fn test_read_resource_result_mixed() {
        let result = ReadResourceResult::new();

        result.add_text_resource(
            "file:///text.txt".to_string(),
            "text content".to_string(),
            None
        );

        result.add_blob_resource(
            "file:///data.bin".to_string(),
            "binaryblob".to_string(),
            None
        );

        result.add_meta("batch_id".to_string(), "batch123".to_string()).unwrap();

        let wrapper = ReadResourceResultWrapper::new(result);
        let json = ReadResourceResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["contents"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["contents"][0]["text"], "text content");
        assert_eq!(parsed["contents"][1]["blob"], "binaryblob");
        assert_eq!(parsed["_meta"]["batch_id"], "batch123");
    }
}