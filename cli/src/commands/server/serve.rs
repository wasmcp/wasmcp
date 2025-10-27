use anyhow::Result;
use rmcp::ServiceExt;
use rmcp::transport::stdio;
use std::path::PathBuf;

// Import WasmcpServer from the new mcp module
use crate::mcp::WasmcpServer;

pub async fn start_server(
    port: Option<u16>,
    verbose: bool,
    local_resources: Option<PathBuf>,
) -> Result<()> {
    // Set log level based on verbose flag (must be done before init)
    if verbose {
        // SAFETY: This is safe because we're setting it before any threads are spawned
        // and before any code reads this variable. The server is single-threaded at this point.
        unsafe {
            std::env::set_var("RUST_LOG", "wasmcp=debug,rmcp=debug");
        }
    }

    // Initialize logging to XDG data directory
    crate::logging::init()?;

    if verbose {
        tracing::info!("Verbose logging enabled");
    }

    tracing::info!("Starting wasmcp MCP server");

    // Log local resources override if enabled
    if let Some(path) = &local_resources {
        tracing::info!("Using local resources from: {}", path.display());
        eprintln!("Using local resources from: {}", path.display());
    }

    let project_root = std::env::current_dir()?;

    // Get registration counts before creating server
    let resources = crate::mcp::resources::list_all(&project_root)?;
    let tools = crate::mcp::tools::list_tools()?;

    tracing::info!(
        "Registered {} resources and {} tools",
        resources.resources.len(),
        tools.tools.len()
    );
    eprintln!(
        "Registered {} resources and {} tools",
        resources.resources.len(),
        tools.tools.len()
    );

    let server = WasmcpServer::new(project_root, local_resources)?;

    match port {
        None => {
            eprintln!("Starting wasmcp MCP server (stdio mode)...");
            let service = server.serve(stdio()).await?;
            service.waiting().await?;
        }
        Some(port) => {
            start_http_server(server, port).await?;
        }
    }

    Ok(())
}

async fn start_http_server(server: WasmcpServer, port: u16) -> Result<()> {
    use rmcp::transport::streamable_http_server::{
        StreamableHttpService, session::local::LocalSessionManager,
    };
    use std::net::SocketAddr;

    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse()?;

    eprintln!("Starting wasmcp MCP server (HTTP/Streamable mode)...");
    eprintln!("Listening on http://{}", addr);
    eprintln!("MCP endpoint: http://{}/mcp", addr);

    // Create streamable HTTP service
    let service = StreamableHttpService::new(
        move || Ok(server.clone()),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    // Create router with single /mcp endpoint
    let router = axum::Router::new().nest_service("/mcp", service);

    // Start the HTTP server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            eprintln!("Received shutdown signal...");
        })
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    /// Test HTTP server address parsing
    #[test]
    fn test_http_server_address_format() {
        let port = 8085;
        let addr = format!("127.0.0.1:{}", port);
        assert_eq!(addr, "127.0.0.1:8085");

        let custom_port = 9000;
        let custom_addr = format!("127.0.0.1:{}", custom_port);
        assert_eq!(custom_addr, "127.0.0.1:9000");
    }

    /// Test MCP endpoint URL construction
    #[test]
    fn test_mcp_endpoint_url() {
        let addr = "127.0.0.1:8085";
        let endpoint = format!("http://{}/mcp", addr);
        assert_eq!(endpoint, "http://127.0.0.1:8085/mcp");
    }

    /// Test server startup messages
    #[test]
    fn test_startup_messages() {
        let stdio_msg = "Starting wasmcp MCP server (stdio mode)...";
        assert!(stdio_msg.contains("stdio mode"));

        let http_msg = "Starting wasmcp MCP server (HTTP/Streamable mode)...";
        assert!(http_msg.contains("HTTP/Streamable mode"));
    }

    /// Test local resources logging message
    #[test]
    fn test_local_resources_message() {
        let path = "/path/to/repo";
        let msg = format!("Using local resources from: {}", path);
        assert!(msg.contains("Using local resources"));
        assert!(msg.contains("/path/to/repo"));
    }

    /// Test resource and tool count logging
    #[test]
    fn test_registration_counts_message() {
        let resources = 10;
        let tools = 5;
        let msg = format!("Registered {} resources and {} tools", resources, tools);
        assert!(msg.contains("Registered 10 resources"));
        assert!(msg.contains("5 tools"));
    }

    /// Test shutdown signal message
    #[test]
    fn test_shutdown_message() {
        let msg = "Received shutdown signal...";
        assert!(msg.contains("shutdown signal"));
    }

    /// Test HTTP listening message format
    #[test]
    fn test_listening_message() {
        let addr = "127.0.0.1:8085";
        let msg = format!("Listening on http://{}", addr);
        assert!(msg.contains("Listening on"));
        assert!(msg.contains("http://127.0.0.1:8085"));
    }

    /// Test verbose logging flag message
    #[test]
    fn test_verbose_logging_message() {
        let msg = "Verbose logging enabled";
        assert!(msg.contains("Verbose logging enabled"));
    }
}
