use std::borrow::Cow;
use std::sync::Arc;

use anyhow::{Result, anyhow};
#[cfg(any(feature = "resources", feature = "prompts", feature = "tools"))]
use base64::Engine;
#[cfg(feature = "resources")]
use rmcp::model::{
    AnnotateAble, ListResourcesResult, RawResource, ReadResourceResult, ResourceContents,
};
// Import rmcp types for protocol compliance
#[cfg(feature = "tools")]
use rmcp::model::{CallToolResult, Content, ListToolsResult, Tool};
#[cfg(feature = "prompts")]
use rmcp::model::{
    GetPromptResult, ListPromptsResult, Prompt, PromptArgument, PromptMessage,
    PromptMessageContent, PromptMessageRole,
};
use serde_json::{Value, json};

/// Adapter that converts between WIT types and rmcp types for protocol compliance
pub struct WitMcpAdapter;

impl WitMcpAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Convert WIT protocol version enum to actual protocol version string
    fn convert_protocol_version(
        &self,
        version: crate::bindings::wasmcp::mcp::core_types::ProtocolVersion,
    ) -> rmcp::model::ProtocolVersion {
        use rmcp::model::ProtocolVersion;

        use crate::bindings::wasmcp::mcp::core_types::ProtocolVersion as WitVersion;

        match version {
            WitVersion::V20250326 => ProtocolVersion::V_2025_03_26,
            WitVersion::V20250618 => ProtocolVersion::V_2025_06_18,
        }
    }

    /// Convert WIT InitializeResponse to rmcp ServerInfo
    pub fn convert_initialize_to_rmcp(
        &self,
        response: crate::bindings::wasmcp::mcp::core_types::InitializeResponse,
    ) -> Result<rmcp::model::ServerInfo> {
        use rmcp::model::{Implementation, ServerCapabilities, ServerInfo};

        Ok(ServerInfo {
            protocol_version: self.convert_protocol_version(response.protocol_version),
            capabilities: ServerCapabilities {
                #[cfg(feature = "tools")]
                tools: response
                    .capabilities
                    .tools
                    .map(|_| rmcp::model::ToolsCapability {
                        list_changed: Some(false),
                    }),
                #[cfg(not(feature = "tools"))]
                tools: None,

                #[cfg(feature = "resources")]
                resources: response.capabilities.resources.map(|_| {
                    rmcp::model::ResourcesCapability {
                        subscribe: None,
                        list_changed: Some(false),
                    }
                }),
                #[cfg(not(feature = "resources"))]
                resources: None,

                #[cfg(feature = "prompts")]
                prompts: response
                    .capabilities
                    .prompts
                    .map(|_| rmcp::model::PromptsCapability {
                        list_changed: Some(false),
                    }),
                #[cfg(not(feature = "prompts"))]
                prompts: None,

                ..Default::default()
            },
            server_info: Implementation {
                name: response.server_info.name,
                version: response.server_info.version,
            },
            instructions: response.instructions,
        })
    }
}

