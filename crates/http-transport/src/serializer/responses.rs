//! Server response serialization
//!
//! Main serialization functions for converting MCP server responses to JSON-RPC format.

use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    CallToolResult, CompleteResult, ErrorCode, GetPromptResult, Implementation, InitializeResult,
    ListPromptsResult, ListResourceTemplatesResult, ListResourcesResult, ListToolsResult,
    McpResource, Prompt, PromptMessage, ReadResourceResult, RequestId, ResourceContents,
    ResourceTemplate, ServerCapabilities, ServerResult, Tool,
};
use crate::serializer::content::convert_content_block;
use crate::serializer::types::{
    convert_blob_data, convert_text_data, protocol_version_to_string, role_to_string,
    JsonBlobResourceContents, JsonCallToolResult, JsonCompleteResult, JsonGetPromptResult,
    JsonImplementation, JsonInitializeResult, JsonListPromptsResult,
    JsonListResourceTemplatesResult, JsonListResourcesResult, JsonListToolsResult, JsonPrompt,
    JsonPromptArgument, JsonPromptCapability, JsonPromptMessage, JsonReadResourceResult,
    JsonRequestId, JsonResource, JsonResourceCapability, JsonResourceContents,
    JsonResourceTemplate, JsonServerCapabilities, JsonTextResourceContents, JsonTool,
    JsonToolAnnotations, JsonToolCapability,
};
use serde_json::{json, Value};

// =============================================================================
// CONVERSION FUNCTIONS
// =============================================================================

fn convert_server_capabilities(caps: &ServerCapabilities) -> JsonServerCapabilities {
    use crate::bindings::wasmcp::mcp_v20250618::mcp::{ServerLists, ServerSubscriptions};

    JsonServerCapabilities {
        completions: caps
            .completions
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        experimental: caps.experimental.as_ref().map(|exp| {
            exp.iter()
                .filter_map(|(k, v)| serde_json::from_str(v).ok().map(|val| (k.clone(), val)))
                .collect()
        }),
        logging: caps
            .logging
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        prompts: caps
            .list_changed
            .as_ref()
            .filter(|lc| lc.contains(ServerLists::PROMPTS))
            .map(|_| JsonPromptCapability {
                list_changed: Some(true),
            }),
        resources: {
            let list_changed = caps
                .list_changed
                .as_ref()
                .map(|lc| lc.contains(ServerLists::RESOURCES))
                .unwrap_or(false);
            let subscribe = caps
                .subscriptions
                .as_ref()
                .map(|s| s.contains(ServerSubscriptions::RESOURCES))
                .unwrap_or(false);
            if list_changed || subscribe {
                Some(JsonResourceCapability {
                    list_changed: if list_changed { Some(true) } else { None },
                    subscribe: if subscribe { Some(true) } else { None },
                })
            } else {
                None
            }
        },
        tools: caps
            .list_changed
            .as_ref()
            .filter(|lc| lc.contains(ServerLists::TOOLS))
            .map(|_| JsonToolCapability {
                list_changed: Some(true),
            }),
    }
}

fn convert_implementation(impl_info: &Implementation) -> JsonImplementation {
    JsonImplementation {
        name: impl_info.name.clone(),
        title: impl_info.title.clone(),
        version: impl_info.version.clone(),
    }
}

fn convert_initialize_result(result: &InitializeResult) -> JsonInitializeResult {
    JsonInitializeResult {
        protocol_version: protocol_version_to_string(&result.protocol_version),
        capabilities: convert_server_capabilities(&result.capabilities),
        server_info: convert_implementation(&result.server_info),
        instructions: result.options.as_ref().and_then(|o| o.instructions.clone()),
    }
}

fn convert_tool(tool: &Tool) -> Result<JsonTool, String> {
    let input_schema: Value = serde_json::from_str(&tool.input_schema)
        .map_err(|e| format!("Invalid tool input schema JSON: {}", e))?;

    Ok(JsonTool {
        name: tool.name.clone(),
        input_schema,
        description: tool.options.as_ref().and_then(|o| o.description.clone()),
        title: tool.options.as_ref().and_then(|o| o.title.clone()),
        annotations: tool.options.as_ref().and_then(|o| {
            o.annotations.as_ref().map(|a| JsonToolAnnotations {
                title: a.title.clone(),
                read_only_hint: a.read_only_hint,
                destructive_hint: a.destructive_hint,
                idempotent_hint: a.idempotent_hint,
                open_world_hint: a.open_world_hint,
            })
        }),
    })
}

fn convert_list_tools_result(result: &ListToolsResult) -> Result<JsonListToolsResult, String> {
    let tools = result
        .tools
        .iter()
        .map(convert_tool)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(JsonListToolsResult {
        tools,
        next_cursor: result.next_cursor.clone(),
    })
}

fn convert_call_tool_result(result: &CallToolResult) -> Result<JsonCallToolResult, String> {
    let content = result
        .content
        .iter()
        .map(convert_content_block)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(JsonCallToolResult {
        content,
        is_error: result.is_error,
    })
}

