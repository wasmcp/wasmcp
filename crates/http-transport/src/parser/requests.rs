//! Request parsing for MCP protocol
//!
//! This module handles parsing JSON-RPC requests into WIT ClientRequest types.

use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    CallToolRequest, ClientRequest, CompleteRequest, CompletionArgument, CompletionContext,
    CompletionPromptReference, CompletionReference, GetPromptRequest, InitializeRequest,
    ListPromptsRequest, ListResourceTemplatesRequest, ListResourcesRequest, ListToolsRequest,
    LogLevel, PingRequest, ProgressToken, ReadResourceRequest,
};
use crate::parser::types::{
    convert_client_capabilities, convert_implementation, parse_protocol_version,
    JsonInitializeRequestParams,
};
use serde_json::Value;

/// Parse a JSON-RPC request into a ClientRequest
pub fn parse_client_request(json: &Value) -> Result<ClientRequest, String> {
    let method = json
        .get("method")
        .and_then(|m| m.as_str())
        .ok_or("Missing method field")?;

    let params = json.get("params");

    match method {
        "initialize" => parse_initialize_request(params),
        "tools/list" => parse_list_tools_request(params),
        "tools/call" => parse_call_tool_request(params),
        "resources/list" => parse_list_resources_request(params),
        "resources/read" => parse_read_resource_request(params),
        "resources/templates/list" => parse_list_resource_templates_request(params),
        "prompts/list" => parse_list_prompts_request(params),
        "prompts/get" => parse_get_prompt_request(params),
        "completion/complete" => parse_complete_request(params),
        "logging/setLevel" => parse_set_log_level_request(params),
        "ping" => parse_ping_request(params),
        "resources/subscribe" => parse_resource_subscribe_request(params),
        "resources/unsubscribe" => parse_resource_unsubscribe_request(params),
        _ => Err(format!("Unsupported method: {}", method)),
    }
}

fn parse_initialize_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let params = params.ok_or("Missing params for initialize request")?;

    let json_params: JsonInitializeRequestParams = serde_json::from_value(params.clone())
        .map_err(|e| format!("Invalid initialize params: {}", e))?;

    let protocol_version = parse_protocol_version(&json_params.protocol_version)?;
    let capabilities = convert_client_capabilities(json_params.capabilities);
    let client_info = convert_implementation(json_params.client_info);

    Ok(ClientRequest::Initialize(InitializeRequest {
        protocol_version,
        capabilities,
        client_info,
    }))
}

fn parse_list_tools_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let cursor = params
        .and_then(|p| p.get("cursor"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string());

    Ok(ClientRequest::ToolsList(ListToolsRequest { cursor }))
}

fn parse_call_tool_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let params = params.ok_or("Missing params for tools/call request")?;

    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or("Missing 'name' field in tools/call params")?
        .to_string();

    let arguments = params
        .get("arguments")
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| format!("Failed to serialize arguments: {}", e))?;

    Ok(ClientRequest::ToolsCall(CallToolRequest {
        name,
        arguments,
    }))
}

fn parse_list_resources_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let cursor = params
        .and_then(|p| p.get("cursor"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string());

    Ok(ClientRequest::ResourcesList(ListResourcesRequest {
        cursor,
    }))
}

fn parse_read_resource_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let params = params.ok_or("Missing params for resources/read request")?;

    let uri = params
        .get("uri")
        .and_then(|u| u.as_str())
        .ok_or("Missing 'uri' field in resources/read params")?
        .to_string();

    Ok(ClientRequest::ResourcesRead(ReadResourceRequest { uri }))
}

fn parse_list_resource_templates_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let cursor = params
        .and_then(|p| p.get("cursor"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string());

    Ok(ClientRequest::ResourcesTemplatesList(
        ListResourceTemplatesRequest { cursor },
    ))
}

fn parse_list_prompts_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let cursor = params
        .and_then(|p| p.get("cursor"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string());

    Ok(ClientRequest::PromptsList(ListPromptsRequest { cursor }))
}

fn parse_get_prompt_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let params = params.ok_or("Missing params for prompts/get request")?;

    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or("Missing 'name' field in prompts/get params")?
        .to_string();

    let arguments = params
        .get("arguments")
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| format!("Failed to serialize prompt arguments: {}", e))?;

    Ok(ClientRequest::PromptsGet(GetPromptRequest {
        name,
        arguments,
    }))
}

