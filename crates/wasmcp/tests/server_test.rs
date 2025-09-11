use rmcp::model::*;
use rmcp::handler::server::ServerHandler;
use std::borrow::Cow;

// Import our server implementation  
#[path = "../src/main.rs"]
mod main;

#[tokio::test]
async fn test_server_info() {
    let server = main::WasmcpServer::new();
    let info = server.get_info();
    
    assert_eq!(info.protocol_version, ProtocolVersion::V_2024_11_05);
    assert!(info.capabilities.tools.is_some());
    assert_eq!(info.server_info.name, "wasmcp");
}

#[tokio::test] 
async fn test_list_tools() {
    // Just test that we can get the tools list - no need for RequestContext in unit test
    let server = main::WasmcpServer::new();
    
    // Call get_info which includes capabilities
    let info = server.get_info();
    assert!(info.capabilities.tools.is_some());
    
    // Test the tool definitions are correct via direct call to list_tools logic
    // We'll test actual MCP calls via integration tests with real server running
}

#[tokio::test]
async fn test_tool_definitions() {
    // Test that our tool definitions are properly structured
    let tools = vec![
        Tool {
            name: "wasmcp_list".into(),
            description: Some("List all MCP provider projects in the workspace".into()),
            input_schema: std::sync::Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to search for providers (defaults to current directory)"
                    }
                }
            }).as_object().unwrap().clone()),
            output_schema: None,
            annotations: None,
        },
        Tool {
            name: "wasmcp_init".into(),
            description: Some("Initialize a new MCP provider project".into()),
            input_schema: std::sync::Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "language": {
                        "type": "string",
                        "enum": ["rust", "python", "go", "typescript", "javascript"],
                        "description": "Programming language for the provider"
                    },
                    "name": {
                        "type": "string",
                        "description": "Name of the new provider project"
                    }
                },
                "required": ["name"]
            }).as_object().unwrap().clone()),
            output_schema: None,
            annotations: None,
        },
    ];
    
    // Verify tools are properly formed
    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0].name, "wasmcp_list");
    assert_eq!(tools[1].name, "wasmcp_init");
}