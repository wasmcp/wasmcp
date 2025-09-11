use crate::error::McpError;
use crate::protocol::types::{InitializeResponse, ServerCapabilities};
use std::sync::Arc;
use std::borrow::Cow;

/// Protocol adapter for converting between internal types and rmcp types
pub struct ProtocolAdapter;

impl ProtocolAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Convert internal InitializeResponse to rmcp ServerInfo
    pub fn initialize_to_rmcp(
        &self,
        response: InitializeResponse,
    ) -> Result<rmcp::model::ServerInfo, McpError> {
        use rmcp::model::ServerInfo;

        Ok(ServerInfo {
            protocol_version: response.protocol_version.to_rmcp(),
            capabilities: self.capabilities_to_rmcp(response.capabilities),
            server_info: rmcp::model::Implementation {
                name: response.server_info.name,
                version: response.server_info.version,
            },
            instructions: response.instructions,
        })
    }

    /// Convert internal ServerCapabilities to rmcp ServerCapabilities
    fn capabilities_to_rmcp(&self, caps: ServerCapabilities) -> rmcp::model::ServerCapabilities {
        rmcp::model::ServerCapabilities {
            #[cfg(feature = "tools")]
            tools: caps.tools.map(|_| rmcp::model::ToolsCapability {
                list_changed: Some(false),
            }),
            #[cfg(not(feature = "tools"))]
            tools: None,

            #[cfg(feature = "resources")]
            resources: caps.resources.map(|r| rmcp::model::ResourcesCapability {
                subscribe: r.subscribe,
                list_changed: r.list_changed,
            }),
            #[cfg(not(feature = "resources"))]
            resources: None,

            #[cfg(feature = "prompts")]
            prompts: caps.prompts.map(|_| rmcp::model::PromptsCapability {
                list_changed: Some(false),
            }),
            #[cfg(not(feature = "prompts"))]
            prompts: None,

            ..Default::default()
        }
    }
}

// Tools feature conversions
#[cfg(feature = "tools")]
impl ProtocolAdapter {
    /// Convert internal ContentBlock to rmcp Content
    pub fn content_block_to_rmcp(
        &self,
        block: crate::protocol::types::ContentBlock,
    ) -> rmcp::model::Content {
        use crate::protocol::types::ContentBlock;
        use rmcp::model::Content;

        match block {
            ContentBlock::Text { text } => Content::text(text),
            ContentBlock::Image { data, mime_type } => {
                use base64::Engine;
                let encoded = base64::engine::general_purpose::STANDARD.encode(&data);
                Content::image(encoded, mime_type)
            }
            ContentBlock::Audio { data, mime_type } => {
                // rmcp doesn't have direct audio support, convert to text with description
                Content::text(format!(
                    "[Audio content: {} - {} bytes]",
                    mime_type,
                    data.len()
                ))
            }
            #[cfg(feature = "resources")]
            ContentBlock::ResourceLink {
                uri,
                name,
                description,
                mime_type,
                size,
            } => {
                use rmcp::model::RawResource;
                Content::resource_link(RawResource {
                    uri,
                    name,
                    description,
                    mime_type,
                    size: size.map(|s| s as u32),
                })
            }
            #[cfg(feature = "resources")]
            ContentBlock::EmbeddedResource { contents } => {
                use crate::protocol::types::ResourceContents;
                use rmcp::model::ResourceContents as RmcpResourceContents;
                
                let resource_contents = match contents {
                    ResourceContents::Text { uri, mime_type, text } => {
                        RmcpResourceContents::TextResourceContents {
                            uri,
                            mime_type,
                            text,
                            meta: None,
                        }
                    }
                    ResourceContents::Blob { uri, mime_type, blob } => {
                        use base64::Engine;
                        RmcpResourceContents::BlobResourceContents {
                            uri,
                            mime_type,
                            blob: base64::engine::general_purpose::STANDARD.encode(&blob),
                            meta: None,
                        }
                    }
                };
                Content::resource(resource_contents)
            }
        }
    }

    /// Convert internal CallToolResponse to rmcp CallToolResult
    pub fn call_tool_response_to_rmcp(
        &self,
        response: crate::protocol::types::tools::CallToolResponse,
    ) -> Result<rmcp::model::CallToolResult, McpError> {
        let content = response
            .content
            .into_iter()
            .map(|c| self.content_block_to_rmcp(c))
            .collect();

        let meta = response.meta.map(|fields| {
            let mut meta_obj = rmcp::model::Meta::new();
            for (key, value) in fields {
                meta_obj.insert(key, serde_json::Value::String(value));
            }
            meta_obj
        });

        Ok(rmcp::model::CallToolResult {
            content,
            structured_content: response.structured_content,
            is_error: Some(response.is_error),
            meta,
        })
    }

