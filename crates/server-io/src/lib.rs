//! Unified Server I/O Implementation
//!
//! Implements the server-io interface with transport-variant support.
//! Handles bidirectional JSON-RPC message exchange with transport-specific formatting.
//!
//! Architecture:
//! - parser: Handles JSON-RPC parsing for client messages
//! - serializer: Handles JSON-RPC serialization for server messages
//! - stream_reader: Handles bounded memory stream I/O
//!
//! This component provides full spec-compliant MCP 2025-06-18 message handling
//! for both HTTP (SSE) and stdio (newline-delimited) transports via WIT variants.

#![allow(warnings)]

mod bindings {
    wit_bindgen::generate!({
        world: "server-io",
        generate_all,
    });
}

mod parser;
mod serializer;
mod stream_reader;

#[cfg(test)]
mod tests;

use bindings::exports::wasmcp::mcp_v20250618::server_io::{Guest, IoError, TransportType};
use bindings::wasi::io::streams::{InputStream, OutputStream, StreamError};
use bindings::wasmcp::mcp_v20250618::mcp::*;

use crate::stream_reader::StreamConfig;

struct ServerIo;

impl Guest for ServerIo {
    // =========================================================================
    // PARSE FUNCTIONS (reading FROM client)
    // =========================================================================

    fn parse_request(
        transport: TransportType,
        input: &InputStream,
    ) -> Result<(RequestId, ClientRequest), IoError> {
        // Read JSON-RPC from input stream (transport-specific)
        let json_str = read_transport_input(&transport, input)?;

        // Parse as JSON
        let json: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| IoError::Serialization(format!("Invalid JSON: {}", e)))?;

        // Extract request ID
        let id = json
            .get("id")
            .ok_or_else(|| IoError::Unexpected("Missing 'id' field in request".to_string()))?;
        let request_id = parser::parse_request_id(id)?;

        // Parse client request
        let client_request = parser::parse_client_request(&json)?;

        Ok((request_id, client_request))
    }

    fn parse_result(
        transport: TransportType,
        input: &InputStream,
    ) -> Result<(RequestId, ClientResult), IoError> {
        // Read JSON-RPC from input stream (transport-specific)
        let json_str = read_transport_input(&transport, input)?;

        // Parse as JSON
        let json: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| IoError::Serialization(format!("Invalid JSON: {}", e)))?;

        // Extract request ID
        let id = json
            .get("id")
            .ok_or_else(|| IoError::Unexpected("Missing 'id' field in response".to_string()))?;
        let request_id = parser::parse_request_id(id)?;

        // Parse client result from JSON
        let response = parser::parse_client_response(&json)?;

        match response {
            Ok(client_result) => Ok((request_id, client_result)),
            Err(_) => Err(IoError::Unexpected(
                "Expected result, got error response".to_string(),
            )),
        }
    }

    fn parse_error(
        transport: TransportType,
        input: &InputStream,
    ) -> Result<(Option<RequestId>, ErrorCode), IoError> {
        // Read JSON-RPC from input stream (transport-specific)
        let json_str = read_transport_input(&transport, input)?;

        // Parse as JSON
        let json: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| IoError::Serialization(format!("Invalid JSON: {}", e)))?;

        // Extract request ID (optional for errors per JSON-RPC spec)
        let request_id = json.get("id").and_then(|id| {
            if id.is_null() {
                None
            } else {
                parser::parse_request_id(id).ok()
            }
        });

        // Parse error code from JSON
        let response = parser::parse_client_response(&json)?;

        match response {
            Err(error_code) => Ok((request_id, error_code)),
            Ok(_) => Err(IoError::Unexpected(
                "Expected error, got result response".to_string(),
            )),
        }
    }

    fn parse_notification(
        transport: TransportType,
        input: &InputStream,
    ) -> Result<ClientNotification, IoError> {
        // Read JSON-RPC from input stream (transport-specific)
        let json_str = read_transport_input(&transport, input)?;

        // Parse as JSON
        let json: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| IoError::Serialization(format!("Invalid JSON: {}", e)))?;

        // Parse client notification
        parser::parse_client_notification(&json)
    }

    // =========================================================================
    // WRITE FUNCTIONS (writing TO client)
    // =========================================================================

    fn write_request(
        transport: TransportType,
        output: &OutputStream,
        request: ServerRequest,
    ) -> Result<(), IoError> {
        // Generate a unique request ID for server-initiated requests
        use std::sync::atomic::{AtomicI64, Ordering};
        static REQUEST_COUNTER: AtomicI64 = AtomicI64::new(1);
        let request_id = REQUEST_COUNTER.fetch_add(1, Ordering::SeqCst);

        let (method, params) = serialize_server_request(&request);

        let json_rpc = serde_json::json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params
        });

        write_transport_output(&transport, output, &json_rpc)
    }

    fn write_result(
        transport: TransportType,
        output: &OutputStream,
        id: RequestId,
        result: ServerResult,
    ) -> Result<(), IoError> {
        let json_rpc = serializer::serialize_jsonrpc_response(&id, Ok(&result));
        write_transport_output(&transport, output, &json_rpc)
    }

    fn write_error(
        transport: TransportType,
        output: &OutputStream,
        id: Option<RequestId>,
        error: ErrorCode,
    ) -> Result<(), IoError> {
        let (code, message) = serializer::serialize_error_code(&error);

        let json_rpc = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id.as_ref().map(serialize_request_id),
            "error": {
                "code": code,
                "message": message
            }
        });

        write_transport_output(&transport, output, &json_rpc)
    }

    fn write_notification(
        transport: TransportType,
        output: &OutputStream,
        notification: ServerNotification,
    ) -> Result<(), IoError> {
        let (method, params) = serialize_server_notification(&notification);

        let json_rpc = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        write_transport_output(&transport, output, &json_rpc)
    }
}

