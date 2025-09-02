//! Helper traits and utilities for MCP tool implementation.

// Re-export commonly used types for cleaner imports
pub use crate::bindings::fastertools::mcp::{
    tools::ToolResult,
    types::{ErrorCode, McpError},
};

// Internal imports (not re-exported)
use crate::bindings::fastertools::mcp::types::{ContentBlock, TextContent};

/// Trait for implementing MCP tools with modern async support.
///
/// # Example
/// ```no_run
/// struct MyTool;
///
/// impl Tool for MyTool {
///     const NAME: &'static str = "my_tool";
///     const DESCRIPTION: &'static str = "Does something useful";
///     
///     async fn execute(args: Option<String>) -> Result<ToolResult, McpError> {
///         // Tool implementation
///         Ok(text_result("Success"))
///     }
/// }
/// ```
pub trait Tool: Sized {
    /// The tool's unique name.
    const NAME: &'static str;

    /// Human-readable description of what the tool does.
    const DESCRIPTION: &'static str;

    /// Get the JSON schema for this tool's input as a JSON string.
    ///
    /// Defaults to an empty object schema.
    fn input_schema() -> String {
        r#"{"type":"object","properties":{}}"#.to_string()
    }

    /// Execute the tool with JSON string arguments.
    ///
    /// # Errors
    /// Returns `McpError` if the tool execution fails.
    async fn execute(args: Option<String>) -> Result<ToolResult, McpError>;
}

/// Helper macro to register tools with the MCP provider.
///
/// # Example
/// ```no_run
/// register_tools!(EchoTool, WeatherTool, MultiWeatherTool);
/// ```
#[macro_export]
macro_rules! register_tools {
    ($($tool:ty),* $(,)?) => {
        use $crate::bindings::exports::fastertools::mcp::tools_capabilities::Guest as ToolsGuest;
        use $crate::bindings::exports::fastertools::mcp::core_capabilities::Guest as CoreGuest;
        
        impl ToolsGuest for $crate::Component {
            #[allow(clippy::needless_pass_by_value)]
            fn handle_list_tools(
                _request: $crate::bindings::fastertools::mcp::tools::ListToolsRequest,
            ) -> Result<
                $crate::bindings::fastertools::mcp::tools::ListToolsResponse,
                $crate::helpers::McpError,
            > {
                let tools = vec![
                    $(
                        $crate::bindings::fastertools::mcp::tools::Tool {
                            base: $crate::bindings::fastertools::mcp::types::BaseMetadata {
                                name: <$tool as $crate::helpers::Tool>::NAME.to_string(),
                                title: Some(<$tool as $crate::helpers::Tool>::NAME.to_string()),
                            },
                            description: Some(
                                <$tool as $crate::helpers::Tool>::DESCRIPTION.to_string()
                            ),
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

            #[allow(clippy::needless_pass_by_value)]
            fn handle_call_tool(
                request: $crate::bindings::fastertools::mcp::tools::CallToolRequest,
            ) -> Result<$crate::helpers::ToolResult, $crate::helpers::McpError> {
                match request.name.as_str() {
                    $(
                        <$tool as $crate::helpers::Tool>::NAME => {
                            // Use spin's executor to run the async function
                            spin_sdk::http::run(
                                <$tool as $crate::helpers::Tool>::execute(request.arguments)
                            )
                        }
                    ),*
                    _ => Err($crate::helpers::McpError {
                        code: $crate::helpers::ErrorCode::ToolNotFound,
                        message: format!("Unknown tool: {}", request.name),
                        data: None,
                    })
                }
            }
        }
        
        impl CoreGuest for $crate::Component {
            fn handle_initialize(
                _request: $crate::bindings::fastertools::mcp::session::InitializeRequest,
            ) -> Result<
                $crate::bindings::fastertools::mcp::session::InitializeResponse,
                $crate::helpers::McpError,
            > {
                Ok($crate::bindings::fastertools::mcp::session::InitializeResponse {
                    protocol_version: $crate::protocol_version(),
                    capabilities: $crate::bindings::fastertools::mcp::session::ServerCapabilities {
                        experimental: None,
                        logging: None,
                        completions: None,
                        prompts: None,
                        resources: None,
                        tools: Some($crate::bindings::fastertools::mcp::session::ToolsCapability {
                            list_changed: None,
                        }),
                    },
                    server_info: {
                        let (name, version, title) = $crate::server_info();
                        $crate::bindings::fastertools::mcp::session::ImplementationInfo {
                            name,
                            version,
                            title: Some(title),
                        }
                    },
                    instructions: None,
                    meta: None,
                })
            }

            fn handle_initialized() -> Result<(), $crate::helpers::McpError> {
                Ok(())
            }

            fn handle_ping() -> Result<(), $crate::helpers::McpError> {
                Ok(())
            }

            fn handle_shutdown() -> Result<(), $crate::helpers::McpError> {
                Ok(())
            }
            
            fn get_auth_config() -> Option<$crate::bindings::fastertools::mcp::authorization::ProviderAuthConfig> {
                // Delegate to the auth_config function in lib.rs
                $crate::auth_config()
            }
        }
        
        // Export the WIT bindings
        $crate::bindings::export!(Component with_types_in $crate::bindings);
    };
}

/// Creates a text result for successful tool execution.
#[must_use]
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

/// Creates an error result for tool execution failures.
#[must_use]
pub fn error_result(message: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: message.into(),
            annotations: None,
            meta: None,
        })],
        structured_content: None,
        is_error: Some(true),
        meta: None,
    }
}

/// Extension trait for converting strings to tool results.
pub trait IntoToolResult {
    /// Convert this value into a successful tool result.
    fn into_result(self) -> ToolResult;

    /// Convert this value into an error tool result.
    fn into_error(self) -> ToolResult;
}

impl IntoToolResult for String {
    fn into_result(self) -> ToolResult {
        text_result(self)
    }

    fn into_error(self) -> ToolResult {
        error_result(self)
    }
}

impl IntoToolResult for &str {
    fn into_result(self) -> ToolResult {
        text_result(self)
    }

    fn into_error(self) -> ToolResult {
        error_result(self)
    }
}

/// Parse JSON arguments into a strongly-typed struct.
///
/// # Example
/// ```no_run
/// #[derive(Deserialize)]
/// struct MyArgs {
///     message: String,
///     count: Option<u32>,
/// }
///
/// async fn execute(args: Option<String>) -> Result<ToolResult, McpError> {
///     let args: MyArgs = parse_args(&args)?;
///     // Use args.message and args.count directly
/// }
/// ```
///
/// # Errors
/// Returns `McpError` if arguments are missing or don't match the expected type.
pub fn parse_args<T>(args: &Option<String>) -> Result<T, McpError>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let args_str = args.as_ref().ok_or_else(|| McpError {
        code: ErrorCode::InvalidParams,
        message: "No arguments provided".to_string(),
        data: None,
    })?;

    serde_json::from_str(args_str).map_err(|e| McpError {
        code: ErrorCode::InvalidParams,
        message: format!("Invalid arguments: {e}"),
        data: None,
    })
}

