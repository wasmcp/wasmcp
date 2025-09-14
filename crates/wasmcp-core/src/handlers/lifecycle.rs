use crate::{traits::McpLifecycleHandler, ErrorCode, McpError};
use rmcp::model::InitializeRequestParam;
use serde_json::Value;

/// The generic, public-facing `initialize` function.
///
/// Its sole responsibility is to handle the JSON-RPC protocol layer:
/// 1. Deserialize the incoming JSON `Value` into a strongly-typed `InitializeRequestParam`.
/// 2. Delegate the core logic to a provider via the `McpLifecycleHandler` trait.
/// 3. Serialize the resulting `InitializeResult` back into a JSON `Value`.
///
/// This function is essential because it prevents code duplication. The logic for
/// handling JSON is written once here and is reused by both the WASM and native targets.
pub fn initialize(
    provider: &dyn McpLifecycleHandler,
    params: Option<Value>,
) -> Result<Value, McpError> {
    let request_params: InitializeRequestParam = if let Some(p) = params {
        serde_json::from_value(p).map_err(|e| McpError {
            code: ErrorCode::InvalidParams,
            message: format!("Invalid params: {e}"),
            data: None,
        })?
    } else {
        // Handle the case where the client sends no parameters.
        InitializeRequestParam::default()
    };

    let result = provider.initialize(request_params)?;

    // Serialize the strongly-typed result back to a generic JSON Value for the transport layer.
    Ok(serde_json::to_value(result).unwrap())
}

/// Generic handler for the 'clientInitialized' notification.
pub fn client_initialized(provider: &dyn McpLifecycleHandler) -> Result<Value, McpError> {
    provider.client_initialized()?;
    Ok(Value::Null)
}

/// Generic handler for the 'shutdown' request.
pub fn shutdown(provider: &dyn McpLifecycleHandler) -> Result<Value, McpError> {
    provider.shutdown()?;
    Ok(serde_json::json!({}))
}