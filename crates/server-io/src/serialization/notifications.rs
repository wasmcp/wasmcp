//! Server notification serialization
//!
//! Handles serialization of all MCP notification types to JSON-RPC format.

use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    LogLevel, NotificationOptions, ProgressToken, ServerNotification,
};
use crate::serialization::server_messages::serialize_request_id;

/// Serialize server notification to method name and params
pub fn serialize_server_notification(
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

/// Serialize notification options (meta and extras)
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

/// Convert LogLevel enum to string
pub fn log_level_to_string(level: &LogLevel) -> &'static str {
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
