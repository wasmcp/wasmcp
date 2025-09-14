use crate::capabilities::resources_provider::ResourcesProvider;
use serde_json::Value;
use wasmcp_core::{handlers::resources, McpError};

pub fn list_resources(params: Option<Value>) -> Result<Value, McpError> {
    let provider = ResourcesProvider;
    resources::list_resources(&provider, params)
}

pub fn read_resource(params: Option<Value>) -> Result<Value, McpError> {
    let provider = ResourcesProvider;
    resources::read_resource(&provider, params)
}