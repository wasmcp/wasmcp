//! Notifications writer implementation for HTTP/SSE transport.
//!
//! Handles all notification types including logging, progress updates,
//! and change notifications for tools, resources, and prompts.

use crate::bindings::exports::wasmcp::mcp::notifications_writer::Guest;
use crate::bindings::wasi::io::streams::{OutputStream, StreamError};
use crate::bindings::wasmcp::mcp::protocol::{
    ProgressToken, LogLevel, LogMessage, ClientNotification,
    ChangeNotificationType, UpdateNotificationType
};
use crate::utils::{build_jsonrpc_notification, escape_json_string, write_message, JsonObjectBuilder};

pub struct NotificationsWriter;

impl Guest for NotificationsWriter {
    fn log(out: &OutputStream, message: LogMessage) -> Result<(), StreamError> {
        // Convert log level to string
        let level_str = match message.level {
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Notice => "notice",
            LogLevel::Warning => "warning",
            LogLevel::Error => "error",
            LogLevel::Critical => "critical",
            LogLevel::Alert => "alert",
            LogLevel::Emergency => "emergency",
        };

        // Convert data bytes to string, handling invalid UTF-8
        let data_str = match String::from_utf8(message.data) {
            Ok(s) => s,
            Err(_) => {
                // Log data must be valid UTF-8 per spec
                return Err(StreamError::Closed);
            }
        };

        // Build the params object
        let mut params = JsonObjectBuilder::new();
        params.add_field("\"level\"", &format!("\"{level_str}\""));
        params.add_string("data", &data_str);
        params.add_optional_string("logger", message.logger.as_deref());

        let notification = build_jsonrpc_notification("notifications/log", Some(&params.build()));
        write_message(out, &notification)
    }

    fn send(out: &OutputStream, notification: ClientNotification) -> Result<(), StreamError> {
        let json = build_jsonrpc_notification(&notification.method, notification.params.as_deref());
        write_message(out, &json)
    }

    fn send_list_changed(out: &OutputStream, change: ChangeNotificationType) -> Result<(), StreamError> {
        let method = match change {
            ChangeNotificationType::Tools => "notifications/tools/list_changed",
            ChangeNotificationType::Resources => "notifications/resources/list_changed",
            ChangeNotificationType::Prompts => "notifications/prompts/list_changed",
        };

        let notification = build_jsonrpc_notification(method, None);
        write_message(out, &notification)
    }

    fn send_updated(out: &OutputStream, update: UpdateNotificationType) -> Result<(), StreamError> {
        let (method, params) = match update {
            UpdateNotificationType::Resource(uri) => {
                let params = format!("{{\"uri\":\"{}\"}}", escape_json_string(&uri));
                ("notifications/resources/updated", params)
            }
        };

        let notification = build_jsonrpc_notification(method, Some(&params));
        write_message(out, &notification)
    }

    fn send_progress(
        out: &OutputStream,
        progress_token: ProgressToken,
        progress: f64,
        total: Option<f64>,
        message: Option<String>,
    ) -> Result<(), StreamError> {
        // Format the progress token
        let token_str = match progress_token {
            ProgressToken::String(s) => format!("\"{}\"", escape_json_string(&s)),
            ProgressToken::Integer(n) => n.to_string(),
        };

        // Build params object
        let mut params = JsonObjectBuilder::new();
        params.add_field("progressToken", &token_str);
        params.add_number("progress", progress);
        params.add_optional_number("total", total);
        params.add_optional_string("message", message.as_deref());

        let notification = build_jsonrpc_notification("notifications/progress", Some(&params.build()));
        write_message(out, &notification)
    }
}