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
use crate::bindings::wasmcp::mcp_v20250618::mcp::{ClientRequest, ErrorCode, ProtocolVersion, ServerResult};
use crate::common;

pub struct StdioTransportGuest;

impl Guest for StdioTransportGuest {
    fn run() -> Result<(), ()> {
        // Get stdio streams
        let stdin = get_stdin();
        let stdout = get_stdout();

        // Event loop: read messages from stdin, process, write to stdout
        loop {
            // Parse incoming message
            let message = match common::parse_mcp_message(&stdin) {
                Ok(msg) => msg,
                Err(e) => {
                    eprintln!("[ERROR] Failed to parse message: {}", e);
                    continue;
                }
            };

            // Handle message based on type
            match message {
                common::McpMessage::Request(request_id, client_request) => {
                    // Handle initialize specially (capabilities discovery)
                    if let ClientRequest::Initialize(init_req) = &client_request {
                        handle_initialize(&stdout, request_id, init_req)?;
                        continue;
                    }

                    // Handle ping directly
                    if matches!(client_request, ClientRequest::Ping(_)) {
                        if let Err(e) = common::handle_ping() {
                            write_error(&stdout, Some(request_id), e);
                            continue;
                        }
                        if let Err(e) = common::write_mcp_result(&stdout, &request_id, ServerResult::Ping) {
                            eprintln!("[ERROR] Failed to write ping result: {:?}", e);
                        }
                        continue;
                    }

                    // Handle logging/setLevel directly
                    if let ClientRequest::LoggingSetLevel(level) = &client_request {
                        let level_str = log_level_to_string(*level);
                        if let Err(e) = common::handle_set_log_level(level_str) {
                            write_error(&stdout, Some(request_id.clone()), e);
                            continue;
                        }
                        if let Err(e) = common::write_mcp_result(&stdout, &request_id, ServerResult::LoggingSetLevel) {
                            eprintln!("[ERROR] Failed to write setLevel result: {:?}", e);
                        }
                        continue;
                    }

                    // Delegate everything else to middleware
                    let proto_ver = ProtocolVersion::V20250618; // TODO: Parse from init
                    match common::delegate_to_middleware(
                        request_id.clone(),
                        client_request,
                        proto_ver,
                        None, // TODO: Session support for stdio
                        &stdout,
                    ) {
                        Ok(result) => {
                            if let Err(e) =
                                common::write_mcp_result(&stdout, &request_id, result)
                            {
                                eprintln!("[ERROR] Failed to write result: {:?}", e);
                            }
                        }
                        Err(e) => {
                            write_error(&stdout, Some(request_id), e);
                        }
                    }
                }

                common::McpMessage::Notification(notification) => {
                    let proto_ver = ProtocolVersion::V20250618;
                    if let Err(e) = common::delegate_notification(notification, proto_ver, None) {
                        eprintln!("[ERROR] Notification handling failed: {:?}", e);
                    }
                }

                common::McpMessage::Result(result_id, client_result) => {
                    // Bidirectional MCP: handle result from client
                    use crate::bindings::wasmcp::mcp_v20250618::server_handler::{
                        ResultCtx, handle_result,
                    };

                    let ctx = ResultCtx {
                        id: result_id,
                        protocol_version: "2025-06-18".to_string(),
                        session: None,
                        user: None,
                    };

                    handle_result(&ctx, client_result);
                }

                common::McpMessage::Error(error_id, error_code) => {
                    // Bidirectional MCP: handle error from client
                    use crate::bindings::wasmcp::mcp_v20250618::server_handler::{
                        ErrorCtx, handle_error,
                    };

                    let ctx = ErrorCtx {
                        id: error_id,
                        protocol_version: "2025-06-18".to_string(),
                        session: None,
                        user: None,
                    };

                    handle_error(&ctx, &error_code);
                }
            }
        }
    }
}

/// Handle initialize request with capability discovery
fn handle_initialize(
    stdout: &crate::bindings::wasi::io::streams::OutputStream,
    request_id: crate::bindings::wasmcp::mcp_v20250618::mcp::RequestId,
    _init_req: &crate::bindings::wasmcp::mcp_v20250618::mcp::InitializeRequest,
) -> Result<(), ()> {
    // Discover capabilities from downstream
    let capabilities = common::discover_capabilities_for_init(ProtocolVersion::V20250618);

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
            protocol_version: ProtocolVersion::V20250618,
            options: None,
        },
    );

    if let Err(e) = common::write_mcp_result(stdout, &request_id, result) {
        eprintln!("[ERROR] Failed to write initialize result: {:?}", e);
        return Err(());
    }

    Ok(())
}

/// Write JSON-RPC error to stdout
fn write_error(
    stdout: &crate::bindings::wasi::io::streams::OutputStream,
    id: Option<crate::bindings::wasmcp::mcp_v20250618::mcp::RequestId>,
    error: ErrorCode,
) {
    use crate::bindings::wasmcp::mcp_v20250618::server_io;

    if let Err(e) = server_io::write_error(stdout, id.as_ref(), &error) {
        eprintln!("[ERROR] Failed to write error: {:?}", e);
    }
}

/// Convert LogLevel enum to string
fn log_level_to_string(level: crate::bindings::wasmcp::mcp_v20250618::mcp::LogLevel) -> String {
    use crate::bindings::wasmcp::mcp_v20250618::mcp::LogLevel;

    match level {
        LogLevel::Debug => "debug".to_string(),
        LogLevel::Info => "info".to_string(),
        LogLevel::Notice => "notice".to_string(),
        LogLevel::Warning => "warning".to_string(),
        LogLevel::Error => "error".to_string(),
        LogLevel::Critical => "critical".to_string(),
        LogLevel::Alert => "alert".to_string(),
        LogLevel::Emergency => "emergency".to_string(),
    }
}