// =============================================================================
// TRANSPORT-SPECIFIC STREAM I/O
// =============================================================================

fn read_transport_input(
    transport: &TransportType,
    stream: &InputStream,
) -> Result<String, IoError> {
    match transport {
        TransportType::Http => {
            // HTTP: Read entire stream (SSE formatted)
            let config = StreamConfig::default();
            let bytes = stream_reader::read_bytes_chunked(stream, &config)
                .map_err(|e| IoError::Unexpected(e))?;
            String::from_utf8(bytes)
                .map_err(|e| IoError::Unexpected(format!("Invalid UTF-8: {}", e)))
        }
        TransportType::Stdio => {
            // Stdio: Read until newline delimiter
            read_line(stream)
        }
    }
}

fn read_line(stream: &InputStream) -> Result<String, IoError> {
    const MAX_LINE_SIZE: usize = 10 * 1024 * 1024; // 10MB max per line
    const CHUNK_SIZE: usize = 4096; // Read 4KB chunks
    let mut buffer = Vec::new();

    loop {
        if buffer.len() >= MAX_LINE_SIZE {
            return Err(IoError::Unexpected(format!(
                "Line exceeds maximum size of {} bytes",
                MAX_LINE_SIZE
            )));
        }

        let chunk = stream.read(CHUNK_SIZE as u64).map_err(|e| IoError::Stream(e))?;

        if chunk.is_empty() {
            if buffer.is_empty() {
                return Err(IoError::Stream(StreamError::Closed));
            } else {
                break;
            }
        }

        if let Some(pos) = chunk.iter().position(|&b| b == b'\n') {
            buffer.extend_from_slice(&chunk[..pos]);
            break;
        } else {
            buffer.extend_from_slice(&chunk);
        }
    }

    String::from_utf8(buffer).map_err(|e| IoError::Unexpected(format!("Invalid UTF-8: {}", e)))
}

fn write_transport_output(
    transport: &TransportType,
    stream: &OutputStream,
    data: &serde_json::Value,
) -> Result<(), IoError> {
    let formatted = match transport {
        TransportType::Http => serializer::format_sse_event(data),
        TransportType::Stdio => serializer::format_json_line(data),
    };

    let bytes = formatted.as_bytes();
    let mut offset = 0;

    while offset < bytes.len() {
        match stream.check_write() {
            Ok(0) => break,
            Ok(budget) => {
                let chunk_size = (bytes.len() - offset).min(budget as usize);
                stream
                    .write(&bytes[offset..offset + chunk_size])
                    .map_err(|e| IoError::Stream(e))?;
                offset += chunk_size;
            }
            Err(e) => return Err(IoError::Stream(e)),
        }
    }

    Ok(())
}