// Tools feature methods
#[cfg(feature = "tools")]
impl WitMcpAdapter {
    /// Call a tool using WIT handler interface
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Option<serde_json::Map<String, Value>>,
        auth_context: Option<&crate::bindings::wasmcp::mcp::authorization_types::AuthContext>,
    ) -> Result<CallToolResult> {
        // Convert arguments Map directly to JSON string for WIT interface
        let args_str = arguments.map(|args| serde_json::to_string(&args).unwrap());

        let request = crate::bindings::wasmcp::mcp::tool_types::CallToolRequest {
            name: name.to_string(),
            arguments: args_str,
            progress_token: None,
            meta: None,
        };

        let response = crate::bindings::wasmcp::mcp::tools_capabilities::handle_call_tool(&request, auth_context)
            .map_err(|e| anyhow!("Call tool failed: {}", e.message))?;

        // Convert WIT result to rmcp result
        let content = response
            .content
            .into_iter()
            .map(|c| {
                match c {
                    crate::bindings::wasmcp::mcp::types::ContentBlock::Text(t) => {
                        Content::text(t.text)
                    }
                    crate::bindings::wasmcp::mcp::types::ContentBlock::Image(i) => {
                        // Convert from Vec<u8> to base64 string
                        let data = base64::engine::general_purpose::STANDARD.encode(&i.data);
                        Content::image(data, i.mime_type)
                    }
                    crate::bindings::wasmcp::mcp::types::ContentBlock::Audio(a) => {
                        // Note: rmcp doesn't have direct audio support, convert to text with description
                        // Could also consider using embedded resource for audio data
                        let _data = base64::engine::general_purpose::STANDARD.encode(&a.data);
                        Content::text(format!(
                            "[Audio content: {} - {} bytes]",
                            a.mime_type,
                            a.data.len()
                        ))
                    }
                    crate::bindings::wasmcp::mcp::types::ContentBlock::ResourceLink(r) => {
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
                    crate::bindings::wasmcp::mcp::types::ContentBlock::EmbeddedResource(e) => {
                        // Convert embedded resource
                        use rmcp::model::ResourceContents;
                        let resource_contents = match e.contents {
                            crate::bindings::wasmcp::mcp::types::ResourceContents::Text(t) => {
                                ResourceContents::TextResourceContents {
                                    uri: t.uri,
                                    mime_type: t.mime_type,
                                    text: t.text,
                                    meta: None,
                                }
                            }
                            crate::bindings::wasmcp::mcp::types::ResourceContents::Blob(b) => {
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
        response: crate::bindings::wasmcp::mcp::tool_types::ListToolsResponse,
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

// Resources feature methods
#[cfg(feature = "resources")]
impl WitMcpAdapter {
    /// Convert WIT ListResourcesResponse to rmcp ListResourcesResult
    pub fn convert_list_resources_to_rmcp(
        &self,
        response: crate::bindings::wasmcp::mcp::resource_types::ListResourcesResponse,
    ) -> Result<ListResourcesResult> {
        let resources = response
            .resources
            .into_iter()
            .map(|r| {
                RawResource {
                    uri: r.uri.clone(),
                    name: r.base.title.unwrap_or(r.base.name),
                    description: r.description,
                    mime_type: r.mime_type,
                    size: None,
                }
                .no_annotation()
            })
            .collect();

        Ok(ListResourcesResult {
            resources,
            next_cursor: response.next_cursor,
        })
    }

    /// Convert WIT ReadResourceResponse to rmcp ReadResourceResult
    pub fn convert_read_resource_to_rmcp(
        &self,
        response: crate::bindings::wasmcp::mcp::resource_types::ReadResourceResponse,
    ) -> Result<ReadResourceResult> {
        use crate::bindings::wasmcp::mcp::types::ResourceContents as WitResourceContents;

        let contents = response
            .contents
            .into_iter()
            .map(|c| match c {
                WitResourceContents::Text(text) => ResourceContents::TextResourceContents {
                    uri: text.uri,
                    mime_type: text.mime_type,
                    text: text.text,
                },
                WitResourceContents::Blob(blob) => ResourceContents::BlobResourceContents {
                    uri: blob.uri,
                    mime_type: blob.mime_type,
                    blob: base64::engine::general_purpose::STANDARD.encode(&blob.blob),
                },
            })
            .collect();

        Ok(ReadResourceResult { contents })
    }
}

// Prompts feature methods
#[cfg(feature = "prompts")]
impl WitMcpAdapter {
    /// Convert WIT ListPromptsResponse to rmcp ListPromptsResult
    pub fn convert_list_prompts_to_rmcp(
        &self,
        response: crate::bindings::wasmcp::mcp::prompt_types::ListPromptsResponse,
    ) -> Result<ListPromptsResult> {
        let prompts = response
            .prompts
            .into_iter()
            .map(|p| {
                let arguments = p.arguments.map(|args| {
                    args.into_iter()
                        .map(|a| PromptArgument {
                            name: a.base.name,
                            description: a.description,
                            required: a.required,
                        })
                        .collect()
                });

                Prompt {
                    name: p.base.name,
                    description: p.description,
                    arguments,
                }
            })
            .collect();

        Ok(ListPromptsResult {
            prompts,
            next_cursor: response.next_cursor,
        })
    }

    /// Convert WIT GetPromptResponse to rmcp GetPromptResult
    pub fn convert_get_prompt_to_rmcp(
        &self,
        response: crate::bindings::wasmcp::mcp::prompt_types::GetPromptResponse,
    ) -> Result<GetPromptResult> {
        use crate::bindings::wasmcp::mcp::types::{ContentBlock, MessageRole};

        let messages = response
            .messages
            .into_iter()
            .map(|m| {
                let role = match m.role {
                    MessageRole::User => PromptMessageRole::User,
                    MessageRole::Assistant => PromptMessageRole::Assistant,
                    MessageRole::System => PromptMessageRole::Assistant, /* Map System to
                                                                          * Assistant for rmcp
                                                                          * compatibility */
                };

                let content_text = match m.content {
                    ContentBlock::Text(t) => t.text,
                    _ => String::new(), // Skip non-text content
                };

                PromptMessage {
                    role,
                    content: PromptMessageContent::text(content_text),
                }
            })
            .collect();

        Ok(GetPromptResult {
            messages,
            description: response.description,
        })
    }
}
