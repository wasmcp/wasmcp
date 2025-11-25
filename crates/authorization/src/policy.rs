//! Policy-based authorization using Regorous (Rego interpreter)

use crate::bindings::wasmcp::auth::types::JwtClaims;
use crate::bindings::wasmcp::mcp_v20250618::mcp::{ClientMessage, Session};
use crate::error::{AuthError, Result};
use regorus::{Engine, Value};
use serde_json::json;
use std::collections::HashMap;

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
        jwt_claims: &JwtClaims,
        request: &ClientMessage,
        _session: Option<&Session>,
        http_context: Option<
            &crate::bindings::exports::wasmcp::mcp_v20250618::server_auth::HttpContext,
        >,
    ) -> Result<bool> {
        // Build the input for policy evaluation
        let input = build_policy_input(jwt_claims, request, http_context)?;

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
fn build_policy_input(
    jwt_claims: &JwtClaims,
    request: &ClientMessage,
    http_context: Option<
        &crate::bindings::exports::wasmcp::mcp_v20250618::server_auth::HttpContext,
    >,
) -> Result<Value> {
    // Extract MCP context from ClientMessage
    let mcp_context = extract_mcp_context(request);

    // Build claims map from JwtClaims
    let mut claims_map = HashMap::new();
    claims_map.insert(
        "sub".to_string(),
        serde_json::Value::String(jwt_claims.subject.clone()),
    );
    if let Some(ref iss) = jwt_claims.issuer {
        claims_map.insert("iss".to_string(), serde_json::Value::String(iss.clone()));
    }
    if !jwt_claims.audience.is_empty() {
        claims_map.insert("aud".to_string(), serde_json::json!(jwt_claims.audience));
    }
    if let Some(exp) = jwt_claims.expiration {
        claims_map.insert("exp".to_string(), serde_json::json!(exp));
    }
    if let Some(iat) = jwt_claims.issued_at {
        claims_map.insert("iat".to_string(), serde_json::json!(iat));
    }
    if let Some(nbf) = jwt_claims.not_before {
        claims_map.insert("nbf".to_string(), serde_json::json!(nbf));
    }
    if !jwt_claims.scopes.is_empty() {
        claims_map.insert(
            "scope".to_string(),
            serde_json::Value::String(jwt_claims.scopes.join(" ")),
        );
    }
    // Add custom claims
    for (k, v) in &jwt_claims.custom_claims {
        if let Ok(json_val) = serde_json::from_str(v) {
            claims_map.insert(k.clone(), json_val);
        } else {
            claims_map.insert(k.clone(), serde_json::Value::String(v.clone()));
        }
    }

    // Build HTTP context if provided
    let http_ctx = if let Some(http) = http_context {
        let headers: HashMap<String, String> = http
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        json!({
            "method": http.method,
            "path": http.path,
            "headers": headers
        })
    } else {
        json!(null)
    };

    // Build input JSON
    let input = json!({
        "token": {
            "sub": jwt_claims.subject,
            "iss": jwt_claims.issuer.clone().unwrap_or_default(),
            "claims": claims_map,
            "scopes": jwt_claims.scopes
        },
        "mcp": mcp_context,
        "http": http_ctx
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
