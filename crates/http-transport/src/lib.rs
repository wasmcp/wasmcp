//! HTTP transport component for the Model Context Protocol (MCP)
//!
//! This component provides an HTTP-to-MCP bridge, converting incoming HTTP requests
//! into MCP protocol messages that can be processed by composed MCP handler components.
//!
//! The component:
//! - Accepts HTTP requests via the WASI HTTP interface
//! - Extracts request bodies as input streams
//! - Creates MCP request resources from the input streams
//! - Forwards MCP requests to the handler chain
//! - Returns JSON responses for successful requests
//! - Provides appropriate error responses with HTTP status codes

// Generated code - not formatted or linted
#[rustfmt::skip]
#[allow(clippy::all)]
#[allow(dead_code)]
#[allow(unused_imports)]
#[allow(non_snake_case)]
mod bindings;

pub mod http;

use bindings::exports::wasi::http::incoming_handler::{Guest, IncomingRequest, ResponseOutparam};
use bindings::wasi::http::types::{
    Headers, OutgoingBody, OutgoingResponse, ResponseOutparam as WasiResponseOutparam,
};
use bindings::wasmcp::mcp::incoming_handler::{handle, Request};

struct Component;

impl Guest for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        // Validate HTTP method is POST
        let method = request.method();
        if !matches!(method, bindings::wasi::http::types::Method::Post) {
            send_error_response(
                response_out,
                405,
                b"Method Not Allowed: Only POST requests are supported",
            );
            return;
        }

        // Validate path is /mcp (strict enforcement for MCP streamable-http transport)
        let path = request.path_with_query().unwrap_or_default();
        if !path.starts_with("/mcp") {
            send_error_response(response_out, 404, b"Not Found: MCP endpoint is POST /mcp");
            return;
        }

        // Get the incoming HTTP request body
        let incoming_body = match request.consume() {
            Ok(body) => body,
            Err(_) => {
                // Request has already been consumed or error occurred
                send_error_response(
                    response_out,
                    400,
                    b"Bad Request: Unable to consume request body",
                );
                return;
            }
        };

        // Get the input stream from the body
        let input_stream = incoming_body.stream().expect("Failed to get input stream");

        // Create an MCP request from the HTTP request body stream
        let mcp_request = match Request::from_http_stream(&input_stream) {
            Ok(req) => req,
            Err(e) => {
                send_error_response(
                    response_out,
                    400,
                    format!("Failed to parse MCP request: {:?}", e).as_bytes(),
                );
                return;
            }
        };

        // Create the response and get its output stream
        let headers = Headers::new();
        let content_type = http::content_type_for_response(false);
        headers
            .set("content-type", &[content_type.as_bytes().to_vec()])
            .expect("Failed to set content-type header");

        let response = OutgoingResponse::new(headers);
        let response_body = response.body().expect("Failed to get response body");

        // Set the response
        WasiResponseOutparam::set(response_out, Ok(response));

        {
            let output_stream = response_body
                .write()
                .expect("Failed to get output stream from response body");

            // Forward the MCP request to the imported handler with the output stream
            handle(mcp_request, output_stream);

            // Stream is dropped here when it goes out of scope
        }

        // Finish the response body
        OutgoingBody::finish(response_body, None).expect("Failed to finish response body");
    }
}

fn send_error_response(response_out: ResponseOutparam, status: u16, message: &[u8]) {
    let headers = Headers::new();
    let content_type = http::content_type_for_response(true);
    headers
        .set("content-type", &[content_type.as_bytes().to_vec()])
        .expect("Failed to set content-type header");

    let response = OutgoingResponse::new(headers);
    response
        .set_status_code(status)
        .expect("Failed to set status code");

    let response_body = response.body().expect("Failed to get response body");
    {
        let output_stream = response_body
            .write()
            .expect("Failed to get output stream from response body");

        // Write error message
        output_stream
            .blocking_write_and_flush(message)
            .expect("Failed to write error message");

        // Stream is dropped here when it goes out of scope
    }

    // Set the response and finish
    WasiResponseOutparam::set(response_out, Ok(response));
    OutgoingBody::finish(response_body, None).expect("Failed to finish error response body");
}

bindings::export!(Component with_types_in bindings);
