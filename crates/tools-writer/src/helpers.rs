//! Helper functions for JSON conversion and stream writing.

use crate::bindings::exports::wasmcp::mcp::tools_list_result::{Tool, ToolAnnotations, ToolHints};
use crate::bindings::wasi::io::streams::{OutputStream, StreamError};
use crate::bindings::wasmcp::mcp::content::{
    AudioContent, BlobResourceContents, ContentBlock, EmbeddedResource, EmbeddedResourceContent,
    ImageContent, ResourceLinkContent, TextContent, TextResourceContents,
};
use crate::bindings::wasmcp::mcp::types::Meta;
use crate::types::ContentBlockState;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::{json, Value};

/// Write data to the output stream, handling backpressure.
/// Does NOT flush - caller must flush when appropriate.
pub fn write_to_stream(output: &OutputStream, data: &[u8]) -> Result<(), StreamError> {
    let mut offset = 0;
    while offset < data.len() {
        let capacity = output.check_write().map_err(|_| StreamError::Closed)?;
        if capacity == 0 {
            // No capacity - must use blocking write for remainder
            output
                .blocking_write_and_flush(&data[offset..])
                .map_err(|_| StreamError::Closed)?;
            return Ok(()); // blocking_write_and_flush already flushed
        }

        let chunk_size = (capacity as usize).min(data.len() - offset);
        output
            .write(&data[offset..offset + chunk_size])
            .map_err(|_| StreamError::Closed)?;
        offset += chunk_size;
    }
    // Note: Caller must flush when appropriate
    Ok(())
}

pub fn tool_to_json(tool: &Tool) -> Value {
    let mut result = json!({
        "name": tool.name,
        "inputSchema": serde_json::from_str::<Value>(&tool.input_schema).unwrap_or(json!({}))
    });

    if let Some(options) = &tool.options {
        if let Some(desc) = &options.description {
            result["description"] = json!(desc);
        }
        if let Some(title) = &options.title {
            result["title"] = json!(title);
        }
        if let Some(output) = &options.output_schema {
            result["outputSchema"] = serde_json::from_str::<Value>(output).unwrap_or(json!({}));
        }
        if let Some(annotations) = &options.annotations {
            result["annotations"] = tool_annotations_to_json(annotations);
        }
    }

    result
}

pub fn tool_annotations_to_json(annotations: &ToolAnnotations) -> Value {
    let mut result = json!({});

    if let Some(title) = &annotations.title {
        result["title"] = json!(title);
    }

    let hints = annotations.hints;
    if hints.contains(ToolHints::READ_ONLY) {
        result["readOnlyHint"] = json!(true);
    }
    if hints.contains(ToolHints::DESTRUCTIVE) {
        result["destructiveHint"] = json!(true);
    }
    if hints.contains(ToolHints::IDEMPOTENT) {
        result["idempotentHint"] = json!(true);
    }
    if hints.contains(ToolHints::OPEN_WORLD) {
        result["openWorldHint"] = json!(true);
    }

    result
}

