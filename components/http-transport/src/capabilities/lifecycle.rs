use crate::provider::WasmProvider;
use serde_json::Value;
use wasmcp_core::{lifecycle, McpError};

pub fn initialize(params: Option<Value>) -> Result<Value, McpError> {
    let provider = WasmProvider;
    lifecycle::initialize(&provider, params)
}

pub fn client_initialized() -> Result<Value, McpError> {
    let provider = WasmProvider;
    lifecycle::client_initialized(&provider)
}

pub fn shutdown() -> Result<Value, McpError> {
    let provider = WasmProvider;
    lifecycle::shutdown(&provider)
}