mod bindings;
mod helpers;

use helpers::{Error, ResultBuilder, ToolBuilder, parse_args, get_required_string};

pub struct Component;

// Implement the core interface for initialization
impl bindings::exports::fastertools::mcp::core::Guest for Component {
    fn handle_initialize(_request: bindings::fastertools::mcp::session::InitializeRequest) 
        -> Result<bindings::fastertools::mcp::session::InitializeResponse, bindings::fastertools::mcp::types::McpError> {
        Ok(bindings::fastertools::mcp::session::InitializeResponse {
            protocol_version: "2025-06-18".to_string(),
            capabilities: bindings::fastertools::mcp::session::ServerCapabilities {
                tools: Some(bindings::fastertools::mcp::session::ToolsCapability { 
                    list_changed: Some(false) 
                }),
                // Not implementing resources or prompts - null components will handle these
                resources: None,
                prompts: None,
                experimental: None,
                logging: None,
                completions: None,
            },
            server_info: bindings::fastertools::mcp::session::ImplementationInfo {
                name: "rust_helpers".to_string(),
                version: "0.1.0".to_string(),
                title: Some("rust-helpers Handler".to_string()),
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

// Implement the tool handler interface
impl bindings::exports::fastertools::mcp::tool_handler::Guest for Component {
    fn handle_list_tools(_request: bindings::fastertools::mcp::tools::ListToolsRequest) 
        -> Result<bindings::fastertools::mcp::tools::ListToolsResponse, bindings::fastertools::mcp::types::McpError> {
        
        // Using helpers to build tool definitions - cleaner and less boilerplate
        let tools = vec![
            ToolBuilder::new("echo")
                .description("Echo a message back to the user")
                .param("message", "string")
                .required("message")
                .build(),
                
            ToolBuilder::new("get_weather")
                .description("Get weather information for a location")
                .param("location", "string")
                .required("location")
                .build(),
            // Add more tools here
        ];
        
        Ok(bindings::fastertools::mcp::tools::ListToolsResponse {
            tools,
            next_cursor: None,
            meta: None,
        })
    }
    
    fn handle_call_tool(request: bindings::fastertools::mcp::tools::CallToolRequest) 
        -> Result<bindings::fastertools::mcp::tools::ToolResult, bindings::fastertools::mcp::types::McpError> {
        
        // Using helper to parse arguments
        let args = parse_args(&request.arguments)?;
        
        match request.name.as_str() {
            "echo" => {
                // Using helper to get required field
                let message = get_required_string(&args, "message")?;
                
                // Using helper to build result
                Ok(ResultBuilder::success()
                    .text(format!("Echo: {}", message))
                    .build())
            },
            
            "get_weather" => {
                let location = get_required_string(&args, "location")?;
                
                // Simple mock weather response - replace with real API call
                Ok(ResultBuilder::success()
                    .text(format!("Weather for {}: 20°C, Sunny", location))
                    .build())
            },
            
            _ => {
                // Using helper for error
                Err(Error::tool_not_found(&request.name))
            }
        }
    }
}

// Export the component
// The WIT world (tools-handler) determines what interfaces are required
// Resources and prompts will be provided by null components during composition
bindings::export!(Component with_types_in bindings);