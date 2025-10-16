//! JSON-RPC request parsing for MCP protocol
//!
//! This module handles parsing JSON-RPC requests into WIT types.
//! Serde handles validation automatically.

use crate::bindings::wasmcp::mcp::protocol::{
    CallToolRequest, CancelledNotification, ClientCapabilities, ClientNotification, ClientRequest,
    ClientResponse, CommonNotification, CompleteRequest, CompletionArgument, CompletionContext,
    CompletionPromptReference, CompletionReference, ElicitResult, ElicitResultAction,
    ElicitResultContent, Error, ErrorCode, GetPromptRequest, Implementation, InitializeRequest,
    ListPromptsRequest, ListResourceTemplatesRequest, ListResourcesRequest, ListRootsResult,
    ListToolsRequest, LogLevel, ProgressNotification, ProgressToken, ProtocolVersion,
    ReadResourceRequest, RequestId, Role, Root, SamplingContent, SamplingCreateMessageResult,
};
use serde::Deserialize;
use serde_json::Value;

// =============================================================================
// SHADOW TYPES FOR DESERIALIZATION
// =============================================================================

#[derive(Deserialize)]
#[serde(untagged)]
enum JsonRequestId {
    Number(i64),
    String(String),
}

impl From<JsonRequestId> for RequestId {
    fn from(id: JsonRequestId) -> Self {
        match id {
            JsonRequestId::Number(n) => RequestId::Number(n),
            JsonRequestId::String(s) => RequestId::String(s),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonInitializeRequestParams {
    protocol_version: String,
    capabilities: JsonClientCapabilities,
    client_info: JsonImplementation,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonClientCapabilities {
    #[serde(default)]
    elicitation: Option<Value>,
    #[serde(default)]
    experimental: Option<Vec<(String, Value)>>,
    #[serde(default)]
    roots: Option<JsonRootsCapability>,
    #[serde(default)]
    sampling: Option<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonRootsCapability {
    #[serde(default)]
    list_changed: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonImplementation {
    name: String,
    #[serde(default)]
    title: Option<String>,
    version: String,
}

// =============================================================================
// CONVERSION FUNCTIONS
// =============================================================================

fn parse_protocol_version(s: &str) -> Result<ProtocolVersion, String> {
    match s {
        "2025-06-18" => Ok(ProtocolVersion::V20250618),
        "2025-03-26" => Ok(ProtocolVersion::V20250326),
        "2024-11-05" => Ok(ProtocolVersion::V20241105),
        _ => Err(format!("Unsupported protocol version: {}", s)),
    }
}

fn convert_client_capabilities(caps: JsonClientCapabilities) -> ClientCapabilities {
    use crate::bindings::wasmcp::mcp::protocol::ClientLists;

    ClientCapabilities {
        elicitation: caps
            .elicitation
            .and_then(|v| serde_json::to_string(&v).ok()),
        experimental: caps.experimental.map(|exp| {
            exp.into_iter()
                .filter_map(|(k, v)| serde_json::to_string(&v).ok().map(|s| (k, s)))
                .collect()
        }),
        list_changed: caps.roots.and_then(|r| r.list_changed).and_then(|lc| {
            if lc {
                Some(ClientLists::ROOTS)
            } else {
                None
            }
        }),
        sampling: caps.sampling.and_then(|v| serde_json::to_string(&v).ok()),
    }
}

fn convert_implementation(impl_info: JsonImplementation) -> Implementation {
    Implementation {
        name: impl_info.name,
        title: impl_info.title,
        version: impl_info.version,
    }
}

// =============================================================================
// PUBLIC API
// =============================================================================

/// Parse a JSON-RPC request ID
pub fn parse_request_id(value: &Value) -> Result<RequestId, String> {
    serde_json::from_value::<JsonRequestId>(value.clone())
        .map(RequestId::from)
        .map_err(|e| format!("Invalid request ID: {}", e))
}

/// Parse a JSON-RPC request into a ClientRequest
pub fn parse_client_request(json: &Value) -> Result<ClientRequest, String> {
    let method = json
        .get("method")
        .and_then(|m| m.as_str())
        .ok_or("Missing method field")?;

    let params = json.get("params");

    match method {
        "initialize" => parse_initialize_request(params),
        "tools/list" => parse_list_tools_request(params),
        "tools/call" => parse_call_tool_request(params),
        "resources/list" => parse_list_resources_request(params),
        "resources/read" => parse_read_resource_request(params),
        "resources/templates/list" => parse_list_resource_templates_request(params),
        "prompts/list" => parse_list_prompts_request(params),
        "prompts/get" => parse_get_prompt_request(params),
        "completion/complete" => parse_complete_request(params),
        "logging/setLevel" => parse_set_log_level_request(params),
        "ping" => parse_ping_request(params),
        "resources/subscribe" => parse_resource_subscribe_request(params),
        "resources/unsubscribe" => parse_resource_unsubscribe_request(params),
        _ => Err(format!("Unsupported method: {}", method)),
    }
}

fn parse_initialize_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let params = params.ok_or("Missing params for initialize request")?;

    let json_params: JsonInitializeRequestParams = serde_json::from_value(params.clone())
        .map_err(|e| format!("Invalid initialize params: {}", e))?;

    let protocol_version = parse_protocol_version(&json_params.protocol_version)?;
    let capabilities = convert_client_capabilities(json_params.capabilities);
    let client_info = convert_implementation(json_params.client_info);

    Ok(ClientRequest::Initialize(InitializeRequest {
        protocol_version,
        capabilities,
        client_info,
    }))
}

fn parse_list_tools_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let cursor = params
        .and_then(|p| p.get("cursor"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string());

    Ok(ClientRequest::ToolsList(ListToolsRequest { cursor }))
}

fn parse_call_tool_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let params = params.ok_or("Missing params for tools/call request")?;

    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or("Missing 'name' field in tools/call params")?
        .to_string();

    let arguments = params
        .get("arguments")
        .map(|args| serde_json::to_string(args))
        .transpose()
        .map_err(|e| format!("Failed to serialize arguments: {}", e))?;

    Ok(ClientRequest::ToolsCall(CallToolRequest {
        name,
        arguments,
    }))
}

fn parse_list_resources_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let cursor = params
        .and_then(|p| p.get("cursor"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string());

    Ok(ClientRequest::ResourcesList(ListResourcesRequest {
        cursor,
    }))
}

fn parse_read_resource_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let params = params.ok_or("Missing params for resources/read request")?;

    let uri = params
        .get("uri")
        .and_then(|u| u.as_str())
        .ok_or("Missing 'uri' field in resources/read params")?
        .to_string();

    Ok(ClientRequest::ResourcesRead(ReadResourceRequest { uri }))
}

fn parse_list_resource_templates_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let cursor = params
        .and_then(|p| p.get("cursor"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string());

    Ok(ClientRequest::ResourcesTemplatesList(
        ListResourceTemplatesRequest { cursor },
    ))
}

fn parse_list_prompts_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let cursor = params
        .and_then(|p| p.get("cursor"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string());

    Ok(ClientRequest::PromptsList(ListPromptsRequest { cursor }))
}

fn parse_get_prompt_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let params = params.ok_or("Missing params for prompts/get request")?;

    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or("Missing 'name' field in prompts/get params")?
        .to_string();

    let arguments = params
        .get("arguments")
        .map(|args| serde_json::to_string(args))
        .transpose()
        .map_err(|e| format!("Failed to serialize prompt arguments: {}", e))?;

    Ok(ClientRequest::PromptsGet(GetPromptRequest {
        name,
        arguments,
    }))
}

fn parse_complete_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let params = params.ok_or("Missing params for completion/complete request")?;

    let ref_obj = params
        .get("ref")
        .ok_or("Missing 'ref' field in completion/complete params")?;

    let completion_ref = if let Some(prompt_name) = ref_obj.get("name").and_then(|n| n.as_str()) {
        // It's a prompt reference
        CompletionReference::Prompt(CompletionPromptReference {
            name: prompt_name.to_string(),
            title: ref_obj
                .get("title")
                .and_then(|t| t.as_str())
                .map(|s| s.to_string()),
        })
    } else if let Some(uri) = ref_obj.as_str() {
        // It's a resource template URI
        CompletionReference::ResourceTemplate(uri.to_string())
    } else {
        return Err("Invalid 'ref' field: must be prompt object or URI string".to_string());
    };

    let argument_obj = params
        .get("argument")
        .ok_or("Missing 'argument' field in completion/complete params")?;

    let argument = CompletionArgument {
        name: argument_obj
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or("Missing 'name' in argument")?
            .to_string(),
        value: argument_obj
            .get("value")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'value' in argument")?
            .to_string(),
    };

    let context = params
        .get("context")
        .map(|ctx| {
            let arguments = ctx
                .get("arguments")
                .map(|args| serde_json::to_string(args))
                .transpose()
                .map_err(|e| format!("Failed to serialize context arguments: {}", e))?;
            Ok::<CompletionContext, String>(CompletionContext { arguments })
        })
        .transpose()?;

    Ok(ClientRequest::CompletionComplete(CompleteRequest {
        argument,
        ref_: completion_ref,
        context,
    }))
}

fn parse_set_log_level_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let params = params.ok_or("Missing params for logging/setLevel request")?;

