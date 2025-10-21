mod resources;
mod server;
mod tools;

use anyhow::Result;
use clap::Args;

#[derive(Debug, Args)]
pub struct ServerArgs {
    /// Port for HTTP server (uses stdio if not specified)
    #[arg(long)]
    pub port: Option<u16>,

    /// Enable verbose logging
    #[arg(long, short = 'v')]
    pub verbose: bool,
}

pub async fn handle_server_command(args: ServerArgs) -> Result<()> {
    server::start_server(args.port, args.verbose).await
}
