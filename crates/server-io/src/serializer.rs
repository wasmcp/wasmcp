//! JSON-RPC serialization for MCP protocol with transport-specific formatting
//!
//! This module handles conversion from WIT types to JSON-RPC 2.0 format
//! with support for both HTTP (SSE) and stdio (newline-delimited) transports.

use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    Annotations, BlobData, CallToolResult, CompleteResult, ContentBlock, ErrorCode,
    GetPromptResult, Implementation, InitializeResult, ListPromptsResult,
    ListResourceTemplatesResult, ListResourcesResult, ListToolsResult, McpResource, Prompt,
    PromptMessage, ProtocolVersion, ReadResourceResult, RequestId, ResourceContents,
    ResourceTemplate, Role, ServerCapabilities, ServerResult, TextData, Tool,
};
use crate::stream_reader::{StreamConfig, read_blob_stream, read_text_stream};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

// =============================================================================
// SHADOW TYPES FOR SERIALIZATION
// =============================================================================
// These mirror WIT types but are serializable to JSON

#[derive(Serialize)]
#[serde(untagged)]
pub enum JsonRequestId {
    Number(i64),
    String(String),
}

impl From<&RequestId> for JsonRequestId {
    fn from(id: &RequestId) -> Self {
        match id {
            RequestId::Number(n) => JsonRequestId::Number(*n),
            RequestId::String(s) => JsonRequestId::String(s.clone()),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonInitializeResult {
    protocol_version: String,
    capabilities: JsonServerCapabilities,
    server_info: JsonImplementation,
    #[serde(skip_serializing_if = "Option::is_none")]
    instructions: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    completions: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    experimental: Option<Vec<(String, Value)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    logging: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompts: Option<JsonPromptCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resources: Option<JsonResourceCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<JsonToolCapability>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonPromptCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    list_changed: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonResourceCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    list_changed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subscribe: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonToolCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    list_changed: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonImplementation {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    version: String,
}

// Content-related shadow types for streaming support

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonAnnotations {
    #[serde(skip_serializing_if = "Option::is_none")]
    audience: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_modified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonTextContent {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    annotations: Option<JsonAnnotations>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonImageContent {
    data: String, // base64-encoded
    mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    annotations: Option<JsonAnnotations>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonResourceContent {
    uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blob: Option<String>, // base64-encoded
    #[serde(skip_serializing_if = "Option::is_none")]
    mime_type: Option<String>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum JsonContentBlock {
    Text(JsonTextContent),
    Image(JsonImageContent),
    Audio(JsonImageContent), // Audio has same structure as Image (data + mimeType)
    Resource(JsonResourceContent),
}

// Tool-related shadow types

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonToolAnnotations {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    read_only_hint: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    destructive_hint: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    idempotent_hint: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    open_world_hint: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonTool {
    name: String,
    input_schema: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    annotations: Option<JsonToolAnnotations>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonListToolsResult {
    tools: Vec<JsonTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonCallToolResult {
    content: Vec<JsonContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_error: Option<bool>,
}

// Resource-related shadow types

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonResource {
    uri: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonListResourcesResult {
    resources: Vec<JsonResource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonTextResourceContents {
    uri: String,
    mime_type: Option<String>,
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonBlobResourceContents {
    uri: String,
    mime_type: Option<String>,
    blob: String, // base64-encoded
}

#[derive(Serialize)]
#[serde(untagged)]
enum JsonResourceContents {
    Text(JsonTextResourceContents),
    Blob(JsonBlobResourceContents),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonReadResourceResult {
    contents: Vec<JsonResourceContents>,
}

// Prompt-related shadow types

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonPromptArgument {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonPrompt {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    arguments: Option<Vec<JsonPromptArgument>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonListPromptsResult {
    prompts: Vec<JsonPrompt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonPromptMessage {
    role: String,
    content: JsonContentBlock,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonGetPromptResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    messages: Vec<JsonPromptMessage>,
}

// Resource template shadow types

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonResourceTemplate {
    uri_template: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mime_type: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonListResourceTemplatesResult {
    resource_templates: Vec<JsonResourceTemplate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<String>,
}

// Completion shadow types

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonCompleteResult {
    values: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    has_more: Option<bool>,
}

// =============================================================================
// CONVERSION FUNCTIONS
// =============================================================================

fn protocol_version_to_string(version: &ProtocolVersion) -> String {
    match version {
        ProtocolVersion::V20251125 => "2025-11-25".to_string(),
        ProtocolVersion::V20250618 => "2025-06-18".to_string(),
        ProtocolVersion::V20250326 => "2025-03-26".to_string(),
        ProtocolVersion::V20241105 => "2024-11-05".to_string(),
    }
}

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

fn role_to_string(role: &Role) -> String {
    match role {
        Role::User => "user".to_string(),
        Role::Assistant => "assistant".to_string(),
    }
}

fn convert_annotations(annotations: &Annotations) -> JsonAnnotations {
    JsonAnnotations {
        audience: annotations
            .audience
            .as_ref()
            .map(|roles| roles.iter().map(role_to_string).collect()),
        last_modified: annotations.last_modified.clone(),
        priority: annotations.priority,
    }
}

/// Convert TextData (string or stream) to String
///
/// For text-stream, reads the stream in chunks with bounded memory.
fn convert_text_data(data: &TextData) -> Result<String, String> {
    match data {
        TextData::Text(s) => Ok(s.clone()),
        TextData::TextStream(stream) => {
            let config = StreamConfig::default();
            read_text_stream(stream, &config)
        }
    }
}

/// Convert BlobData (bytes or stream) to base64 String
///
/// For blob-stream, reads the stream in chunks with bounded memory.
fn convert_blob_data(data: &BlobData) -> Result<String, String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    match data {
        BlobData::Blob(bytes) => Ok(BASE64.encode(bytes)),
        BlobData::BlobStream(stream) => {
            let config = StreamConfig::default();
            read_blob_stream(stream, &config)
        }
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

/// Convert ContentBlock with streaming support
///
/// This demonstrates the streaming infrastructure in action.
/// Handles text-stream and blob-stream variants with bounded memory.
pub fn convert_content_block(block: &ContentBlock) -> Result<JsonContentBlock, String> {
    match block {
        ContentBlock::Text(text_content) => {
            let text = convert_text_data(&text_content.text)?;
            Ok(JsonContentBlock::Text(JsonTextContent {
                text,
                annotations: text_content
                    .options
                    .as_ref()
                    .and_then(|o| o.annotations.as_ref())
                    .map(convert_annotations),
            }))
        }
        ContentBlock::Image(image_content) => {
            let data = convert_blob_data(&image_content.data)?;
            Ok(JsonContentBlock::Image(JsonImageContent {
                data,
                mime_type: image_content.mime_type.clone(),
                annotations: image_content
                    .options
                    .as_ref()
                    .and_then(|o| o.annotations.as_ref())
                    .map(convert_annotations),
            }))
        }
        ContentBlock::Audio(audio_content) => {
            let data = convert_blob_data(&audio_content.data)?;
            Ok(JsonContentBlock::Audio(JsonImageContent {
                data,
                mime_type: audio_content.mime_type.clone(),
                annotations: audio_content
                    .options
                    .as_ref()
                    .and_then(|o| o.annotations.as_ref())
                    .map(convert_annotations),
            }))
        }
        ContentBlock::ResourceLink(link) => Ok(JsonContentBlock::Resource(JsonResourceContent {
            uri: link.uri.clone(),
            text: None,
            blob: None,
            mime_type: link.options.as_ref().and_then(|o| o.mime_type.clone()),
        })),
        ContentBlock::EmbeddedResource(embedded) => {
            use crate::bindings::wasmcp::mcp_v20250618::mcp::ResourceContents;
            match &embedded.resource {
                ResourceContents::Text(text_res) => {
                    let text = convert_text_data(&text_res.text)?;
                    Ok(JsonContentBlock::Resource(JsonResourceContent {
                        uri: text_res.uri.clone(),
                        text: Some(text),
                        blob: None,
                        mime_type: text_res.options.as_ref().and_then(|o| o.mime_type.clone()),
                    }))
                }
                ResourceContents::Blob(blob_res) => {
                    let blob = convert_blob_data(&blob_res.blob)?;
                    Ok(JsonContentBlock::Resource(JsonResourceContent {
                        uri: blob_res.uri.clone(),
                        text: None,
                        blob: Some(blob),
                        mime_type: blob_res.options.as_ref().and_then(|o| o.mime_type.clone()),
                    }))
                }
            }
        }
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
        // Empty success responses
        ServerResult::Ping => json!({}),
        ServerResult::LoggingSetLevel => json!({}),

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

/// Format a JSON value as an SSE event (HTTP transport)
pub fn format_sse_event(data: &Value) -> String {
    // SSE format: "data: <json>\n\n"
    format!(
        "data: {}\n\n",
        serde_json::to_string(data).unwrap_or_default()
    )
}

/// Format a JSON value as a newline-delimited JSON line (stdio transport)
pub fn format_json_line(data: &Value) -> String {
    // Newline-delimited format: "<json>\n"
    format!("{}\n", serde_json::to_string(data).unwrap_or_default())
}
