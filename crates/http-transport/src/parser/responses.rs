//! Client response parsing for MCP protocol
//!
//! This module handles parsing JSON-RPC responses into WIT types.

use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    ClientResult, ElicitResult, ElicitResultAction, ElicitResultContent, Error, ErrorCode,
    ListRootsResult, Role, Root, SamplingCreateMessageResult,
};
use crate::parser::content::parse_content_block;
use serde_json::Value;

/// Parse a JSON-RPC response (result or error) into Result<ClientResult, ErrorCode>
pub fn parse_client_response(
    json: &Value,
) -> Result<Result<ClientResult, ErrorCode>, String> {
    // Check if it's an error response
    if let Some(error_obj) = json.get("error") {
        let code = error_obj
            .get("code")
            .and_then(|c| c.as_i64())
            .ok_or("Missing or invalid 'code' in error")?;

        let message = error_obj
            .get("message")
            .and_then(|m| m.as_str())
            .ok_or("Missing 'message' in error")?
            .to_string();

        let data = error_obj
            .get("data")
            .and_then(|d| serde_json::to_string(d).ok());

        let error = Error {
            code,
            message,
            data,
        };

        // Map JSON-RPC error codes to ErrorCode variants
        let error_code = match code {
            -32700 => ErrorCode::ParseError(error),
            -32600 => ErrorCode::InvalidRequest(error),
            -32601 => ErrorCode::MethodNotFound(error),
            -32602 => ErrorCode::InvalidParams(error),
            -32603 => ErrorCode::InternalError(error),
            -32099..=-32000 => ErrorCode::Server(error),
            -32768..=-32100 => ErrorCode::JsonRpc(error),
            _ => ErrorCode::Mcp(error),
        };

        return Ok(Err(error_code));
    }

    // It's a success response - parse the result
    let result = json
        .get("result")
        .ok_or("Missing 'result' field in response")?;

    // Determine which client response type by checking structure
    // We need to infer the type from the result structure

    // Check for elicit-result (has action field)
    if result.get("action").is_some() {
        return parse_elicit_result(result).map(|r| Ok(ClientResult::ElicitationCreate(r)));
    }

    // Check for list-roots-result (has roots field)
    if result.get("roots").is_some() {
        return parse_list_roots_result(result).map(|r| Ok(ClientResult::RootsList(r)));
    }

    // Check for sampling-create-message-result (has content, model, role, stopReason)
    if result.get("model").is_some() && result.get("role").is_some() {
        return parse_sampling_create_message_result(result)
            .map(|r| Ok(ClientResult::SamplingCreateMessage(r)));
    }

    Err("Unable to determine client response type from result structure".to_string())
}

fn parse_elicit_result(result: &Value) -> Result<ElicitResult, String> {
    let meta = result
        .get("_meta")
        .and_then(|m| serde_json::to_string(m).ok());

    let action_str = result
        .get("action")
        .and_then(|a| a.as_str())
        .ok_or("Missing 'action' in elicit result")?;

    let action = match action_str {
        "accept" => ElicitResultAction::Accept,
        "decline" => ElicitResultAction::Decline,
        "cancel" => ElicitResultAction::Cancel,
        _ => return Err(format!("Invalid action: {}", action_str)),
    };

    // Parse content array if present
    let content = result
        .get("content")
        .and_then(|c| c.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(key, value)| {
                    // Try to determine the type and convert
                    if let Some(s) = value.as_str() {
                        Some((key.clone(), ElicitResultContent::String(s.to_string())))
                    } else if let Some(n) = value.as_f64() {
                        Some((key.clone(), ElicitResultContent::Number(n)))
                    } else {
                        value
                            .as_bool()
                            .map(|b| (key.clone(), ElicitResultContent::Boolean(b)))
                    }
                })
                .collect()
        });

    Ok(ElicitResult {
        meta,
        action,
        content,
    })
}

fn parse_list_roots_result(result: &Value) -> Result<ListRootsResult, String> {
    let meta = result
        .get("_meta")
        .and_then(|m| serde_json::to_string(m).ok());

    let roots_array = result
        .get("roots")
        .and_then(|r| r.as_array())
        .ok_or("Missing or invalid 'roots' array")?;

    let roots: Vec<Root> = roots_array
        .iter()
        .map(|root_obj| {
            let uri = root_obj
                .get("uri")
                .and_then(|u| u.as_str())
                .ok_or("Missing 'uri' in root")?
                .to_string();

            let name = root_obj
                .get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string());

            let meta = root_obj
                .get("_meta")
                .and_then(|m| serde_json::to_string(m).ok());

            Ok(Root { uri, name, meta })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(ListRootsResult { meta, roots })
}

fn parse_sampling_create_message_result(result: &Value) -> Result<SamplingCreateMessageResult, String> {
    let meta = result
        .get("_meta")
        .and_then(|m| serde_json::to_string(m).ok());

    // Parse content - it's a ContentBlock object
    let content_obj = result
        .get("content")
        .ok_or("Missing 'content' in sampling result")?;

    let content = parse_content_block(content_obj)?;

    let model = result
        .get("model")
        .and_then(|m| m.as_str())
        .ok_or("Missing 'model' in sampling result")?
        .to_string();

    let role_str = result
        .get("role")
        .and_then(|r| r.as_str())
        .ok_or("Missing 'role' in sampling result")?;

    let role = match role_str {
        "user" => Role::User,
        "assistant" => Role::Assistant,
        _ => return Err(format!("Invalid role: {}", role_str)),
    };

    let stop_reason = result
        .get("stopReason")
        .and_then(|sr| sr.as_str())
        .map(|s| s.to_string());

    let extra = result
        .get("extra")
        .and_then(|e| serde_json::to_string(e).ok());

    Ok(SamplingCreateMessageResult {
        meta,
        content,
        model,
        role,
        stop_reason,
        extra,
    })
}