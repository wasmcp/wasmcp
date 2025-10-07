//! HTTP transport component for the Model Context Protocol (MCP)
//!
//! This component provides a Streamable HTTP transport implementation that converts
//! incoming HTTP requests into MCP protocol messages for processing by handler chains.
//!
//! The component:
//! - Accepts HTTP POST requests at the /mcp endpoint
//! - Reads the request body containing JSON-RPC messages
//! - Parses the body into a context object
//! - Always returns text/event-stream responses (SSE)
//! - Allows middleware to send notifications/requests before the final response
//! - Handles both requests and notifications/responses appropriately
//!
//! This implementation follows the Streamable HTTP transport specification and
//! leverages the chain-of-responsibility pattern to enable rich server interactions.

mod bindings {
    wit_bindgen::generate!({
        world: "http-transport",
        generate_all,
    });
}

use bindings::exports::wasi::http::incoming_handler::{Guest, IncomingRequest, ResponseOutparam};
use bindings::wasi::http::types::{
    Headers, IncomingBody, Method, OutgoingBody, OutgoingResponse,
    ResponseOutparam as WasiResponseOutparam,
};
use bindings::wasmcp::mcp::context::Context;
use bindings::wasmcp::mcp::incoming_handler::handle;
use bindings::wasmcp::mcp::protocol::{JsonrpcObject};

struct Component;

impl Guest for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        // Validate HTTP method is POST
        let method = request.method();
        if !matches!(method, Method::Post) {
            send_error_response(
                response_out,
                405,
                b"{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32600,\"message\":\"Method Not Allowed: Only POST requests are supported\"},\"id\":null}",
            );
            return;
        }

        // Validate path is /mcp
        let path = request.path_with_query().unwrap_or_default();
        if !path.starts_with("/mcp") {
            send_error_response(
                response_out,
                404,
                b"{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32600,\"message\":\"Not Found: MCP endpoint is /mcp\"},\"id\":null}",
            );
            return;
        }

        // Check Accept header to ensure client can handle SSE
        let headers = request.headers();
        let accept_header = headers.get(&"accept".to_string());
        let accepts_sse = accept_header
            .iter()
            .any(|value| {
                std::str::from_utf8(value)
                    .map(|s| s.contains("text/event-stream") || s.contains("*/*"))
                    .unwrap_or(false)
            });

        if !accepts_sse {
            send_error_response(
                response_out,
                406, // Not Acceptable
                b"{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32600,\"message\":\"Not Acceptable: Client must accept text/event-stream\"},\"id\":null}",
            );
            return;
        }

        // Get Content-Length header if present
        let content_length_values = headers.get(&"content-length".to_string());
        let content_length = content_length_values
            .first()
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .and_then(|s| s.parse::<usize>().ok());

        // Check for Transfer-Encoding: chunked
        let transfer_encoding = headers.get(&"transfer-encoding".to_string());
        let is_chunked = transfer_encoding
            .first()
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .map(|s| s.to_lowercase().contains("chunked"))
            .unwrap_or(false);

        // Get the incoming HTTP request body
        let incoming_body = match request.consume() {
            Ok(body) => body,
            Err(_) => {
                send_error_response(
                    response_out,
                    400,
                    b"{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32600,\"message\":\"Bad Request: Unable to consume request body\"},\"id\":null}",
                );
                return;
            }
        };

        // Validate we have a way to determine body length
        if content_length.is_none() && !is_chunked {
            send_error_response(
                response_out,
                411,  // Length Required
                b"{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32600,\"message\":\"Length Required: Content-Length header or chunked encoding required\"},\"id\":null}",
            );
            return;
        }

        // Read the entire body into bytes
        let bytes = match read_body_to_bytes(incoming_body, content_length, is_chunked) {
            Ok(bytes) => bytes,
            Err(_) => {
                send_error_response(
                    response_out,
                    400,
                    b"{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32700,\"message\":\"Parse error: Failed to read request body\"},\"id\":null}",
                );
                return;
            }
        };

        // Parse the bytes into a context
        let context = match Context::from_bytes(&bytes) {
            Ok(ctx) => ctx,
            Err(e) => {
                let error_msg = format!(
                    "{{\"jsonrpc\":\"2.0\",\"error\":{{\"code\":-32700,\"message\":\"Parse error: {}\"}},\"id\":null}}",
                    e
                );
                send_error_response(response_out, 400, error_msg.as_bytes());
                return;
            }
        };

        // Check what type of JSON-RPC message we received
        let message_type = context.data();
        let (status_code, use_sse) = match message_type {
            JsonrpcObject::Request(_) => {
                // Requests always get SSE responses to allow middleware to send notifications
                (200, true)
            }
            JsonrpcObject::Notification(_) | JsonrpcObject::Result(_) => {
                // Notifications and responses get 202 Accepted with no body
                (202, false)
            }
            JsonrpcObject::Error(_) => {
                // Errors shouldn't be sent TO the server per the MCP spec, but handle gracefully
                (400, false)
            }
        };

        if use_sse {
            // Create SSE response for requests
            let headers = Headers::new();
            headers
                .set(
                    &"content-type".to_string(),
                    &[b"text/event-stream".to_vec()],
                )
                .expect("Failed to set content-type header");
            headers
                .set(
                    &"cache-control".to_string(),
                    &[b"no-cache".to_vec()],
                )
                .expect("Failed to set cache-control header");
            headers
                .set(
                    &"x-accel-buffering".to_string(),
                    &[b"no".to_vec()],
                )
                .expect("Failed to set x-accel-buffering header");

            let response = OutgoingResponse::new(headers);
            response
                .set_status_code(status_code)
                .expect("Failed to set status code");

            let response_body = response.body().expect("Failed to get response body");

            // Set the response before handling (required by WASI HTTP)
            WasiResponseOutparam::set(response_out, Ok(response));

            // Get the output stream and forward to handler
            {
                let output_stream = response_body
                    .write()
                    .expect("Failed to get output stream from response body");

                // Write SSE stream header
                output_stream
                    .blocking_write_and_flush(b"data: {\"type\":\"stream-start\"}\n\n")
                    .ok(); // Ignore error, handler will write actual data

                // Forward the context to the handler chain
                // Handlers can write multiple SSE events before the final response
                handle(context, output_stream);

                // The stream is dropped here when it goes out of scope
            }

            // Finish the response body
            OutgoingBody::finish(response_body, None).expect("Failed to finish response body");
        } else {
            // For notifications/responses, return 202 Accepted with no body
            let headers = Headers::new();
            let response = OutgoingResponse::new(headers);
            response
                .set_status_code(status_code)
                .expect("Failed to set status code");

            let response_body = response.body().expect("Failed to get response body");

            // Set the response
            WasiResponseOutparam::set(response_out, Ok(response));

            if status_code == 202 {
                // Forward to handler anyway (it might need to process the notification)
                {
                    let output_stream = response_body
                        .write()
                        .expect("Failed to get output stream from response body");

                    // Forward the context to the handler
                    handle(context, output_stream);

                    // Handler shouldn't write anything for notifications, but if it does, it's ignored
                }
            }

            // Finish the response body
            OutgoingBody::finish(response_body, None).expect("Failed to finish response body");
        }
    }
}

