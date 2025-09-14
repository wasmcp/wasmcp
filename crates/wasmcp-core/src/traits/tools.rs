use crate::{
    McpError, AuthContext,
    ListToolsRequest, ListToolsResult,
    CallToolRequest, CallToolResult
};

/// Defines the contract for MCP tools providers.
///
/// Tools enable models to interact with external systems, such as querying databases,
/// calling APIs, or performing computations. Each tool is uniquely identified by a name
/// and includes metadata describing its schema.
pub trait McpToolsHandler {
    /// List available tools
    fn list_tools(&self, request: ListToolsRequest) -> Result<ListToolsResult, McpError>;

    /// Execute a tool
    fn call_tool(&self, request: CallToolRequest, context: Option<AuthContext>) -> Result<CallToolResult, McpError>;
}