//! {{ handler_type_capitalized }} handler for MCP.

#[allow(warnings)]
mod bindings;

use bindings::exports::wasmcp::mcp::incoming_handler::{Guest, OutputStream, Request};
use bindings::wasmcp::mcp::incoming_handler as next_handler;
use bindings::wasmcp::mcp::tools_list_result;
use bindings::wasmcp::mcp::tools_call_content;
use bindings::wasmcp::mcp::error_result;
use bindings::wasmcp::mcp::request::{Params, ServerCapabilities};
use serde::Deserialize;
use serde_json::json;

pub struct Component;

impl Component {
    fn handle_tools_list(request: &Request, output: OutputStream) {
        let id = request.id();

        let tools = vec![
            tools_list_result::Tool {
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
                options: Some(tools_list_result::ToolOptions {
                    description: Some("Echo a message back".to_string()),
                    title: Some("Echo".to_string()),
                    output_schema: None,
                    annotations: None,
                    meta: None,
                }),
            },
        ];

        if let Err(e) = tools_list_result::write(&id, output, &tools, None) {
            eprintln!("Failed to write tools list: {:?}", e);
        }
    }

    fn handle_tools_call(request: &Request, output: OutputStream) {
        let id = request.id();

        if let Ok(Params::ToolsCall(params)) = request.params() {
            let result = match params.name.as_str() {
                "echo" => Self::handle_echo(params.arguments.as_deref()),
                _ => {
                    let _ = tools_call_content::write_error(&id, output, &format!("Unknown tool: {}", params.name));
                    return;
                }
            };

            match result {
                Ok(response) => {
                    if let Err(e) = tools_call_content::write_text(&id, output, &response, None) {
                        eprintln!("Failed to write response: {:?}", e);
                    }
                }
                Err(e) => {
                    let _ = tools_call_content::write_error(&id, output, &e.to_string());
                }
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
    fn handle(request: Request, output: OutputStream) {
        if !request.needs(ServerCapabilities::TOOLS) {
            next_handler::handle(request, output);
            return;
        }

        match request.params() {
            Ok(Params::ToolsList(_)) => Self::handle_tools_list(&request, output),
            Ok(Params::ToolsCall(_)) => Self::handle_tools_call(&request, output),
            Ok(_) => unreachable!(),
            Err(error) => {
                let _ = error_result::write(&request.id(), output, &error);
            }
        }
    }
}

bindings::export!(Component with_types_in bindings);