fn parse_complete_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let params = params.ok_or("Missing params for completion/complete request")?;

    let ref_obj = params
        .get("ref")
        .ok_or("Missing 'ref' field in completion/complete params")?;

    let completion_ref = if let Some(prompt_name) = ref_obj.get("name").and_then(|n| n.as_str()) {
        // It's a prompt reference
        CompletionReference::Prompt(CompletionPromptReference {
            name: prompt_name.to_string(),
            title: ref_obj
                .get("title")
                .and_then(|t| t.as_str())
                .map(|s| s.to_string()),
        })
    } else if let Some(uri) = ref_obj.as_str() {
        // It's a resource template URI
        CompletionReference::ResourceTemplate(uri.to_string())
    } else {
        return Err("Invalid 'ref' field: must be prompt object or URI string".to_string());
    };

    let argument_obj = params
        .get("argument")
        .ok_or("Missing 'argument' field in completion/complete params")?;

    let argument = CompletionArgument {
        name: argument_obj
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or("Missing 'name' in argument")?
            .to_string(),
        value: argument_obj
            .get("value")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'value' in argument")?
            .to_string(),
    };

    let context = params
        .get("context")
        .map(|ctx| {
            let arguments = ctx
                .get("arguments")
                .map(serde_json::to_string)
                .transpose()
                .map_err(|e| format!("Failed to serialize context arguments: {}", e))?;
            Ok::<CompletionContext, String>(CompletionContext { arguments })
        })
        .transpose()?;

    Ok(ClientRequest::CompletionComplete(CompleteRequest {
        argument,
        ref_: completion_ref,
        context,
    }))
}

fn parse_set_log_level_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let params = params.ok_or("Missing params for logging/setLevel request")?;

    let level_str = params
        .get("level")
        .and_then(|l| l.as_str())
        .ok_or("Missing 'level' field in logging/setLevel params")?;

    let level = match level_str {
        "debug" => LogLevel::Debug,
        "info" => LogLevel::Info,
        "notice" => LogLevel::Notice,
        "warning" => LogLevel::Warning,
        "error" => LogLevel::Error,
        "critical" => LogLevel::Critical,
        "alert" => LogLevel::Alert,
        "emergency" => LogLevel::Emergency,
        _ => return Err(format!("Invalid log level: {}", level_str)),
    };

    Ok(ClientRequest::LoggingSetLevel(level))
}

fn parse_ping_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let ping_request = if let Some(p) = params {
        let progress_token = p.get("progressToken").and_then(|pt| {
            if let Some(s) = pt.as_str() {
                Some(ProgressToken::String(s.to_string()))
            } else {
                pt.as_i64().map(ProgressToken::Integer)
            }
        });

        let meta = p.get("_meta").and_then(|m| serde_json::to_string(m).ok());

        let extras = p
            .get("extras")
            .and_then(|e| e.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| {
                        serde_json::to_string(v)
                            .ok()
                            .map(|json_str| (k.clone(), json_str))
                    })
                    .collect()
            })
            .unwrap_or_else(Vec::new);

        PingRequest {
            meta,
            progress_token,
            extras,
        }
    } else {
        PingRequest {
            meta: None,
            progress_token: None,
            extras: vec![],
        }
    };

    Ok(ClientRequest::Ping(ping_request))
}

fn parse_resource_subscribe_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let params = params.ok_or("Missing params for resources/subscribe request")?;

    let uri = params
        .get("uri")
        .and_then(|u| u.as_str())
        .ok_or("Missing 'uri' field in resources/subscribe params")?
        .to_string();

    Ok(ClientRequest::ResourcesSubscribe(uri))
}

fn parse_resource_unsubscribe_request(params: Option<&Value>) -> Result<ClientRequest, String> {
    let params = params.ok_or("Missing params for resources/unsubscribe request")?;

    let uri = params
        .get("uri")
        .and_then(|u| u.as_str())
        .ok_or("Missing 'uri' field in resources/unsubscribe params")?
        .to_string();

    Ok(ClientRequest::ResourcesUnsubscribe(uri))
}