pub fn content_block_to_json(block: &ContentBlock) -> Value {
    match block {
        ContentBlock::Text(TextContent { text, options }) => {
            let mut result = json!({
                "type": "text"
            });
            result["text"] = json!(text);
            if let Some(opts) = options {
                if let Some(annotations) = &opts.annotations {
                    result["annotations"] = annotations_to_json(annotations);
                }
            }
            result
        }
        ContentBlock::Image(ImageContent {
            data,
            mime_type,
            options,
        }) => {
            let mut result = json!({
                "type": "image",
                "mimeType": mime_type
            });
            result["data"] = json!(BASE64.encode(data));
            if let Some(opts) = options {
                if let Some(annotations) = &opts.annotations {
                    result["annotations"] = annotations_to_json(annotations);
                }
            }
            result
        }
        ContentBlock::Audio(AudioContent {
            data,
            mime_type,
            options,
        }) => {
            let mut result = json!({
                "type": "audio",
                "mimeType": mime_type
            });
            result["data"] = json!(BASE64.encode(data));
            if let Some(opts) = options {
                if let Some(annotations) = &opts.annotations {
                    result["annotations"] = annotations_to_json(annotations);
                }
            }
            result
        }
        ContentBlock::EmbeddedResource(EmbeddedResourceContent { resource, options }) => {
            let resource_json = match resource {
                EmbeddedResource::Text(TextResourceContents {
                    uri,
                    text,
                    options: res_opts,
                }) => {
                    let mut r = json!({"uri": uri});
                    r["text"] = json!(text);
                    if let Some(opts) = res_opts {
                        if let Some(mime) = &opts.mime_type {
                            r["mimeType"] = json!(mime);
                        }
                    }
                    r
                }
                EmbeddedResource::Blob(BlobResourceContents {
                    uri,
                    blob,
                    options: res_opts,
                }) => {
                    let mut r = json!({"uri": uri});
                    r["blob"] = json!(BASE64.encode(blob));
                    if let Some(opts) = res_opts {
                        if let Some(mime) = &opts.mime_type {
                            r["mimeType"] = json!(mime);
                        }
                    }
                    r
                }
            };

            let mut result = json!({
                "type": "resource",
                "resource": resource_json
            });
            if let Some(opts) = options {
                if let Some(annotations) = &opts.annotations {
                    result["annotations"] = annotations_to_json(annotations);
                }
            }
            result
        }
        ContentBlock::ResourceLink(ResourceLinkContent { uri, name, options }) => {
            let mut result = json!({
                "type": "link",
                "uri": uri,
                "name": name
            });
            if let Some(opts) = options {
                if let Some(title) = &opts.title {
                    result["title"] = json!(title);
                }
                if let Some(desc) = &opts.description {
                    result["description"] = json!(desc);
                }
                if let Some(size) = opts.size {
                    result["size"] = json!(size);
                }
                if let Some(mime) = &opts.mime_type {
                    result["mimeType"] = json!(mime);
                }
                if let Some(annotations) = &opts.annotations {
                    result["annotations"] = annotations_to_json(annotations);
                }
            }
            result
        }
    }
}

pub fn content_block_to_state(block: &ContentBlock) -> Result<ContentBlockState, StreamError> {
    match block {
        ContentBlock::Text(TextContent { text, .. }) => {
            let text_str = text.clone();
            Ok(ContentBlockState::Text { text: text_str })
        }
        ContentBlock::Image(ImageContent {
            data, mime_type, ..
        }) => {
            let data_bytes = data.clone();
            Ok(ContentBlockState::Image {
                data: data_bytes,
                mime_type: mime_type.clone(),
            })
        }
        ContentBlock::Audio(AudioContent {
            data, mime_type, ..
        }) => {
            let data_bytes = data.clone();
            Ok(ContentBlockState::Audio {
                data: data_bytes,
                mime_type: mime_type.clone(),
            })
        }
        ContentBlock::EmbeddedResource(EmbeddedResourceContent { resource, .. }) => {
            match resource {
                EmbeddedResource::Text(TextResourceContents { uri, text, options }) => {
                    let text_str = Some(text.clone());
                    let mime_type = options.as_ref().and_then(|o| o.mime_type.clone());

                    Ok(ContentBlockState::Resource {
                        uri: uri.clone(),
                        text: text_str,
                        blob: None,
                        mime_type,
                    })
                }
                EmbeddedResource::Blob(BlobResourceContents { uri, blob, options }) => {
                    let blob_bytes = Some(blob.clone());
                    let mime_type = options.as_ref().and_then(|o| o.mime_type.clone());

                    Ok(ContentBlockState::Resource {
                        uri: uri.clone(),
                        text: None,
                        blob: blob_bytes,
                        mime_type,
                    })
                }
            }
        }
        ContentBlock::ResourceLink(_) => {
            // ResourceLink is a reference, not embeddable content with state
            // Return an error as we can't stream this type
            Err(StreamError::Closed)
        }
    }
}

pub fn meta_to_option_json(meta: &Meta) -> Option<Value> {
    meta.as_ref().map(|meta_vec| {
        let mut obj = serde_json::Map::new();
        for (key, value) in meta_vec {
            obj.insert(key.clone(), json!(value));
        }
        json!(obj)
    })
}

