use crate::{traits::McpToolsHandler, ErrorCode, McpError, AuthContext};
use rmcp::model::{CallToolRequestParam, PaginatedRequestParam};
use serde_json::Value;

/// Generic handler for the 'tools/list' request.
///
/// Its sole responsibility is to handle the JSON-RPC protocol layer:
/// 1. Deserialize the incoming JSON `Value` into a strongly-typed parameter struct.
/// 2. Delegate the core logic to a provider via the `McpToolsHandler` trait.
/// 3. Serialize the resulting response back into a JSON `Value`.
pub fn list_tools(
    provider: &dyn McpToolsHandler,
    params: Option<Value>,
) -> Result<Value, McpError> {
    let request_params: Option<PaginatedRequestParam> = params
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| McpError {
            code: ErrorCode::InvalidParams,
            message: format!("Invalid params: {e}"),
            data: None,
        })?;

    // Convert rmcp PaginatedRequestParam to wasmcp-core ListToolsRequest
    let request = crate::ListToolsRequest {
        cursor: request_params.and_then(|p| p.cursor),
    };

    let result = provider.list_tools(request)?;

    // Serialize the strongly-typed result back to a generic JSON Value for the transport layer.
    Ok(serde_json::to_value(result).unwrap())
}

/// Generic handler for the 'tools/call' request.
pub fn call_tool(
    provider: &dyn McpToolsHandler,
    params: Option<Value>,
    auth_context: Option<AuthContext>,
) -> Result<Value, McpError> {
    let request_params: CallToolRequestParam = params
        .ok_or_else(|| McpError {
            code: ErrorCode::InvalidParams,
            message: "Missing params".to_string(),
            data: None,
        })
        .and_then(|p| {
            serde_json::from_value(p).map_err(|e| McpError {
                code: ErrorCode::InvalidParams,
                message: format!("Invalid params: {e}"),
                data: None,
            })
        })?;

    // Convert rmcp CallToolRequestParam to wasmcp-core CallToolRequest
    let request = crate::CallToolRequest {
        name: request_params.name.to_string(),
        arguments: request_params.arguments.map(|args| {
            serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string())
        }),
    };

    let result = provider.call_tool(request, auth_context)?;

    // Serialize the strongly-typed result back to a generic JSON Value for the transport layer.
    Ok(serde_json::to_value(result).unwrap())
}