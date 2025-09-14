use crate::auth::AuthContext;
use crate::capabilities::tools_provider::ToolsProvider;
use serde_json::Value;
use wasmcp_core::{handlers::tools, McpError};

pub fn list_tools(params: Option<Value>) -> Result<Value, McpError> {
    let provider = ToolsProvider;
    tools::list_tools(&provider, params)
}

pub fn call_tool(params: Option<Value>, auth_context: Option<&AuthContext>) -> Result<Value, McpError> {
    let provider = ToolsProvider;
    tools::call_tool(&provider, params, auth_context.cloned())
}