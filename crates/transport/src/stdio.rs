//! Stdio transport implementation
//!
//! Handles stdio-specific protocol concerns:
//! - Line-delimited JSON-RPC over stdin/stdout
//! - Process lifecycle via wasi:cli/run
//!
//! Delegates I/O to stdio-server-io via server-io interface

use crate::bindings::exports::wasi::cli::run::Guest;

pub struct StdioTransportGuest;

impl Guest for StdioTransportGuest {
    fn run() -> Result<(), ()> {
        // TODO: Implement stdio transport
        // - Read lines from stdin
        // - Parse using server-io.parse_*()
        // - Route to transport methods or middleware
        // - Write using server-io.write_*()

        Ok(())
    }
}
