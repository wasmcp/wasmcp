//! Tools handler for MCP.

#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        world: "tools-handler",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp::incoming_handler::{Guest, OutputStream};

use bindings::wasmcp::mcp::protocol::{Tool, ToolOptions, RequestMethod, Id, ToolsCallParams, Error, ErrorCode, JsonrpcObject};
use bindings::wasmcp::mcp::context::{Context, ServerCapabilities};
use bindings::wasmcp::mcp::error_writer;
use bindings::wasmcp::mcp::list_tools_writer;
use bindings::wasmcp::mcp::content_tool_writer;
use bindings::wasmcp::mcp::incoming_handler as next_handler;

use serde::Deserialize;
use serde_json::json;

pub struct Component;

impl Component {
    fn handle_tools_list(id: &Id, out: OutputStream) {

        let tools = vec![
            Tool {
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
                options: Some(ToolOptions {
                    description: Some("Echo a message back".to_string()),
                    title: Some("Echo".to_string()),
                    output_schema: None,
                    annotations: None,
                    meta: None,
                }),
            },
        ];

        if let Err(e) = list_tools_writer::send(id, out, &tools) {
            eprintln!("Failed to write tools list: {:?}", e);
        }
    }

    fn handle_tools_call(id: &Id, params: &ToolsCallParams, out: OutputStream) {
        let result = match params.name.as_str() {
            "echo" => Self::handle_echo(params.arguments.as_deref()),
            _ => {
                let _ = error_writer::send(&id, out, &Error {
                    code: ErrorCode::InvalidParams,
                    message: format!("Unknown tool: {}", params.name),
                    data: None,
                    id: Some(id.clone()),
                });
                    return;
                }
            };

            match result {
                Ok(response) => {
                    if let Err(e) = content_tool_writer::send_text(&id, out, &response) {
                        eprintln!("Failed to write response: {:?}", e);
                    }
                }
                Err(e) => {
                    let _ = error_writer::send(&id, out, &Error {
                        code: ErrorCode::InternalError,
                        message: format!("Tool execution error: {}", e),
                        data: None,
                        id: Some(id.clone()),
                    });
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


impl Guest for Component {
    fn handle(ctx: Context, out: OutputStream) {
        // ctx.register_capabilities(&ServerCapabilities {
        //     tools: Some(true),
        //     com
        // });

        let JsonrpcObject::Request(request) = ctx.data() else {
          return next_handler::handle(ctx, out);
        };

        match request.method {
            RequestMethod::ToolsList(_) => Self::handle_tools_list(&request.id, out),
            RequestMethod::ToolsCall(params) => Self::handle_tools_call(&request.id, &params, out),
            _ => next_handler::handle(ctx, out),

        }
    }
}

bindings::export!(Component with_types_in bindings);
