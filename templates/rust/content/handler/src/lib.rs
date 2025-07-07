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
// This macro generates WebAssembly bindings, so it's only compiled for wasm targets
#[cfg(target_arch = "wasm32")]
wasmcp::create_handler!(
    tools: [EchoTool],
);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_echo_tool_metadata() {
        assert_eq!(EchoTool::NAME, "echo");
        assert_eq!(EchoTool::DESCRIPTION, "Echo a message back to the user");
    }
    
    #[test]
    fn test_echo_tool_schema() {
        let schema = EchoTool::input_schema();
        
        // Check that it's an object schema
        assert_eq!(schema["type"], "object");
        
        // Check that message property exists
        assert!(schema["properties"]["message"].is_object());
        assert_eq!(schema["properties"]["message"]["type"], "string");
        
        // Check required fields
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("message")));
    }
    
    #[test]
    fn test_echo_tool_execute_success() {
        let args = json!({
            "message": "Hello, world!"
        });
        
        let result = EchoTool::execute(args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Echo: Hello, world!");
    }
    
    #[test]
    fn test_echo_tool_execute_missing_message() {
        let args = json!({});
        
        let result = EchoTool::execute(args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Missing message field");
    }
    
    #[test]
    fn test_echo_tool_execute_null_message() {
        let args = json!({
            "message": null
        });
        
        let result = EchoTool::execute(args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Missing message field");
    }
    
    #[test]
    fn test_echo_tool_execute_non_string_message() {
        let args = json!({
            "message": 42
        });
        
        let result = EchoTool::execute(args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Missing message field");
    }
    
    #[test]
    fn test_echo_tool_execute_various_messages() {
        let test_cases = vec![
            ("", "Echo: "),
            ("test", "Echo: test"),
            ("Hello, 世界!", "Echo: Hello, 世界!"),
            ("Line 1\nLine 2", "Echo: Line 1\nLine 2"),
            (r#"{"json": "value"}"#, r#"Echo: {"json": "value"}"#),
        ];
        
        for (input, expected) in test_cases {
            let args = json!({ "message": input });
            let result = EchoTool::execute(args);
            assert!(result.is_ok(), "Failed for input: {}", input);
            assert_eq!(result.unwrap(), expected);
        }
    }
}