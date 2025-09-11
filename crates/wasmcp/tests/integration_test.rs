use std::time::Duration;
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, StreamableHttpServerConfig,
    session::local::LocalSessionManager,
};
use rmcp::handler::server::ServerHandler;
use rmcp::model::*;

// Import server modules at top level
#[path = "../src/main.rs"]
mod main;
#[path = "../src/runtime_impl.rs"]
mod runtime_impl;
#[path = "../src/tools/mod.rs"]
mod tools;
#[path = "../src/deps/mod.rs"]
mod deps;

#[tokio::test]
async fn test_server_starts_and_listens() {
    let bind_addr = "127.0.0.1:0"; // Use port 0 for automatic assignment
    
    let config = StreamableHttpServerConfig {
        stateful_mode: true,
        sse_keep_alive: Some(Duration::from_secs(15)),
    };
    
    let service = StreamableHttpService::new(
        || Ok(main::WasmcpServer::new()),
        LocalSessionManager::default().into(),
        config,
    );

    let app = axum::Router::new().nest_service("/mcp", service);

    let listener = tokio::net::TcpListener::bind(bind_addr).await.unwrap();
    let actual_addr = listener.local_addr().unwrap();
    
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Test POST without session creates a new session
    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://{}/mcp", actual_addr))
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}}}"#)
        .send()
        .await
        .unwrap();
    
    // Debug: print response details
    if response.status() != 200 {
        let status = response.status();
        let body = response.text().await.unwrap();
        panic!("Expected 200, got {}: {}", status, body);
    }
    
    // Should return 200 OK with SSE stream (new session created)
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-type").unwrap(), "text/event-stream");
    
    handle.abort();
}

#[tokio::test] 
async fn test_mcp_protocol_flow() {
    let bind_addr = "127.0.0.1:0";
    
    let config = StreamableHttpServerConfig {
        stateful_mode: false,  // Use stateless mode for simple testing
        sse_keep_alive: None,
    };
    
    let service = StreamableHttpService::new(
        || Ok(main::WasmcpServer::new()),
        LocalSessionManager::default().into(),
        config,
    );

    let app = axum::Router::new().nest_service("/mcp", service);

    let listener = tokio::net::TcpListener::bind(bind_addr).await.unwrap();
    let actual_addr = listener.local_addr().unwrap();
    
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let base_url = format!("http://{}/mcp", actual_addr);
    
    let client = reqwest::Client::new();
    
    // In stateless mode, POST gets immediate response
    let init_response = client
        .post(&base_url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}}}"#)
        .send()
        .await
        .unwrap();
    
    // In stateless mode, should get 200 OK with SSE stream containing response
    assert_eq!(init_response.status(), 200);
    
    handle.abort();
}

#[tokio::test]
async fn test_tool_handlers_directly() {
    let server = main::WasmcpServer::new();
    
    // Test that server info is correct
    let info = server.get_info();
    assert_eq!(info.protocol_version, ProtocolVersion::V_2024_11_05);
    assert_eq!(info.server_info.name, "wasmcp");
    assert!(info.capabilities.tools.is_some());
    
    // Test that we have the expected tools
    // We can't call list_tools without RequestContext, but we can verify get_info
    assert!(info.instructions.unwrap().contains("wasmcp_list"));
}

#[tokio::test]
async fn test_tools_execute_successfully() {
    // Test wasmcp_list
    let list_result = tools::project::wasmcp_list(None).await;
    assert!(list_result.is_ok());
    let result = list_result.unwrap();
    assert!(!result.content.is_empty());
    
    // Test wasmcp_init with missing name (should error)
    let init_result = tools::project::wasmcp_init(Some(serde_json::json!({}))).await;
    assert!(init_result.is_err());
    
    // Test wasmcp_build  
    let build_result = tools::build::wasmcp_build(None).await;
    assert!(build_result.is_ok());
}

#[tokio::test]
async fn test_dependency_checking() {
    // Test the dependency checking system
    let (installed, missing) = deps::check_all_dependencies();
    
    // Should have at least categorized dependencies
    assert!(installed.len() + missing.len() > 0);
    
    // Test that tools with no deps are always available
    assert!(deps::is_tool_available("wasmcp_list"));
    assert!(deps::is_tool_available("wasmcp_check_deps"));
    
    // Test that get_tool_dependencies returns correct deps
    assert_eq!(deps::get_tool_dependencies("wasmcp_list"), Vec::<&str>::new());
    assert_eq!(deps::get_tool_dependencies("wasmcp_build"), vec!["make"]);
    assert_eq!(deps::get_tool_dependencies("wasmcp_serve_spin"), vec!["spin"]);
}

#[tokio::test] 
async fn test_wasmcp_check_deps_tool() {
    // Test the check_deps tool
    let result = tools::deps::wasmcp_check_deps(None).await;
    assert!(result.is_ok());
    
    let tool_result = result.unwrap();
    // Should have structured content
    assert!(tool_result.structured_content.is_some());
    
    // Parse the structured content
    let structured = tool_result.structured_content.unwrap();
    assert!(structured.get("dependencies").is_some());
    assert!(structured.get("tools").is_some());
    assert!(structured.get("missing_dependencies").is_some());
}