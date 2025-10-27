mod clean;
pub mod daemon; // Public for internal daemon entry point
mod logs;
mod manager;
mod restart;
mod serve;
mod status;
mod stop;

pub use serve::start_server;

// Export WasmcpServer for testing (now from mcp module)
#[allow(unused_imports)] // Used in integration tests
pub use crate::mcp::WasmcpServer;

use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct ServerArgs {
    #[command(subcommand)]
    pub command: Option<ServerCommand>,

    /// Port for HTTP server (default: 8085, use --stdio for stdio mode)
    #[arg(long, default_value = "8085", global = true)]
    pub port: u16,

    /// Use stdio transport instead of HTTP
    #[arg(long, global = true)]
    pub stdio: bool,

    /// Enable verbose logging
    #[arg(long, short = 'v', global = true)]
    pub verbose: bool,

    /// Override GitHub resource fetching with local filesystem reads.
    /// Provide absolute path to repository root (e.g., /path/to/wasmcp).
    /// All resource URIs will read from local files instead of GitHub.
    /// Branch placeholders in URIs are ignored - always reads local working tree.
    #[arg(long, value_name = "PATH", global = true)]
    pub local_resources: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum ServerCommand {
    /// Run server in foreground (default)
    Serve,

    /// Start server as background daemon
    Start,

    /// Stop background daemon
    Stop,

    /// Restart background daemon
    Restart,

    /// Show daemon status and health
    Status,

    /// View daemon logs
    Logs {
        /// Follow log output (like tail -f)
        #[arg(short = 'f', long)]
        follow: bool,
    },

    /// Clean up daemon state files
    Clean,
}

pub async fn handle_server_command(args: ServerArgs) -> Result<()> {
    let port = if args.stdio { None } else { Some(args.port) };

    match args.command {
        // Default to serve if no subcommand specified (backward compatibility)
        None | Some(ServerCommand::Serve) => {
            serve::start_server(port, args.verbose, args.local_resources).await
        }
        Some(ServerCommand::Start) => daemon::start(port, args.verbose, args.local_resources).await,
        Some(ServerCommand::Stop) => stop::stop().await,
        Some(ServerCommand::Restart) => {
            restart::restart(port, args.verbose, args.local_resources).await
        }
        Some(ServerCommand::Status) => status::status().await,
        Some(ServerCommand::Logs { follow }) => logs::logs(follow).await,
        Some(ServerCommand::Clean) => clean::clean().await,
    }
}