fn convert_resource(resource: &McpResource) -> JsonResource {
    JsonResource {
        uri: resource.uri.clone(),
        name: resource.name.clone(),
        description: resource
            .options
            .as_ref()
            .and_then(|o| o.description.clone()),
        mime_type: resource.options.as_ref().and_then(|o| o.mime_type.clone()),
        size: resource.options.as_ref().and_then(|o| o.size),
    }
}

fn convert_list_resources_result(result: &ListResourcesResult) -> JsonListResourcesResult {
    JsonListResourcesResult {
        resources: result.resources.iter().map(convert_resource).collect(),
        next_cursor: result.next_cursor.clone(),
    }
}

fn convert_resource_contents(contents: &ResourceContents) -> Result<JsonResourceContents, String> {
    match contents {
        ResourceContents::Text(text_res) => {
            let text = convert_text_data(&text_res.text)?;
            Ok(JsonResourceContents::Text(JsonTextResourceContents {
                uri: text_res.uri.clone(),
                mime_type: text_res.options.as_ref().and_then(|o| o.mime_type.clone()),
                text,
            }))
        }
        ResourceContents::Blob(blob_res) => {
            let blob = convert_blob_data(&blob_res.blob)?;
            Ok(JsonResourceContents::Blob(JsonBlobResourceContents {
                uri: blob_res.uri.clone(),
                mime_type: blob_res.options.as_ref().and_then(|o| o.mime_type.clone()),
                blob,
            }))
        }
    }
}

fn convert_read_resource_result(
    result: &ReadResourceResult,
) -> Result<JsonReadResourceResult, String> {
    let contents = result
        .contents
        .iter()
        .map(convert_resource_contents)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(JsonReadResourceResult { contents })
}

fn convert_prompt(prompt: &Prompt) -> JsonPrompt {
    JsonPrompt {
        name: prompt.name.clone(),
        description: prompt.options.as_ref().and_then(|o| o.description.clone()),
        arguments: prompt.options.as_ref().and_then(|o| {
            o.arguments.as_ref().map(|args| {
                args.iter()
                    .map(|arg| JsonPromptArgument {
                        name: arg.name.clone(),
                        description: arg.description.clone(),
                        required: arg.required,
                    })
                    .collect()
            })
        }),
    }
}

fn convert_list_prompts_result(result: &ListPromptsResult) -> JsonListPromptsResult {
    JsonListPromptsResult {
        prompts: result.prompts.iter().map(convert_prompt).collect(),
        next_cursor: result.next_cursor.clone(),
    }
}

fn convert_prompt_message(message: &PromptMessage) -> Result<JsonPromptMessage, String> {
    Ok(JsonPromptMessage {
        role: role_to_string(&message.role),
        content: convert_content_block(&message.content)?,
    })
}

fn convert_get_prompt_result(result: &GetPromptResult) -> Result<JsonGetPromptResult, String> {
    let messages = result
        .messages
        .iter()
        .map(convert_prompt_message)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(JsonGetPromptResult {
        description: result.description.clone(),
        messages,
    })
}

fn convert_resource_template(template: &ResourceTemplate) -> JsonResourceTemplate {
    JsonResourceTemplate {
        uri_template: template.uri_template.clone(),
        name: template.name.clone(),
        description: template
            .options
            .as_ref()
            .and_then(|o| o.description.clone()),
        mime_type: template.options.as_ref().and_then(|o| o.mime_type.clone()),
    }
}

fn convert_list_resource_templates_result(
    result: &ListResourceTemplatesResult,
) -> JsonListResourceTemplatesResult {
    JsonListResourceTemplatesResult {
        resource_templates: result
            .resource_templates
            .iter()
            .map(convert_resource_template)
            .collect(),
        next_cursor: result.next_cursor.clone(),
    }
}

fn convert_complete_result(result: &CompleteResult) -> JsonCompleteResult {
    JsonCompleteResult {
        values: result.values.clone(),
        total: result.total,
        has_more: result.has_more,
    }
}

// =============================================================================
// PUBLIC API
// =============================================================================

/// Serialize a JSON-RPC response (success or error) to a JSON value
pub fn serialize_jsonrpc_response(
    id: &RequestId,
    result: Result<&ServerResult, &ErrorCode>,
) -> Value {
    match result {
        Ok(response) => {
            let result_value = serialize_server_response(response);
            json!({
                "jsonrpc": "2.0",
                "id": JsonRequestId::from(id),
                "result": result_value
            })
        }
        Err(error) => {
            let (code, message) = serialize_error_code(error);
            json!({
                "jsonrpc": "2.0",
                "id": JsonRequestId::from(id),
                "error": {
                    "code": code,
                    "message": message
                }
            })
        }
    }
}

