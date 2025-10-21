mod r#impl;
mod resources;
mod tools;

pub use r#impl::start_server;

// Export WasmcpServer for testing
#[allow(unused_imports)]  // Used in integration tests
pub use r#impl::WasmcpServer;

use anyhow::Result;
use clap::Args;

#[derive(Debug, Args)]
pub struct ServerArgs {
    /// Port for HTTP server (default: 8085, use --stdio for stdio mode)
    #[arg(long, default_value = "8085")]
    pub port: u16,

    /// Use stdio transport instead of HTTP
    #[arg(long)]
    pub stdio: bool,

    /// Enable verbose logging
    #[arg(long, short = 'v')]
    pub verbose: bool,
}

pub async fn handle_server_command(args: ServerArgs) -> Result<()> {
    let port = if args.stdio { None } else { Some(args.port) };
    start_server(port, args.verbose).await
}
