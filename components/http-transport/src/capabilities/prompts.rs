use crate::capabilities::prompts_provider::PromptsProvider;
use serde_json::Value;
use wasmcp_core::{handlers::prompts, McpError};

pub fn list_prompts(params: Option<Value>) -> Result<Value, McpError> {
    let provider = PromptsProvider;
    prompts::list_prompts(&provider, params)
}

pub fn get_prompt(params: Option<Value>) -> Result<Value, McpError> {
    let provider = PromptsProvider;
    prompts::get_prompt(&provider, params)
}