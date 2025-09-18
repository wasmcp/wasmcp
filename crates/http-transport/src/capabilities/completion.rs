use crate::error::{ErrorCode, McpError};
use serde_json::Value;

pub fn complete(_params: Option<Value>) -> Result<Value, McpError> {
    // TODO: Implement completion when WIT interface is defined
    Err(McpError {
        code: ErrorCode::InternalError,
        message: "Completion not yet implemented".to_string(),
        data: None,
    })
}