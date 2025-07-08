use wasmcp::{AsyncToolHandler, AsyncResourceHandler, json};
use std::time::Duration;

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
        
        // Simulate async processing
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        Ok(format!("Echo: {}", message))
    }
}

// Example of async HTTP request tool
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
                "timeout_seconds": {
                    "type": "number",
                    "description": "Request timeout in seconds",
                    "default": 30
                }
            },
            "required": ["url"]
        })
    }
    
    async fn execute_async(args: serde_json::Value) -> Result<String, String> {
        let url = args["url"]
            .as_str()
            .ok_or("Missing url field")?;
        
        let timeout = args["timeout_seconds"]
            .as_u64()
            .unwrap_or(30);
        
        // Simulate HTTP request with timeout
        match tokio::time::timeout(
            Duration::from_secs(timeout),
            simulate_http_request(url)
        ).await {
            Ok(result) => result,
            Err(_) => Err(format!("Request timed out after {} seconds", timeout))
        }
    }
}

// Example of async file operations tool
struct FileOperationsTool;

impl AsyncToolHandler for FileOperationsTool {
    const NAME: &'static str = "file_operations";
    const DESCRIPTION: &'static str = "Perform async file operations";
    
    fn input_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["read", "write", "list"],
                    "description": "Operation to perform"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory path"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write (for write operation)"
                }
            },
            "required": ["operation", "path"]
        })
    }
    
    async fn execute_async(args: serde_json::Value) -> Result<String, String> {
        let operation = args["operation"]
            .as_str()
            .ok_or("Missing operation field")?;
        
        let path = args["path"]
            .as_str()
            .ok_or("Missing path field")?;
        
        match operation {
            "read" => simulate_file_read(path).await,
            "write" => {
                let content = args["content"]
                    .as_str()
                    .ok_or("Missing content field for write operation")?;
                simulate_file_write(path, content).await
            },
            "list" => simulate_directory_list(path).await,
            _ => Err(format!("Unknown operation: {}", operation))
        }
    }
}

// Example async resource
struct ConfigResource;

impl AsyncResourceHandler for ConfigResource {
    const URI: &'static str = "config://app-config";
    const NAME: &'static str = "Application Configuration";
    const DESCRIPTION: Option<&'static str> = Some("Dynamic application configuration");
    const MIME_TYPE: Option<&'static str> = Some("application/json");
    
    async fn read_async() -> Result<String, String> {
        // Simulate async config loading
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        Ok(json!({
            "version": "1.0.0",
            "environment": "production",
            "features": {
                "async_processing": true,
                "http_requests": true,
                "file_operations": true
            },
            "limits": {
                "max_timeout": 300,
                "max_file_size": 1048576
            }
        }).to_string())
    }
}

// Helper functions to simulate async operations
async fn simulate_http_request(url: &str) -> Result<String, String> {
    // Simulate network delay
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Simulate different responses based on URL
    match url {
        url if url.starts_with("https://") => {
            Ok(json!({
                "status": "success",
                "url": url,
                "response_time_ms": 100,
                "body": "Simulated HTTP response"
            }).to_string())
        },
        _ => Err("Invalid URL: must start with https://".to_string())
    }
}

async fn simulate_file_read(path: &str) -> Result<String, String> {
    tokio::time::sleep(Duration::from_millis(20)).await;
    
    match path {
        "/etc/config.json" => Ok(json!({"config": "value"}).to_string()),
        "/var/log/app.log" => Ok("2024-01-01 10:00:00 INFO Application started\n2024-01-01 10:00:01 INFO Ready to serve requests".to_string()),
        _ => Err(format!("File not found: {}", path))
    }
}

async fn simulate_file_write(path: &str, content: &str) -> Result<String, String> {
    tokio::time::sleep(Duration::from_millis(30)).await;
    
    if path.starts_with("/tmp/") {
        Ok(format!("Successfully wrote {} bytes to {}", content.len(), path))
    } else {
        Err("Write access denied: can only write to /tmp/".to_string())
    }
}

async fn simulate_directory_list(path: &str) -> Result<String, String> {
    tokio::time::sleep(Duration::from_millis(40)).await;
    
    match path {
        "/tmp" => Ok(json!(["file1.txt", "file2.json", "subdir/"]).to_string()),
        "/etc" => Ok(json!(["config.json", "hosts", "passwd"]).to_string()),
        _ => Err(format!("Directory not found: {}", path))
    }
}

