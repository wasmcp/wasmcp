//! MCP tool definitions and handlers
//!
//! This module organizes tools by category for better maintainability.
//! Each submodule handles a specific set of related tools.

pub mod compose;
pub mod registry;

#[cfg(test)]
mod registry_tests;

use rmcp::ErrorData as McpError;
use rmcp::model::*;
use schemars::{JsonSchema, schema_for};
use std::sync::Arc;

// Re-export tool argument types for use by server
pub use compose::ComposeArgs;
pub use registry::{AddComponentArgs, AddProfileArgs, RegistryListArgs, RemoveArgs};

/// List all available tools from all categories
pub fn list_tools() -> Result<ListToolsResult, McpError> {
    // Helper to convert schemars Schema to serde_json::Value with error handling
    fn to_schema<T: JsonSchema>()
    -> Result<Arc<serde_json::Map<String, serde_json::Value>>, McpError> {
        let schema = schema_for!(T);
        let json_value = serde_json::to_value(schema).map_err(|e| {
            McpError::internal_error(format!("Failed to serialize schema: {}", e), None)
        })?;
        let object = json_value
            .as_object()
            .ok_or_else(|| McpError::internal_error("Schema is not a JSON object", None))?
            .clone();
        Ok(Arc::new(object))
    }

    Ok(ListToolsResult {
        tools: vec![
            Tool {
                name: "compose".into(),
                title: None,
                description: Some("Compose WASM components into an MCP server".into()),
                input_schema: to_schema::<ComposeArgs>()?,
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: "registry_list".into(),
                title: None,
                description: Some("List registry components, profiles, and aliases".into()),
                input_schema: to_schema::<RegistryListArgs>()?,
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: "registry_add_component".into(),
                title: None,
                description: Some("Add a component alias to the registry".into()),
                input_schema: to_schema::<AddComponentArgs>()?,
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: "registry_add_profile".into(),
                title: None,
                description: Some("Add or update a composition profile".into()),
                input_schema: to_schema::<AddProfileArgs>()?,
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: "registry_remove".into(),
                title: None,
                description: Some("Remove a component alias or profile".into()),
                input_schema: to_schema::<RemoveArgs>()?,
                output_schema: None,
                annotations: None,
                icons: None,
            },
        ],
        next_cursor: None,
    })
}

/// Call a tool by name with given arguments
pub async fn call_tool(
    tool_name: &str,
    arguments: serde_json::Map<String, serde_json::Value>,
) -> Result<CallToolResult, McpError> {
    let args_value = serde_json::Value::Object(arguments);

    match tool_name {
        "compose" => {
            let args: ComposeArgs = serde_json::from_value(args_value)
                .map_err(|e| McpError::invalid_params(format!("Invalid arguments: {}", e), None))?;
            compose::execute(args).await
        }
        "registry_list" => {
            let args: RegistryListArgs = serde_json::from_value(args_value)
                .map_err(|e| McpError::invalid_params(format!("Invalid arguments: {}", e), None))?;
            registry::list_tool(args).await
        }
        "registry_add_component" => {
            let args: AddComponentArgs = serde_json::from_value(args_value)
                .map_err(|e| McpError::invalid_params(format!("Invalid arguments: {}", e), None))?;
            registry::add_component_tool(args).await
        }
        "registry_add_profile" => {
            let args: AddProfileArgs = serde_json::from_value(args_value)
                .map_err(|e| McpError::invalid_params(format!("Invalid arguments: {}", e), None))?;
            registry::add_profile_tool(args).await
        }
        "registry_remove" => {
            let args: RemoveArgs = serde_json::from_value(args_value)
                .map_err(|e| McpError::invalid_params(format!("Invalid arguments: {}", e), None))?;
            registry::remove_tool(args).await
        }
        _ => Err(McpError::method_not_found::<CallToolRequestMethod>()),
    }
}
