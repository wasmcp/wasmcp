//! Tools handler for MCP.

#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        world: "tools-handler",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp::message_handler::{Guest, OutputStream};

use bindings::wasmcp::mcp::error_writer;
use bindings::wasmcp::mcp::message_context::MessageContext;
use bindings::wasmcp::mcp::message_handler as next_handler;
use bindings::wasmcp::mcp::protocol::{
    ErrorCode, Id, McpError, McpMessage, RequestMethod, ServerCapability, Tool, ToolsCallParams,
    ToolsCapabilityOptions,
};
use bindings::wasmcp::mcp::tools_writer;

use serde::Deserialize;
use serde_json::json;

pub struct ToolsHandler;

impl ToolsHandler {
    fn handle_tools_list(out: OutputStream, id: &Id) {
        let tools = vec![Tool {
            name: "echo".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "The message to echo"
                    }
                },
                "required": ["message"]
            })
            .to_string(),
            options: None,
        }];

        if let Err(e) = tools_writer::write_tools(out, id, &tools) {
            eprintln!("Failed to write tools list: {e:?}");
        }
    }

    fn handle_tools_call(out: OutputStream, id: &Id, params: &ToolsCallParams) {
        let result = match params.name.as_str() {
            "echo" => Self::handle_echo(params.arguments.as_deref()),
            _ => {
                let _ = error_writer::write_error(
                    out,
                    &McpError {
                        code: ErrorCode::InvalidParams,
                        message: format!("Unknown tool: {}", params.name),
                        data: None,
                        id: Some(id.clone()),
                    },
                );
                return;
            }
        };

        match result {
            Ok(response) => {
                if let Err(e) = tools_writer::write_text(out, id, &response) {
                    eprintln!("Failed to write response: {e:?}");
                }
            }
            Err(e) => {
                let _ = error_writer::write_error(
                    out,
                    &McpError {
                        code: ErrorCode::InternalError,
                        message: format!("Tool execution error: {e}"),
                        data: None,
                        id: Some(id.clone()),
                    },
                );
            }
        }
    }

    fn handle_echo(arguments: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
        #[derive(Deserialize)]
        struct EchoArgs {
            message: String,
        }

        let args: EchoArgs = match arguments {
            Some(json_str) => serde_json::from_str(json_str)?,
            None => return Err("Missing arguments".into()),
        };

        Ok(format!("Echo: {}", args.message))
    }
}

impl Guest for ToolsHandler {
    fn handle(ctx: MessageContext, out: OutputStream) {
        ctx.register_capability(&ServerCapability::Tools(ToolsCapabilityOptions::empty()));

        let McpMessage::Request(request) = ctx.message() else {
            return next_handler::handle(ctx, out);
        };

        match request.method {
            RequestMethod::ToolsList(_) => Self::handle_tools_list(out, &request.id),
            RequestMethod::ToolsCall(params) => Self::handle_tools_call(out, &request.id, &params),
            _ => next_handler::handle(ctx, out),
        }
    }
}

bindings::export!(ToolsHandler with_types_in bindings);