fn annotations_to_json(annotations: &crate::bindings::wasmcp::mcp::content::Annotations) -> Value {
    let mut result = json!({});

    // priority is f64, not Option<f64>
    result["priority"] = json!(annotations.priority);

    if let Some(audience) = &annotations.audience {
        result["audience"] = json!(audience
            .iter()
            .map(|r| match r {
                crate::bindings::wasmcp::mcp::content::Role::User => "user",
                crate::bindings::wasmcp::mcp::content::Role::Assistant => "assistant",
            })
            .collect::<Vec<_>>());
    }

    if let Some(last_modified) = &annotations.last_modified {
        result["lastModified"] = json!(last_modified);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta_field_serialization() {
        // Test None case
        let none_meta: Option<Vec<(String, String)>> = None;
        let result = meta_to_option_json(&none_meta);
        assert_eq!(result, None);

        // Test Some case
        let some_meta = Some(vec![
            ("version".to_string(), "1.0".to_string()),
            ("timestamp".to_string(), "2024-01-01T00:00:00Z".to_string()),
        ]);
        let result = meta_to_option_json(&some_meta).unwrap();
        assert_eq!(result["version"], "1.0");
        assert_eq!(result["timestamp"], "2024-01-01T00:00:00Z");
    }

    #[test]
    fn test_tool_json_conversion() {
        use crate::bindings::exports::wasmcp::mcp::tools_list_result::{
            Tool, ToolAnnotations, ToolHints, ToolOptions,
        };

        let tool = Tool {
            name: "test_tool".to_string(),
            input_schema: r#"{"type":"object"}"#.to_string(),
            options: Some(ToolOptions {
                description: Some("A test tool".to_string()),
                title: Some("Test Tool".to_string()),
                output_schema: None,
                annotations: Some(ToolAnnotations {
                    title: None,
                    hints: ToolHints::READ_ONLY | ToolHints::IDEMPOTENT,
                }),
                meta: None,
            }),
        };

        let result = tool_to_json(&tool);
        assert_eq!(result["name"], "test_tool");
        assert_eq!(result["description"], "A test tool");
        assert_eq!(result["title"], "Test Tool");
        assert_eq!(result["annotations"]["readOnlyHint"], true);
        assert_eq!(result["annotations"]["idempotentHint"], true);
    }

    #[test]
    fn test_content_block_formats() {
        use crate::bindings::wasmcp::mcp::content::{ContentBlock, TextContent};

        // Test text block
        let text_block = ContentBlock::Text(TextContent {
            text: "Hello, World!".to_string(),
            options: None,
        });
        let result = content_block_to_json(&text_block);
        assert_eq!(result["type"], "text");
        assert_eq!(result["text"], "Hello, World!");
    }

    #[test]
    fn test_content_annotations() {
        use crate::bindings::wasmcp::mcp::content::{Annotations, Role};

        let annotations = Annotations {
            priority: 0.9,
            audience: Some(vec![Role::User, Role::Assistant]),
            last_modified: Some("2024-01-01T12:00:00Z".to_string()),
        };

        let result = annotations_to_json(&annotations);
        assert_eq!(result["priority"], 0.9);
        assert_eq!(result["audience"][0], "user");
        assert_eq!(result["audience"][1], "assistant");
        assert_eq!(result["lastModified"], "2024-01-01T12:00:00Z");
    }

    #[test]
    fn test_embedded_resource_format() {
        use crate::bindings::wasmcp::mcp::content::{
            ContentBlock, EmbeddedResource, EmbeddedResourceContent, TextResourceContents,
        };

        let embedded = ContentBlock::EmbeddedResource(EmbeddedResourceContent {
            resource: EmbeddedResource::Text(TextResourceContents {
                uri: "data:text/plain;base64,SGVsbG8=".to_string(),
                text: "Hello".to_string(),
                options: None,
            }),
            options: None,
        });

        let result = content_block_to_json(&embedded);
        assert_eq!(result["type"], "resource");
        assert_eq!(result["resource"]["uri"], "data:text/plain;base64,SGVsbG8=");
        assert_eq!(result["resource"]["text"], "Hello");
    }
}
