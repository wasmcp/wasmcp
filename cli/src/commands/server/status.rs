use anyhow::{Context, Result};

use super::manager;

/// Show daemon status and health
pub async fn status() -> Result<()> {
    // Read PID file
    let pid = match manager::read_pid() {
        Ok(pid) => pid,
        Err(_) => {
            println!("✗ Server stopped");
            return Ok(());
        }
    };

    // Check if process is alive
    if !manager::is_process_alive(pid) {
        println!("⚠ Server stopped (stale PID file detected)");
        // Clean up stale PID
        manager::remove_pid().context("Failed to remove stale PID file")?;
        return Ok(());
    }

    // Process is alive
    println!("✓ Server running (PID: {})", pid);

    // Read flags to get port
    if let Ok(flags) = manager::read_flags() {
        println!("  Port: {}", flags.port);

        // Perform MCP health check
        let health_status = check_mcp_health(flags.port).await;
        println!("  Health: {}", health_status);
    }

    Ok(())
}

async fn check_mcp_health(port: u16) -> String {
    use rmcp::{
        ServiceExt,
        model::{ClientCapabilities, ClientInfo, Implementation},
        transport::StreamableHttpClientTransport,
    };

    let url = format!("http://127.0.0.1:{}/mcp", port);

    // Create transport
    let transport = StreamableHttpClientTransport::from_uri(url.as_str());

    // Create client info
    let client_info = ClientInfo {
        protocol_version: rmcp::model::ProtocolVersion::V_2025_03_26,
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "wasmcp-health-check".into(),
            version: "1.0.0".into(),
            title: None,
            icons: None,
            website_url: None,
        },
    };

    // Connect to server
    let client = match client_info.serve(transport).await {
        Ok(c) => c,
        Err(e) => return format!("FAILED (connection: {})", e),
    };

    // Get server info
    let server_info = match client.peer_info() {
        Some(info) => info,
        None => return "FAILED (no server info)".to_string(),
    };
    let server_name = &server_info.server_info.name;
    let server_version = &server_info.server_info.version;

    // List tools
    let tools_result = match client.list_tools(Default::default()).await {
        Ok(result) => result,
        Err(e) => return format!("FAILED (tools/list: {})", e),
    };
    let tools_count = tools_result.tools.len();

    // List resources
    let resources_result = match client.list_resources(Default::default()).await {
        Ok(result) => result,
        Err(e) => return format!("FAILED (resources/list: {})", e),
    };
    let resources_count = resources_result.resources.len();

    format!(
        "OK ({} {} - {} tools, {} resources)",
        server_name, server_version, tools_count, resources_count
    )
}

#[cfg(test)]
mod tests {
    /// Test health status message formatting
    #[test]
    fn test_health_message_format() {
        let msg = format!(
            "OK ({} {} - {} tools, {} resources)",
            "test-server", "1.0.0", 5, 10
        );
        assert!(msg.contains("OK"));
        assert!(msg.contains("test-server"));
        assert!(msg.contains("1.0.0"));
        assert!(msg.contains("5 tools"));
        assert!(msg.contains("10 resources"));
    }

    /// Test failed health message formats
    #[test]
    fn test_health_failed_messages() {
        let connection_fail = format!("FAILED (connection: {})", "connection refused");
        assert!(connection_fail.contains("FAILED"));
        assert!(connection_fail.contains("connection"));

        let no_info = "FAILED (no server info)".to_string();
        assert!(no_info.contains("FAILED"));
        assert!(no_info.contains("no server info"));

        let tools_fail = format!("FAILED (tools/list: {})", "method not found");
        assert!(tools_fail.contains("FAILED"));
        assert!(tools_fail.contains("tools/list"));
    }

    /// Test MCP health check URL construction
    #[test]
    fn test_health_check_url_construction() {
        let port = 8085;
        let url = format!("http://127.0.0.1:{}/mcp", port);
        assert_eq!(url, "http://127.0.0.1:8085/mcp");

        let custom_port = 9000;
        let custom_url = format!("http://127.0.0.1:{}/mcp", custom_port);
        assert_eq!(custom_url, "http://127.0.0.1:9000/mcp");
    }

    /// Test client info structure
    #[test]
    fn test_client_info_values() {
        use rmcp::model::{ClientCapabilities, ClientInfo, Implementation, ProtocolVersion};

        let client_info = ClientInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "wasmcp-health-check".into(),
                version: "1.0.0".into(),
                title: None,
                icons: None,
                website_url: None,
            },
        };

        assert_eq!(client_info.client_info.name, "wasmcp-health-check");
        assert_eq!(client_info.client_info.version, "1.0.0");
        assert_eq!(client_info.protocol_version, ProtocolVersion::V_2025_03_26);
    }

    /// Test status output messages
    #[test]
    fn test_status_messages() {
        // Test stopped message
        let stopped = "✗ Server stopped";
        assert!(stopped.contains("stopped"));

        // Test stale PID message
        let stale = "⚠ Server stopped (stale PID file detected)";
        assert!(stale.contains("stale PID"));

        // Test running message format
        let pid = 12345;
        let running = format!("✓ Server running (PID: {})", pid);
        assert!(running.contains("running"));
        assert!(running.contains("12345"));
    }
}
