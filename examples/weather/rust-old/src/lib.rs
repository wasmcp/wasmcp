mod bindings;

pub struct Component;

// Manually implement all the traits using raw bindings
impl bindings::exports::fastertools::mcp::core::Guest for Component {
    fn handle_initialize(_request: bindings::fastertools::mcp::session::InitializeRequest) 
        -> Result<bindings::fastertools::mcp::session::InitializeResponse, bindings::fastertools::mcp::types::McpError> {
        Ok(bindings::fastertools::mcp::session::InitializeResponse {
            protocol_version: "2025-06-18".to_string(),
            capabilities: bindings::fastertools::mcp::session::ServerCapabilities {
                tools: Some(bindings::fastertools::mcp::session::ToolsCapability { 
                    list_changed: Some(false) 
                }),
                resources: None,
                prompts: None,
                experimental: None,
                logging: None,
                completions: None,
            },
            server_info: bindings::fastertools::mcp::session::ImplementationInfo {
                name: "rust_weather".to_string(),
                version: "0.1.0".to_string(),
                title: Some("Rust Weather Example".to_string()),
            },
            instructions: None,
            meta: None,
        })
    }
    
    fn handle_initialized() -> Result<(), bindings::fastertools::mcp::types::McpError> {
        Ok(())
    }
    
    fn handle_ping() -> Result<(), bindings::fastertools::mcp::types::McpError> {
        Ok(())
    }
    
    fn handle_shutdown() -> Result<(), bindings::fastertools::mcp::types::McpError> {
        Ok(())
    }
}

impl bindings::exports::fastertools::mcp::tool_handler::Guest for Component {
    fn handle_list_tools(_request: bindings::fastertools::mcp::tools::ListToolsRequest) 
        -> Result<bindings::fastertools::mcp::tools::ListToolsResponse, bindings::fastertools::mcp::types::McpError> {
        Ok(bindings::fastertools::mcp::tools::ListToolsResponse {
            tools: vec![
                bindings::fastertools::mcp::tools::Tool {
                    base: bindings::fastertools::mcp::types::BaseMetadata {
                        name: "echo".to_string(),
                        title: Some("Echo Tool".to_string()),
                    },
                    description: Some("Echo a message back to the user".to_string()),
                    input_schema: r#"{"type": "object", "properties": {"message": {"type": "string"}}, "required": ["message"]}"#.to_string(),
                    output_schema: None,
                    annotations: None,
                    meta: None,
                },
                bindings::fastertools::mcp::tools::Tool {
                    base: bindings::fastertools::mcp::types::BaseMetadata {
                        name: "get_weather".to_string(),
                        title: Some("Weather Tool".to_string()),
                    },
                    description: Some("Get weather information for a location".to_string()),
                    input_schema: r#"{"type": "object", "properties": {"location": {"type": "string"}}, "required": ["location"]}"#.to_string(),
                    output_schema: None,
                    annotations: None,
                    meta: None,
                },
            ],
            next_cursor: None,
            meta: None,
        })
    }
    
    fn handle_call_tool(request: bindings::fastertools::mcp::tools::CallToolRequest) 
        -> Result<bindings::fastertools::mcp::tools::ToolResult, bindings::fastertools::mcp::types::McpError> {
        let args = if let Some(args_str) = &request.arguments {
            serde_json::from_str::<serde_json::Value>(args_str)
                .map_err(|e| bindings::fastertools::mcp::types::McpError {
                    code: bindings::fastertools::mcp::types::ErrorCode::InvalidParams,
                    message: format!("Invalid arguments: {}", e),
                    data: None,
                })?
        } else {
            serde_json::Value::Object(serde_json::Map::new())
        };
        
        match request.name.as_str() {
            "echo" => {
                let message = args["message"].as_str()
                    .ok_or_else(|| bindings::fastertools::mcp::types::McpError {
                        code: bindings::fastertools::mcp::types::ErrorCode::InvalidParams,
                        message: "Missing message field".to_string(),
                        data: None,
                    })?;
                
                Ok(bindings::fastertools::mcp::tools::ToolResult {
                    content: vec![bindings::fastertools::mcp::types::ContentBlock::Text(
                        bindings::fastertools::mcp::types::TextContent {
                            text: format!("Echo: {}", message),
                            annotations: None,
                            meta: None,
                        }
                    )],
                    is_error: Some(false),
                    structured_content: None,
                    meta: None,
                })
            },
            "get_weather" => {
                let location = args["location"].as_str()
                    .ok_or_else(|| bindings::fastertools::mcp::types::McpError {
                        code: bindings::fastertools::mcp::types::ErrorCode::InvalidParams,
                        message: "Missing location field".to_string(),
                        data: None,
                    })?;
                
                // For now, just return a static response to test
                Ok(bindings::fastertools::mcp::tools::ToolResult {
                    content: vec![bindings::fastertools::mcp::types::ContentBlock::Text(
                        bindings::fastertools::mcp::types::TextContent {
                            text: format!("Weather for {}: 20°C, Sunny", location),
                            annotations: None,
                            meta: None,
                        }
                    )],
                    is_error: Some(false),
                    structured_content: None,
                    meta: None,
                })
            },
            _ => Err(bindings::fastertools::mcp::types::McpError {
                code: bindings::fastertools::mcp::types::ErrorCode::ToolNotFound,
                message: format!("Unknown tool: {}", request.name),
                data: None,
            })
        }
    }
}

bindings::export!(Component with_types_in bindings);