//! Server request serialization
//!
//! Handles serialization of all MCP server request types to JSON-RPC format.

use crate::bindings::wasmcp::mcp_v20251125::mcp::{
    BlobData, ElicitRequest, EnumSchema, IncludeContext, ModelPreferences, NumberSchemaType,
    PrimitiveSchemaDefinition, ProgressToken, RequestedSchema, Role, SamplingContentBlock,
    SamplingMessage, ServerRequest, StringSchemaFormat, TextData, ToolResultContentBlock,
};
use crate::serializer;

// Note: Schema record types (StringSchema, NumberSchema, etc.) are inline within
// PrimitiveSchemaDefinition variant and accessed via pattern matching

/// Serialize server request to method name and params
pub fn serialize_server_request(request: &ServerRequest) -> (&'static str, serde_json::Value) {
    match request {
        ServerRequest::ElicitationCreate(elicit_req) => {
            ("elicitation/create", serialize_elicit_request(elicit_req))
        }
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

/// Serialize elicit request (form or url variant)
fn serialize_elicit_request(req: &ElicitRequest) -> serde_json::Value {
    match req {
        ElicitRequest::Form(form) => serde_json::json!({
            "message": form.message,
            "requestedSchema": serialize_requested_schema(&form.requested_schema),
        }),
        ElicitRequest::Url(url_req) => serde_json::json!({
            "elicitationId": url_req.elicitation_id,
            "message": url_req.message,
            "url": url_req.url,
        }),
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
        PrimitiveSchemaDefinition::EnumSchema(e) => serialize_enum_schema(e),
    }
}

/// Serialize enum schema (now a variant with 4 cases)
fn serialize_enum_schema(e: &EnumSchema) -> serde_json::Value {
    match e {
        EnumSchema::UntitledSingleSelect(s) => {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "enum".to_string(),
                serde_json::Value::Array(
                    s.enum_
                        .iter()
                        .map(|v| serde_json::Value::String(v.clone()))
                        .collect(),
                ),
            );
            if let Some(ref d) = s.default {
                obj.insert("default".to_string(), serde_json::Value::String(d.clone()));
            }
            if let Some(ref d) = s.description {
                obj.insert(
                    "description".to_string(),
                    serde_json::Value::String(d.clone()),
                );
            }
            serde_json::Value::Object(obj)
        }
        EnumSchema::TitledSingleSelect(s) => {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "oneOf".to_string(),
                serde_json::Value::Array(
                    s.one_of
                        .iter()
                        .map(
                            |opt| serde_json::json!({"const": opt.const_value, "title": opt.title}),
                        )
                        .collect(),
                ),
            );
            if let Some(ref d) = s.default {
                obj.insert("default".to_string(), serde_json::Value::String(d.clone()));
            }
            if let Some(ref d) = s.description {
                obj.insert(
                    "description".to_string(),
                    serde_json::Value::String(d.clone()),
                );
            }
            if let Some(ref t) = s.title {
                obj.insert("title".to_string(), serde_json::Value::String(t.clone()));
            }
            serde_json::Value::Object(obj)
        }
        EnumSchema::UntitledMultiSelect(s) => {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "type".to_string(),
                serde_json::Value::String("array".to_string()),
            );
            obj.insert(
                "items".to_string(),
                serde_json::json!({"enum": s.enum_.iter().map(|v| serde_json::Value::String(v.clone())).collect::<Vec<_>>()}),
            );
            if let Some(ref d) = s.description {
                obj.insert(
                    "description".to_string(),
                    serde_json::Value::String(d.clone()),
                );
            }
            serde_json::Value::Object(obj)
        }
        EnumSchema::TitledMultiSelect(s) => {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "type".to_string(),
                serde_json::Value::String("array".to_string()),
            );
            obj.insert(
                "items".to_string(),
                serde_json::json!({"oneOf": s.one_of.iter().map(|opt| serde_json::json!({"const": opt.const_value, "title": opt.title})).collect::<Vec<_>>()}),
            );
            if let Some(ref d) = s.description {
                obj.insert(
                    "description".to_string(),
                    serde_json::Value::String(d.clone()),
                );
            }
            if let Some(ref t) = s.title {
                obj.insert("title".to_string(), serde_json::Value::String(t.clone()));
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
        "content": serialize_sampling_content_block(&msg.content)
    })
}

/// Serialize sampling content block (text, image, audio, tool-use, tool-result)
fn serialize_sampling_content_block(content: &SamplingContentBlock) -> serde_json::Value {
    match content {
        SamplingContentBlock::Text(t) => {
            let text_str = match &t.text {
                TextData::Text(s) => s.clone(),
                TextData::TextStream(_) => "[stream]".to_string(),
            };
            serde_json::json!({"type": "text", "text": text_str})
        }
        SamplingContentBlock::Image(img) => {
            let data_b64 = blob_data_to_base64(&img.data);
            serde_json::json!({"type": "image", "data": data_b64, "mimeType": img.mime_type})
        }
        SamplingContentBlock::Audio(audio) => {
            let data_b64 = blob_data_to_base64(&audio.data);
            serde_json::json!({"type": "audio", "data": data_b64, "mimeType": audio.mime_type})
        }
        SamplingContentBlock::ToolUse(tu) => {
            let input_val = serde_json::from_str::<serde_json::Value>(&tu.input).ok();
            serde_json::json!({"type": "tool_use", "id": tu.id, "name": tu.name, "input": input_val})
        }
        SamplingContentBlock::ToolResult(tr) => {
            let content_arr: Vec<serde_json::Value> = tr.content.iter().map(|block| {
                match block {
                    ToolResultContentBlock::Text(t) => {
                        let s = match &t.text { TextData::Text(s) => s.clone(), TextData::TextStream(_) => "[stream]".to_string() };
                        serde_json::json!({"type": "text", "text": s})
                    }
                    ToolResultContentBlock::Image(img) => serde_json::json!({"type": "image", "data": blob_data_to_base64(&img.data), "mimeType": img.mime_type}),
                    ToolResultContentBlock::Audio(audio) => serde_json::json!({"type": "audio", "data": blob_data_to_base64(&audio.data), "mimeType": audio.mime_type}),
                }
            }).collect();
            serde_json::json!({"type": "tool_result", "toolUseId": tr.tool_use_id, "content": content_arr})
        }
    }
}

/// Extract base64-encoded string from BlobData (inline only; streams return empty string)
fn blob_data_to_base64(data: &BlobData) -> String {
    use base64::Engine as _;
    match data {
        BlobData::Blob(bytes) => base64::engine::general_purpose::STANDARD.encode(bytes),
        BlobData::BlobStream(_) => String::new(),
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
