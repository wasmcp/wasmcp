//! Shadow types for JSON serialization
//!
//! These types mirror WIT types but are serializable to JSON.
//! They act as an intermediate representation between WIT and JSON.

use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    Annotations, BlobData, ProtocolVersion, Role, TextData,
};
use crate::stream_reader::{read_blob_stream, read_text_stream, StreamConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// =============================================================================
// REQUEST ID TYPES
// =============================================================================

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRequestId {
    Number(i64),
    String(String),
}

impl From<&crate::bindings::wasmcp::mcp_v20250618::mcp::RequestId> for JsonRequestId {
    fn from(id: &crate::bindings::wasmcp::mcp_v20250618::mcp::RequestId) -> Self {
        use crate::bindings::wasmcp::mcp_v20250618::mcp::RequestId;
        match id {
            RequestId::Number(n) => JsonRequestId::Number(*n),
            RequestId::String(s) => JsonRequestId::String(s.clone()),
        }
    }
}

// =============================================================================
// INITIALIZATION TYPES
// =============================================================================

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonInitializeResult {
    pub protocol_version: String,
    pub capabilities: JsonServerCapabilities,
    pub server_info: JsonImplementation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completions: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Vec<(String, Value)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<JsonPromptCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<JsonResourceCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<JsonToolCapability>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonPromptCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonResourceCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonToolCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonImplementation {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub version: String,
}

// =============================================================================
// CONTENT TYPES
// =============================================================================

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonAnnotations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonTextContent {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<JsonAnnotations>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonImageContent {
    pub data: String, // base64-encoded
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<JsonAnnotations>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonResourceContent {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>, // base64-encoded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum JsonContentBlock {
    Text(JsonTextContent),
    Image(JsonImageContent),
    Audio(JsonImageContent), // Audio has same structure as Image (data + mimeType)
    Resource(JsonResourceContent),
}

// =============================================================================
// TOOL TYPES
// =============================================================================

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonToolAnnotations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only_hint: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destructive_hint: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotent_hint: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_world_hint: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonTool {
    pub name: String,
    pub input_schema: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<JsonToolAnnotations>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonListToolsResult {
    pub tools: Vec<JsonTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonCallToolResult {
    pub content: Vec<JsonContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

// =============================================================================
// RESOURCE TYPES
// =============================================================================

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonResource {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonListResourcesResult {
    pub resources: Vec<JsonResource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonTextResourceContents {
    pub uri: String,
    pub mime_type: Option<String>,
    pub text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonBlobResourceContents {
    pub uri: String,
    pub mime_type: Option<String>,
    pub blob: String, // base64-encoded
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum JsonResourceContents {
    Text(JsonTextResourceContents),
    Blob(JsonBlobResourceContents),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonReadResourceResult {
    pub contents: Vec<JsonResourceContents>,
}

// =============================================================================
// PROMPT TYPES
// =============================================================================

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonPromptArgument {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonPrompt {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<JsonPromptArgument>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonListPromptsResult {
    pub prompts: Vec<JsonPrompt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonPromptMessage {
    pub role: String,
    pub content: JsonContentBlock,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonGetPromptResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub messages: Vec<JsonPromptMessage>,
}

// =============================================================================
// RESOURCE TEMPLATE TYPES
// =============================================================================

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonResourceTemplate {
    pub uri_template: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonListResourceTemplatesResult {
    pub resource_templates: Vec<JsonResourceTemplate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

// =============================================================================
// COMPLETION TYPES
// =============================================================================

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonCompleteResult {
    pub values: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
}

// =============================================================================
// CONVERSION HELPER FUNCTIONS
// =============================================================================

pub fn protocol_version_to_string(version: &ProtocolVersion) -> String {
    match version {
        ProtocolVersion::V20250618 => "2025-06-18".to_string(),
        ProtocolVersion::V20250326 => "2025-03-26".to_string(),
        ProtocolVersion::V20241105 => "2024-11-05".to_string(),
    }
}

pub fn role_to_string(role: &Role) -> String {
    match role {
        Role::User => "user".to_string(),
        Role::Assistant => "assistant".to_string(),
    }
}

pub fn convert_annotations(annotations: &Annotations) -> JsonAnnotations {
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
pub fn convert_text_data(data: &TextData) -> Result<String, String> {
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
pub fn convert_blob_data(data: &BlobData) -> Result<String, String> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

    match data {
        BlobData::Blob(bytes) => Ok(BASE64.encode(bytes)),
        BlobData::BlobStream(stream) => {
            let config = StreamConfig::default();
            read_blob_stream(stream, &config)
        }
    }
}
