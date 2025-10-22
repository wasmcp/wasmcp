//! HTTP/SSE Client Notifications Implementation
//!
//! Implements client-notifications functions for Server-Sent Events (SSE) transport.
//! Writes JSON-RPC notifications as SSE events to the provided output stream.

#![allow(warnings)]

mod bindings {
    wit_bindgen::generate!({
        world: "http-notifications",
        generate_all,
    });
}

use bindings::exports::wasmcp::server::notifications::Guest;
use bindings::exports::wasmcp::server::notifications::NotificationError;
use bindings::wasi::io::streams::{OutputStream, StreamError};
use bindings::wasmcp::protocol::mcp::*;

struct HttpClientNotifications;

impl Guest for HttpClientNotifications {
    fn log(
        output: &OutputStream,
        message: String,
        level: LogLevel,
        logger: Option<String>,
    ) -> Result<(), NotificationError> {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/message",
            "params": {
                "level": log_level_to_string(&level),
                "logger": logger,
                "data": message,
            }
        });

        write_sse_event(output, &notification)
    }

    fn list_changed(output: &OutputStream, changes: ServerLists) -> Result<(), NotificationError> {
        // Send separate notification for each changed list type
        if changes.contains(ServerLists::TOOLS) {
            let notification = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/tools/list_changed"
            });
            write_sse_event(output, &notification)?;
        }

        if changes.contains(ServerLists::RESOURCES) {
            let notification = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/resources/list_changed"
            });
            write_sse_event(output, &notification)?;
        }

        if changes.contains(ServerLists::PROMPTS) {
            let notification = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/prompts/list_changed"
            });
            write_sse_event(output, &notification)?;
        }

        Ok(())
    }

    fn updated(
        output: &OutputStream,
        updates: ServerSubscriptions,
    ) -> Result<(), NotificationError> {
        if updates.contains(ServerSubscriptions::RESOURCES) {
            let notification = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/resources/updated"
            });
            write_sse_event(output, &notification)?;
        }

        Ok(())
    }

    fn request(output: &OutputStream, request: ServerRequest) -> Result<(), NotificationError> {
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

    fn progress(
        output: &OutputStream,
        token: ProgressToken,
        progress: f64,
        total: Option<f64>,
        message: Option<String>,
    ) -> Result<(), NotificationError> {
        let progress_token_value = match token {
            ProgressToken::String(s) => serde_json::Value::String(s),
            ProgressToken::Integer(i) => serde_json::Value::Number(serde_json::Number::from(i)),
        };

        let mut params = serde_json::Map::new();
        params.insert("progressToken".to_string(), progress_token_value);
        params.insert("progress".to_string(), serde_json::json!(progress));

        if let Some(t) = total {
            params.insert("total".to_string(), serde_json::json!(t));
        }

        if let Some(m) = message {
            params.insert("message".to_string(), serde_json::Value::String(m));
        }

        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/progress",
            "params": params
        });

        write_sse_event(output, &notification)
    }
}

// Helper functions

fn write_sse_event(
    stream: &OutputStream,
    data: &serde_json::Value,
) -> Result<(), NotificationError> {
    // Format as SSE event: "data: {json}\n\n"
    let json_str = serde_json::to_string(data).map_err(|e| {
        NotificationError::Serialization(format!("JSON serialization failed: {}", e))
    })?;
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
                stream
                    .write(chunk)
                    .map_err(|e| NotificationError::Io(e))?;
                offset += chunk_size;
            }
            Err(e) => {
                return Err(NotificationError::Io(e));
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
        "content": match msg.content {
            ContentBlock::Text(_) => "text",
            ContentBlock::Image(_) => "image",
            ContentBlock::Audio(_) => "audio",
            ContentBlock::ResourceLink(_) => "resource",
            ContentBlock::EmbeddedResource(_) => "embedded-resource",
        }
    })
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

bindings::export!(HttpClientNotifications with_types_in bindings);
