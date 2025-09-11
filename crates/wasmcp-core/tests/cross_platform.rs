// Cross-platform tests for wasmcp-core
// These tests should pass in both native and WASM environments

use wasmcp_core::protocol::types::{InitializeResponse, ProtocolVersion, ServerCapabilities, Implementation};
use wasmcp_core::protocol::ProtocolAdapter;

#[test]
fn test_protocol_version_conversion() {
    let v1 = ProtocolVersion::V20250326;
    let rmcp_v1 = v1.to_rmcp();
    assert_eq!(rmcp_v1, rmcp::model::ProtocolVersion::V_2025_03_26);

    let v2 = ProtocolVersion::V20250618;
    let rmcp_v2 = v2.to_rmcp();
    assert_eq!(rmcp_v2, rmcp::model::ProtocolVersion::V_2025_06_18);
}

#[test]
fn test_initialize_response_conversion() {
    let response = InitializeResponse {
        protocol_version: ProtocolVersion::V20250618,
        capabilities: ServerCapabilities::default(),
        server_info: Implementation {
            name: "test-server".to_string(),
            version: "1.0.0".to_string(),
        },
        instructions: Some("Test instructions".to_string()),
    };

    let adapter = ProtocolAdapter::new();
    let rmcp_info = adapter.initialize_to_rmcp(response).unwrap();
    
    assert_eq!(rmcp_info.protocol_version, rmcp::model::ProtocolVersion::V_2025_06_18);
    assert_eq!(rmcp_info.server_info.name, "test-server");
    assert_eq!(rmcp_info.server_info.version, "1.0.0");
    assert_eq!(rmcp_info.instructions, Some("Test instructions".to_string()));
}

#[cfg(feature = "tools")]
#[test]
fn test_content_block_conversion() {
    use wasmcp_core::protocol::types::ContentBlock;
    
    let text_block = ContentBlock::Text {
        text: "Hello, world!".to_string(),
    };
    
    let adapter = ProtocolAdapter::new();
    let rmcp_content = adapter.content_block_to_rmcp(text_block);
    
    // Just verify it doesn't panic - we can't easily inspect internal rmcp Content structure
    // The conversion itself succeeding is the test
    let _ = rmcp_content;
}