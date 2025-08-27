// Optional thin helpers for MCP protocol
// These are convenience functions that reduce boilerplate without hiding the protocol

use crate::bindings::fastertools::mcp::{
    tools::{Tool, ToolResult},
    types::{BaseMetadata, ContentBlock, ErrorCode, McpError, TextContent},
};
use serde_json::Value;

/// Helper for creating MCP errors with common error codes
pub struct Error;

impl Error {
    pub fn invalid_params(message: impl Into<String>) -> McpError {
        McpError {
            code: ErrorCode::InvalidParams,
            message: message.into(),
            data: None,
        }
    }

    pub fn tool_not_found(name: impl Into<String>) -> McpError {
        McpError {
            code: ErrorCode::ToolNotFound,
            message: format!("Unknown tool: {}", name.into()),
            data: None,
        }
    }

    pub fn internal(message: impl Into<String>) -> McpError {
        McpError {
            code: ErrorCode::InternalError,
            message: message.into(),
            data: None,
        }
    }
}

/// Helper for creating tool definitions with less boilerplate
pub struct ToolBuilder {
    name: String,
    description: String,
    input_schema: serde_json::Map<String, Value>,
    required: Vec<String>,
}

impl ToolBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            input_schema: serde_json::Map::new(),
            required: Vec::new(),
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn param(mut self, name: impl Into<String>, type_: impl Into<String>) -> Self {
        let name = name.into();
        let mut properties = if let Some(Value::Object(props)) = self.input_schema.get("properties") {
            props.clone()
        } else {
            serde_json::Map::new()
        };
        
        let mut param = serde_json::Map::new();
        param.insert("type".to_string(), Value::String(type_.into()));
        properties.insert(name, Value::Object(param));
        
        self.input_schema.insert("properties".to_string(), Value::Object(properties));
        self
    }

    pub fn required(mut self, name: impl Into<String>) -> Self {
        self.required.push(name.into());
        self
    }

    pub fn build(mut self) -> Tool {
        self.input_schema.insert("type".to_string(), Value::String("object".to_string()));
        
        if !self.required.is_empty() {
            self.input_schema.insert(
                "required".to_string(),
                Value::Array(self.required.into_iter().map(Value::String).collect()),
            );
        }

        Tool {
            base: BaseMetadata {
                name: self.name.clone(),
                title: Some(self.name),
            },
            description: Some(self.description),
            input_schema: serde_json::to_string(&Value::Object(self.input_schema))
                .unwrap_or_else(|_| "{}".to_string()),
            output_schema: None,
            annotations: None,
            meta: None,
        }
    }
}

/// Helper for creating tool results with less boilerplate
pub struct ResultBuilder {
    content: Vec<ContentBlock>,
    is_error: bool,
}

impl ResultBuilder {
    pub fn success() -> Self {
        Self {
            content: Vec::new(),
            is_error: false,
        }
    }

    pub fn error() -> Self {
        Self {
            content: Vec::new(),
            is_error: true,
        }
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.content.push(ContentBlock::Text(TextContent {
            text: text.into(),
            annotations: None,
            meta: None,
        }));
        self
    }

    pub fn build(self) -> ToolResult {
        ToolResult {
            content: self.content,
            is_error: Some(self.is_error),
            structured_content: None,
            meta: None,
        }
    }
}

/// Helper for parsing tool arguments
pub fn parse_args(args: &Option<String>) -> Result<Value, McpError> {
    if let Some(args_str) = args {
        serde_json::from_str(args_str)
            .map_err(|e| Error::invalid_params(format!("Invalid arguments: {}", e)))
    } else {
        Ok(Value::Object(serde_json::Map::new()))
    }
}

/// Helper to get a required string field from parsed arguments
pub fn get_required_string<'a>(args: &'a Value, field: &str) -> Result<&'a str, McpError> {
    args.get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::invalid_params(format!("Missing required field: {}", field)))
}

/// Helper to get an optional string field from parsed arguments
pub fn get_optional_string<'a>(args: &'a Value, field: &str) -> Option<&'a str> {
    args.get(field).and_then(|v| v.as_str())
}