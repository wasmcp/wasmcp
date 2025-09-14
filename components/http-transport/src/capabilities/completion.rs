use crate::capabilities::completion_provider::CompletionProvider;
use serde_json::Value;
use wasmcp_core::{handlers::completion, McpError};

pub fn complete(params: Option<Value>) -> Result<Value, McpError> {
    let provider = CompletionProvider;
    completion::complete(&provider, params)
}