use std::borrow::Cow;
use std::sync::Arc;
use anyhow::{anyhow, Result};
use rmcp::model::{
    Tool, CallToolResult, Content, Prompt, GetPromptResult, PromptMessage, 
    Resource, ReadResourceResult, ResourceContents, RawResource, PromptArgument,
    PromptMessageRole, PromptMessageContent, AnnotateAble
};
use serde_json::Value;

use crate::bindings::wasmcp::mcp::handler;

/// Adapter that bridges between rmcp types and our WIT interface
/// This adapter doesn't use the rmcp handler traits since we implement ServerHandler directly,
/// but provides async methods that convert between WIT and rmcp types
#[derive(Clone)]
pub struct WitMcpAdapter;

impl WitMcpAdapter {
    pub fn new() -> Self {
        Self
    }

    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        // Call the WIT interface to get tools
        let wit_tools = handler::list_tools();
        
        // Convert WIT tools to rmcp tools
        let tools = wit_tools
            .into_iter()
            .map(|t| {
                let schema_value: Value = serde_json::from_str(&t.input_schema)
                    .unwrap_or_else(|_| serde_json::json!({}));
                
                let schema = match schema_value {
                    Value::Object(map) => map,
                    _ => serde_json::Map::new(),
                };
                
                Tool {
                    name: Cow::Owned(t.name),
                    description: Some(Cow::Owned(t.description)),
                    input_schema: Arc::new(schema),
                    output_schema: None,
                    annotations: None,
                }
            })
            .collect();
        
        Ok(tools)
    }

    pub async fn call_tool(&self, name: &str, arguments: Option<serde_json::Map<String, Value>>) -> Result<CallToolResult> {
        // Convert arguments to string for WIT interface
        let args_str = match arguments {
            Some(args) => serde_json::to_string(&args)?,
            None => "{}".to_string(),
        };
        
        // Call the WIT interface
        let result = handler::call_tool(name, &args_str);
        
        // Convert WIT result to rmcp result
        match result {
            handler::ToolResult::Text(text) => {
                Ok(CallToolResult {
                    content: Some(vec![Content::text(text)]),
                    structured_content: None,
                    is_error: None,
                })
            }
            handler::ToolResult::Error(err) => {
                Ok(CallToolResult {
                    content: Some(vec![Content::text(err.message)]),
                    structured_content: None,
                    is_error: Some(true),
                })
            }
        }
    }

    pub async fn list_resources(&self) -> Result<Vec<Resource>> {
        // Call the WIT interface to get resources
        let wit_resources = handler::list_resources();
        
        // Convert WIT resources to rmcp resources
        let resources = wit_resources
            .into_iter()
            .map(|r| RawResource {
                uri: r.uri,
                name: r.name,
                description: r.description,
                mime_type: r.mime_type,
                size: None,
            }.no_annotation())
            .collect();
        
        Ok(resources)
    }

    pub async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult> {
        // Call the WIT interface
        let result = handler::read_resource(uri);
        
        // Convert WIT result to rmcp result
        match result {
            handler::ResourceResult::Contents(contents) => {
                let resource_content = if let Some(text) = contents.text {
                    ResourceContents::TextResourceContents {
                        uri: contents.uri,
                        mime_type: contents.mime_type,
                        text,
                    }
                } else if let Some(blob) = contents.blob {
                    ResourceContents::BlobResourceContents {
                        uri: contents.uri,
                        mime_type: contents.mime_type,
                        blob: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, blob),
                    }
                } else {
                    ResourceContents::TextResourceContents {
                        uri: contents.uri,
                        mime_type: contents.mime_type,
                        text: String::new(),
                    }
                };
                
                Ok(ReadResourceResult {
                    contents: vec![resource_content],
                })
            }
            handler::ResourceResult::Error(err) => {
                Err(anyhow!("Resource error ({}): {}", err.code, err.message))
            }
        }
    }

    pub async fn list_prompts(&self) -> Result<Vec<Prompt>> {
        // Call the WIT interface to get prompts
        let wit_prompts = handler::list_prompts();
        
        // Convert WIT prompts to rmcp prompts
        let prompts = wit_prompts
            .into_iter()
            .map(|p| {
                let arguments = p.arguments
                    .into_iter()
                    .map(|a| PromptArgument {
                        name: a.name,
                        description: a.description,
                        required: Some(a.required),
                    })
                    .collect();
                
                Prompt {
                    name: p.name,
                    description: p.description,
                    arguments: Some(arguments),
                }
            })
            .collect();
        
        Ok(prompts)
    }

    pub async fn get_prompt(&self, name: &str, arguments: Option<serde_json::Map<String, Value>>) -> Result<GetPromptResult> {
        // Convert arguments to string for WIT interface
        let args_str = match arguments {
            Some(args) => serde_json::to_string(&args)?,
            None => "{}".to_string(),
        };
        
        // Call the WIT interface
        let result = handler::get_prompt(name, &args_str);
        
        // Convert WIT result to rmcp result
        match result {
            handler::PromptResult::Messages(messages) => {
                let prompt_messages = messages
                    .into_iter()
                    .map(|m| {
                        let role = match m.role.as_str() {
                            "assistant" => PromptMessageRole::Assistant,
                            _ => PromptMessageRole::User,
                        };
                        PromptMessage {
                            role,
                            content: PromptMessageContent::text(m.content),
                        }
                    })
                    .collect();
                
                Ok(GetPromptResult {
                    messages: prompt_messages,
                    description: None,
                })
            }
            handler::PromptResult::Error(err) => {
                Err(anyhow!("Prompt error ({}): {}", err.code, err.message))
            }
        }
    }
}