    let level_str = params
        .get("level")
        .and_then(|l| l.as_str())
        .ok_or("Missing 'level' field in logging/setLevel params")?;

    let level = match level_str {
        "debug" => LogLevel::Debug,
        "info" => LogLevel::Info,
        "notice" => LogLevel::Notice,
        "warning" => LogLevel::Warning,
        "error" => LogLevel::Error,
        "critical" => LogLevel::Critical,
        "alert" => LogLevel::Alert,
        "emergency" => LogLevel::Emergency,
        _ => return Err(format!("Invalid log level: {}", level_str)),
    };

    Ok(ClientRequest::LoggingSetLevel(level))
}

fn parse_ping_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    use crate::bindings::wasmcp::mcp::protocol::{PingRequest, ProgressToken};

    let ping_request = if let Some(p) = params {
        let progress_token = p.get("progressToken").and_then(|pt| {
            if let Some(s) = pt.as_str() {
                Some(ProgressToken::String(s.to_string()))
            } else if let Some(i) = pt.as_i64() {
                Some(ProgressToken::Integer(i))
            } else {
                None
            }
        });

        let meta = p.get("_meta").and_then(|m| serde_json::to_string(m).ok());

        let extras = p
            .get("extras")
            .and_then(|e| e.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| {
                        serde_json::to_string(v)
                            .ok()
                            .map(|json_str| (k.clone(), json_str))
                    })
                    .collect()
            })
            .unwrap_or_else(Vec::new);

        PingRequest {
            meta,
            progress_token,
            extras,
        }
    } else {
        PingRequest {
            meta: None,
            progress_token: None,
            extras: vec![],
        }
    };

    Ok(ClientRequest::Ping(ping_request))
}

