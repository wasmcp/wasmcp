// Simplified helpers for MCP with string-based JSON types
use std::future::Future;
use serde_json::Value;

// Re-export commonly used types for cleaner imports
pub use crate::bindings::fastertools::mcp::{
    tools::ToolResult,
    types::{McpError, ErrorCode},
};

// Internal imports (not re-exported)
use crate::bindings::fastertools::mcp::types::{ContentBlock, TextContent};

/// Trait for MCP tools - now using standard JSON strings
pub trait Tool: Sized {
    /// The tool's name
    const NAME: &'static str;
    
    /// The tool's description  
    const DESCRIPTION: &'static str;
    
    /// Get the JSON schema for this tool's input as a JSON string
    fn input_schema() -> String {
        // Default empty object schema
        r#"{"type":"object","properties":{}}"#.to_string()
    }
    
    /// Execute the tool with JSON string arguments
    fn execute(args: Option<String>) -> impl Future<Output = Result<ToolResult, McpError>>;
}

/// Helper macro to register tools
#[macro_export]
macro_rules! register_tools {
    ($($tool:ty),* $(,)?) => {
        {
            fn handle_list_tools(_request: $crate::bindings::fastertools::mcp::tools::ListToolsRequest) 
                -> Result<$crate::bindings::fastertools::mcp::tools::ListToolsResponse, $crate::helpers::McpError> {
                
                let tools = vec![
                    $(
                        $crate::bindings::fastertools::mcp::tools::Tool {
                            base: $crate::bindings::fastertools::mcp::types::BaseMetadata {
                                name: <$tool as $crate::helpers::Tool>::NAME.to_string(),
                                title: Some(<$tool as $crate::helpers::Tool>::NAME.to_string()),
                            },
                            description: Some(<$tool as $crate::helpers::Tool>::DESCRIPTION.to_string()),
                            input_schema: <$tool as $crate::helpers::Tool>::input_schema(),
                            output_schema: None,
                            annotations: None,
                            meta: None,
                        }
                    ),*
                ];
                
                Ok($crate::bindings::fastertools::mcp::tools::ListToolsResponse {
                    tools,
                    next_cursor: None,
                    meta: None,
                })
            }
            
            fn handle_call_tool(request: $crate::bindings::fastertools::mcp::tools::CallToolRequest) 
                -> Result<$crate::helpers::ToolResult, $crate::helpers::McpError> {
                
                // Pass JSON string directly - no conversion needed!
                match request.name.as_str() {
                    $(
                        <$tool as $crate::helpers::Tool>::NAME => {
                            // Always use spin's executor to run the async function
                            spin_sdk::http::run(<$tool as $crate::helpers::Tool>::execute(request.arguments))
                        }
                    ),*
                    _ => Err($crate::helpers::McpError {
                        code: $crate::helpers::ErrorCode::ToolNotFound,
                        message: format!("Unknown tool: {}", request.name),
                        data: None,
                    })
                }
            }
            
            (handle_list_tools, handle_call_tool)
        }
    };
}

// Helper to create a text result
pub fn text_result(text: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: text.into(),
            annotations: None,
            meta: None,
        })],
        structured_content: None,
        is_error: Some(false),
        meta: None,
    }
}

// Extension trait for easier result creation
pub trait IntoToolResult {
    fn into_result(self) -> ToolResult;
}

impl IntoToolResult for String {
    fn into_result(self) -> ToolResult {
        text_result(self)
    }
}

impl IntoToolResult for &str {
    fn into_result(self) -> ToolResult {
        text_result(self)
    }
}

// Helper to extract a field from JSON arguments
pub fn get_json_field(args: &Option<String>, field: &str) -> Result<Option<Value>, McpError> {
    match args {
        Some(json_str) => {
            let obj: Value = serde_json::from_str(json_str)
                .map_err(|e| McpError {
                    code: ErrorCode::InvalidParams,
                    message: format!("Invalid JSON arguments: {}", e),
                    data: None,
                })?;
            Ok(obj.get(field).cloned())
        }
        None => Ok(None),
    }
}

// Helper to extract a string field from JSON arguments
pub fn get_string_field(args: &Option<String>, field: &str) -> Result<Option<String>, McpError> {
    get_json_field(args, field)?
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .map_or(Ok(None), |s| Ok(Some(s)))
}