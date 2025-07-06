use serde_json::Value;

// MCP Feature Types

pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub execute: fn(Value) -> Result<String, String>,
}

pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub read: fn() -> Result<String, String>,
}

impl Resource {
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }
}

pub struct Prompt {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Option<Vec<PromptArgument>>,
    pub resolve: fn(Value) -> Result<Vec<PromptMessage>, String>,
}

impl Prompt {
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_argument(mut self, name: impl Into<String>, description: impl Into<String>, required: bool) -> Self {
        let arg = PromptArgument {
            name: name.into(),
            description: Some(description.into()),
            required: Some(required),
        };
        if let Some(ref mut args) = self.arguments {
            args.push(arg);
        } else {
            self.arguments = Some(vec![arg]);
        }
        self
    }
}

#[derive(Clone)]
pub struct PromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: Option<bool>,
}

#[derive(Clone)]
pub struct PromptMessage {
    pub role: PromptRole,
    pub content: String,
}

#[derive(Clone)]
pub enum PromptRole {
    User,
    Assistant,
}

// Builder functions

pub fn create_tool(
    name: impl Into<String>,
    description: impl Into<String>,
    input_schema: Value,
    execute: fn(Value) -> Result<String, String>,
) -> Tool {
    Tool {
        name: name.into(),
        description: description.into(),
        input_schema,
        execute,
    }
}

pub fn create_resource(
    uri: impl Into<String>,
    name: impl Into<String>,
    read: fn() -> Result<String, String>,
) -> Resource {
    Resource {
        uri: uri.into(),
        name: name.into(),
        description: None,
        mime_type: None,
        read,
    }
}

pub fn create_prompt(
    name: impl Into<String>,
    resolve: fn(Value) -> Result<Vec<PromptMessage>, String>,
) -> Prompt {
    Prompt {
        name: name.into(),
        description: None,
        arguments: None,
        resolve,
    }
}

// Re-export for convenience
pub use serde_json::json;

// Handler creation macro
#[macro_export]
macro_rules! create_handler {
    (
        tools: $tools:expr,
        resources: $resources:expr,
        prompts: $prompts:expr
    ) => {
        wit_bindgen::generate!({
            world: "mcp-handler",
            path: ".wit",
            exports: {
                "component:mcp/handler": McpComponent
            }
        });

        use exports::component::mcp::handler::*;

        struct McpComponent;

        impl Guest for McpComponent {
            fn list_tools() -> Vec<exports::component::mcp::handler::Tool> {
                $tools()
                    .into_iter()
                    .map(|tool| exports::component::mcp::handler::Tool {
                        name: tool.name,
                        description: tool.description,
                        input_schema: tool.input_schema.to_string(),
                    })
                    .collect()
            }
            
            fn call_tool(name: String, arguments: String) -> ToolResult {
                let args = match serde_json::from_str::<serde_json::Value>(&arguments) {
                    Ok(v) => v,
                    Err(e) => return ToolResult::Error(Error {
                        code: -32602,
                        message: format!("Invalid JSON arguments: {}", e),
                        data: None,
                    }),
                };
                
                for tool in $tools() {
                    if tool.name == name {
                        match (tool.execute)(args) {
                            Ok(result) => return ToolResult::Text(result),
                            Err(e) => return ToolResult::Error(Error {
                                code: -32603,
                                message: e,
                                data: None,
                            }),
                        }
                    }
                }
                
                ToolResult::Error(Error {
                    code: -32601,
                    message: format!("Unknown tool: {}", name),
                    data: None,
                })
            }
            
            fn list_resources() -> Vec<ResourceInfo> {
                $resources()
                    .into_iter()
                    .map(|resource| ResourceInfo {
                        uri: resource.uri,
                        name: resource.name,
                        description: resource.description,
                        mime_type: resource.mime_type,
                    })
                    .collect()
            }
            
            fn read_resource(uri: String) -> Result<ResourceContents, Error> {
                for resource in $resources() {
                    if resource.uri == uri {
                        match (resource.read)() {
                            Ok(contents) => return Ok(ResourceContents {
                                uri: resource.uri.clone(),
                                mime_type: resource.mime_type,
                                text: Some(contents),
                                blob: None,
                            }),
                            Err(e) => return Err(Error {
                                code: -32603,
                                message: e,
                                data: None,
                            }),
                        }
                    }
                }
                
                Err(Error {
                    code: -32601,
                    message: format!("Resource not found: {}", uri),
                    data: None,
                })
            }
            
            fn list_prompts() -> Vec<Prompt> {
                $prompts()
                    .into_iter()
                    .map(|prompt| Prompt {
                        name: prompt.name,
                        description: prompt.description,
                        arguments: prompt.arguments.map(|args| {
                            args.into_iter()
                                .map(|arg| PromptArgument {
                                    name: arg.name,
                                    description: arg.description,
                                    required: arg.required.unwrap_or(false),
                                })
                                .collect()
                        }).unwrap_or_default(),
                    })
                    .collect()
            }
            
            fn get_prompt(name: String, arguments: String) -> Result<Vec<PromptMessage>, Error> {
                let args = if arguments.is_empty() {
                    serde_json::Value::Object(serde_json::Map::new())
                } else {
                    match serde_json::from_str::<serde_json::Value>(&arguments) {
                        Ok(v) => v,
                        Err(e) => return Err(Error {
                            code: -32602,
                            message: format!("Invalid JSON arguments: {}", e),
                            data: None,
                        }),
                    }
                };
                
                for prompt in $prompts() {
                    if prompt.name == name {
                        match (prompt.resolve)(args) {
                            Ok(messages) => return Ok(messages.into_iter()
                                .map(|msg| PromptMessage {
                                    role: match msg.role {
                                        $crate::PromptRole::User => "user".to_string(),
                                        $crate::PromptRole::Assistant => "assistant".to_string(),
                                    },
                                    content: msg.content,
                                })
                                .collect()),
                            Err(e) => return Err(Error {
                                code: -32603,
                                message: e,
                                data: None,
                            }),
                        }
                    }
                }
                
                Err(Error {
                    code: -32601,
                    message: format!("Prompt not found: {}", name),
                    data: None,
                })
            }
        }
    };
}

// Macros for easier creation

#[macro_export]
macro_rules! tool {
    ($name:expr, $desc:expr, $schema:expr, $execute:expr) => {
        $crate::create_tool($name, $desc, $schema, $execute)
    };
}

#[macro_export]
macro_rules! resource {
    ($uri:expr, $name:expr, $read:expr) => {
        $crate::create_resource($uri, $name, $read)
    };
}

#[macro_export]
macro_rules! prompt {
    ($name:expr, $resolve:expr) => {
        $crate::create_prompt($name, $resolve)
    };
}