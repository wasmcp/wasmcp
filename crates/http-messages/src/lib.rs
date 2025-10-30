//! HTTP/SSE Server Messages Implementation
//!
//! Implements server-messages functions for Server-Sent Events (SSE) transport.
//! Writes JSON-RPC notifications and requests as SSE events to the provided output stream.

#![allow(warnings)]

mod bindings {
    wit_bindgen::generate!({
        world: "http-messages",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp_v20250618::server_messages::Guest;
use bindings::exports::wasmcp::mcp_v20250618::server_messages::MessageError;
use bindings::wasi::io::streams::{InputStream, OutputStream, StreamError};
use bindings::wasmcp::mcp_v20250618::mcp::*;

struct HttpMessages;

impl Guest for HttpMessages {
    fn notify(output: &OutputStream, notification: ServerNotification) -> Result<(), MessageError> {
        let (method, params) = serialize_server_notification(&notification);

        let json_rpc = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        write_sse_event(output, &json_rpc)
    }

    fn request(output: &OutputStream, request: ServerRequest) -> Result<(), MessageError> {
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

        write_sse_event(output, &json_rpc)
    }
}

// Helper functions

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
    // The spec allows: params?: { _meta?: {...}, [key: string]: unknown }
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

fn write_sse_event(stream: &OutputStream, data: &serde_json::Value) -> Result<(), MessageError> {
    // Format as SSE event: "data: {json}\n\n"
    let json_str = serde_json::to_string(data)
        .map_err(|e| MessageError::Serialization(format!("JSON serialization failed: {}", e)))?;
    let event_data = format!("data: {}\n\n", json_str);

    // Write using check_write() to respect budget
    let bytes = event_data.as_bytes();
    let mut offset = 0;

    while offset < bytes.len() {
        match stream.check_write() {
            Ok(0) => break, // No budget available - stop writing
            Ok(budget) => {
                let chunk_size = (bytes.len() - offset).min(budget as usize);
                let chunk = &bytes[offset..offset + chunk_size];
                stream.write(chunk).map_err(|e| MessageError::Io(e))?;
                offset += chunk_size;
            }
            Err(e) => {
                return Err(MessageError::Io(e));
            }
        }
    }

    Ok(())
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

fn serialize_request_id(id: &RequestId) -> serde_json::Value {
    match id {
        RequestId::String(s) => serde_json::Value::String(s.clone()),
        RequestId::Number(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
    }
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

// Helper functions for reading streams

fn read_text_stream(stream: &InputStream) -> Result<String, String> {
    const MAX_SIZE: u64 = 50 * 1024 * 1024; // 50MB
    const CHUNK_SIZE: u64 = 64 * 1024; // 64KB

    let bytes = read_stream_chunked(stream, MAX_SIZE, CHUNK_SIZE)?;
    String::from_utf8(bytes).map_err(|e| format!("Invalid UTF-8: {}", e))
}

fn read_blob_stream(stream: &InputStream) -> Result<String, String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    const MAX_SIZE: u64 = 50 * 1024 * 1024; // 50MB
    const CHUNK_SIZE: u64 = 64 * 1024; // 64KB

    let bytes = read_stream_chunked(stream, MAX_SIZE, CHUNK_SIZE)?;
    Ok(BASE64.encode(&bytes))
}

fn read_stream_chunked(
    stream: &InputStream,
    max_size: u64,
    chunk_size: u64,
) -> Result<Vec<u8>, String> {
    let mut buffer = Vec::new();
    let mut total_read = 0u64;

    loop {
        let remaining = max_size.saturating_sub(total_read);
        if remaining == 0 {
            return Err(format!("Stream exceeds maximum size of {} bytes", max_size));
        }

        let to_read = remaining.min(chunk_size);

        match stream.blocking_read(to_read) {
            Ok(chunk) => {
                if chunk.is_empty() {
                    break;
                }
                total_read += chunk.len() as u64;
                buffer.extend_from_slice(&chunk);
            }
            Err(StreamError::Closed) => {
                break;
            }
            Err(e) => {
                return Err(format!("Stream read error: {:?}", e));
            }
        }
    }

    Ok(buffer)
}

fn serialize_content_block(content: &ContentBlock) -> serde_json::Value {
    match content {
        ContentBlock::Text(text_content) => {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "type".to_string(),
                serde_json::Value::String("text".to_string()),
            );

            // Serialize text data
            let text_value = match &text_content.text {
                TextData::Text(s) => s.clone(),
                TextData::TextStream(stream) => {
                    // Read the stream with bounded memory
                    read_text_stream(stream)
                        .unwrap_or_else(|e| format!("[error reading text stream: {}]", e))
                }
            };
            obj.insert("text".to_string(), serde_json::Value::String(text_value));

            // Add optional fields
            if let Some(ref opts) = text_content.options {
                if let Some(ref annotations) = opts.annotations {
                    obj.insert(
                        "annotations".to_string(),
                        serialize_annotations(annotations),
                    );
                }
                if let Some(ref meta) = opts.meta {
                    if let Ok(meta_value) = serde_json::from_str::<serde_json::Value>(meta) {
                        obj.insert("_meta".to_string(), meta_value);
                    }
                }
            }

            serde_json::Value::Object(obj)
        }
        ContentBlock::Image(image_content) => {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "type".to_string(),
                serde_json::Value::String("image".to_string()),
            );

            // Serialize blob data
            let data_value = match &image_content.data {
                BlobData::Blob(bytes) => {
                    use base64::{Engine as _, engine::general_purpose};
                    general_purpose::STANDARD.encode(bytes)
                }
                BlobData::BlobStream(stream) => {
                    // Read the stream with bounded memory
                    read_blob_stream(stream)
                        .unwrap_or_else(|e| format!("[error reading blob stream: {}]", e))
                }
            };
            obj.insert("data".to_string(), serde_json::Value::String(data_value));
            obj.insert(
                "mimeType".to_string(),
                serde_json::Value::String(image_content.mime_type.clone()),
            );

            // Add optional fields
            if let Some(ref opts) = image_content.options {
                if let Some(ref annotations) = opts.annotations {
                    obj.insert(
                        "annotations".to_string(),
                        serialize_annotations(annotations),
                    );
                }
                if let Some(ref meta) = opts.meta {
                    if let Ok(meta_value) = serde_json::from_str::<serde_json::Value>(meta) {
                        obj.insert("_meta".to_string(), meta_value);
                    }
                }
            }

            serde_json::Value::Object(obj)
        }
        ContentBlock::Audio(audio_content) => {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "type".to_string(),
                serde_json::Value::String("audio".to_string()),
            );

            // Serialize blob data
            let data_value = match &audio_content.data {
                BlobData::Blob(bytes) => {
                    use base64::{Engine as _, engine::general_purpose};
                    general_purpose::STANDARD.encode(bytes)
                }
                BlobData::BlobStream(stream) => {
                    // Read the stream with bounded memory
                    read_blob_stream(stream)
                        .unwrap_or_else(|e| format!("[error reading blob stream: {}]", e))
                }
            };
            obj.insert("data".to_string(), serde_json::Value::String(data_value));
            obj.insert(
                "mimeType".to_string(),
                serde_json::Value::String(audio_content.mime_type.clone()),
            );

            // Add optional fields
            if let Some(ref opts) = audio_content.options {
                if let Some(ref annotations) = opts.annotations {
                    obj.insert(
                        "annotations".to_string(),
                        serialize_annotations(annotations),
                    );
                }
                if let Some(ref meta) = opts.meta {
                    if let Ok(meta_value) = serde_json::from_str::<serde_json::Value>(meta) {
                        obj.insert("_meta".to_string(), meta_value);
                    }
                }
            }

            serde_json::Value::Object(obj)
        }
        ContentBlock::ResourceLink(_) | ContentBlock::EmbeddedResource(_) => {
            // These are not part of SamplingMessage content according to the spec
            // SamplingMessage only supports Text, Image, and Audio
            serde_json::json!({
                "type": "text",
                "text": "[unsupported content type in sampling message]"
            })
        }
    }
}

fn serialize_annotations(annotations: &Annotations) -> serde_json::Value {
    let mut obj = serde_json::Map::new();

    if let Some(ref audience) = annotations.audience {
        let audience_array: Vec<serde_json::Value> = audience
            .iter()
            .map(|role| {
                serde_json::Value::String(
                    match role {
                        Role::User => "user",
                        Role::Assistant => "assistant",
                    }
                    .to_string(),
                )
            })
            .collect();
        obj.insert(
            "audience".to_string(),
            serde_json::Value::Array(audience_array),
        );
    }

    if let Some(ref last_modified) = annotations.last_modified {
        obj.insert(
            "lastModified".to_string(),
            serde_json::Value::String(last_modified.clone()),
        );
    }

    if let Some(priority) = annotations.priority {
        obj.insert("priority".to_string(), serde_json::json!(priority));
    }

    serde_json::Value::Object(obj)
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

bindings::export!(HttpMessages with_types_in bindings);
