use wasmcp::{ToolHandler, json};

// Define your tools as zero-sized types
struct EchoTool;

impl ToolHandler for EchoTool {
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

// Add more tools here...

// Generate the MCP handler implementation
wasmcp::create_handler!(
    tools: [EchoTool],
);