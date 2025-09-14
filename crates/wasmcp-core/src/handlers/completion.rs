use crate::{traits::McpCompletionHandler, ErrorCode, McpError};
use serde_json::Value;

/// Generic handler for the 'completion/complete' request.
pub fn complete(
    _provider: &dyn McpCompletionHandler,
    _params: Option<Value>,
) -> Result<Value, McpError> {
    // TODO: Implement completion when WIT interface is defined
    Err(McpError {
        code: ErrorCode::InternalError,
        message: "Completion not yet implemented".to_string(),
        data: None,
    })
}