// =============================================================================
// SERIALIZATION HELPERS FOR SERVER MESSAGES
// =============================================================================

fn serialize_request_id(id: &RequestId) -> serde_json::Value {
    match id {
        RequestId::String(s) => serde_json::Value::String(s.clone()),
        RequestId::Number(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
    }
}

fn serialize_server_notification(
    notification: &ServerNotification,
) -> (&'static str, serde_json::Value) {
    match notification {
        ServerNotification::ToolsListChanged(opts) => (
            "notifications/tools/list_changed",
            serialize_notification_options(opts),
        ),
        ServerNotification::ResourcesListChanged(opts) => (
            "notifications/resources/list_changed",
            serialize_notification_options(opts),
        ),
        ServerNotification::ResourcesUpdated(resource_updated) => {
            let mut params = serde_json::Map::new();
            params.insert(
                "uri".to_string(),
                serde_json::Value::String(resource_updated.uri.clone()),
            );

            if let Some(ref meta) = resource_updated.meta {
                if let Ok(meta_value) = serde_json::from_str::<serde_json::Value>(meta) {
                    params.insert("_meta".to_string(), meta_value);
                }
            }

            (
                "notifications/resources/updated",
                serde_json::Value::Object(params),
            )
        }
        ServerNotification::PromptsListChanged(opts) => (
            "notifications/prompts/list_changed",
            serialize_notification_options(opts),
        ),
        ServerNotification::Log(log_msg) => (
            "notifications/message",
            serde_json::json!({
                "level": log_level_to_string(&log_msg.level),
                "logger": log_msg.logger,
                "data": log_msg.data,
            }),
        ),
        ServerNotification::Cancellation(cancelled) => (
            "notifications/cancelled",
            serde_json::json!({
                "requestId": serialize_request_id(&cancelled.request_id),
                "reason": cancelled.reason,
            }),
        ),
        ServerNotification::Progress(progress) => {
            let progress_token_value = match &progress.progress_token {
                ProgressToken::String(s) => serde_json::Value::String(s.clone()),
                ProgressToken::Integer(i) => {
                    serde_json::Value::Number(serde_json::Number::from(*i))
                }
            };

            let mut params = serde_json::Map::new();
            params.insert("progressToken".to_string(), progress_token_value);
            params.insert("progress".to_string(), serde_json::json!(progress.progress));

            if let Some(ref t) = progress.total {
                params.insert("total".to_string(), serde_json::json!(t));
            }

            if let Some(ref m) = progress.message {
                params.insert("message".to_string(), serde_json::Value::String(m.clone()));
            }

            ("notifications/progress", serde_json::Value::Object(params))
        }
    }
}

fn serialize_notification_options(opts: &NotificationOptions) -> serde_json::Value {
    let mut params = serde_json::Map::new();

    // Add _meta field if present
    if let Some(ref meta) = opts.meta {
        if let Ok(meta_value) = serde_json::from_str::<serde_json::Value>(meta) {
            params.insert("_meta".to_string(), meta_value);
        }
    }

    // Unpack extras as arbitrary key-value pairs at the root params level
    if let Some(ref extras) = opts.extras {
        if let Ok(serde_json::Value::Object(extras_obj)) =
            serde_json::from_str::<serde_json::Value>(extras)
        {
            for (key, value) in extras_obj {
                params.insert(key, value);
            }
        }
    }

    serde_json::Value::Object(params)
}

fn serialize_server_request(request: &ServerRequest) -> (&'static str, serde_json::Value) {
    match request {
        ServerRequest::ElicitationCreate(elicit_req) => (
            "elicitation/create",
            serde_json::json!({
                "message": elicit_req.message,
                "requestedSchema": serialize_requested_schema(&elicit_req.requested_schema),
            }),
        ),
        ServerRequest::RootsList(roots_req) => {
            let mut params = serde_json::Map::new();
            if let Some(ref token) = roots_req.progress_token {
                let token_value = match token {
                    ProgressToken::String(s) => serde_json::Value::String(s.clone()),
                    ProgressToken::Integer(i) => {
                        serde_json::Value::Number(serde_json::Number::from(*i))
                    }
                };
                params.insert("progressToken".to_string(), token_value);
            }
            ("roots/list", serde_json::Value::Object(params))
        }
        ServerRequest::SamplingCreateMessage(sampling_req) => (
            "sampling/createMessage",
            serde_json::json!({
                "messages": sampling_req.messages.iter().map(serialize_sampling_message).collect::<Vec<_>>(),
                "modelPreferences": sampling_req.model_preferences.as_ref().map(serialize_model_preferences),
                "systemPrompt": sampling_req.system_prompt,
                "includeContext": serialize_include_context(&sampling_req.include_context),
                "temperature": sampling_req.temperature,
                "maxTokens": sampling_req.max_tokens,
                "stopSequences": sampling_req.stop_sequences,
                "metadata": sampling_req.metadata.as_ref().and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()),
            }),
        ),
        ServerRequest::Ping(ping_req) => {
            let mut params = serde_json::Map::new();
            if let Some(ref token) = ping_req.progress_token {
                let token_value = match token {
                    ProgressToken::String(s) => serde_json::Value::String(s.clone()),
                    ProgressToken::Integer(i) => {
                        serde_json::Value::Number(serde_json::Number::from(*i))
                    }
                };
                params.insert("progressToken".to_string(), token_value);
            }
            if let Some(ref meta) = ping_req.meta {
                if let Ok(meta_value) = serde_json::from_str::<serde_json::Value>(meta) {
                    params.insert("_meta".to_string(), meta_value);
                }
            }
            for (k, v) in &ping_req.extras {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(v) {
                    params.insert(k.clone(), value);
                }
            }
            ("ping", serde_json::Value::Object(params))
        }
    }
}

