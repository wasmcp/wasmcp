//! Request context component for the Model Context Protocol (MCP)
//!
//! This component provides request parsing and context management for MCP handlers.
//! It is responsible for:
//! - Parsing JSON-RPC requests from input streams
//! - Validating request structure and extracting typed components
//! - Managing request-scoped context for middleware
//! - Providing lazy evaluation of method and parameter variants
//!
//! ## Request Lifecycle
//!
//! 1. Input stream is buffered and parsed as JSON-RPC
//! 2. Request resource is created with internal JSON representation
//! 3. Methods like `id()`, `method()`, and `params()` parse on-demand
//! 4. Middleware can attach context via `get()`/`set()` operations
//! 5. `to_json()` consumes the resource to prevent memory duplication
//!
//! ## Context Management
//!
//! Each request maintains an in-memory context map for middleware to attach
//! metadata during request processing. This context is scoped to the request
//! lifetime and is not persisted across requests.

// Generated code - not formatted or linted
#[rustfmt::skip]
#[allow(clippy::all)]
#[allow(dead_code)]
#[allow(unused_imports)]
#[allow(non_snake_case)]
mod bindings;

use bindings::exports::wasmcp::mcp::request::{
    Arguments, CompletionArgument, CompletionContext, CompletionParams, CompletionPromptReference,
    CompletionRef, Feature, Guest, GuestRequest, InitializeParams, Params,
};
use bindings::wasi::io::streams::{InputStream, StreamError};
use bindings::wasmcp::mcp::error::{Error as McpError, ErrorCode};
use bindings::wasmcp::mcp::types::{
    ClientCapabilities, Id, Implementation, ProtocolVersion, ServerCapabilities,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::RwLock;

struct Component;

struct Request {
    parsed: Value,
    context: RwLock<HashMap<String, Vec<u8>>>,
    capabilities: RwLock<Option<ServerCapabilities>>,
}

// JSON-RPC request structure for validation
#[derive(Deserialize, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

impl GuestRequest for Request {
    fn from_http_stream(
        input: &InputStream,
    ) -> Result<bindings::exports::wasmcp::mcp::request::Request, StreamError> {
        // Read entire HTTP request body until EOF.
        // HTTP bodies are fully buffered, so use non-blocking read().
        // Read in 64KB chunks until we get a short read (indicating end of body).

        let mut buffer = Vec::new();
        let chunk_size = 65536;

        loop {
            match input.read(chunk_size) {
                Ok(chunk) => {
                    let was_full_read = chunk.len() == chunk_size as usize;
                    buffer.extend_from_slice(&chunk);

                    // If we read less than requested, we've reached the end of the HTTP body
                    if !was_full_read {
                        break;
                    }
                }
                Err(e) => return Err(e),
            }
        }

        // Parse the complete request body as JSON-RPC
        Self::parse_message(&buffer)
    }

    fn from_stdio_stream(
        input: &InputStream,
    ) -> Result<bindings::exports::wasmcp::mcp::request::Request, StreamError> {
        // Read newline-delimited JSON-RPC message from stdio.
        // Stdio transport provides newline-delimited messages (\n or \r\n).
        // Reads until newline found, supports persistent connections with multiple messages.

        let mut buffer = Vec::new();
        let chunk_size = 65536;

        loop {
            match input.blocking_read(chunk_size) {
                Ok(chunk) => {
                    if chunk.is_empty() {
                        // EOF without newline - unexpected for stdio
                        return Err(StreamError::Closed);
                    }

                    buffer.extend_from_slice(&chunk);

                    // Check for newline delimiter
                    if let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                        // Extract message up to (not including) newline
                        let mut message_bytes = buffer[..newline_pos].to_vec();

                        // Strip trailing \r if present (CRLF line endings)
                        if message_bytes.last() == Some(&b'\r') {
                            message_bytes.pop();
                        }

                        // Parse and return the complete message
                        return Self::parse_message(&message_bytes);
                    }

                    // No newline yet - continue reading
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn id(&self) -> Id {
        self.parsed
            .get("id")
            .and_then(|id| {
                if let Some(num) = id.as_i64() {
                    Some(Id::Number(num))
                } else {
                    id.as_str().map(|s| Id::String(s.to_string()))
                }
            })
            .unwrap_or(Id::String(String::new()))
    }

    fn feature(&self) -> Feature {
        let method_str = self
            .parsed
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("");

        match method_str {
            "initialize" => Feature::Initialize,
            "tools/list" | "tools/call" => Feature::Tools,
            "resources/list" | "resources/read" | "resources/templates/list" => Feature::Resources,
            "prompts/list" | "prompts/get" => Feature::Prompts,
            "completion/complete" => Feature::Completion,
            _ => Feature::Initialize, // Default to initialize for unknown methods
        }
    }

    fn params(&self) -> Result<Params, McpError> {
        let method_str = self
            .parsed
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("");

        let params_value = self.parsed.get("params");

        match method_str {
            "initialize" => {
                if let Some(params) = params_value {
                    match parse_initialize_params(params) {
                        Ok(init_params) => Ok(Params::Initialize(init_params)),
                        Err(e) => Err(McpError {
                            code: ErrorCode::InvalidParams,
                            message: format!("Invalid initialize params: {}", e),
                            data: None,
                        }),
                    }
                } else {
                    Err(McpError {
                        code: ErrorCode::InvalidParams,
                        message: "Missing params for initialize method".to_string(),
                        data: None,
                    })
                }
            }
            "tools/list" => {
                let cursor = params_value
                    .and_then(|p| p.get("cursor"))
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string());
                Ok(Params::ToolsList(cursor.unwrap_or_default()))
            }
            "tools/call" => {
                if let Some(params) = params_value {
                    let name = params
                        .get("name")
                        .and_then(|n| n.as_str())
                        .ok_or_else(|| McpError {
                            code: ErrorCode::InvalidParams,
                            message: "Missing 'name' field in tools/call params".to_string(),
                            data: None,
                        })?
                        .to_string();
                    let arguments = params.get("arguments").map(|a| a.to_string());

                    Ok(Params::ToolsCall(Arguments { name, arguments }))
                } else {
                    Err(McpError {
                        code: ErrorCode::InvalidParams,
                        message: "Missing params for tools/call method".to_string(),
                        data: None,
                    })
                }
            }
            "resources/list" => {
                let cursor = params_value
                    .and_then(|p| p.get("cursor"))
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string());
                Ok(Params::ResourcesList(cursor.unwrap_or_default()))
            }
            "resources/read" => {
                let uri = params_value
                    .and_then(|p| p.get("uri"))
                    .and_then(|u| u.as_str())
                    .ok_or_else(|| McpError {
                        code: ErrorCode::InvalidParams,
                        message: "Missing 'uri' field in resources/read params".to_string(),
                        data: None,
                    })?
                    .to_string();
                Ok(Params::ResourcesRead(uri))
            }
            "resources/templates/list" => {
                let cursor = params_value
                    .and_then(|p| p.get("cursor"))
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string());
                Ok(Params::ResourcesTemplatesList(cursor.unwrap_or_default()))
            }
            "prompts/list" => {
                let cursor = params_value
                    .and_then(|p| p.get("cursor"))
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string());
                Ok(Params::PromptsList(cursor.unwrap_or_default()))
            }
            "prompts/get" => {
                if let Some(params) = params_value {
                    let name = params
                        .get("name")
                        .and_then(|n| n.as_str())
                        .ok_or_else(|| McpError {
                            code: ErrorCode::InvalidParams,
                            message: "Missing 'name' field in prompts/get params".to_string(),
                            data: None,
                        })?
                        .to_string();
                    let arguments = params.get("arguments").map(|a| a.to_string());

                    Ok(Params::PromptsGet(Arguments { name, arguments }))
                } else {
                    Err(McpError {
                        code: ErrorCode::InvalidParams,
                        message: "Missing params for prompts/get method".to_string(),
                        data: None,
                    })
                }
            }
            "completion/complete" => {
                if let Some(params) = params_value {
                    match parse_completion_params(params) {
                        Ok(comp_params) => Ok(Params::CompletionComplete(comp_params)),
                        Err(e) => Err(McpError {
                            code: ErrorCode::InvalidParams,
                            message: format!("Invalid completion params: {}", e),
                            data: None,
                        }),
                    }
                } else {
                    Err(McpError {
                        code: ErrorCode::InvalidParams,
                        message: "Missing params for completion/complete method".to_string(),
                        data: None,
                    })
                }
            }
            _ => Err(McpError {
                code: ErrorCode::MethodNotFound,
                message: format!("Unknown method: {}", method_str),
                data: None,
            }),
        }
    }

    fn get(&self, key: String) -> Result<Option<Vec<u8>>, ()> {
        let context = self.context.read().map_err(|_| ())?;
        Ok(context.get(&key).cloned())
    }

    fn set(&self, key: String, value: Vec<u8>) -> Result<(), ()> {
        let mut context = self.context.write().map_err(|_| ())?;
        context.insert(key, value);
        Ok(())
    }

    fn needs(&self, capabilities: ServerCapabilities) -> bool {
        let feature = self.feature();

        match feature {
            Feature::Initialize => {
                // Register capabilities for initialize requests and forward to next handler
                let _ = self.add_capabilities_internal(capabilities);
                false
            }
            Feature::Tools => capabilities.contains(ServerCapabilities::TOOLS),
            Feature::Resources => capabilities.contains(ServerCapabilities::RESOURCES),
            Feature::Prompts => capabilities.contains(ServerCapabilities::PROMPTS),
            Feature::Completion => capabilities.contains(ServerCapabilities::COMPLETIONS),
        }
    }

    fn get_capabilities(&self) -> Result<Option<ServerCapabilities>, ()> {
        let caps = self.capabilities.read().map_err(|_| ())?;
        Ok(*caps)
    }
}

