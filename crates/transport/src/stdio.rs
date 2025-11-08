//! Stdio transport implementation
//!
//! Handles stdio-specific protocol concerns:
//! - Line-delimited JSON-RPC over stdin/stdout
//! - Process lifecycle via wasi:cli/run
//!
//! Delegates I/O to server-io via common wrappers

use crate::bindings::exports::wasi::cli::run::Guest;
use crate::bindings::wasi::cli::stdin::get_stdin;
use crate::bindings::wasi::cli::stdout::get_stdout;
use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    ClientRequest, ErrorCode, ProtocolVersion, ServerResult,
};
use crate::common;

pub struct StdioTransportGuest;

impl Guest for StdioTransportGuest {
    fn run() -> Result<(), ()> {
        // Get stdio streams
        let stdin = get_stdin();
        let stdout = get_stdout();

        // Track protocol version from initialize (default to latest)
        let mut protocol_version = ProtocolVersion::V20250618;

        // Event loop: read messages from stdin, process, write to stdout
        loop {
            // Parse incoming message (blocks waiting for input)
            let message = match common::parse_mcp_message(
                &stdin,
                common::stdio_read_limit(),
                &common::stdio_frame(),
            ) {
                Ok(msg) => msg,
                Err(e) => {
                    // Stream closed means client disconnected - exit gracefully
                    if e.contains("Stream closed") {
                        return Ok(());
                    }
                    eprintln!("[ERROR] Failed to parse message: {}", e);
                    continue;
                }
            };

            // Handle message based on type
            match message {
                common::McpMessage::Request(request_id, client_request) => {
                    // Handle initialize specially (capabilities discovery)
                    if let ClientRequest::Initialize(init_req) = &client_request {
                        protocol_version = handle_initialize(&stdout, request_id, init_req)?;
                        continue;
                    }

                    // Handle ping directly
                    if matches!(client_request, ClientRequest::Ping(_)) {
                        if let Err(e) = common::handle_ping() {
                            write_error(&stdout, Some(request_id), e);
                            continue;
                        }
                        if let Err(e) = common::write_mcp_result(
                            &stdout,
                            request_id,
                            ServerResult::Ping,
                            &common::stdio_frame(),
                        ) {
                            eprintln!("[ERROR] Failed to write ping result: {:?}", e);
                        }
                        continue;
                    }

                    // Handle logging/setLevel directly
                    if let ClientRequest::LoggingSetLevel(level) = &client_request {
                        let level_str = common::protocol::log_level_to_string(*level);
                        if let Err(e) = common::handle_set_log_level(level_str) {
                            write_error(&stdout, Some(request_id.clone()), e);
                            continue;
                        }
                        if let Err(e) = common::write_mcp_result(
                            &stdout,
                            request_id,
                            ServerResult::LoggingSetLevel,
                            &common::stdio_frame(),
                        ) {
                            eprintln!("[ERROR] Failed to write setLevel result: {:?}", e);
                        }
                        continue;
                    }

                    // Delegate everything else to middleware
                    match common::delegate_to_middleware(
                        request_id.clone(),
                        client_request,
                        protocol_version,
                        Some("0"),     // Session ID "0" indicates stdio mode
                        None,          // No identity in stdio mode
                        String::new(), // No session bucket in stdio
                        &stdout,
                        &common::stdio_frame(),
                    ) {
                        Ok(result) => {
                            if let Err(e) = common::write_mcp_result(
                                &stdout,
                                request_id.clone(),
                                result,
                                &common::stdio_frame(),
                            ) {
                                eprintln!("[ERROR] Failed to write result: {:?}", e);
                            }
                        }
                        Err(e) => {
                            write_error(&stdout, Some(request_id), e);
                        }
                    }
                }

                common::McpMessage::Notification(notification) => {
                    if let Err(e) = common::delegate_notification(
                        notification,
                        protocol_version,
                        Some("0"),     // Session ID "0" indicates stdio mode
                        String::new(), // No session bucket in stdio
                        &common::stdio_frame(),
                    ) {
                        eprintln!("[ERROR] Notification handling failed: {:?}", e);
                    }
                }

                common::McpMessage::Result(result_id, client_result) => {
                    // Bidirectional MCP: handle result from client
                    use crate::bindings::wasmcp::mcp_v20250618::mcp::ClientMessage;
                    use crate::bindings::wasmcp::mcp_v20250618::server_handler::handle;

                    let ctx = common::create_message_context(
                        None,
                        protocol_version,
                        Some("0"), // Session ID "0" indicates stdio mode
                        None,
                        "",
                        &common::stdio_frame(),
                    );

                    let message = ClientMessage::Result((result_id, client_result));
                    handle(&ctx, message);
                }

                common::McpMessage::Error(error_id, error_code) => {
                    // Bidirectional MCP: handle error from client
                    use crate::bindings::wasmcp::mcp_v20250618::mcp::ClientMessage;
                    use crate::bindings::wasmcp::mcp_v20250618::server_handler::handle;

                    let ctx = common::create_message_context(
                        None,
                        protocol_version,
                        Some("0"), // Session ID "0" indicates stdio mode
                        None,
                        "",
                        &common::stdio_frame(),
                    );

                    let message = ClientMessage::Error((error_id, error_code));
                    handle(&ctx, message);
                }
            }
        }
    }
}

/// Handle initialize request with capability discovery
/// Returns the negotiated protocol version
fn handle_initialize(
    stdout: &crate::bindings::wasi::io::streams::OutputStream,
    request_id: crate::bindings::wasmcp::mcp_v20250618::mcp::RequestId,
    init_req: &crate::bindings::wasmcp::mcp_v20250618::mcp::InitializeRequest,
) -> Result<ProtocolVersion, ()> {
    // Use client's requested protocol version (for now, we only support one version)
    let protocol_version = init_req.protocol_version;

    // Discover capabilities from downstream
    let capabilities =
        common::discover_capabilities_for_init(protocol_version, &common::stdio_frame());

    // Create initialize result
    let result = ServerResult::Initialize(
        crate::bindings::wasmcp::mcp_v20250618::mcp::InitializeResult {
            meta: None,
            server_info: crate::bindings::wasmcp::mcp_v20250618::mcp::Implementation {
                name: "wasmcp-server".to_string(),
                title: None,
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            capabilities,
            protocol_version,
            options: None,
        },
    );

    if let Err(e) = common::write_mcp_result(stdout, request_id, result, &common::stdio_frame()) {
        eprintln!("[ERROR] Failed to write initialize result: {:?}", e);
        return Err(());
    }

    Ok(protocol_version)
}

/// Write JSON-RPC error to stdout
fn write_error(
    stdout: &crate::bindings::wasi::io::streams::OutputStream,
    id: Option<crate::bindings::wasmcp::mcp_v20250618::mcp::RequestId>,
    error: ErrorCode,
) {
    use crate::bindings::wasmcp::mcp_v20250618::mcp::ServerMessage;
    use crate::bindings::wasmcp::mcp_v20250618::server_io;

    let message = ServerMessage::Error((id, error));
    if let Err(e) = server_io::send_message(stdout, message, &common::stdio_frame()) {
        eprintln!("[ERROR] Failed to write error: {:?}", e);
    }
}