fn serialize_requested_schema(schema: &RequestedSchema) -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": schema.properties.iter().map(|(k, v)| {
            (k.clone(), serialize_primitive_schema(v))
        }).collect::<serde_json::Map<_, _>>(),
        "required": schema.required,
    })
}

fn serialize_primitive_schema(schema: &PrimitiveSchemaDefinition) -> serde_json::Value {
    match schema {
        PrimitiveSchemaDefinition::StringSchema(s) => {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "type".to_string(),
                serde_json::Value::String("string".to_string()),
            );
            if let Some(ref desc) = s.description {
                obj.insert(
                    "description".to_string(),
                    serde_json::Value::String(desc.clone()),
                );
            }
            if let Some(ref title) = s.title {
                obj.insert(
                    "title".to_string(),
                    serde_json::Value::String(title.clone()),
                );
            }
            if let Some(ref format) = s.format {
                let format_str = match format {
                    StringSchemaFormat::Uri => "uri",
                    StringSchemaFormat::Email => "email",
                    StringSchemaFormat::Date => "date",
                    StringSchemaFormat::DateTime => "date-time",
                };
                obj.insert(
                    "format".to_string(),
                    serde_json::Value::String(format_str.to_string()),
                );
            }
            if let Some(min_len) = s.min_length {
                obj.insert("minLength".to_string(), serde_json::json!(min_len));
            }
            if let Some(max_len) = s.max_length {
                obj.insert("maxLength".to_string(), serde_json::json!(max_len));
            }
            serde_json::Value::Object(obj)
        }
        PrimitiveSchemaDefinition::NumberSchema(n) => {
            let mut obj = serde_json::Map::new();
            let type_str = match n.type_ {
                NumberSchemaType::Number => "number",
                NumberSchemaType::Integer => "integer",
            };
            obj.insert(
                "type".to_string(),
                serde_json::Value::String(type_str.to_string()),
            );
            if let Some(ref desc) = n.description {
                obj.insert(
                    "description".to_string(),
                    serde_json::Value::String(desc.clone()),
                );
            }
            if let Some(ref title) = n.title {
                obj.insert(
                    "title".to_string(),
                    serde_json::Value::String(title.clone()),
                );
            }
            if let Some(minimum) = n.minimum {
                obj.insert("minimum".to_string(), serde_json::json!(minimum));
            }
            if let Some(maximum) = n.maximum {
                obj.insert("maximum".to_string(), serde_json::json!(maximum));
            }
            serde_json::Value::Object(obj)
        }
        PrimitiveSchemaDefinition::BooleanSchema(b) => {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "type".to_string(),
                serde_json::Value::String("boolean".to_string()),
            );
            if let Some(ref desc) = b.description {
                obj.insert(
                    "description".to_string(),
                    serde_json::Value::String(desc.clone()),
                );
            }
            if let Some(ref title) = b.title {
                obj.insert(
                    "title".to_string(),
                    serde_json::Value::String(title.clone()),
                );
            }
            if let Some(default) = b.default {
                obj.insert("default".to_string(), serde_json::json!(default));
            }
            serde_json::Value::Object(obj)
        }
        PrimitiveSchemaDefinition::EnumSchema(e) => {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "enum".to_string(),
                serde_json::Value::Array(
                    e.enum_
                        .iter()
                        .map(|s| serde_json::Value::String(s.clone()))
                        .collect(),
                ),
            );
            if let Some(ref desc) = e.description {
                obj.insert(
                    "description".to_string(),
                    serde_json::Value::String(desc.clone()),
                );
            }
            if let Some(ref title) = e.title {
                obj.insert(
                    "title".to_string(),
                    serde_json::Value::String(title.clone()),
                );
            }
            if let Some(ref enum_names) = e.enum_names {
                obj.insert(
                    "enumNames".to_string(),
                    serde_json::Value::Array(
                        enum_names
                            .iter()
                            .map(|s| serde_json::Value::String(s.clone()))
                            .collect(),
                    ),
                );
            }
            serde_json::Value::Object(obj)
        }
    }
}