// Add more tools here...

// Generate the MCP handler implementation
// This macro generates WebAssembly bindings, so it's only compiled for wasm targets
#[cfg(target_arch = "wasm32")]
wasmcp::create_handler!(
    tools: [EchoTool, HttpRequestTool, FileOperationsTool],
    resources: [ConfigResource],
);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[tokio::test]
    async fn test_echo_tool_metadata() {
        assert_eq!(EchoTool::NAME, "echo");
        assert_eq!(EchoTool::DESCRIPTION, "Echo a message back to the user");
    }
    
    #[tokio::test]
    async fn test_echo_tool_schema() {
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
    
    #[tokio::test]
    async fn test_echo_tool_execute_success() {
        let args = json!({
            "message": "Hello, world!"
        });
        
        let result = EchoTool::execute_async(args).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Echo: Hello, world!");
    }
    
    #[tokio::test]
    async fn test_echo_tool_execute_missing_message() {
        let args = json!({});
        
        let result = EchoTool::execute_async(args).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Missing message field");
    }
    
    #[tokio::test]
    async fn test_http_request_tool_success() {
        let args = json!({
            "url": "https://api.example.com/data"
        });
        
        let result = HttpRequestTool::execute_async(args).await;
        assert!(result.is_ok());
        
        let response: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(response["status"], "success");
        assert_eq!(response["url"], "https://api.example.com/data");
    }
    
    #[tokio::test]
    async fn test_http_request_tool_invalid_url() {
        let args = json!({
            "url": "http://insecure.com"
        });
        
        let result = HttpRequestTool::execute_async(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must start with https://"));
    }
    
    #[tokio::test]
    async fn test_http_request_tool_timeout() {
        let args = json!({
            "url": "https://slow-api.com",
            "timeout_seconds": 0
        });
        
        let result = HttpRequestTool::execute_async(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("timed out"));
    }
    
    #[tokio::test]
    async fn test_file_operations_tool_read() {
        let args = json!({
            "operation": "read",
            "path": "/etc/config.json"
        });
        
        let result = FileOperationsTool::execute_async(args).await;
        assert!(result.is_ok());
        
        let content: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(content["config"], "value");
    }
    
    #[tokio::test]
    async fn test_file_operations_tool_write() {
        let args = json!({
            "operation": "write",
            "path": "/tmp/test.txt",
            "content": "Hello, world!"
        });
        
        let result = FileOperationsTool::execute_async(args).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Successfully wrote 13 bytes"));
    }
    
    #[tokio::test]
    async fn test_file_operations_tool_write_denied() {
        let args = json!({
            "operation": "write",
            "path": "/etc/passwd",
            "content": "malicious content"
        });
        
        let result = FileOperationsTool::execute_async(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Write access denied"));
    }
    
    #[tokio::test]
    async fn test_file_operations_tool_list() {
        let args = json!({
            "operation": "list",
            "path": "/tmp"
        });
        
        let result = FileOperationsTool::execute_async(args).await;
        assert!(result.is_ok());
        
        let files: Vec<String> = serde_json::from_str(&result.unwrap()).unwrap();
        assert!(files.contains(&"file1.txt".to_string()));
        assert!(files.contains(&"file2.json".to_string()));
    }
    
    #[tokio::test]
    async fn test_config_resource_read() {
        let result = ConfigResource::read_async().await;
        assert!(result.is_ok());
        
        let config: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(config["version"], "1.0.0");
        assert_eq!(config["environment"], "production");
        assert!(config["features"]["async_processing"].as_bool().unwrap());
    }
    
    #[tokio::test]
    async fn test_concurrent_operations() {
        use tokio::time::Instant;
        
        let start = Instant::now();
        
        // Execute multiple operations concurrently
        let tasks = vec![
            EchoTool::execute_async(json!({"message": "test1"})),
            EchoTool::execute_async(json!({"message": "test2"})),
            EchoTool::execute_async(json!({"message": "test3"})),
        ];
        
        let results = futures::future::join_all(tasks).await;
        let duration = start.elapsed();
        
        // All should succeed
        for result in results {
            assert!(result.is_ok());
        }
        
        // Should complete faster than if run sequentially (3 * 10ms = 30ms)
        // Due to concurrent execution, should be closer to 10ms
        assert!(duration.as_millis() < 25);
    }
}