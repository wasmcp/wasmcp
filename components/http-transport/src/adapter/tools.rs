use super::WitMcpAdapter;
use anyhow::Result;
use base64::Engine;
use rmcp::model::{CallToolResult, Content, ListToolsResult, Tool};
use serde_json::{json, Value};
use std::borrow::Cow;
use std::sync::Arc;

impl WitMcpAdapter {
    /// Convert rmcp call tool request to WIT types
    pub fn convert_call_tool_request(
        &self,
        name: &str,
        arguments: Option<serde_json::Map<String, Value>>,
    ) -> crate::bindings::wasmcp::mcp::tools_types::CallToolRequest {
        // Convert arguments Map directly to JSON string for WIT interface
        let args_str = arguments.map(|args| serde_json::to_string(&args).unwrap());

        crate::bindings::wasmcp::mcp::tools_types::CallToolRequest {
            name: name.to_string(),
            arguments: args_str,
            progress_token: None,
            meta: None,
        }
    }

    /// Convert WIT call tool response to rmcp result
    pub fn convert_call_tool_to_rmcp(
        &self,
        response: crate::bindings::wasmcp::mcp::tools_types::CallToolResult,
    ) -> Result<CallToolResult> {
        // Convert WIT result to rmcp result
        let content = response
            .content
            .into_iter()
            .map(|c| {
                match c {
                    crate::bindings::wasmcp::mcp::mcp_types::ContentBlock::Text(t) => {
                        Content::text(t.text)
                    }
                    crate::bindings::wasmcp::mcp::mcp_types::ContentBlock::Image(i) => {
                        // Convert from Vec<u8> to base64 string
                        let data = base64::engine::general_purpose::STANDARD.encode(&i.data);
                        Content::image(data, i.mime_type)
                    }
                    crate::bindings::wasmcp::mcp::mcp_types::ContentBlock::Audio(a) => {
                        // Note: rmcp doesn't have direct audio support, convert to text with description
                        Content::text(format!(
                            "[Audio content: {} - {} bytes]",
                            a.mime_type,
                            a.data.len()
                        ))
                    }
                    crate::bindings::wasmcp::mcp::mcp_types::ContentBlock::ResourceLink(r) => {
                        // Create a resource link
                        use rmcp::model::RawResource;
                        Content::resource_link(RawResource {
                            uri: r.uri,
                            name: r.name,
                            description: r.description,
                            mime_type: r.mime_type,
                            size: r.size.map(|s| s as u32),
                        })
                    }
                    crate::bindings::wasmcp::mcp::mcp_types::ContentBlock::EmbeddedResource(e) => {
                        // Convert embedded resource
                        use rmcp::model::ResourceContents;
                        let resource_contents = match e.contents {
                            crate::bindings::wasmcp::mcp::mcp_types::ResourceContents::Text(t) => {
                                ResourceContents::TextResourceContents {
                                    uri: t.uri,
                                    mime_type: t.mime_type,
                                    text: t.text,
                                    meta: None,
                                }
                            }
                            crate::bindings::wasmcp::mcp::mcp_types::ResourceContents::Blob(b) => {
                                ResourceContents::BlobResourceContents {
                                    uri: b.uri,
                                    mime_type: b.mime_type,
                                    blob: base64::engine::general_purpose::STANDARD.encode(&b.blob),
                                    meta: None,
                                }
                            }
                        };
                        Content::resource(resource_contents)
                    }
                }
            })
            .collect::<Vec<_>>();

        // Convert meta fields from Vec<(String, String)> to Meta (JsonObject)
        let meta = response.meta.map(|fields| {
            let mut meta_obj = rmcp::model::Meta::new();
            for (key, value) in fields {
                // Parse the JSON string value back to a serde_json::Value
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&value) {
                    meta_obj.insert(key, parsed);
                } else {
                    // If it's not valid JSON, insert as a string
                    meta_obj.insert(key, serde_json::Value::String(value));
                }
            }
            meta_obj
        });

        // Convert structured_content from JSON string to Value
        let structured_content = response
            .structured_content
            .and_then(|json_str| serde_json::from_str(&json_str).ok());

        Ok(CallToolResult {
            content,
            structured_content,
            is_error: response.is_error,
            meta,
        })
    }

    /// Convert WIT ListToolsResponse to rmcp ListToolsResult
    pub fn convert_list_tools_to_rmcp(
        &self,
        response: crate::bindings::wasmcp::mcp::tools_types::ListToolsResult,
    ) -> Result<ListToolsResult> {
        let tools = response
            .tools
            .into_iter()
            .map(|t| {
                // Parse the JSON string schema
                let schema_value: Value =
                    serde_json::from_str(&t.input_schema).unwrap_or_else(|_| json!({}));

                let schema = match schema_value {
                    Value::Object(map) => map,
                    _ => serde_json::Map::new(),
                };

                Tool {
                    name: Cow::Owned(t.base.name),
                    description: t.description.map(Cow::Owned),
                    input_schema: Arc::new(schema),
                    output_schema: None,
                    annotations: None,
                }
            })
            .collect();

        Ok(ListToolsResult {
            tools,
            next_cursor: response.next_cursor,
        })
    }
}