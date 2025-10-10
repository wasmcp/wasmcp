//! Notification response writers
//!
//! Implements serialization for MCP server-to-client notifications:
//! - Log messages
//! - Custom notifications
//! - List changed notifications
//! - Resource update notifications
//! - Progress notifications

use crate::bindings::wasmcp::mcp::output::{
    finish_message, start_message, write_message_contents, IoError,
};
use crate::bindings::wasmcp::mcp::protocol::{
    ChangeNotificationType, ClientNotification, LogLevel, LogMessage, ProgressToken,
    UpdateNotificationType,
};
use crate::utils::escape_json_string;

/// Write a log message notification.
pub fn log(message: LogMessage) -> Result<(), IoError> {
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

    let mut params = String::from("{");
    params.push_str(&format!(r#""level":"{}""#, level_str));

    if let Some(logger) = &message.logger {
        params.push_str(&format!(r#","logger":"{}""#, escape_json_string(logger)));
    }

    // Encode data as base64
    let data_b64 =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &message.data);
    params.push_str(&format!(r#","data":"{}""#, data_b64));
    params.push('}');

    let notification = format!(
        r#"{{"jsonrpc":"2.0","method":"notifications/message","params":{}}}"#,
        params
    );

    start_message()?;
    write_message_contents(&notification.into_bytes())?;
    finish_message()
}

/// Send a custom client notification.
pub fn notify(notification: ClientNotification) -> Result<(), IoError> {
    let mut msg = format!(
        r#"{{"jsonrpc":"2.0","method":"{}""#,
        escape_json_string(&notification.method)
    );

    if let Some(params) = &notification.params {
        msg.push_str(&format!(r#","params":{}"#, params));
    }

    msg.push('}');

    start_message()?;
    write_message_contents(&msg.into_bytes())?;
    finish_message()
}

/// Send a list_changed notification.
pub fn notify_list_changed(change: ChangeNotificationType) -> Result<(), IoError> {
    let method = match change {
        ChangeNotificationType::Tools => "notifications/tools/list_changed",
        ChangeNotificationType::Resources => "notifications/resources/list_changed",
        ChangeNotificationType::Prompts => "notifications/prompts/list_changed",
    };

    let notification = format!(r#"{{"jsonrpc":"2.0","method":"{}"}}"#, method);

    start_message()?;
    write_message_contents(&notification.into_bytes())?;
    finish_message()
}

/// Send a resource updated notification.
pub fn notify_updated(update: UpdateNotificationType) -> Result<(), IoError> {
    let UpdateNotificationType::Resource(uri) = update;

    let notification = format!(
        r#"{{"jsonrpc":"2.0","method":"notifications/resources/updated","params":{{"uri":"{}"}}}}"#,
        escape_json_string(&uri)
    );

    start_message()?;
    write_message_contents(&notification.into_bytes())?;
    finish_message()
}

/// Send a progress notification.
pub fn notify_progress(
    progress_token: ProgressToken,
    progress: f64,
    total: Option<f64>,
    message: Option<String>,
) -> Result<(), IoError> {
    let token_json = match progress_token {
        ProgressToken::String(s) => format!(r#""{}""#, escape_json_string(&s)),
        ProgressToken::Integer(n) => n.to_string(),
    };

    let mut params = format!(
        r#"{{"progressToken":{},"progress":{}"#,
        token_json, progress
    );

    if let Some(t) = total {
        params.push_str(&format!(r#","total":{}"#, t));
    }

    if let Some(msg) = &message {
        params.push_str(&format!(r#","message":"{}""#, escape_json_string(msg)));
    }

    params.push('}');

    let notification = format!(
        r#"{{"jsonrpc":"2.0","method":"notifications/progress","params":{}}}"#,
        params
    );

    start_message()?;
    write_message_contents(&notification.into_bytes())?;
    finish_message()
}
