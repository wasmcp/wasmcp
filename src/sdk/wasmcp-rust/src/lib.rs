use serde_json::Value;

// Core traits that define MCP capabilities
// Using traits allows for zero-cost abstractions and compile-time polymorphism

pub trait ToolHandler: Sized {
    const NAME: &'static str;
    const DESCRIPTION: &'static str;
    
    fn input_schema() -> Value;
    fn execute(args: Value) -> Result<String, String>;
}

pub trait ResourceHandler: Sized {
    const URI: &'static str;
    const NAME: &'static str;
    const DESCRIPTION: Option<&'static str> = None;
    const MIME_TYPE: Option<&'static str> = None;
    
    fn read() -> Result<String, String>;
}

pub trait PromptHandler: Sized {
    const NAME: &'static str;
    const DESCRIPTION: Option<&'static str> = None;
    
    type Arguments: PromptArguments;
    
    fn resolve(args: Value) -> Result<Vec<PromptMessage>, String>;
}

// Trait for prompt arguments - allows compile-time validation
pub trait PromptArguments {
    fn schema() -> Vec<PromptArgument>;
}

// Types
#[derive(Clone)]
pub struct PromptArgument {
    pub name: &'static str,
    pub description: Option<&'static str>,
    pub required: bool,
}

#[derive(Clone)]
pub struct PromptMessage {
    pub role: PromptRole,
    pub content: String,
}

#[derive(Clone, Copy)]
pub enum PromptRole {
    User,
    Assistant,
}

// Macro for implementing handlers with zero runtime overhead
#[macro_export]
macro_rules! create_handler {
    (
        $(tools: [$($tool:ty),* $(,)?],)?
        $(resources: [$($resource:ty),* $(,)?],)?
        $(prompts: [$($prompt:ty),* $(,)?])?
    ) => {
        #[allow(warnings)]
        mod bindings;

        use bindings::exports::wasmcp::mcp::handler::Guest;
        use bindings::exports::wasmcp::mcp::handler::{
            Tool as WitTool,
            ResourceInfo as WitResourceInfo,
            ResourceContents as WitResourceContents,
            Prompt as WitPrompt,
            PromptMessage as WitPromptMessage,
            PromptArgument as WitPromptArgument,
            Error as WitError,
            ToolResult,
        };

        struct Component;

        impl Guest for Component {
            fn list_tools() -> Vec<WitTool> {
                vec![
                    $($(
                        WitTool {
                            name: <$tool as $crate::ToolHandler>::NAME.to_string(),
                            description: <$tool as $crate::ToolHandler>::DESCRIPTION.to_string(),
                            input_schema: <$tool as $crate::ToolHandler>::input_schema().to_string(),
                        },
                    )*)?
                ]
            }
            
            fn call_tool(name: String, arguments: String) -> ToolResult {
                let args = match serde_json::from_str(&arguments) {
                    Ok(v) => v,
                    Err(e) => return ToolResult::Error(WitError {
                        code: -32602,
                        message: format!("Invalid JSON arguments: {}", e),
                        data: None,
                    }),
                };
                
                // Compile-time dispatch - no vtables, no dynamic dispatch
                $($(
                    if name == <$tool as $crate::ToolHandler>::NAME {
                        return match <$tool as $crate::ToolHandler>::execute(args) {
                            Ok(result) => ToolResult::Text(result),
                            Err(e) => ToolResult::Error(WitError {
                                code: -32603,
                                message: e,
                                data: None,
                            }),
                        };
                    }
                )*)?
                
                ToolResult::Error(WitError {
                    code: -32601,
                    message: format!("Unknown tool: {}", name),
                    data: None,
                })
            }
            
            fn list_resources() -> Vec<WitResourceInfo> {
                vec![
                    $($(
                        WitResourceInfo {
                            uri: <$resource as $crate::ResourceHandler>::URI.to_string(),
                            name: <$resource as $crate::ResourceHandler>::NAME.to_string(),
                            description: <$resource as $crate::ResourceHandler>::DESCRIPTION.map(|s| s.to_string()),
                            mime_type: <$resource as $crate::ResourceHandler>::MIME_TYPE.map(|s| s.to_string()),
                        },
                    )*)?
                ]
            }
            
            fn read_resource(uri: String) -> Result<WitResourceContents, WitError> {
                $($(
                    if uri == <$resource as $crate::ResourceHandler>::URI {
                        return match <$resource as $crate::ResourceHandler>::read() {
                            Ok(contents) => Ok(WitResourceContents {
                                uri: <$resource as $crate::ResourceHandler>::URI.to_string(),
                                mime_type: <$resource as $crate::ResourceHandler>::MIME_TYPE.map(|s| s.to_string()),
                                text: Some(contents),
                                blob: None,
                            }),
                            Err(e) => Err(WitError {
                                code: -32603,
                                message: e,
                                data: None,
                            }),
                        };
                    }
                )*)?
                
                Err(WitError {
                    code: -32601,
                    message: format!("Resource not found: {}", uri),
                    data: None,
                })
            }
            
            fn list_prompts() -> Vec<WitPrompt> {
                vec![
                    $($(
                        WitPrompt {
                            name: <$prompt as $crate::PromptHandler>::NAME.to_string(),
                            description: <$prompt as $crate::PromptHandler>::DESCRIPTION.map(|s| s.to_string()),
                            arguments: <<$prompt as $crate::PromptHandler>::Arguments as $crate::PromptArguments>::schema()
                                .into_iter()
                                .map(|arg| WitPromptArgument {
                                    name: arg.name.to_string(),
                                    description: arg.description.map(|s| s.to_string()),
                                    required: arg.required,
                                })
                                .collect(),
                        },
                    )*)?
                ]
            }
            
            fn get_prompt(name: String, arguments: String) -> Result<Vec<WitPromptMessage>, WitError> {
                let args = if arguments.is_empty() {
                    serde_json::Value::Object(serde_json::Map::new())
                } else {
                    match serde_json::from_str(&arguments) {
                        Ok(v) => v,
                        Err(e) => return Err(WitError {
                            code: -32602,
                            message: format!("Invalid JSON arguments: {}", e),
                            data: None,
                        }),
                    }
                };
                
                $($(
                    if name == <$prompt as $crate::PromptHandler>::NAME {
                        return match <$prompt as $crate::PromptHandler>::resolve(args) {
                            Ok(messages) => Ok(messages.into_iter()
                                .map(|msg| WitPromptMessage {
                                    role: match msg.role {
                                        $crate::PromptRole::User => "user".to_string(),
                                        $crate::PromptRole::Assistant => "assistant".to_string(),
                                    },
                                    content: msg.content,
                                })
                                .collect()),
                            Err(e) => Err(WitError {
                                code: -32603,
                                message: e,
                                data: None,
                            }),
                        };
                    }
                )*)?
                
                Err(WitError {
                    code: -32601,
                    message: format!("Prompt not found: {}", name),
                    data: None,
                })
            }
        }
        
        bindings::export!(Component with_types_in bindings);
    };
}

// Re-export for convenience
pub use serde_json::{json, Value as Json};

// Derive macro for easy argument schemas (could be in separate crate)
// Example usage:
// #[derive(PromptArgs)]
// struct GreetingArgs {
//     #[arg(description = "Name to greet")]
//     name: String,
//     #[arg(description = "Use formal greeting", required = false)]
//     formal: Option<bool>,
// }