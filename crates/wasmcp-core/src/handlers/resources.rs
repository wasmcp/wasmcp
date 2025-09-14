use crate::{traits::McpResourcesHandler, ErrorCode, McpError};
use rmcp::model::{PaginatedRequestParam, ReadResourceRequestParam};
use serde_json::Value;

/// Generic handler for the 'resources/list' request.
pub fn list_resources(
    provider: &dyn McpResourcesHandler,
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

    let request = crate::ListResourcesRequest {
        cursor: request_params.and_then(|p| p.cursor),
    };

    let result = provider.list_resources(request)?;

    Ok(serde_json::to_value(result).unwrap())
}

/// Generic handler for the 'resources/read' request.
pub fn read_resource(
    provider: &dyn McpResourcesHandler,
    params: Option<Value>,
) -> Result<Value, McpError> {
    let request_params: ReadResourceRequestParam = params
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

    let request = crate::ReadResourceRequest {
        uri: request_params.uri,
    };

    let result = provider.read_resource(request)?;

    Ok(serde_json::to_value(result).unwrap())
}