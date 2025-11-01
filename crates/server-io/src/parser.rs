//! JSON-RPC request parsing for MCP protocol
//!
//! This module handles parsing JSON-RPC requests into WIT types.
//! Serde handles validation automatically.

use crate::bindings::exports::wasmcp::mcp_v20250618::server_io::IoError;
use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    Annotations, Blob, BlobData, CallToolRequest, CancelledNotification, ClientCapabilities,
    ClientNotification, ClientRequest, ClientResult, CompleteRequest, CompletionArgument,
    CompletionContext, CompletionPromptReference, CompletionReference, ContentBlock,
    ContentOptions, ElicitResult, ElicitResultAction, ElicitResultContent, Error, ErrorCode,
    GetPromptRequest, Implementation, InitializeRequest, ListPromptsRequest,
    ListResourceTemplatesRequest, ListResourcesRequest, ListRootsResult, ListToolsRequest,
    LogLevel, NotificationOptions, PingRequest, ProgressNotification, ProgressToken,
    ProtocolVersion, ReadResourceRequest, RequestId, Role, Root, SamplingCreateMessageResult,
    TextContent, TextData,
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

fn parse_protocol_version(s: &str) -> Result<ProtocolVersion, IoError> {
    match s {
        "2025-06-18" => Ok(ProtocolVersion::V20250618),
        "2025-03-26" => Ok(ProtocolVersion::V20250326),
        "2024-11-05" => Ok(ProtocolVersion::V20241105),
        _ => Err(IoError::Serialization(format!(
            "Unsupported protocol version: {}",
            s
        ))),
    }
}

fn convert_client_capabilities(caps: JsonClientCapabilities) -> ClientCapabilities {
    use crate::bindings::wasmcp::mcp_v20250618::mcp::ClientLists;

    ClientCapabilities {
        elicitation: caps
            .elicitation
            .and_then(|v| serde_json::to_string(&v).ok()),
        experimental: caps.experimental.map(|exp| {
            exp.into_iter()
                .filter_map(|(k, v)| serde_json::to_string(&v).ok().map(|s| (k, s)))
                .collect()
        }),
        list_changed: caps
            .roots
            .and_then(|r| r.list_changed)
            .and_then(|lc| if lc { Some(ClientLists::ROOTS) } else { None }),
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
pub fn parse_request_id(value: &Value) -> Result<RequestId, IoError> {
    serde_json::from_value::<JsonRequestId>(value.clone())
        .map(RequestId::from)
        .map_err(|e| IoError::Serialization(format!("Invalid request ID: {}", e)))
}

/// Parse a JSON-RPC request into a ClientRequest
pub fn parse_client_request(json: &Value) -> Result<ClientRequest, IoError> {
    let method = json
        .get("method")
        .and_then(|m| m.as_str())
        .ok_or_else(|| IoError::Serialization("Missing method field".to_string()))?;

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
        _ => Err(IoError::Serialization(format!(
            "Unsupported method: {}",
            method
        ))),
    }
}

fn parse_initialize_request(params: Option<&Value>) -> Result<ClientRequest, IoError> {
    let params = params.ok_or_else(|| {
        IoError::Serialization("Missing params for initialize request".to_string())
    })?;

    let json_params: JsonInitializeRequestParams = serde_json::from_value(params.clone())
        .map_err(|e| IoError::Serialization(format!("Invalid initialize params: {}", e)))?;

    let protocol_version = parse_protocol_version(&json_params.protocol_version)?;
    let capabilities = convert_client_capabilities(json_params.capabilities);
    let client_info = convert_implementation(json_params.client_info);

    Ok(ClientRequest::Initialize(InitializeRequest {
        protocol_version,
        capabilities,
        client_info,
    }))
}

fn parse_list_tools_request(params: Option<&Value>) -> Result<ClientRequest, IoError> {
    let cursor = params
        .and_then(|p| p.get("cursor"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string());

    Ok(ClientRequest::ToolsList(ListToolsRequest { cursor }))
}

fn parse_call_tool_request(params: Option<&Value>) -> Result<ClientRequest, IoError> {
    let params = params.ok_or_else(|| {
        IoError::Serialization("Missing params for tools/call request".to_string())
    })?;

    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| {
            IoError::Serialization("Missing 'name' field in tools/call params".to_string())
        })?
        .to_string();

    let arguments = params
        .get("arguments")
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| IoError::Serialization(format!("Failed to serialize arguments: {}", e)))?;

    Ok(ClientRequest::ToolsCall(CallToolRequest {
        name,
        arguments,
    }))
}