fn parse_resource_subscribe_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let params = params.ok_or("Missing params for resources/subscribe request")?;

    let uri = params
        .get("uri")
        .and_then(|u| u.as_str())
        .ok_or("Missing 'uri' field in resources/subscribe params")?
        .to_string();

    Ok(ClientRequest::ResourcesSubscribe(uri))
}

fn parse_resource_unsubscribe_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let params = params.ok_or("Missing params for resources/unsubscribe request")?;

    let uri = params
        .get("uri")
        .and_then(|u| u.as_str())
        .ok_or("Missing 'uri' field in resources/unsubscribe params")?
        .to_string();

    Ok(ClientRequest::ResourcesUnsubscribe(uri))
}

// =============================================================================
// CLIENT NOTIFICATION PARSING
// =============================================================================

/// Parse a JSON-RPC notification into a ClientNotification
pub fn parse_client_notification(json: &Value) -> Result<ClientNotification, String> {
    let method = json
        .get("method")
        .and_then(|m| m.as_str())
        .ok_or("Missing method field in notification")?;

    let params = json.get("params");

    match method {
        "notifications/initialized" => {
            let common = parse_common_notification(params)?;
            Ok(ClientNotification::Initialized(common))
        }
        "notifications/roots/list_changed" => {
            let common = parse_common_notification(params)?;
            Ok(ClientNotification::RootsListChanged(common))
        }
        "notifications/cancelled" => {
            let params = params.ok_or("Missing params for cancelled notification")?;

            let request_id_value = params
                .get("requestId")
                .ok_or("Missing 'requestId' field in cancelled notification")?;
            let request_id = parse_request_id(request_id_value)?;

            let reason = params
                .get("reason")
                .and_then(|r| r.as_str())
                .map(|s| s.to_string());

            Ok(ClientNotification::Cancelled(CancelledNotification {
                request_id,
                reason,
            }))
        }
        "notifications/progress" => {
            let params = params.ok_or("Missing params for progress notification")?;

            let progress_token_value = params
                .get("progressToken")
                .ok_or("Missing 'progressToken' field in progress notification")?;

            let progress_token = if let Some(s) = progress_token_value.as_str() {
                ProgressToken::String(s.to_string())
            } else if let Some(i) = progress_token_value.as_i64() {
                ProgressToken::Integer(i)
            } else {
                return Err("Invalid 'progressToken' field: must be string or integer".to_string());
            };

            let progress = params
                .get("progress")
                .and_then(|p| p.as_f64())
                .ok_or("Missing or invalid 'progress' field in progress notification")?;

            let total = params.get("total").and_then(|t| t.as_f64());

            let message = params
                .get("message")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string());

            Ok(ClientNotification::Progress(ProgressNotification {
                progress_token,
                progress,
                total,
                message,
            }))
        }
        _ => Err(format!("Unsupported notification method: {}", method)),
    }
}

