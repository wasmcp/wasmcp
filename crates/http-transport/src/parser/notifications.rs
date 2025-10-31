//! Notification parsing for MCP protocol
//!
//! This module handles parsing JSON-RPC notifications into WIT types.

use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    CancelledNotification, ClientNotification, NotificationOptions, ProgressNotification,
    ProgressToken,
};
use crate::parser::types::parse_request_id;
use serde_json::Value;

/// Parse a JSON-RPC notification into a ClientNotification
pub fn parse_client_notification(json: &Value) -> Result<ClientNotification, String> {
    let method = json
        .get("method")
        .and_then(|m| m.as_str())
        .ok_or("Missing method field in notification")?;

    let params = json.get("params");

    match method {
        "notifications/initialized" => {
            let opts = parse_notification_options(params)?;
            Ok(ClientNotification::Initialized(opts))
        }
        "notifications/roots/list_changed" => {
            let opts = parse_notification_options(params)?;
            Ok(ClientNotification::RootsListChanged(opts))
        }
        "notifications/cancelled" => {
            let params = params.ok_or("Missing params for cancelled notification")?;

            let request_id = params
                .get("requestId")
                .ok_or("Missing 'requestId' in cancelled notification")?;
            let request_id = parse_request_id(request_id)?;

            let reason = params
                .get("reason")
                .and_then(|r| r.as_str())
                .map(|s| s.to_string());

            Ok(ClientNotification::Cancelled(CancelledNotification {
                request_id,
                reason,
            }))
        }
        "notifications/progress" => {
            let params = params.ok_or("Missing params for progress notification")?;

            let progress_token = params
                .get("progressToken")
                .ok_or("Missing 'progressToken' in progress notification")?;

            let progress_token = if let Some(s) = progress_token.as_str() {
                ProgressToken::String(s.to_string())
            } else if let Some(i) = progress_token.as_i64() {
                ProgressToken::Integer(i)
            } else {
                return Err("Invalid progressToken: must be string or integer".to_string());
            };

            let progress = params
                .get("progress")
                .and_then(|p| p.as_f64())
                .ok_or("Missing or invalid 'progress' in progress notification")?;

            let total = params.get("total").and_then(|t| t.as_f64());

            let message = params
                .get("message")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string());

            Ok(ClientNotification::Progress(ProgressNotification {
                progress_token,
                progress,
                total,
                message,
            }))
        }
        _ => Err(format!("Unsupported notification method: {}", method)),
    }
}

/// Parse notification options (_meta and extras)
pub(crate) fn parse_notification_options(params: Option<&Value>) -> Result<NotificationOptions, String> {
    if let Some(p) = params {
        let meta = p.get("_meta").and_then(|m| serde_json::to_string(m).ok());

        let extras = p.get("extras").and_then(|e| serde_json::to_string(e).ok());

        Ok(NotificationOptions { meta, extras })
    } else {
        Ok(NotificationOptions {
            meta: None,
            extras: None,
        })
    }
}