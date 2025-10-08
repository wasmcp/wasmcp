//! Initialize writer implementation for HTTP/SSE transport.
//!
//! Handles the initialization response which includes server info,
//! capabilities, and protocol version.

use crate::bindings::exports::wasmcp::mcp::initialize_writer::{Guest, InitializeResult};
use crate::bindings::wasi::io::streams::{OutputStream, StreamError};
use crate::bindings::wasmcp::mcp::protocol::Id;
use crate::utils::{
    build_jsonrpc_response, format_meta_field, write_message,
    JsonObjectBuilder,
};

pub struct InitializeWriter;

impl Guest for InitializeWriter {
    fn send(
        id: Id,
        out: OutputStream,
        result: InitializeResult,
    ) -> Result<(), StreamError> {
        let mut result_obj = JsonObjectBuilder::new();

        // Server info object
        let mut server_info = JsonObjectBuilder::new();
        server_info.add_string("name", &result.server_info.name);
        server_info.add_string("version", &result.server_info.version);
        result_obj.add_field("serverInfo", &server_info.build());

        // Protocol version
        let protocol_version_str = match result.protocol_version {
            crate::bindings::wasmcp::mcp::protocol::ProtocolVersion::V20250618 => "2025-06-18",
            crate::bindings::wasmcp::mcp::protocol::ProtocolVersion::V20250326 => "2025-03-26",
            crate::bindings::wasmcp::mcp::protocol::ProtocolVersion::V20241105 => "2024-11-05",
        };
        result_obj.add_string("protocolVersion", protocol_version_str);

        // Build capabilities object
        let capabilities_json = build_capabilities(&result.capabilities);
        result_obj.add_field("capabilities", &capabilities_json);

        // Add optional fields
        if let Some(options) = result.options {
            if let Some(instructions) = options.instructions {
                result_obj.add_string("instructions", &instructions);
            }

            if let Some(meta_field) = format_meta_field(&options.meta) {
                result_obj.add_field("_meta", &meta_field);
            }
        }

        let response = build_jsonrpc_response(&id, &result_obj.build());
        write_message(&out, &response)
    }
}

/// Build the capabilities object for the initialization result.
fn build_capabilities(capabilities: &crate::bindings::wasmcp::mcp::protocol::ServerCapabilities) -> String {
    let mut caps = JsonObjectBuilder::new();

    // Tools capability
    if let Some(tools) = &capabilities.tools {
        let mut tools_obj = JsonObjectBuilder::new();
        if let Some(list_changed) = tools.list_changed {
            tools_obj.add_bool("listChanged", list_changed);
        }
        caps.add_field("tools", &tools_obj.build());
    }

    // Resources capability
    if let Some(resources) = &capabilities.resources {
        let mut res_obj = JsonObjectBuilder::new();
        if let Some(list_changed) = resources.list_changed {
            res_obj.add_bool("listChanged", list_changed);
        }
        if let Some(subscribe) = resources.subscribe {
            res_obj.add_bool("subscribe", subscribe);
        }
        caps.add_field("resources", &res_obj.build());
    }

    // Prompts capability
    if let Some(prompts) = &capabilities.prompts {
        let mut prompts_obj = JsonObjectBuilder::new();
        if let Some(list_changed) = prompts.list_changed {
            prompts_obj.add_bool("listChanged", list_changed);
        }
        caps.add_field("prompts", &prompts_obj.build());
    }

    // Logging capability - already a JSON string
    if let Some(logging) = &capabilities.logging {
        caps.add_field("logging", logging);
    }

    // Completions capability - already a JSON string
    if let Some(completions) = &capabilities.completions {
        caps.add_field("completions", completions);
    }

    // Experimental capabilities - list of key-value JSON pairs
    if let Some(experimental) = &capabilities.experimental {
        if !experimental.is_empty() {
            let mut exp_obj = JsonObjectBuilder::new();
            for (key, value) in experimental {
                exp_obj.add_field(key, value);
            }
            caps.add_field("experimental", &exp_obj.build());
        }
    }

    caps.build()
}