/// Parse common notification fields (_meta and extras)
fn parse_common_notification(params: Option<&Value>) -> Result<CommonNotification, String> {
    let (meta, extras) = if let Some(p) = params {
        let meta = p.get("_meta").and_then(|m| serde_json::to_string(m).ok());

        let extras = p.get("extras").and_then(|e| serde_json::to_string(e).ok());

        (meta, extras)
    } else {
        (None, None)
    };

    Ok(CommonNotification { meta, extras })
}

// =============================================================================
// CLIENT RESPONSE PARSING
// =============================================================================

/// Parse a JSON-RPC response into a Result<ClientResponse, ErrorCode>
pub fn parse_client_response(json: &Value) -> Result<Result<ClientResponse, ErrorCode>, String> {
    // Check for error response
    if let Some(error_obj) = json.get("error") {
        let id_value = json.get("id");
        let id = id_value
            .and_then(|v| if v.is_null() { None } else { Some(v) })
            .map(|v| parse_request_id(v))
            .transpose()?;

        let code = error_obj
            .get("code")
            .and_then(|c| c.as_i64())
            .ok_or("Missing or invalid 'code' in error response")?;

        let message = error_obj
            .get("message")
            .and_then(|m| m.as_str())
            .ok_or("Missing 'message' in error response")?
            .to_string();

        let data = error_obj
            .get("data")
            .and_then(|d| serde_json::to_string(d).ok());

        let error = Error {
            id,
            code,
            message,
            data,
        };

        // Map to appropriate ErrorCode variant based on code
        let error_code = match code {
            -32700 => ErrorCode::ParseError(error),
            -32600 => ErrorCode::InvalidRequest(error),
            -32601 => ErrorCode::MethodNotFound(error),
            -32602 => ErrorCode::InvalidParams(error),
            -32603 => ErrorCode::InternalError(error),
            -32099..=-32000 => ErrorCode::Server(error),
            -32768..=-32001 => ErrorCode::JsonRpc(error),
            _ => ErrorCode::Mcp(error),
        };

        return Ok(Err(error_code));
    }

    // Success response - get result field
    let result = json
        .get("result")
        .ok_or("Missing 'result' field in response")?;

    // Infer response type from result structure
    // elicitation-create has "action" field
    if result.get("action").is_some() {
        return parse_elicit_result(result).map(|r| Ok(ClientResponse::ElicitationCreate(r)));
    }

    // roots-list has "roots" array
    if result.get("roots").is_some() {
        return parse_list_roots_result(result).map(|r| Ok(ClientResponse::RootsList(r)));
    }

    // sampling-create-message has "model" and "role" fields
    if result.get("model").is_some() && result.get("role").is_some() {
        return parse_sampling_create_message_result(result)
            .map(|r| Ok(ClientResponse::SamplingCreateMessage(r)));
    }

    Err("Unable to determine response type from result structure".to_string())
}

