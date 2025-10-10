//! Lifecycle response writers
//!
//! Implements response serialization for lifecycle-related MCP methods:
//! - `initialize` - Server initialization with capabilities
//! - `ping` - Health check response

use crate::bindings::wasmcp::mcp::output::{
    finish_message, start_message, write_message_contents, IoError,
};
use crate::bindings::wasmcp::mcp::protocol::{
    Id, InitializeResult, ProtocolVersion, ServerCapabilities,
};
use crate::utils::{escape_json_string, JsonObjectBuilder};

/// Write an initialization response.
///
/// Serializes the server info, capabilities, and protocol version to JSON-RPC 2.0 format.
pub fn write_initialization(id: Id, result: InitializeResult) -> Result<(), IoError> {
    let mut result_obj = JsonObjectBuilder::new();

    // Server info
    let mut server_info = JsonObjectBuilder::new();
    server_info.add_string("name", &result.server_info.name);
    server_info.add_string("version", &result.server_info.version);
    if let Some(title) = &result.server_info.title {
        server_info.add_string("title", title);
    }
    result_obj.add_raw_json("serverInfo", &server_info.build());

    // Protocol version
    let protocol_version_str = match result.protocol_version {
        ProtocolVersion::V20250618 => "2025-06-18",
        ProtocolVersion::V20250326 => "2025-03-26",
        ProtocolVersion::V20241105 => "2024-11-05",
    };
    result_obj.add_string("protocolVersion", protocol_version_str);

    // Capabilities
    let capabilities_json = build_capabilities_json(&result.capabilities);
    result_obj.add_raw_json("capabilities", &capabilities_json);

    // Optional fields
    if let Some(options) = &result.options {
        if let Some(instructions) = &options.instructions {
            result_obj.add_string("instructions", instructions);
        }
        if let Some(meta) = &options.meta {
            if !meta.is_empty() {
                let meta_obj = build_meta_json(meta);
                result_obj.add_raw_json("_meta", &meta_obj);
            }
        }
    }

    let response = build_json_rpc_response(&id, &result_obj.build());

    start_message()?;
    write_message_contents(&response.into_bytes())?;
    finish_message()
}

/// Write a pong response (empty result).
///
/// Responds to a ping request with an empty successful result.
pub fn write_pong(id: Id) -> Result<(), IoError> {
    let response = build_json_rpc_response(&id, "{}");

    start_message()?;
    write_message_contents(&response.into_bytes())?;
    finish_message()
}

/// Build capabilities JSON object.
fn build_capabilities_json(caps: &ServerCapabilities) -> String {
    let mut obj = JsonObjectBuilder::new();

    // Tools capability
    if let Some(tools) = &caps.tools {
        let mut tools_obj = JsonObjectBuilder::new();
        if let Some(list_changed) = tools.list_changed {
            tools_obj.add_bool("listChanged", list_changed);
        }
        let tools_json = tools_obj.build();
        if tools_json != "{}" {
            obj.add_raw_json("tools", &tools_json);
        }
    }

    // Prompts capability
    if let Some(prompts) = &caps.prompts {
        let mut prompts_obj = JsonObjectBuilder::new();
        if let Some(list_changed) = prompts.list_changed {
            prompts_obj.add_bool("listChanged", list_changed);
        }
        let prompts_json = prompts_obj.build();
        if prompts_json != "{}" {
            obj.add_raw_json("prompts", &prompts_json);
        }
    }

    // Resources capability
    if let Some(resources) = &caps.resources {
        let mut resources_obj = JsonObjectBuilder::new();
        if let Some(list_changed) = resources.list_changed {
            resources_obj.add_bool("listChanged", list_changed);
        }
        if let Some(subscribe) = resources.subscribe {
            resources_obj.add_bool("subscribe", subscribe);
        }
        let resources_json = resources_obj.build();
        if resources_json != "{}" {
            obj.add_raw_json("resources", &resources_json);
        }
    }

    // Completions capability (raw JSON)
    if let Some(completions) = &caps.completions {
        obj.add_raw_json("completions", completions);
    }

    // Logging capability (raw JSON)
    if let Some(logging) = &caps.logging {
        obj.add_raw_json("logging", logging);
    }

    // Experimental capabilities
    if let Some(experimental) = &caps.experimental {
        if !experimental.is_empty() {
            let mut exp_obj = JsonObjectBuilder::new();
            for (key, value) in experimental {
                exp_obj.add_raw_json(key, value);
            }
            obj.add_raw_json("experimental", &exp_obj.build());
        }
    }

    obj.build()
}

/// Build meta JSON object from key-value pairs.
fn build_meta_json(meta: &[(String, String)]) -> String {
    let mut obj = JsonObjectBuilder::new();
    for (key, value) in meta {
        obj.add_string(key, value);
    }
    obj.build()
}

/// Build a JSON-RPC 2.0 response.
fn build_json_rpc_response(id: &Id, result: &str) -> String {
    let id_str = format_id(id);
    format!(r#"{{"jsonrpc":"2.0","id":{},"result":{}}}"#, id_str, result)
}

/// Format an ID value as JSON.
fn format_id(id: &Id) -> String {
    match id {
        Id::Number(n) => n.to_string(),
        Id::String(s) => format!(r#""{}""#, escape_json_string(s)),
    }
}
