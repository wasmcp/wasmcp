use rmcp::ServerHandler;
use std::path::PathBuf;
use wasmcp::commands::server::WasmcpServer;

// Helper to create test server
fn create_test_server() -> WasmcpServer {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    WasmcpServer::new(project_root).expect("Failed to create test server")
}

#[test]
fn test_server_info() {
    let server = create_test_server();
    let info = server.get_info();

    assert_eq!(info.server_info.name, "wasmcp-mcp-server");
    assert!(info.capabilities.tools.is_some());
    assert!(info.capabilities.resources.is_some());
    assert!(info.instructions.is_some());
}

#[test]
fn test_server_creation() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let result = WasmcpServer::new(project_root);
    assert!(result.is_ok(), "Server creation should succeed");
}

#[test]
fn test_server_info_has_correct_capabilities() {
    let server = create_test_server();
    let info = server.get_info();

    // Check that tools capability is properly set
    assert!(info.capabilities.tools.is_some());

    // Check that resources capability is properly set
    assert!(info.capabilities.resources.is_some());

    // Check website URL is set
    assert_eq!(
        info.server_info.website_url,
        Some("https://github.com/wasmcp/wasmcp".into())
    );
}