/// Parse elicitation-create result
fn parse_elicit_result(result: &Value) -> Result<ElicitResult, String> {
    let meta = result
        .get("_meta")
        .and_then(|m| serde_json::to_string(m).ok());

    let action_str = result
        .get("action")
        .and_then(|a| a.as_str())
        .ok_or("Missing 'action' in elicit result")?;

    let action = match action_str {
        "accept" => ElicitResultAction::Accept,
        "decline" => ElicitResultAction::Decline,
        "cancel" => ElicitResultAction::Cancel,
        _ => return Err(format!("Invalid action: {}", action_str)),
    };

    // Parse content object if present
    let content = result
        .get("content")
        .and_then(|c| c.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(key, value)| {
                    if let Some(s) = value.as_str() {
                        Some((key.clone(), ElicitResultContent::String(s.to_string())))
                    } else if let Some(n) = value.as_f64() {
                        Some((key.clone(), ElicitResultContent::Number(n)))
                    } else if let Some(b) = value.as_bool() {
                        Some((key.clone(), ElicitResultContent::Boolean(b)))
                    } else {
                        None
                    }
                })
                .collect()
        });

    Ok(ElicitResult {
        meta,
        action,
        content,
    })
}

/// Parse roots-list result
fn parse_list_roots_result(result: &Value) -> Result<ListRootsResult, String> {
    let meta = result
        .get("_meta")
        .and_then(|m| serde_json::to_string(m).ok());

    let roots_array = result
        .get("roots")
        .and_then(|r| r.as_array())
        .ok_or("Missing or invalid 'roots' array in roots-list result")?;

    let roots: Vec<Root> = roots_array
        .iter()
        .map(|root_value| {
            let uri = root_value
                .get("uri")
                .and_then(|u| u.as_str())
                .ok_or("Missing 'uri' in root object")?
                .to_string();

            let name = root_value
                .get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string());

            let meta = root_value
                .get("_meta")
                .and_then(|m| serde_json::to_string(m).ok());

            Ok(Root { uri, name, meta })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(ListRootsResult { meta, roots })
}

/// Parse sampling-create-message result
fn parse_sampling_create_message_result(
    result: &Value,
) -> Result<SamplingCreateMessageResult, String> {
    let meta = result
        .get("_meta")
        .and_then(|m| serde_json::to_string(m).ok());

    // Parse content - it should be an enum value
    let content_str = result
        .get("content")
        .and_then(|c| c.as_str())
        .ok_or("Missing or invalid 'content' in sampling result")?;

    let content = match content_str {
        "text-content" => SamplingContent::TextContent,
        "image-content" => SamplingContent::ImageContent,
        "audio-content" => SamplingContent::AudioContent,
        _ => return Err(format!("Invalid sampling content type: {}", content_str)),
    };

    let model = result
        .get("model")
        .and_then(|m| m.as_str())
        .ok_or("Missing 'model' in sampling result")?
        .to_string();

    let role_str = result
        .get("role")
        .and_then(|r| r.as_str())
        .ok_or("Missing 'role' in sampling result")?;

    let role = match role_str {
        "user" => Role::User,
        "assistant" => Role::Assistant,
        _ => return Err(format!("Invalid role: {}", role_str)),
    };

    let stop_reason = result
        .get("stopReason")
        .and_then(|sr| sr.as_str())
        .map(|s| s.to_string());

    let extra = result
        .get("extra")
        .and_then(|e| serde_json::to_string(e).ok());

    Ok(SamplingCreateMessageResult {
        meta,
        content,
        model,
        role,
        stop_reason,
        extra,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_request_id() {
        let num_id = json!(42);
        let str_id = json!("test-123");

        let parsed_num = parse_request_id(&num_id).unwrap();
        let parsed_str = parse_request_id(&str_id).unwrap();

        assert!(matches!(parsed_num, RequestId::Number(42)));
        assert!(matches!(parsed_str, RequestId::String(s) if s == "test-123"));
    }

    #[test]
    fn test_parse_protocol_version() {
        assert!(matches!(
            parse_protocol_version("2025-06-18"),
            Ok(ProtocolVersion::V20250618)
        ));
        assert!(parse_protocol_version("invalid").is_err());
    }

    #[test]
    fn test_parse_initialize_request() {
        let request = json!({
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "roots": {
                        "listChanged": true
                    }
                },
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        });

        let parsed = parse_client_request(&request).unwrap();
        assert!(matches!(parsed, ClientRequest::Initialize(_)));
    }
}
