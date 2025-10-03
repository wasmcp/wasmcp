#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        world: "stdio-transport",
        generate_all,
    });
}

use bindings::exports::wasi::cli::run::Guest;
use bindings::wasi::cli::{stderr, stdin, stdout};
use bindings::wasmcp::mcp::incoming_handler;
use bindings::wasmcp::mcp::request::Request;

struct Component;

impl Guest for Component {
    fn run() -> Result<(), ()> {
        // MCP over stdio is a persistent connection that handles multiple messages
        // Loop continuously reading newline-delimited JSON-RPC messages from stdin
        loop {
            // Get stdin for reading JSON-RPC requests
            let input_stream = stdin::get_stdin();

            // Get stdout for writing JSON-RPC responses
            let output_stream = stdout::get_stdout();

            // Parse the incoming JSON-RPC request from stdin (reads until newline)
            let mcp_request = match Request::from_stdio_stream(&input_stream) {
                Ok(req) => req,
                Err(bindings::wasi::io::streams::StreamError::Closed) => {
                    // stdin closed - this is normal shutdown, exit cleanly
                    return Ok(());
                }
                Err(e) => {
                    // Other error - log to stderr and exit
                    let error_stream = stderr::get_stderr();
                    let error_msg = format!("Failed to parse JSON-RPC request: {:?}\n", e);
                    let _ = error_stream.blocking_write_and_flush(error_msg.as_bytes());
                    return Err(());
                }
            };

            // Forward the request to the handler chain
            // The handler will write the JSON-RPC response directly to stdout
            incoming_handler::handle(mcp_request, output_stream);
        }
    }
}

bindings::export!(Component with_types_in bindings);
