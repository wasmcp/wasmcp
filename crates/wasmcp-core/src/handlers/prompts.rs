use crate::{traits::McpPromptsHandler, ErrorCode, McpError};
use rmcp::model::{PaginatedRequestParam, GetPromptRequestParam};
use serde_json::Value;

/// Generic handler for the 'prompts/list' request.
pub fn list_prompts(
    provider: &dyn McpPromptsHandler,
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

    let request = crate::ListPromptsRequest {
        cursor: request_params.and_then(|p| p.cursor),
    };

    let result = provider.list_prompts(request)?;

    Ok(serde_json::to_value(result).unwrap())
}

/// Generic handler for the 'prompts/get' request.
pub fn get_prompt(
    provider: &dyn McpPromptsHandler,
    params: Option<Value>,
) -> Result<Value, McpError> {
    let request_params: GetPromptRequestParam = params
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

    let request = crate::GetPromptRequest {
        name: request_params.name,
        arguments: request_params.arguments.map(|args| {
            serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string())
        }),
    };

    let result = provider.get_prompt(request)?;

    Ok(serde_json::to_value(result).unwrap())
}