    /// Convert internal ListToolsResponse to rmcp ListToolsResult
    pub fn list_tools_response_to_rmcp(
        &self,
        response: crate::protocol::types::tools::ListToolsResponse,
    ) -> Result<rmcp::model::ListToolsResult, McpError> {
        let tools = response
            .tools
            .into_iter()
            .map(|t| {
                let schema = match t.input_schema {
                    serde_json::Value::Object(map) => map,
                    _ => serde_json::Map::new(),
                };

                rmcp::model::Tool {
                    name: Cow::Owned(t.name),
                    description: t.description.map(Cow::Owned),
                    input_schema: Arc::new(schema),
                    output_schema: None,
                    annotations: None,
                }
            })
            .collect();

        Ok(rmcp::model::ListToolsResult {
            tools,
            next_cursor: response.next_cursor,
        })
    }
}

// Resources feature conversions
#[cfg(feature = "resources")]
impl ProtocolAdapter {
    /// Convert internal ListResourcesResponse to rmcp ListResourcesResult
    pub fn list_resources_response_to_rmcp(
        &self,
        response: crate::protocol::types::resources::ListResourcesResponse,
    ) -> Result<rmcp::model::ListResourcesResult, McpError> {
        let resources = response
            .resources
            .into_iter()
            .map(|r| rmcp::model::RawResource {
                uri: r.uri,
                name: r.name,
                description: r.description,
                mime_type: r.mime_type,
                size: r.size.map(|s| s as u32),
            })
            .collect();

        Ok(rmcp::model::ListResourcesResult {
            resources,
            next_cursor: response.next_cursor,
        })
    }

    /// Convert internal ReadResourceResponse to rmcp ReadResourceResult
    pub fn read_resource_response_to_rmcp(
        &self,
        response: crate::protocol::types::resources::ReadResourceResponse,
    ) -> Result<rmcp::model::ReadResourceResult, McpError> {
        use crate::protocol::types::ResourceContents;
        use rmcp::model::ResourceContents as RmcpResourceContents;
        use base64::Engine;
        
        let contents = match response.contents {
            ResourceContents::Text { uri, mime_type, text } => {
                RmcpResourceContents::TextResourceContents {
                    uri,
                    mime_type,
                    text,
                    meta: None,
                }
            }
            ResourceContents::Blob { uri, mime_type, blob } => {
                use base64::Engine;
                RmcpResourceContents::BlobResourceContents {
                    uri,
                    mime_type,
                    blob: base64::engine::general_purpose::STANDARD.encode(&blob),
                    meta: None,
                }
            }
        };

        Ok(rmcp::model::ReadResourceResult {
            contents,
            annotations: None,
        })
    }
}

// Prompts feature conversions
#[cfg(feature = "prompts")]
impl ProtocolAdapter {
    /// Convert internal ListPromptsResponse to rmcp ListPromptsResult
    pub fn list_prompts_response_to_rmcp(
        &self,
        response: crate::protocol::types::prompts::ListPromptsResponse,
    ) -> Result<rmcp::model::ListPromptsResult, McpError> {
        let prompts = response
            .prompts
            .into_iter()
            .map(|p| {
                let arguments = p
                    .arguments
                    .into_iter()
                    .map(|arg| rmcp::model::PromptArgument {
                        name: Cow::Owned(arg.name),
                        description: arg.description.map(Cow::Owned),
                        required: Some(arg.required),
                    })
                    .collect();

                rmcp::model::Prompt {
                    name: Cow::Owned(p.name),
                    description: p.description.map(Cow::Owned),
                    arguments: Some(arguments),
                    annotations: None,
                }
            })
            .collect();

        Ok(rmcp::model::ListPromptsResult {
            prompts,
            next_cursor: response.next_cursor,
        })
    }

    /// Convert internal GetPromptResponse to rmcp GetPromptResult
    pub fn get_prompt_response_to_rmcp(
        &self,
        response: crate::protocol::types::prompts::GetPromptResponse,
    ) -> Result<rmcp::model::GetPromptResult, McpError> {
        use crate::protocol::types::prompts::PromptMessageRole;
        
        let messages = response
            .messages
            .into_iter()
            .map(|msg| {
                let role = match msg.role {
                    PromptMessageRole::User => rmcp::model::PromptMessageRole::User,
                    PromptMessageRole::Assistant => rmcp::model::PromptMessageRole::Assistant,
                    PromptMessageRole::System => rmcp::model::PromptMessageRole::System,
                };

                let content = msg
                    .content
                    .into_iter()
                    .map(|c| self.content_block_to_prompt_content(c))
                    .collect();

                rmcp::model::PromptMessage {
                    role,
                    content,
                }
            })
            .collect();

        Ok(rmcp::model::GetPromptResult {
            messages,
            description: None,
        })
    }

    fn content_block_to_prompt_content(
        &self,
        block: crate::protocol::types::ContentBlock,
    ) -> rmcp::model::PromptMessageContent {
        use crate::protocol::types::ContentBlock;
        use rmcp::model::PromptMessageContent;

        match block {
            ContentBlock::Text { text } => PromptMessageContent::text(text),
            ContentBlock::Image { data, mime_type } => {
                use base64::Engine;
                let encoded = base64::engine::general_purpose::STANDARD.encode(&data);
                PromptMessageContent::image(encoded, mime_type)
            }
            _ => PromptMessageContent::text("[Unsupported content type]".to_string()),
        }
    }
}