impl Request {
    /// Internal helper to add capabilities to the request context
    fn add_capabilities_internal(&self, capabilities: ServerCapabilities) -> Result<(), ()> {
        let mut caps = self.capabilities.write().map_err(|_| ())?;
        match &mut *caps {
            Some(existing) => {
                // Merge capabilities by OR-ing the flags
                *existing |= capabilities;
            }
            None => {
                *caps = Some(capabilities);
            }
        }
        Ok(())
    }

    /// Parse and validate message bytes as JSON-RPC
    fn parse_message(
        message_bytes: &[u8],
    ) -> Result<bindings::exports::wasmcp::mcp::request::Request, StreamError> {
        // Convert bytes to string
        let json_string =
            String::from_utf8(message_bytes.to_vec()).map_err(|_| StreamError::Closed)?;

        // Parse JSON
        let parsed_value: Value =
            serde_json::from_str(&json_string).map_err(|_| StreamError::Closed)?;

        // Validate JSON-RPC structure
        let _json_rpc: JsonRpcRequest =
            serde_json::from_value(parsed_value.clone()).map_err(|_| StreamError::Closed)?;

        // Create the Request resource
        let request = Request {
            parsed: parsed_value,
            context: RwLock::new(HashMap::new()),
            capabilities: RwLock::new(None),
        };

        Ok(bindings::exports::wasmcp::mcp::request::Request::new(
            request,
        ))
    }
}