/// Read an incoming body stream to bytes
///
/// Handles both Content-Length and chunked Transfer-Encoding.
fn read_body_to_bytes(
    body: IncomingBody,
    content_length: Option<usize>,
    is_chunked: bool,
) -> Result<Vec<u8>, ()> {
    let stream = body.stream().map_err(|_| ())?;

    let mut bytes = if let Some(len) = content_length {
        Vec::with_capacity(len)
    } else {
        Vec::new()
    };

    if let Some(expected_length) = content_length {
        // Content-Length is specified: read exactly that many bytes
        while bytes.len() < expected_length {
            let remaining = expected_length - bytes.len();
            let chunk_size = remaining.min(65536);

            match stream.blocking_read(chunk_size as u64) {
                Ok(chunk) => {
                    if chunk.is_empty() {
                        // Stream closed before we got all expected bytes
                        return Err(());
                    }
                    bytes.extend_from_slice(&chunk);
                }
                Err(_) => {
                    // Stream error before we got all expected bytes
                    return Err(());
                }
            }
        }
    } else if is_chunked {
        // Transfer-Encoding: chunked - read until stream ends
        loop {
            match stream.blocking_read(65536) {
                Ok(chunk) => {
                    if chunk.is_empty() {
                        break;
                    }
                    bytes.extend_from_slice(&chunk);
                }
                Err(_) => {
                    break;
                }
            }
        }
    } else {
        return Err(());
    }

    // Consume the body to get any trailers (we ignore them for MCP)
    let _trailers = IncomingBody::finish(body);

    Ok(bytes)
}

/// Send an error response with the given status code and message
fn send_error_response(response_out: ResponseOutparam, status: u16, message: &[u8]) {
    let headers = Headers::new();
    headers
        .set(
            &"content-type".to_string(),
            &[b"application/json".to_vec()],
        )
        .expect("Failed to set content-type header");

    let response = OutgoingResponse::new(headers);
    response
        .set_status_code(status)
        .expect("Failed to set status code");

    let response_body = response.body().expect("Failed to get response body");

    // Set the response
    WasiResponseOutparam::set(response_out, Ok(response));

    {
        let output_stream = response_body
            .write()
            .expect("Failed to get output stream from response body");

        // Write error message
        output_stream
            .blocking_write_and_flush(message)
            .expect("Failed to write error message");
    }

    // Finish the response body
    OutgoingBody::finish(response_body, None).expect("Failed to finish error response body");
}

bindings::export!(Component with_types_in bindings);