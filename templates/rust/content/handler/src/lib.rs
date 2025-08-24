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

struct HttpRequestTool;

impl AsyncToolHandler for HttpRequestTool {
    const NAME: &'static str = "http_request";
    const DESCRIPTION: &'static str = "Make an HTTP GET request to a URL";
    
    fn input_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": { 
                    "type": "string", 
                    "description": "URL to make request to" 
                },
                "method": {
                    "type": "string",
                    "enum": ["GET", "POST", "PUT", "DELETE"],
                    "default": "GET",
                    "description": "HTTP method"
                }
            },
            "required": ["url"]
        })
    }
    
    async fn execute_async(args: serde_json::Value) -> Result<String, String> {
        let url = args["url"]
            .as_str()
            .ok_or("Missing url field")?;
        
        let method_str = args["method"]
            .as_str()
            .unwrap_or("GET");
        
        // Parse URL to extract components
        let (scheme, host, path) = parse_url(url)?;
        
        // Create headers
        let headers = Fields::new();
        headers.append(&"Host".to_string(), host.as_bytes())
            .map_err(|e| format!("Failed to set Host header: {:?}", e))?;
        headers.append(&"User-Agent".to_string(), b"wasmcp/1.0")
            .map_err(|e| format!("Failed to set User-Agent header: {:?}", e))?;
        
        // Determine HTTP method
        let method = match method_str {
            "GET" => Method::Get,
            "POST" => Method::Post,
            "PUT" => Method::Put,
            "DELETE" => Method::Delete,
            _ => Method::Get,
        };
        
        // Create the request
        let request = OutgoingRequest::new(headers);
        request.set_method(&method)
            .map_err(|e| format!("Failed to set method: {:?}", e))?;
        request.set_scheme(Some(&scheme))
            .map_err(|e| format!("Failed to set scheme: {:?}", e))?;
        request.set_authority(Some(&host))
            .map_err(|e| format!("Failed to set authority: {:?}", e))?;
        request.set_path_with_query(Some(&path))
            .map_err(|e| format!("Failed to set path: {:?}", e))?;
        
        // Send the request
        let future_response = outgoing_handler::handle(request, None)
            .map_err(|e| format!("Request failed: {:?}", e))?;
        
        // Poll the future until we get a response
        let response = loop {
            match future_response.get() {
                Some(result) => break result,
                None => {
                    // Subscribe to the pollable and wait
                    let pollable = future_response.subscribe();
                    pollable.block();
                }
            }
        }.map_err(|_| "Failed to get response".to_string())?
        .map_err(|e| format!("Response error: {:?}", e))?;
        
        // Read response
        let status = response.status();
        
        // Get response body
        let body = response.consume()
            .map_err(|_| "Failed to get response body".to_string())?;
        
        let body_stream = body.stream()
            .map_err(|_| "Failed to get body stream".to_string())?;
        
        let mut body_bytes = Vec::new();
        loop {
            let chunk = body_stream.blocking_read(4096)
                .map_err(|e| format!("Failed to read response: {:?}", e))?;
            
            if chunk.is_empty() {
                break;
            }
            
            body_bytes.extend_from_slice(&chunk);
        }
        
        let body = String::from_utf8(body_bytes)
            .unwrap_or_else(|_| "Binary response (not UTF-8)".to_string());
        
        Ok(json!({
            "status": status,
            "body": body
        }).to_string())
    }
}

fn parse_url(url: &str) -> Result<(Scheme, String, String), String> {
    // Simple URL parsing
    let (scheme, rest) = if url.starts_with("https://") {
        (Scheme::Https, &url[8..])
    } else if url.starts_with("http://") {
        (Scheme::Http, &url[7..])
    } else {
        return Err("URL must start with http:// or https://".to_string());
    };
    
    // Find the path separator
    let (host, path) = if let Some(pos) = rest.find('/') {
        (&rest[..pos], &rest[pos..])
    } else {
        (rest, "/")
    };
    
    Ok((scheme, host.to_string(), path.to_string()))
}

// Define a simple resource handler
struct ConfigResource;

impl AsyncResourceHandler for ConfigResource {
    const URI: &'static str = "config://app-config";
    const NAME: &'static str = "Application Configuration";
    const DESCRIPTION: Option<&'static str> = Some("Application configuration settings");
    const MIME_TYPE: Option<&'static str> = Some("application/json");
    
    async fn read_async() -> Result<String, String> {
        let content = json!({
            "app_name": "MCP Test App",
            "version": "1.0.0",
            "debug": true,
            "features": {
                "http_requests": true,
                "echo": true
            }
        });
        Ok(content.to_string())
    }
}

// Generate the MCP handler implementation
#[cfg(target_arch = "wasm32")]
wasmcp::create_handler!(
    tools: [EchoTool, HttpRequestTool, KvStoreTool],
    resources: [ConfigResource],
);