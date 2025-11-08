//! Policy-based authorization using Regorous (Rego interpreter)

use crate::bindings::wasmcp::mcp_v20250618::mcp::{ClientMessage, Session};
use crate::error::{AuthError, Result};
use crate::jwt::TokenInfo;
use regorus::{Engine, Value};
use serde_json::json;

/// Policy engine for authorization decisions
pub struct PolicyEngine {
    engine: Engine,
}

impl PolicyEngine {
    /// Create a new policy engine with policy and data
    pub fn new_with_policy_and_data(policy: &str, data: Option<&str>) -> Result<Self> {
        let mut engine = Engine::new();

        // Add the policy
        engine
            .add_policy("authorization.rego".to_string(), policy.to_string())
            .map_err(|e| AuthError::Internal(format!("Failed to add policy: {}", e)))?;

        // Add data if provided
        if let Some(data_json) = data {
            let data_value = Value::from_json_str(data_json)
                .map_err(|e| AuthError::Internal(format!("Failed to parse policy data: {}", e)))?;
            engine
                .add_data(data_value)
                .map_err(|e| AuthError::Internal(format!("Failed to add policy data: {}", e)))?;
        }

        Ok(Self { engine })
    }

    /// Evaluate authorization policy
    pub fn evaluate(
        &mut self,
        token_info: &TokenInfo,
        request: &ClientMessage,
        _session: Option<&Session>,
    ) -> Result<bool> {
        // Build the input for policy evaluation
        let input = build_policy_input(token_info, request)?;

        // Set the input
        self.engine.set_input(input);

        // Evaluate the allow rule
        match self
            .engine
            .eval_rule("data.mcp.authorization.allow".to_string())
        {
            Ok(value) => {
                // Check if the result is a boolean true
                match value {
                    Value::Bool(b) => Ok(b),
                    Value::Undefined => Ok(false), // Undefined means not allowed
                    _ => Err(AuthError::Internal(
                        "Policy returned non-boolean value".to_string(),
                    )),
                }
            }
            Err(e) => Err(AuthError::Internal(format!(
                "Policy evaluation failed: {}",
                e
            ))),
        }
    }
}

/// Build policy input from request context
fn build_policy_input(token_info: &TokenInfo, request: &ClientMessage) -> Result<Value> {
    // Extract MCP context from ClientMessage
    let mcp_context = extract_mcp_context(request);

    // Build input JSON
    let input = json!({
        "token": {
            "sub": token_info.sub,
            "iss": token_info.iss,
            "claims": token_info.claims,
            "scopes": token_info.scopes
        },
        "mcp": mcp_context
    });

    Value::from_json_str(&input.to_string())
        .map_err(|e| AuthError::Internal(format!("Failed to build policy input: {}", e)))
}

/// Extract MCP context from ClientMessage
fn extract_mcp_context(message: &ClientMessage) -> serde_json::Value {
    match message {
        ClientMessage::Request((_, req)) => {
            // Extract method and params from the request variant
            match req {
                crate::bindings::wasmcp::mcp_v20250618::mcp::ClientRequest::Initialize(_) => {
                    json!({ "method": "initialize" })
                }
                crate::bindings::wasmcp::mcp_v20250618::mcp::ClientRequest::Ping(_) => {
                    json!({ "method": "ping" })
                }
                crate::bindings::wasmcp::mcp_v20250618::mcp::ClientRequest::ToolsList(_) => {
                    json!({ "method": "tools/list" })
                }
                crate::bindings::wasmcp::mcp_v20250618::mcp::ClientRequest::ToolsCall(call) => {
                    json!({
                        "method": "tools/call",
                        "tool": call.name,
                        "arguments": parse_json_string(&call.arguments)
                    })
                }
                crate::bindings::wasmcp::mcp_v20250618::mcp::ClientRequest::ResourcesList(_) => {
                    json!({ "method": "resources/list" })
                }
                crate::bindings::wasmcp::mcp_v20250618::mcp::ClientRequest::ResourcesTemplatesList(_) => {
                    json!({ "method": "resources/templates/list" })
                }
                crate::bindings::wasmcp::mcp_v20250618::mcp::ClientRequest::ResourcesRead(read) => {
                    json!({
                        "method": "resources/read",
                        "uri": read.uri
                    })
                }
                crate::bindings::wasmcp::mcp_v20250618::mcp::ClientRequest::ResourcesSubscribe(uri) => {
                    json!({
                        "method": "resources/subscribe",
                        "uri": uri
                    })
                }
                crate::bindings::wasmcp::mcp_v20250618::mcp::ClientRequest::ResourcesUnsubscribe(uri) => {
                    json!({
                        "method": "resources/unsubscribe",
                        "uri": uri
                    })
                }
                crate::bindings::wasmcp::mcp_v20250618::mcp::ClientRequest::PromptsList(_) => {
                    json!({ "method": "prompts/list" })
                }
                crate::bindings::wasmcp::mcp_v20250618::mcp::ClientRequest::PromptsGet(get) => {
                    json!({
                        "method": "prompts/get",
                        "name": get.name,
                        "arguments": parse_json_string(&get.arguments)
                    })
                }
                crate::bindings::wasmcp::mcp_v20250618::mcp::ClientRequest::LoggingSetLevel(level) => {
                    json!({
                        "method": "logging/setLevel",
                        "level": format!("{:?}", level)
                    })
                }
                crate::bindings::wasmcp::mcp_v20250618::mcp::ClientRequest::CompletionComplete(complete) => {
                    json!({
                        "method": "completion/complete",
                        "ref": format!("{:?}", complete.ref_)
                    })
                }
            }
        }
        ClientMessage::Result((_, _)) => {
            json!({ "type": "result" })
        }
        ClientMessage::Error((_, _)) => {
            json!({ "type": "error" })
        }
        ClientMessage::Notification(notif) => match notif {
            crate::bindings::wasmcp::mcp_v20250618::mcp::ClientNotification::Initialized(_) => {
                json!({ "method": "notifications/initialized" })
            }
            crate::bindings::wasmcp::mcp_v20250618::mcp::ClientNotification::RootsListChanged(
                _,
            ) => {
                json!({ "method": "notifications/roots/list_changed" })
            }
            crate::bindings::wasmcp::mcp_v20250618::mcp::ClientNotification::Cancelled(cancel) => {
                json!({
                    "method": "notifications/cancelled",
                    "request_id": format!("{:?}", cancel.request_id)
                })
            }
            crate::bindings::wasmcp::mcp_v20250618::mcp::ClientNotification::Progress(progress) => {
                json!({
                    "method": "notifications/progress",
                    "progress_token": format!("{:?}", progress.progress_token),
                    "progress": progress.progress
                })
            }
        },
    }
}

/// Parse JSON string (optional field that might be JSON)
fn parse_json_string(json_str: &Option<String>) -> serde_json::Value {
    json_str
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(serde_json::Value::Null)
}