fn serialize_sampling_message(msg: &SamplingMessage) -> serde_json::Value {
    serde_json::json!({
        "role": match msg.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        },
        "content": serialize_content_block(&msg.content)
    })
}

fn serialize_content_block(content: &ContentBlock) -> serde_json::Value {
    // Use serializer module for full content block serialization
    // This handles all content types including streams
    match serializer::convert_content_block(content) {
        Ok(json_block) => serde_json::to_value(json_block).unwrap_or_else(|e| {
            serde_json::json!({
                "type": "text",
                "text": format!("[error serializing content: {}]", e)
            })
        }),
        Err(e) => serde_json::json!({
            "type": "text",
            "text": format!("[error converting content: {}]", e)
        }),
    }
}

fn serialize_model_preferences(prefs: &ModelPreferences) -> serde_json::Value {
    serde_json::json!({
        "hints": prefs.hints.as_ref().map(|hints| {
            hints.iter().map(|h| serde_json::json!({
                "name": h.name,
                "extra": h.extra.as_ref().and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()),
            })).collect::<Vec<_>>()
        }),
        "costPriority": prefs.cost_priority,
        "speedPriority": prefs.speed_priority,
        "intelligencePriority": prefs.intelligence_priority,
    })
}

fn serialize_include_context(ctx: &IncludeContext) -> &'static str {
    match ctx {
        IncludeContext::None => "none",
        IncludeContext::ThisServer => "thisServer",
        IncludeContext::AllServers => "allServers",
    }
}

fn log_level_to_string(level: &LogLevel) -> &'static str {
    match level {
        LogLevel::Debug => "debug",
        LogLevel::Info => "info",
        LogLevel::Notice => "notice",
        LogLevel::Warning => "warning",
        LogLevel::Error => "error",
        LogLevel::Critical => "critical",
        LogLevel::Alert => "alert",
        LogLevel::Emergency => "emergency",
    }
}

// Re-export internal modules for use by serializer
use serializer::convert_content_block;

bindings::export!(ServerIo with_types_in bindings);
