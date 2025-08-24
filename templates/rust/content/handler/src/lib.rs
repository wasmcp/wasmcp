use wasmcp::{AsyncToolHandler, AsyncResourceHandler, json};
use bindings::wasi::http::outgoing_handler;
use bindings::wasi::http::types::{Method, Scheme, Fields, OutgoingRequest};
use spin_sdk::key_value::Store;

// Define your tools as zero-sized types
struct EchoTool;

impl AsyncToolHandler for EchoTool {
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
    
    async fn execute_async(args: serde_json::Value) -> Result<String, String> {
        let message = args["message"]
            .as_str()
            .ok_or("Missing message field")?;
        
        Ok(format!("Echo: {}", message))
    }
}

// KV Store tool for testing Spin KV functionality
struct KvStoreTool;

impl AsyncToolHandler for KvStoreTool {
    const NAME: &'static str = "kv_store";
    const DESCRIPTION: &'static str = "Get or set values in Spin key-value store";
    
    fn input_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": { 
                    "type": "string",
                    "enum": ["get", "set", "delete", "exists"],
                    "description": "Action to perform"
                },
                "key": {
                    "type": "string",
                    "description": "Key to operate on"
                },
                "value": {
                    "type": "string",
                    "description": "Value to set (required for 'set' action)"
                }
            },
            "required": ["action", "key"]
        })
    }
    
    async fn execute_async(args: serde_json::Value) -> Result<String, String> {
        let action = args["action"].as_str().ok_or("Missing action field")?;
        let key = args["key"].as_str().ok_or("Missing key field")?;
        
        // Open the default store
        let store = Store::open_default()
            .map_err(|e| format!("Failed to open KV store: {:?}", e))?;
        
        match action {
            "get" => {
                match store.get(key) {
                    Ok(Some(value)) => {
                        let value_str = String::from_utf8(value)
                            .unwrap_or_else(|_| "Binary data (not UTF-8)".to_string());
                        Ok(json!({
                            "exists": true,
                            "value": value_str
                        }).to_string())
                    }
                    Ok(None) => {
                        Ok(json!({
                            "exists": false,
                            "value": null
                        }).to_string())
                    }
                    Err(e) => Err(format!("Failed to get value: {:?}", e))
                }
            }
            "set" => {
                let value = args["value"]
                    .as_str()
                    .ok_or("Missing value field for set action")?;
                
                store.set(key, value.as_bytes())
                    .map_err(|e| format!("Failed to set value: {:?}", e))?;
                
                Ok(json!({
                    "success": true,
                    "key": key,
                    "value": value
                }).to_string())
            }
            "delete" => {
                store.delete(key)
                    .map_err(|e| format!("Failed to delete key: {:?}", e))?;
                
                Ok(json!({
                    "success": true,
                    "key": key,
                    "deleted": true
                }).to_string())
            }
            "exists" => {
                let exists = store.exists(key)
                    .map_err(|e| format!("Failed to check existence: {:?}", e))?;
                
                Ok(json!({
                    "key": key,
                    "exists": exists
                }).to_string())
            }
            _ => Err(format!("Unknown action: {}", action))
        }
    }
}

// Generate the MCP handler implementation
#[cfg(target_arch = "wasm32")]
wasmcp::create_handler!(
    tools: [EchoTool, KvStoreTool],
);