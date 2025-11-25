//! Server request serialization
//!
//! Handles serialization of all MCP server request types to JSON-RPC format.

use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    ContentBlock, IncludeContext, ModelPreferences, NumberSchemaType, PrimitiveSchemaDefinition,
    ProgressToken, RequestedSchema, Role, SamplingMessage, ServerRequest, StringSchemaFormat,
};
use crate::serializer;

// Note: Schema record types (StringSchema, NumberSchema, etc.) are inline within
// PrimitiveSchemaDefinition variant and accessed via pattern matching

/// Serialize server request to method name and params
pub fn serialize_server_request(request: &ServerRequest) -> (&'static str, serde_json::Value) {
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

/// Serialize requested schema for elicitation
fn serialize_requested_schema(schema: &RequestedSchema) -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": schema.properties.iter().map(|(k, v)| {
            (k.clone(), serialize_primitive_schema(v))
        }).collect::<serde_json::Map<_, _>>(),
        "required": schema.required,
    })
}

/// Serialize primitive schema definition (string, number, boolean, enum)
pub fn serialize_primitive_schema(schema: &PrimitiveSchemaDefinition) -> serde_json::Value {
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

/// Serialize sampling message (role + content)
fn serialize_sampling_message(msg: &SamplingMessage) -> serde_json::Value {
    serde_json::json!({
        "role": match msg.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        },
        "content": serialize_content_block(&msg.content)
    })
}

/// Serialize content block using serializer module
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

/// Serialize model preferences
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

/// Serialize include context enum
fn serialize_include_context(ctx: &IncludeContext) -> &'static str {
    match ctx {
        IncludeContext::None => "none",
        IncludeContext::ThisServer => "thisServer",
        IncludeContext::AllServers => "allServers",
    }
}