fn parse_list_resources_request(params: Option<&Value>) -> Result<ClientRequest, IoError> {
    let cursor = params
        .and_then(|p| p.get("cursor"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string());

    Ok(ClientRequest::ResourcesList(ListResourcesRequest {
        cursor,
    }))
}

fn parse_read_resource_request(params: Option<&Value>) -> Result<ClientRequest, IoError> {
    let params = params.ok_or_else(|| {
        IoError::Serialization("Missing params for resources/read request".to_string())
    })?;

    let uri = params
        .get("uri")
        .and_then(|u| u.as_str())
        .ok_or_else(|| {
            IoError::Serialization("Missing 'uri' field in resources/read params".to_string())
        })?
        .to_string();

    Ok(ClientRequest::ResourcesRead(ReadResourceRequest { uri }))
}

fn parse_list_resource_templates_request(params: Option<&Value>) -> Result<ClientRequest, IoError> {
    let cursor = params
        .and_then(|p| p.get("cursor"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string());

    Ok(ClientRequest::ResourcesTemplatesList(
        ListResourceTemplatesRequest { cursor },
    ))
}

fn parse_list_prompts_request(params: Option<&Value>) -> Result<ClientRequest, IoError> {
    let cursor = params
        .and_then(|p| p.get("cursor"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string());

    Ok(ClientRequest::PromptsList(ListPromptsRequest { cursor }))
}

fn parse_get_prompt_request(params: Option<&Value>) -> Result<ClientRequest, IoError> {
    let params = params.ok_or_else(|| {
        IoError::Serialization("Missing params for prompts/get request".to_string())
    })?;

    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| {
            IoError::Serialization("Missing 'name' field in prompts/get params".to_string())
        })?
        .to_string();

    let arguments = params
        .get("arguments")
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| {
            IoError::Serialization(format!("Failed to serialize prompt arguments: {}", e))
        })?;

    Ok(ClientRequest::PromptsGet(GetPromptRequest {
        name,
        arguments,
    }))
}

fn parse_complete_request(params: Option<&Value>) -> Result<ClientRequest, IoError> {
    let params = params.ok_or_else(|| {
        IoError::Serialization("Missing params for completion/complete request".to_string())
    })?;

    let ref_obj = params.get("ref").ok_or_else(|| {
        IoError::Serialization("Missing 'ref' field in completion/complete params".to_string())
    })?;

    let completion_ref = if let Some(ref_obj_map) = ref_obj.as_object() {
        // Check if it's a prompt reference (has "name" field)
        if let Some(prompt_name) = ref_obj_map.get("name").and_then(|n| n.as_str()) {
            CompletionReference::Prompt(CompletionPromptReference {
                name: prompt_name.to_string(),
                title: ref_obj_map
                    .get("title")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string()),
            })
        } else if let Some(uri) = ref_obj_map.get("uriTemplate").and_then(|u| u.as_str()) {
            // It's a resource template reference - create inline struct
            // Note: CompletionReference variant may need different handling
            return Err(IoError::Serialization(
                "Resource template references in completions not fully supported yet".to_string(),
            ));
        } else {
            return Err(IoError::Serialization(
                "Invalid 'ref' object: must have 'name' or 'uriTemplate'".to_string(),
            ));
        }
    } else {
        return Err(IoError::Serialization(
            "Invalid 'ref' field: must be an object".to_string(),
        ));
    };

    let argument_obj = params.get("argument").ok_or_else(|| {
        IoError::Serialization("Missing 'argument' field in completion/complete params".to_string())
    })?;

    let argument = CompletionArgument {
        name: argument_obj
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| IoError::Serialization("Missing 'name' in argument".to_string()))?
            .to_string(),
        value: argument_obj
            .get("value")
            .and_then(|v| v.as_str())
            .ok_or_else(|| IoError::Serialization("Missing 'value' in argument".to_string()))?
            .to_string(),
    };

    let context = params
        .get("context")
        .map(|ctx| {
            let arguments = ctx
                .get("arguments")
                .map(serde_json::to_string)
                .transpose()
                .map_err(|e| {
                    IoError::Serialization(format!("Failed to serialize context arguments: {}", e))
                })?;
            Ok::<CompletionContext, IoError>(CompletionContext { arguments })
        })
        .transpose()?;

    Ok(ClientRequest::CompletionComplete(CompleteRequest {
        argument,
        ref_: completion_ref,
        context,
    }))
}

