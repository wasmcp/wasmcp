// Thin trait-based helpers for MCP - async by default for modern Rust
use serde_json::Value;
use std::future::Future;

// Re-export commonly used types for cleaner imports
pub use crate::bindings::fastertools::mcp::{
    tools::ToolResult,
    types::{McpError, ErrorCode},
};

// Internal imports (not re-exported)
use crate::bindings::fastertools::mcp::{
    tools::Tool as McpTool,
    types::{ContentBlock, TextContent},
};

/// Trait for MCP tools - async by default
pub trait Tool: Sized {
    /// The tool's name
    const NAME: &'static str;
    
    /// The tool's description  
    const DESCRIPTION: &'static str;
    
    /// Get the JSON schema for this tool's input
    fn input_schema() -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }
    
    /// Execute the tool with the given arguments (async by default)
    fn execute(args: Value) -> impl Future<Output = Result<ToolResult, McpError>>;
}

/// Helper macro to register tools in a clean way
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
                            input_schema: <$tool as $crate::helpers::Tool>::input_schema().to_string(),
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
                
                let args = if let Some(args_str) = &request.arguments {
                    serde_json::from_str(args_str)
                        .map_err(|e| $crate::helpers::McpError {
                            code: $crate::helpers::ErrorCode::InvalidParams,
                            message: format!("Invalid arguments: {}", e),
                            data: None,
                        })?
                } else {
                    serde_json::Value::Object(serde_json::Map::new())
                };
                
                match request.name.as_str() {
                    $(
                        <$tool as $crate::helpers::Tool>::NAME => {
                            // Always use spin's executor to run the async function
                            spin_sdk::http::run(<$tool as $crate::helpers::Tool>::execute(args))
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

/// Helper trait for types that can be tool results
pub trait IntoToolResult {
    fn into_result(self) -> ToolResult;
}

// Implement IntoToolResult for common types
impl IntoToolResult for String {
    fn into_result(self) -> ToolResult {
        ToolResult {
            content: vec![ContentBlock::Text(TextContent {
                text: self,
                annotations: None,
                meta: None,
            })],
            is_error: Some(false),
            structured_content: None,
            meta: None,
        }
    }
}

impl IntoToolResult for &str {
    fn into_result(self) -> ToolResult {
        self.to_string().into_result()
    }
}

// Utility functions for manual use
pub fn text_result(text: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: text.into(),
            annotations: None,
            meta: None,
        })],
        is_error: Some(false),
        structured_content: None,
        meta: None,
    }
}

