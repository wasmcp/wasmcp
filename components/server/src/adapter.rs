use std::borrow::Cow;
use std::sync::Arc;
use anyhow::{anyhow, Result};
use rmcp::model::{
    Tool, CallToolResult, Content, Prompt, GetPromptResult, PromptMessage, 
    Resource, ReadResourceResult, ResourceContents, RawResource, PromptArgument,
    PromptMessageRole, PromptMessageContent, AnnotateAble
};
use serde_json::Value;

use crate::bindings::fastertools::mcp::{
    core,
    tool_handler,
    resource_handler,
    prompt_handler,
    session,
    tools,
    resources,
    prompts,
    types,
};

/// Adapter that bridges between rmcp types and WIT handler interfaces
pub struct WitMcpAdapter;

impl WitMcpAdapter {
    pub fn new() -> Self {
        Self
    }
    
    /// Get server info from WIT handlers
    pub fn get_server_info(&self) -> Result<rmcp::model::ServerInfo> {
        // Call the WIT initialize handler to get capabilities
        let request = session::InitializeRequest {
            protocol_version: "2025-06-18".to_string(),
            capabilities: session::ClientCapabilities {
                experimental: None,
                roots: None,
                sampling: None,
                elicitation: None,
            },
            client_info: session::ImplementationInfo {
                name: "wasmcp-server".to_string(),
                version: "0.1.0".to_string(),
                title: Some("WASMCP Server".to_string()),
            },
            meta: None,
        };
        
        let response = core::handle_initialize(&request)
            .map_err(|e| anyhow!("Initialize failed: {}", e.message))?;
        
        Ok(rmcp::model::ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::default(),
            capabilities: rmcp::model::ServerCapabilities {
                tools: if response.capabilities.tools.is_some() {
                    Some(rmcp::model::ToolsCapability {
                        list_changed: response.capabilities.tools
                            .and_then(|t| t.list_changed),
                    })
                } else {
                    None
                },
                resources: if response.capabilities.resources.is_some() {
                    Some(rmcp::model::ResourcesCapability {
                        subscribe: response.capabilities.resources
                            .as_ref()
                            .and_then(|r| r.subscribe),
                        list_changed: response.capabilities.resources
                            .and_then(|r| r.list_changed),
                    })
                } else {
                    None
                },
                prompts: if response.capabilities.prompts.is_some() {
                    Some(rmcp::model::PromptsCapability {
                        list_changed: response.capabilities.prompts
                            .and_then(|p| p.list_changed),
                    })
                } else {
                    None
                },
                ..Default::default()
            },
            server_info: rmcp::model::Implementation {
                name: response.server_info.name,
                version: response.server_info.version,
            },
            instructions: response.instructions,
        })
    }

    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        let request = tools::ListToolsRequest {
            cursor: None,
            progress_token: None,
            meta: None,
        };
        
        let response = tool_handler::handle_list_tools(&request)
            .map_err(|e| anyhow!("List tools failed: {}", e.message))?;
        
        // Convert WIT tools to rmcp tools
        let tools = response.tools
            .into_iter()
            .map(|t| {
                let schema_value: Value = serde_json::from_str(&t.input_schema)
                    .unwrap_or_else(|_| serde_json::json!({}));
                
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
        
        Ok(tools)
    }

    pub async fn call_tool(&self, name: &str, arguments: Option<serde_json::Map<String, Value>>) -> Result<CallToolResult> {
        // Convert arguments to JSON string for WIT interface
        let args_str = arguments.map(|args| serde_json::json!(args).to_string());
        
        let request = tools::CallToolRequest {
            name: name.to_string(),
            arguments: args_str,
            progress_token: None,
            meta: None,
        };
        
        let response = tool_handler::handle_call_tool(&request)
            .map_err(|e| anyhow!("Call tool failed: {}", e.message))?;
        
        // Convert WIT result to rmcp result
        let content = response.content
            .into_iter()
            .filter_map(|c| {
                match c {
                    types::ContentBlock::Text(t) => {
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

    pub async fn list_resources(&self) -> Result<Vec<Resource>> {
        let request = resources::ListResourcesRequest {
            cursor: None,
            progress_token: None,
            meta: None,
        };
        
        let response = resource_handler::handle_list_resources(&request)
            .map_err(|e| anyhow!("List resources failed: {}", e.message))?;
        
        // Convert WIT resources to rmcp resources
        let resources = response.resources
            .into_iter()
            .map(|r| RawResource {
                uri: r.base.name.clone(),
                name: r.base.title.unwrap_or(r.base.name),
                description: r.description,
                mime_type: r.mime_type,
                size: None,
            }.no_annotation())
            .collect();
        
        Ok(resources)
    }

    pub async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult> {
        let request = resources::ReadResourceRequest {
            uri: uri.to_string(),
            progress_token: None,
            meta: None,
        };
        
        let response = resource_handler::handle_read_resource(&request)
            .map_err(|e| anyhow!("Read resource failed: {}", e.message))?;
        
        // For simplicity, convert all contents to text
        // In a real implementation, we'd handle blob content properly
        let mut text_content = String::new();
        for content in response.contents {
            match content {
                types::ResourceContents::Text(t) => {
                    text_content.push_str(&t.text);
                    text_content.push('\n');
                },
                _ => {
                    // Skip non-text content for now
                }
            }
        }
        
        let resource_content = ResourceContents::TextResourceContents {
            uri: uri.to_string(),
            mime_type: Some("text/plain".to_string()),
            text: text_content,
        };
        
        Ok(ReadResourceResult {
            contents: vec![resource_content],
        })
    }

    pub async fn list_prompts(&self) -> Result<Vec<Prompt>> {
        let request = prompts::ListPromptsRequest {
            cursor: None,
            progress_token: None,
            meta: None,
        };
        
        let response = prompt_handler::handle_list_prompts(&request)
            .map_err(|e| anyhow!("List prompts failed: {}", e.message))?;
        
        // Convert WIT prompts to rmcp prompts
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
        
        Ok(prompts)
    }

    pub async fn get_prompt(&self, name: &str, arguments: Option<serde_json::Map<String, Value>>) -> Result<GetPromptResult> {
        // Convert arguments to WIT format (map of strings)
        let args_map = arguments
            .map(|args| {
                args.into_iter()
                    .map(|(k, v)| (k, v.to_string()))
                    .collect()
            });
        
        let request = prompts::GetPromptRequest {
            name: name.to_string(),
            arguments: args_map,
            progress_token: None,
            meta: None,
        };
        
        let response = prompt_handler::handle_get_prompt(&request)
            .map_err(|e| anyhow!("Get prompt failed: {}", e.message))?;
        
        // Convert WIT messages to rmcp messages
        let messages = response.messages
            .into_iter()
            .map(|m| {
                let role = match m.role {
                    prompts::Role::User => PromptMessageRole::User,
                    prompts::Role::Assistant => PromptMessageRole::Assistant,
                };
                
                let content_text = match m.content {
                    types::ContentBlock::Text(t) => t.text,
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