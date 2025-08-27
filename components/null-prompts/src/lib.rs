/// Null prompts implementation - provides empty prompt capabilities
/// This component satisfies prompt-handler requirements with no-op implementations

#[allow(warnings)]
mod bindings;

use bindings::exports::fastertools::mcp::{core, prompt_handler};
use bindings::fastertools::mcp::{
    prompts::{
        ListPromptsRequest, ListPromptsResponse,
        GetPromptRequest, GetPromptResponse,
    },
    session::{InitializeRequest, InitializeResponse, ServerCapabilities, ImplementationInfo},
    types::{McpError, ErrorCode},
};

pub struct Component;

// Implement core interface
impl core::Guest for Component {
    fn handle_initialize(_request: InitializeRequest) -> Result<InitializeResponse, McpError> {
        Ok(InitializeResponse {
            protocol_version: "2025-06-18".to_string(),
            capabilities: ServerCapabilities {
                // We provide prompts capability (even though it's empty)
                prompts: Some(bindings::fastertools::mcp::session::PromptsCapability {
                    list_changed: None,
                }),
                // No other capabilities from this null component
                tools: None,
                resources: None,
                logging: None,
                completions: None,
                experimental: None,
            },
            server_info: ImplementationInfo {
                name: "null-prompts".to_string(),
                version: "0.1.0".to_string(),
                title: Some("Null Prompts Provider".to_string()),
            },
            instructions: Some("This is a null prompts provider - no prompts available".to_string()),
            meta: None,
        })
    }
    
    fn handle_initialized() -> Result<(), McpError> {
        Ok(())
    }
    
    fn handle_ping() -> Result<(), McpError> {
        Ok(())
    }
    
    fn handle_shutdown() -> Result<(), McpError> {
        Ok(())
    }
}

// Implement prompt handler with null behavior
impl prompt_handler::Guest for Component {
    fn handle_list_prompts(_request: ListPromptsRequest) -> Result<ListPromptsResponse, McpError> {
        // Always return empty list
        Ok(ListPromptsResponse {
            prompts: vec![],
            next_cursor: None,
            meta: None,
        })
    }
    
    fn handle_get_prompt(_request: GetPromptRequest) -> Result<GetPromptResponse, McpError> {
        // Always return "not found" for any prompt request
        Err(McpError {
            code: ErrorCode::PromptNotFound,
            message: "No prompts available from null provider".to_string(),
            data: None,
        })
    }
}

bindings::export!(Component with_types_in bindings);