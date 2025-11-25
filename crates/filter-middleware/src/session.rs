use crate::bindings::exports::wasmcp::mcp_v20250618::server_handler::MessageContext;
use crate::bindings::wasmcp::keyvalue::store::TypedValue;
use crate::bindings::wasmcp::mcp_v20250618::mcp::Tool;
use crate::bindings::wasmcp::mcp_v20250618::sessions;

/// Store filtered tool names in session for validation
pub fn store_tool_registry(ctx: &MessageContext, tools: &[Tool]) -> Result<(), String> {
    let session = match &ctx.session {
        Some(s) => s,
        None => return Ok(()), // No session, skip storage
    };

    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
    let registry_json = serde_json::to_string(&tool_names)
        .map_err(|e| format!("Failed to serialize tool registry: {}", e))?;

    let session_obj = sessions::Session::open(&session.session_id, &session.store_id)
        .map_err(|e| format!("Failed to open session: {:?}", e))?;

    session_obj
        .set("filter:tool_registry", &TypedValue::AsString(registry_json))
        .map_err(|e| format!("Failed to set tool registry: {:?}", e))?;

    Ok(())
}

/// Load filtered tool names from session
pub fn load_tool_registry(ctx: &MessageContext) -> Result<Vec<String>, String> {
    let session = match &ctx.session {
        Some(s) => s,
        None => return Err("No session".to_string()),
    };

    let session_obj = sessions::Session::open(&session.session_id, &session.store_id)
        .map_err(|e| format!("Failed to open session: {:?}", e))?;

    let value = session_obj
        .get("filter:tool_registry")
        .map_err(|e| format!("Failed to get tool registry: {:?}", e))?;

    match value {
        Some(TypedValue::AsString(json)) => {
            serde_json::from_str(&json).map_err(|e| format!("Failed to parse tool registry: {}", e))
        }
        _ => Err("Tool registry not found in session".to_string()),
    }
}