/// Parses initialization parameters from JSON
/// Handles client capabilities, protocol version, and client info
fn parse_initialize_params(value: &Value) -> Result<InitializeParams, String> {
    // Parse capabilities
    let capabilities_value = value.get("capabilities");
    let mut capabilities = ClientCapabilities::empty();

    if let Some(caps) = capabilities_value {
        if caps
            .get("elicitation")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            capabilities |= ClientCapabilities::ELICITATION;
        }
        if caps.get("roots").and_then(|v| v.as_bool()).unwrap_or(false) {
            capabilities |= ClientCapabilities::ROOTS;
        }
        if caps
            .get("sampling")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            capabilities |= ClientCapabilities::SAMPLING;
        }
        if caps
            .get("experimental")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            capabilities |= ClientCapabilities::EXPERIMENTAL;
        }
    }

    // Parse client info
    let client_info = value
        .get("clientInfo")
        .map(|ci| Implementation {
            name: ci
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string(),
            title: ci
                .get("title")
                .and_then(|t| t.as_str())
                .map(|s| s.to_string()),
            version: ci
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
        .unwrap_or(Implementation {
            name: String::new(),
            title: None,
            version: String::new(),
        });

    // Parse protocol version
    let protocol_version = value
        .get("protocolVersion")
        .and_then(|pv| pv.as_str())
        .map(|s| match s {
            "2025-06-18" => ProtocolVersion::V20250618,
            "2025-03-26" => ProtocolVersion::V20250326,
            "2024-11-05" => ProtocolVersion::V20241105,
            _ => ProtocolVersion::V20250618,
        })
        .unwrap_or(ProtocolVersion::V20250618);

    Ok(InitializeParams {
        capabilities,
        client_info,
        protocol_version,
    })
}

/// Parses completion parameters from JSON
/// Handles argument, reference (prompt or resource template), and context
fn parse_completion_params(value: &Value) -> Result<CompletionParams, String> {
    // Parse argument
    let argument = value
        .get("argument")
        .map(|a| CompletionArgument {
            name: a
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string(),
            value: a
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
        .unwrap_or(CompletionArgument {
            name: String::new(),
            value: String::new(),
        });

    // Parse ref (either prompt or resource-template)
    let ref_value = value.get("ref");
    let completion_ref = if let Some(rv) = ref_value {
        if let Some(prompt) = rv.get("prompt") {
            CompletionRef::Prompt(CompletionPromptReference {
                name: prompt
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string(),
                title: prompt
                    .get("title")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string()),
            })
        } else if let Some(uri) = rv.get("resourceTemplate").and_then(|rt| rt.as_str()) {
            CompletionRef::ResourceTemplate(uri.to_string())
        } else {
            // Default to empty prompt reference
            CompletionRef::Prompt(CompletionPromptReference {
                name: String::new(),
                title: None,
            })
        }
    } else {
        CompletionRef::Prompt(CompletionPromptReference {
            name: String::new(),
            title: None,
        })
    };

    // Parse context
    let context = value.get("context").map(|c| CompletionContext {
        arguments: c.get("arguments").map(|a| a.to_string()),
    });

    Ok(CompletionParams {
        argument,
        ref_: completion_ref,
        context,
    })
}

impl Guest for Component {
    type Request = Request;
}

bindings::export!(Component with_types_in bindings);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_initialize_request() {
        let json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "elicitation": true,
                    "roots": false,
                    "sampling": true
                },
                "clientInfo": {
                    "name": "TestClient",
                    "version": "1.0.0",
                    "title": "Test Client"
                }
            }
        });

        let parsed = json.as_object().unwrap().get("params").unwrap();
        let params = parse_initialize_params(parsed).unwrap();

        // Check capabilities
        let caps = params.capabilities;
        assert!(caps.contains(bindings::wasmcp::mcp::types::ClientCapabilities::ELICITATION));
        assert!(caps.contains(bindings::wasmcp::mcp::types::ClientCapabilities::SAMPLING));
        assert!(!caps.contains(bindings::wasmcp::mcp::types::ClientCapabilities::ROOTS));
        assert!(!caps.contains(bindings::wasmcp::mcp::types::ClientCapabilities::EXPERIMENTAL));

        // Check client info
        assert_eq!(params.client_info.name, "TestClient");
        assert_eq!(params.client_info.version, "1.0.0");
        assert_eq!(params.client_info.title, Some("Test Client".to_string()));

        // Check protocol version
        assert!(matches!(
            params.protocol_version,
            bindings::wasmcp::mcp::types::ProtocolVersion::V20250618
        ));
    }

    #[test]
    fn test_parse_tools_call_request() {
        let json = json!({
            "jsonrpc": "2.0",
            "id": "call-123",
            "method": "tools/call",
            "params": {
                "name": "calculator",
                "arguments": {
                    "operation": "add",
                    "a": 5,
                    "b": 3
                }
            }
        });

        let request = Request {
            parsed: json.clone(),
            context: RwLock::new(HashMap::new()),
            capabilities: RwLock::new(None),
        };

        // Test feature extraction
        assert!(matches!(request.feature(), Feature::Tools));

        // Test params extraction
        match request.params().unwrap() {
            Params::ToolsCall(args) => {
                assert_eq!(args.name, "calculator");
                assert!(args.arguments.is_some());
                let args_json: serde_json::Value =
                    serde_json::from_str(&args.arguments.unwrap()).unwrap();
                assert_eq!(args_json["operation"], "add");
                assert_eq!(args_json["a"], 5);
                assert_eq!(args_json["b"], 3);
            }
            _ => panic!("Expected ToolsCall params"),
        }

        // Test ID extraction
        match request.id() {
            Id::String(id) => assert_eq!(id, "call-123"),
            _ => panic!("Expected string ID"),
        }
    }

    #[test]
    fn test_parse_resources_read_request() {
        let json = json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "resources/read",
            "params": {
                "uri": "file:///path/to/resource.txt"
            }
        });

        let request = Request {
            parsed: json.clone(),
            context: RwLock::new(HashMap::new()),
            capabilities: RwLock::new(None),
        };

        // Test feature
        assert!(matches!(request.feature(), Feature::Resources));

        // Test params
        match request.params().unwrap() {
            Params::ResourcesRead(uri) => {
                assert_eq!(uri, "file:///path/to/resource.txt");
            }
            _ => panic!("Expected ResourcesRead params"),
        }

        // Test numeric ID
        match request.id() {
            Id::Number(id) => assert_eq!(id, 42),
            _ => panic!("Expected numeric ID"),
        }
    }

    #[test]
    fn test_context_operations() {
        let json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });

        let request = Request {
            parsed: json,
            context: RwLock::new(HashMap::new()),
            capabilities: RwLock::new(None),
        };

        // Test initial get returns None
        assert!(request.get("auth_token".to_string()).unwrap().is_none());

        // Test set and get with text data
        let text_value = "secret123".as_bytes().to_vec();
        request
            .set("auth_token".to_string(), text_value.clone())
            .unwrap();
        assert_eq!(
            request.get("auth_token".to_string()).unwrap(),
            Some(text_value)
        );

        // Test binary data
        let binary_data = vec![1, 2, 3, 4];
        request
            .set("binary_key".to_string(), binary_data.clone())
            .unwrap();
        assert_eq!(
            request.get("binary_key".to_string()).unwrap(),
            Some(binary_data)
        );

        // Test overwrite
        let new_value = "new_value".as_bytes().to_vec();
        request
            .set("auth_token".to_string(), new_value.clone())
            .unwrap();
        assert_eq!(
            request.get("auth_token".to_string()).unwrap(),
            Some(new_value)
        );
    }

    #[test]
    fn test_parse_list_with_cursor() {
        let json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "resources/list",
            "params": {
                "cursor": "next_page_token"
            }
        });

        let request = Request {
            parsed: json,
            context: RwLock::new(HashMap::new()),
            capabilities: RwLock::new(None),
        };

        match request.params().unwrap() {
            Params::ResourcesList(cursor) => {
                assert_eq!(cursor, "next_page_token");
            }
            _ => panic!("Expected ResourcesList params"),
        }
    }

    #[test]
    fn test_parse_prompts_get() {
        let json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "prompts/get",
            "params": {
                "name": "code_review",
                "arguments": {
                    "language": "rust",
                    "style": "detailed"
                }
            }
        });

        let request = Request {
            parsed: json,
            context: RwLock::new(HashMap::new()),
            capabilities: RwLock::new(None),
        };

        match request.params().unwrap() {
            Params::PromptsGet(args) => {
                assert_eq!(args.name, "code_review");
                assert!(args.arguments.is_some());
                let args_json: serde_json::Value =
                    serde_json::from_str(&args.arguments.unwrap()).unwrap();
                assert_eq!(args_json["language"], "rust");
                assert_eq!(args_json["style"], "detailed");
            }
            _ => panic!("Expected PromptsGet params"),
        }
    }

    #[test]
    fn test_parse_completion_request() {
        let json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "completion/complete",
            "params": {
                "argument": {
                    "name": "function_name",
                    "value": "calculate_sum"
                },
                "ref": {
                    "prompt": {
                        "name": "autocomplete",
                        "title": "Function Autocomplete"
                    }
                },
                "context": {
                    "arguments": {
                        "file": "main.rs"
                    }
                }
            }
        });

        let parsed = json.as_object().unwrap().get("params").unwrap();
        let params = parse_completion_params(parsed).unwrap();

        // Check argument
        assert_eq!(params.argument.name, "function_name");
        assert_eq!(params.argument.value, "calculate_sum");

        // Check ref (prompt variant)
        match params.ref_ {
            CompletionRef::Prompt(prompt) => {
                assert_eq!(prompt.name, "autocomplete");
                assert_eq!(prompt.title, Some("Function Autocomplete".to_string()));
            }
            _ => panic!("Expected Prompt variant"),
        }

        // Check context
        assert!(params.context.is_some());
        let context = params.context.unwrap();
        assert!(context.arguments.is_some());
    }

    #[test]
    fn test_parse_completion_with_resource_template() {
        let json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "completion/complete",
            "params": {
                "argument": {
                    "name": "test",
                    "value": "value"
                },
                "ref": {
                    "resourceTemplate": "template:///code/function"
                }
            }
        });

        let parsed = json.as_object().unwrap().get("params").unwrap();
        let params = parse_completion_params(parsed).unwrap();

        match params.ref_ {
            CompletionRef::ResourceTemplate(uri) => {
                assert_eq!(uri, "template:///code/function");
            }
            _ => panic!("Expected ResourceTemplate variant"),
        }
    }

    #[test]
    fn test_missing_params_defaults() {
        let json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list"
        });

        let request = Request {
            parsed: json,
            context: RwLock::new(HashMap::new()),
            capabilities: RwLock::new(None),
        };

        // Should default to empty cursor
        match request.params().unwrap() {
            Params::ToolsList(cursor) => {
                assert_eq!(cursor, "");
            }
            _ => panic!("Expected ToolsList params"),
        }
    }

    #[test]
    fn test_unknown_method_handling() {
        let json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "custom/unknown",
            "params": {}
        });

        let request = Request {
            parsed: json,
            context: RwLock::new(HashMap::new()),
            capabilities: RwLock::new(None),
        };

        // Feature should default to Initialize for unknown methods
        assert!(matches!(request.feature(), Feature::Initialize));

        // Params should return error for unknown method
        match request.params() {
            Err(e) => {
                assert_eq!(e.code, ErrorCode::MethodNotFound);
                assert!(e.message.contains("custom/unknown"));
            }
            Ok(_) => panic!("Expected error for unknown method"),
        }
    }

    #[test]
    fn test_malformed_params_handling() {
        let json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "wrong_field": "value"
            }
        });

        let request = Request {
            parsed: json,
            context: RwLock::new(HashMap::new()),
            capabilities: RwLock::new(None),
        };

        // Should return error for missing required fields
        match request.params() {
            Err(e) => {
                assert_eq!(e.code, ErrorCode::InvalidParams);
                assert!(e.message.contains("name"));
            }
            Ok(_) => panic!("Expected error for missing required fields"),
        }
    }

    #[test]
    fn test_protocol_version_parsing() {
        use bindings::wasmcp::mcp::types::ProtocolVersion;

        // Test 2025-06-18
        let json = json!({ "protocolVersion": "2025-06-18" });
        let params = parse_initialize_params(&json).unwrap();
        assert!(matches!(
            params.protocol_version,
            ProtocolVersion::V20250618
        ));

        // Test 2025-03-26
        let json = json!({ "protocolVersion": "2025-03-26" });
        let params = parse_initialize_params(&json).unwrap();
        assert!(matches!(
            params.protocol_version,
            ProtocolVersion::V20250326
        ));

        // Test 2024-11-05
        let json = json!({ "protocolVersion": "2024-11-05" });
        let params = parse_initialize_params(&json).unwrap();
        assert!(matches!(
            params.protocol_version,
            ProtocolVersion::V20241105
        ));

        // Test unknown version defaults to latest
        let json = json!({ "protocolVersion": "unknown" });
        let params = parse_initialize_params(&json).unwrap();
        assert!(matches!(
            params.protocol_version,
            ProtocolVersion::V20250618
        ));
    }
}