/// Serialize a ServerResult to JSON
///
/// Handles all MCP server response types with proper error propagation.
/// Stream data is read with bounded memory via the streaming infrastructure.
pub fn serialize_server_response(response: &ServerResult) -> Value {
    match response {
        ServerResult::Initialize(init_result) => {
            serde_json::to_value(convert_initialize_result(init_result)).unwrap_or_else(|e| {
                json!({
                    "error": format!("Failed to serialize initialize result: {}", e)
                })
            })
        }
        ServerResult::ToolsList(tools_result) => match convert_list_tools_result(tools_result) {
            Ok(json_result) => serde_json::to_value(json_result).unwrap_or_else(|e| {
                json!({
                    "error": format!("Failed to serialize tools list: {}", e)
                })
            }),
            Err(e) => json!({
                "error": format!("Failed to convert tools list: {}", e)
            }),
        },
        ServerResult::ToolsCall(call_result) => match convert_call_tool_result(call_result) {
            Ok(json_result) => serde_json::to_value(json_result).unwrap_or_else(|e| {
                json!({
                    "error": format!("Failed to serialize tool call result: {}", e)
                })
            }),
            Err(e) => json!({
                "error": format!("Failed to convert tool call result: {}", e)
            }),
        },
        ServerResult::ResourcesList(resources_result) => serde_json::to_value(
            convert_list_resources_result(resources_result),
        )
        .unwrap_or_else(|e| {
            json!({
                "error": format!("Failed to serialize resources list: {}", e)
            })
        }),
        ServerResult::ResourcesRead(read_result) => {
            match convert_read_resource_result(read_result) {
                Ok(json_result) => serde_json::to_value(json_result).unwrap_or_else(|e| {
                    json!({
                        "error": format!("Failed to serialize resource contents: {}", e)
                    })
                }),
                Err(e) => json!({
                    "error": format!("Failed to convert resource contents: {}", e)
                }),
            }
        }
        ServerResult::ResourcesTemplatesList(templates_result) => {
            serde_json::to_value(convert_list_resource_templates_result(templates_result))
                .unwrap_or_else(|e| {
                    json!({
                        "error": format!("Failed to serialize resource templates: {}", e)
                    })
                })
        }
        ServerResult::PromptsList(prompts_result) => {
            serde_json::to_value(convert_list_prompts_result(prompts_result)).unwrap_or_else(|e| {
                json!({
                    "error": format!("Failed to serialize prompts list: {}", e)
                })
            })
        }
        ServerResult::PromptsGet(get_prompt_result) => {
            match convert_get_prompt_result(get_prompt_result) {
                Ok(json_result) => serde_json::to_value(json_result).unwrap_or_else(|e| {
                    json!({
                        "error": format!("Failed to serialize prompt result: {}", e)
                    })
                }),
                Err(e) => json!({
                    "error": format!("Failed to convert prompt result: {}", e)
                }),
            }
        }
        ServerResult::CompletionComplete(complete_result) => {
            serde_json::to_value(convert_complete_result(complete_result)).unwrap_or_else(|e| {
                json!({
                    "error": format!("Failed to serialize completion result: {}", e)
                })
            })
        }
    }
}

/// Convert ErrorCode to JSON-RPC error code and message
pub fn serialize_error_code(error: &ErrorCode) -> (i64, String) {
    match error {
        ErrorCode::ParseError(e) => {
            let msg = e.message.clone();
            (-32700, msg)
        }
        ErrorCode::InvalidRequest(e) => {
            let msg = e.message.clone();
            (-32600, msg)
        }
        ErrorCode::MethodNotFound(e) => {
            let msg = e.message.clone();
            (-32601, msg)
        }
        ErrorCode::InvalidParams(e) => {
            let msg = e.message.clone();
            (-32602, msg)
        }
        ErrorCode::InternalError(e) => {
            let msg = e.message.clone();
            (-32603, msg)
        }
        ErrorCode::Server(e) | ErrorCode::JsonRpc(e) | ErrorCode::Mcp(e) => {
            let code = e.code;
            let msg = e.message.clone();
            (code, msg)
        }
    }
}

/// Format a JSON value as an SSE event
pub fn format_sse_event(data: &Value) -> String {
    // SSE format: "data: <json>\n\n"
    format!(
        "data: {}\n\n",
        serde_json::to_string(data).unwrap_or_default()
    )
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_serialization() {
        let num_id = RequestId::Number(42);
        let str_id = RequestId::String("test-123".to_string());

        let json_num = serde_json::to_value(JsonRequestId::from(&num_id)).unwrap();
        let json_str = serde_json::to_value(JsonRequestId::from(&str_id)).unwrap();

        assert_eq!(json_num, json!(42));
        assert_eq!(json_str, json!("test-123"));
    }

    #[test]
    fn test_protocol_version_conversion() {
        use crate::bindings::wasmcp::mcp_v20250618::mcp::ProtocolVersion;
        assert_eq!(
            protocol_version_to_string(&ProtocolVersion::V20250618),
            "2025-06-18"
        );
        assert_eq!(
            protocol_version_to_string(&ProtocolVersion::V20241105),
            "2024-11-05"
        );
    }

    #[test]
    fn test_sse_event_formatting() {
        let data = json!({"test": "value"});
        let event = format_sse_event(&data);
        assert!(event.starts_with("data: "));
        assert!(event.ends_with("\n\n"));
        assert!(event.contains(r#""test":"value""#));
    }
}
