use serde::Deserialize;
use serde_json::json;

// Simple echo tool for testing
struct EchoTool;

impl wasmcp::ToolHandler for EchoTool {
    const NAME: &'static str = "echo";
    const DESCRIPTION: &'static str = "Echo a message back to the user";
    
    fn input_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "message": { 
                    "type": "string", 
                    "description": "Message to echo back" 
                }
            },
            "required": ["message"]
        })
    }
    
    fn execute(args: serde_json::Value) -> Result<String, String> {
        let message = args["message"]
            .as_str()
            .ok_or("Missing message field")?;
        
        Ok(format!("Echo: {}", message))
    }
}

// Generate the MCP handler implementation
#[cfg(target_arch = "wasm32")]
wasmcp::create_handler!(
    tools: [EchoTool],
);