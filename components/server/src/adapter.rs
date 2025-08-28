use serde_json::{json, Value};
use anyhow::{anyhow, Result};
use std::borrow::Cow;
use std::sync::Arc;

#[cfg(any(feature = "resources", feature = "prompts"))]
use base64::Engine;

// Import rmcp types for protocol compliance
#[cfg(feature = "tools")]
use rmcp::model::{
    Tool, CallToolResult, Content, ListToolsResult,
};

#[cfg(feature = "resources")]
use rmcp::model::{
    ListResourcesResult, ReadResourceResult, ResourceContents, RawResource,
    AnnotateAble,
};

#[cfg(feature = "prompts")]
use rmcp::model::{
    ListPromptsResult, Prompt, GetPromptResult,
    PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent,
};

/// Adapter that converts between WIT types and rmcp types for protocol compliance
pub struct WitMcpAdapter;

impl WitMcpAdapter {
    pub fn new() -> Self {
        Self
    }
}

// Tools feature methods
#[cfg(feature = "tools")]
impl WitMcpAdapter {
    /// Call a tool using WIT handler interface
    pub async fn call_tool(&self, name: &str, arguments: Option<serde_json::Map<String, Value>>) -> Result<CallToolResult> {
        // Convert arguments Map directly to JSON string for WIT interface
        let args_str = arguments.map(|args| serde_json::to_string(&args).unwrap());
        
        let request = crate::bindings::fastertools::mcp::tools::CallToolRequest {
            name: name.to_string(),
            arguments: args_str,
            progress_token: None,
            meta: None,
        };
        
        let response = crate::bindings::fastertools::mcp::tool_handler::handle_call_tool(&request)
            .map_err(|e| anyhow!("Call tool failed: {}", e.message))?;
        
        // Convert WIT result to rmcp result
        let content = response.content
            .into_iter()
            .filter_map(|c| {
                match c {
                    crate::bindings::fastertools::mcp::types::ContentBlock::Text(t) => {
                        Some(Content::text(t.text))
                    },
                    _ => None, // Skip non-text content for now
                }
            })
            .collect::<Vec<_>>();
        
        Ok(CallToolResult {
            content: if content.is_empty() { None } else { Some(content) },
            structured_content: None,
            is_error: response.is_error,
        })
    }
    
    /// Convert WIT ListToolsResponse to rmcp ListToolsResult
    pub fn convert_list_tools_to_rmcp(&self, response: crate::bindings::fastertools::mcp::tools::ListToolsResponse) -> Result<ListToolsResult> {
        let tools = response.tools
            .into_iter()
            .map(|t| {
                // Parse the JSON string schema
                let schema_value: Value = serde_json::from_str(&t.input_schema)
                    .unwrap_or_else(|_| json!({}));
                
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
    pub fn convert_list_resources_to_rmcp(&self, response: crate::bindings::fastertools::mcp::resources::ListResourcesResponse) -> Result<ListResourcesResult> {
        let resources = response.resources
            .into_iter()
            .map(|r| {
                RawResource {
                    uri: r.uri.clone(),
                    name: r.base.title.unwrap_or(r.base.name),
                    description: r.description,
                    mime_type: r.mime_type,
                    size: None,
                }.no_annotation()
            })
            .collect();
        
        Ok(ListResourcesResult {
            resources,
            next_cursor: response.next_cursor,
        })
    }
    
    /// Convert WIT ReadResourceResponse to rmcp ReadResourceResult
    pub fn convert_read_resource_to_rmcp(&self, response: crate::bindings::fastertools::mcp::resources::ReadResourceResponse) -> Result<ReadResourceResult> {
        use crate::bindings::fastertools::mcp::types::ResourceContents as WitResourceContents;
        
        let contents = response.contents
            .into_iter()
            .map(|c| {
                match c {
                    WitResourceContents::Text(text) => {
                        ResourceContents::TextResourceContents {
                            uri: text.uri,
                            mime_type: text.mime_type,
                            text: text.text,
                        }
                    },
                    WitResourceContents::Blob(blob) => {
                        ResourceContents::BlobResourceContents {
                            uri: blob.uri,
                            mime_type: blob.mime_type,
                            blob: base64::engine::general_purpose::STANDARD.encode(&blob.blob),
                        }
                    }
                }
            })
            .collect();
        
        Ok(ReadResourceResult { contents })
    }
}

// Prompts feature methods
#[cfg(feature = "prompts")]
impl WitMcpAdapter {
    /// Convert WIT ListPromptsResponse to rmcp ListPromptsResult
    pub fn convert_list_prompts_to_rmcp(&self, response: crate::bindings::fastertools::mcp::prompts::ListPromptsResponse) -> Result<ListPromptsResult> {
        let prompts = response.prompts
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
    pub fn convert_get_prompt_to_rmcp(&self, response: crate::bindings::fastertools::mcp::prompts::GetPromptResponse) -> Result<GetPromptResult> {
        use crate::bindings::fastertools::mcp::types::{MessageRole, ContentBlock};
        
        let messages = response.messages
            .into_iter()
            .map(|m| {
                let role = match m.role {
                    MessageRole::User => PromptMessageRole::User,
                    MessageRole::Assistant => PromptMessageRole::Assistant,
                    MessageRole::System => PromptMessageRole::System,
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