fn parse_set_log_level_request(params: Option<&Value>) -> Result<ClientRequest, IoError> {
    let params = params.ok_or_else(|| {
        IoError::Serialization("Missing params for logging/setLevel request".to_string())
    })?;

    let level_str = params
        .get("level")
        .and_then(|l| l.as_str())
        .ok_or_else(|| {
            IoError::Serialization("Missing 'level' field in logging/setLevel params".to_string())
        })?;

    let level = match level_str {
        "debug" => LogLevel::Debug,
        "info" => LogLevel::Info,
        "notice" => LogLevel::Notice,
        "warning" => LogLevel::Warning,
        "error" => LogLevel::Error,
        "critical" => LogLevel::Critical,
        "alert" => LogLevel::Alert,
        "emergency" => LogLevel::Emergency,
        _ => {
            return Err(IoError::Serialization(format!(
                "Invalid log level: {}",
                level_str
            )));
        }
    };

    Ok(ClientRequest::LoggingSetLevel(level))
}

fn parse_ping_request(params: Option<&Value>) -> Result<ClientRequest, IoError> {
    let ping_request = if let Some(p) = params {
        let progress_token = p.get("progressToken").and_then(|pt| {
            if let Some(s) = pt.as_str() {
                Some(ProgressToken::String(s.to_string()))
            } else {
                pt.as_i64().map(ProgressToken::Integer)
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

fn parse_resource_subscribe_request(params: Option<&Value>) -> Result<ClientRequest, IoError> {
    let params = params.ok_or_else(|| {
        IoError::Serialization("Missing params for resources/subscribe request".to_string())
    })?;

    let uri = params
        .get("uri")
        .and_then(|u| u.as_str())
        .ok_or_else(|| {
            IoError::Serialization("Missing 'uri' field in resources/subscribe params".to_string())
        })?
        .to_string();

    Ok(ClientRequest::ResourcesSubscribe(uri))
}

fn parse_resource_unsubscribe_request(params: Option<&Value>) -> Result<ClientRequest, IoError> {
    let params = params.ok_or_else(|| {
        IoError::Serialization("Missing params for resources/unsubscribe request".to_string())
    })?;

    let uri = params
        .get("uri")
        .and_then(|u| u.as_str())
        .ok_or_else(|| {
            IoError::Serialization(
                "Missing 'uri' field in resources/unsubscribe params".to_string(),
            )
        })?
        .to_string();

    Ok(ClientRequest::ResourcesUnsubscribe(uri))
}

/// Parse a JSON-RPC notification into a ClientNotification
pub fn parse_client_notification(json: &Value) -> Result<ClientNotification, IoError> {
    let method = json.get("method").and_then(|m| m.as_str()).ok_or_else(|| {
        IoError::Serialization("Missing method field in notification".to_string())
    })?;

    let params = json.get("params");

    match method {
        "notifications/initialized" => {
            let opts = parse_notification_options(params)?;
            Ok(ClientNotification::Initialized(opts))
        }
        "notifications/roots/list_changed" => {
            let opts = parse_notification_options(params)?;
            Ok(ClientNotification::RootsListChanged(opts))
        }
        "notifications/cancelled" => {
            let params = params.ok_or_else(|| {
                IoError::Serialization("Missing params for cancelled notification".to_string())
            })?;

            let request_id = params.get("requestId").ok_or_else(|| {
                IoError::Serialization("Missing 'requestId' in cancelled notification".to_string())
            })?;
            let request_id = parse_request_id(request_id)?;

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
            let params = params.ok_or_else(|| {
                IoError::Serialization("Missing params for progress notification".to_string())
            })?;

            let progress_token = params.get("progressToken").ok_or_else(|| {
                IoError::Serialization(
                    "Missing 'progressToken' in progress notification".to_string(),
                )
            })?;

            let progress_token = if let Some(s) = progress_token.as_str() {
                ProgressToken::String(s.to_string())
            } else if let Some(i) = progress_token.as_i64() {
                ProgressToken::Integer(i)
            } else {
                return Err(IoError::Serialization(
                    "Invalid progressToken: must be string or integer".to_string(),
                ));
            };

            let progress = params
                .get("progress")
                .and_then(|p| p.as_f64())
                .ok_or_else(|| {
                    IoError::Serialization(
                        "Missing or invalid 'progress' in progress notification".to_string(),
                    )
                })?;

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
        _ => Err(IoError::Serialization(format!(
            "Unsupported notification method: {}",
            method
        ))),
    }
}

/// Parse notification options (_meta and extras)
fn parse_notification_options(params: Option<&Value>) -> Result<NotificationOptions, IoError> {
    if let Some(p) = params {
        let meta = p.get("_meta").and_then(|m| serde_json::to_string(m).ok());

        let extras = p.get("extras").and_then(|e| serde_json::to_string(e).ok());

        Ok(NotificationOptions { meta, extras })
    } else {
        Ok(NotificationOptions {
            meta: None,
            extras: None,
        })
    }
}

/// Parse a JSON-RPC response (result or error) into Result<ClientResult, ErrorCode>
pub fn parse_client_response(json: &Value) -> Result<Result<ClientResult, ErrorCode>, IoError> {
    // Check if it's an error response
    if let Some(error_obj) = json.get("error") {
        let code = error_obj
            .get("code")
            .and_then(|c| c.as_i64())
            .ok_or_else(|| {
                IoError::Serialization("Missing or invalid 'code' in error".to_string())
            })?;

        let message = error_obj
            .get("message")
            .and_then(|m| m.as_str())
            .ok_or_else(|| IoError::Serialization("Missing 'message' in error".to_string()))?
            .to_string();

        let data = error_obj
            .get("data")
            .and_then(|d| serde_json::to_string(d).ok());

        let error = Error {
            code,
            message,
            data,
        };

        // Map JSON-RPC error codes to ErrorCode variants
        let error_code = match code {
            -32700 => ErrorCode::ParseError(error),
            -32600 => ErrorCode::InvalidRequest(error),
            -32601 => ErrorCode::MethodNotFound(error),
            -32602 => ErrorCode::InvalidParams(error),
            -32603 => ErrorCode::InternalError(error),
            -32099..=-32000 => ErrorCode::Server(error),
            -32768..=-32100 => ErrorCode::JsonRpc(error),
            _ => ErrorCode::Mcp(error),
        };

        return Ok(Err(error_code));
    }

    // It's a success response - parse the result
    let result = json
        .get("result")
        .ok_or_else(|| IoError::Serialization("Missing 'result' field in response".to_string()))?;

    // Determine which client response type by checking structure
    // We need to infer the type from the result structure

    // Check for elicit-result (has action field)
    if result.get("action").is_some() {
        return parse_elicit_result(result).map(|r| Ok(ClientResult::ElicitationCreate(r)));
    }

    // Check for list-roots-result (has roots field)
    if result.get("roots").is_some() {
        return parse_list_roots_result(result).map(|r| Ok(ClientResult::RootsList(r)));
    }

    // Check for sampling-create-message-result (has content, model, role, stopReason)
    if result.get("model").is_some() && result.get("role").is_some() {
        return parse_sampling_create_message_result(result)
            .map(|r| Ok(ClientResult::SamplingCreateMessage(r)));
    }

    Err(IoError::Serialization(
        "Unable to determine client response type from result structure".to_string(),
    ))
}

fn parse_elicit_result(result: &Value) -> Result<ElicitResult, IoError> {
    let meta = result
        .get("_meta")
        .and_then(|m| serde_json::to_string(m).ok());

    let action_str = result
        .get("action")
        .and_then(|a| a.as_str())
        .ok_or_else(|| IoError::Serialization("Missing 'action' in elicit result".to_string()))?;

    let action = match action_str {
        "accept" => ElicitResultAction::Accept,
        "decline" => ElicitResultAction::Decline,
        "cancel" => ElicitResultAction::Cancel,
        _ => {
            return Err(IoError::Serialization(format!(
                "Invalid action: {}",
                action_str
            )));
        }
    };

    // Parse content array if present
    let content = result
        .get("content")
        .and_then(|c| c.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(key, value)| {
                    // Try to determine the type and convert
                    if let Some(s) = value.as_str() {
                        Some((key.clone(), ElicitResultContent::String(s.to_string())))
                    } else if let Some(n) = value.as_f64() {
                        Some((key.clone(), ElicitResultContent::Number(n)))
                    } else {
                        value
                            .as_bool()
                            .map(|b| (key.clone(), ElicitResultContent::Boolean(b)))
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

fn parse_list_roots_result(result: &Value) -> Result<ListRootsResult, IoError> {
    let meta = result
        .get("_meta")
        .and_then(|m| serde_json::to_string(m).ok());

    let roots_array = result
        .get("roots")
        .and_then(|r| r.as_array())
        .ok_or_else(|| IoError::Serialization("Missing or invalid 'roots' array".to_string()))?;

    let roots: Vec<Root> = roots_array
        .iter()
        .map(|root_obj| {
            let uri = root_obj
                .get("uri")
                .and_then(|u| u.as_str())
                .ok_or_else(|| IoError::Serialization("Missing 'uri' in root".to_string()))?
                .to_string();

            let name = root_obj
                .get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string());

            let meta = root_obj
                .get("_meta")
                .and_then(|m| serde_json::to_string(m).ok());

            Ok(Root { uri, name, meta })
        })
        .collect::<Result<Vec<_>, IoError>>()?;

    Ok(ListRootsResult { meta, roots })
}

fn parse_sampling_create_message_result(
    result: &Value,
) -> Result<SamplingCreateMessageResult, IoError> {
    let meta = result
        .get("_meta")
        .and_then(|m| serde_json::to_string(m).ok());

    // Parse content - it's a ContentBlock object
    let content_obj = result.get("content").ok_or_else(|| {
        IoError::Serialization("Missing 'content' in sampling result".to_string())
    })?;

    let content = parse_content_block(content_obj)?;

    let model = result
        .get("model")
        .and_then(|m| m.as_str())
        .ok_or_else(|| IoError::Serialization("Missing 'model' in sampling result".to_string()))?
        .to_string();

    let role_str = result
        .get("role")
        .and_then(|r| r.as_str())
        .ok_or_else(|| IoError::Serialization("Missing 'role' in sampling result".to_string()))?;

    let role = match role_str {
        "user" => Role::User,
        "assistant" => Role::Assistant,
        _ => {
            return Err(IoError::Serialization(format!(
                "Invalid role: {}",
                role_str
            )));
        }
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

/// Parse a ContentBlock from sampling response JSON
///
/// Sampling responses contain LLM-generated content (text, image, audio)
/// and do not include streams or resource references.
///
/// Runtime constraints enforced:
/// - Only handles inline text/image/audio
/// - Streams: Not supported (LLMs return complete content; streaming is protocol-level)
/// - Resource links: Not supported (LLMs generate content, not references)
///
/// See: `sampling-create-message-result.content` in mcp.wit for rationale
fn parse_content_block(content: &Value) -> Result<ContentBlock, IoError> {
    let content_type = content
        .get("type")
        .and_then(|t| t.as_str())
        .ok_or_else(|| IoError::Serialization("Missing 'type' in content block".to_string()))?;

    match content_type {
        "text" => {
            // Defensive: Check for textStream (not supported in sampling)
            if content.get("textStream").is_some() {
                return Err(IoError::Serialization(
                    "Text streams not supported in sampling responses. \
                     Sampling protocol handles streaming at the message level, not within content blocks."
                        .to_string(),
                ));
            }

            let text = content
                .get("text")
                .and_then(|t| t.as_str())
                .ok_or_else(|| {
                    IoError::Serialization("Missing 'text' field in text content block".to_string())
                })?
                .to_string();

            let options = parse_content_options(content)?;

            Ok(ContentBlock::Text(TextContent {
                text: TextData::Text(text),
                options,
            }))
        }
        "image" => {
            // Defensive: Check for blobStream (not supported in sampling)
            if content.get("blobStream").is_some() {
                return Err(IoError::Serialization(
                    "Image streams not supported in sampling responses. \
                     LLMs return complete generated images, not streams."
                        .to_string(),
                ));
            }

            let data_b64 = content
                .get("data")
                .and_then(|d| d.as_str())
                .ok_or_else(|| {
                    IoError::Serialization(
                        "Missing 'data' field in image content block".to_string(),
                    )
                })?;

            let mime_type = content
                .get("mimeType")
                .and_then(|m| m.as_str())
                .ok_or_else(|| {
                    IoError::Serialization(
                        "Missing 'mimeType' field in image content block".to_string(),
                    )
                })?
                .to_string();

            // Decode base64 data
            use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
            let data = BASE64.decode(data_b64).map_err(|e| {
                IoError::Serialization(format!("Invalid base64 in image data: {}", e))
            })?;

            let options = parse_content_options(content)?;

            Ok(ContentBlock::Image(Blob {
                data: BlobData::Blob(data),
                mime_type,
                options,
            }))
        }
        "audio" => {
            // Defensive: Check for blobStream (not supported in sampling)
            if content.get("blobStream").is_some() {
                return Err(IoError::Serialization(
                    "Audio streams not supported in sampling responses. \
                     LLMs return complete generated audio, not streams."
                        .to_string(),
                ));
            }

            let data_b64 = content
                .get("data")
                .and_then(|d| d.as_str())
                .ok_or_else(|| {
                    IoError::Serialization(
                        "Missing 'data' field in audio content block".to_string(),
                    )
                })?;

            let mime_type = content
                .get("mimeType")
                .and_then(|m| m.as_str())
                .ok_or_else(|| {
                    IoError::Serialization(
                        "Missing 'mimeType' field in audio content block".to_string(),
                    )
                })?
                .to_string();

            // Decode base64 data
            use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
            let data = BASE64.decode(data_b64).map_err(|e| {
                IoError::Serialization(format!("Invalid base64 in audio data: {}", e))
            })?;

            let options = parse_content_options(content)?;

            Ok(ContentBlock::Audio(Blob {
                data: BlobData::Blob(data),
                mime_type,
                options,
            }))
        }
        "resource" => Err(IoError::Serialization(
            "Resource content blocks not expected in sampling responses. \
             LLMs generate new content, not resource references. \
             Resource links are for prompt messages sent to LLMs, not sampling results."
                .to_string(),
        )),
        _ => Err(IoError::Serialization(format!(
            "Unsupported content block type for sampling: '{}'. \
             Expected 'text', 'image', or 'audio'.",
            content_type
        ))),
    }
}

/// Parse optional content-options from a content block JSON object
fn parse_content_options(content: &Value) -> Result<Option<ContentOptions>, IoError> {
    let meta = content
        .get("_meta")
        .and_then(|m| serde_json::to_string(m).ok());

    let annotations = content
        .get("annotations")
        .map(parse_annotations)
        .transpose()?;

    if meta.is_some() || annotations.is_some() {
        Ok(Some(ContentOptions { annotations, meta }))
    } else {
        Ok(None)
    }
}

/// Parse annotations from JSON
fn parse_annotations(annot: &Value) -> Result<Annotations, IoError> {
    let audience = annot
        .get("audience")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    v.as_str().and_then(|s| match s {
                        "user" => Some(Role::User),
                        "assistant" => Some(Role::Assistant),
                        _ => None,
                    })
                })
                .collect::<Vec<_>>()
        })
        .filter(|v| !v.is_empty());

    let last_modified = annot
        .get("lastModified")
        .and_then(|m| m.as_str())
        .map(String::from);

    let priority = annot.get("priority").and_then(|p| p.as_f64());

    Ok(Annotations {
        audience,
        last_modified,
        priority,
    })
}
