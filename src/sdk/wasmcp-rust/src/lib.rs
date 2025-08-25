//! Rust SDK for building MCP (Model Context Protocol) WebAssembly components
//!
//! This crate provides the core traits and macros for implementing MCP handlers in Rust.

#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc
)]

use serde_json::Value;

/// Trait for implementing MCP tools.
///
/// Tools are functions that can be called by MCP clients to perform specific actions.
pub trait ToolHandler: Sized {
    /// The name of the tool as it will appear to MCP clients.
    const NAME: &'static str;

    /// A human-readable description of what the tool does.
    const DESCRIPTION: &'static str;

    /// Returns the JSON schema describing the tool's input parameters.
    fn input_schema() -> Value;

    /// Executes the tool with the given arguments.
    fn execute(args: Value) -> Result<String, String>;
}

/// Trait for implementing MCP resources.
///
/// Resources are data sources that can be read by MCP clients.
pub trait ResourceHandler: Sized {
    /// The URI that uniquely identifies this resource.
    const URI: &'static str;

    /// The human-readable name of the resource.
    const NAME: &'static str;

    /// Optional description of the resource.
    const DESCRIPTION: Option<&'static str> = None;

    /// Optional MIME type of the resource content.
    const MIME_TYPE: Option<&'static str> = None;

    /// Reads and returns the resource content.
    fn read() -> Result<String, String>;
}

/// Trait for implementing MCP prompts.
///
/// Prompts are templates that can be resolved with arguments to produce messages.
pub trait PromptHandler: Sized {
    /// The name of the prompt as it will appear to MCP clients.
    const NAME: &'static str;

    /// Optional description of what the prompt does.
    const DESCRIPTION: Option<&'static str> = None;

    /// The type that defines the prompt's arguments.
    type Arguments: PromptArguments;

    /// Resolves the prompt with the given arguments to produce messages.
    fn resolve(args: Value) -> Result<Vec<PromptMessage>, String>;
}

/// Async trait for implementing MCP tools.
///
/// Tools are functions that can be called by MCP clients to perform specific actions.
/// This async version allows for non-blocking operations like network requests, file I/O, etc.
pub trait AsyncToolHandler: Sized {
    /// The name of the tool as it will appear to MCP clients.
    const NAME: &'static str;

    /// A human-readable description of what the tool does.
    const DESCRIPTION: &'static str;

    /// Returns the JSON schema describing the tool's input parameters.
    fn input_schema() -> Value;

    /// Executes the tool with the given arguments asynchronously.
    fn execute_async(args: Value) -> impl std::future::Future<Output = Result<String, String>>;
}

/// Async trait for implementing MCP resources.
///
/// Resources are data sources that can be read by MCP clients.
/// This async version allows for non-blocking operations like database queries, file reads, etc.
pub trait AsyncResourceHandler: Sized {
    /// The URI that uniquely identifies this resource.
    const URI: &'static str;

    /// The human-readable name of the resource.
    const NAME: &'static str;

    /// Optional description of the resource.
    const DESCRIPTION: Option<&'static str> = None;

    /// Optional MIME type of the resource content.
    const MIME_TYPE: Option<&'static str> = None;

    /// Reads and returns the resource content asynchronously.
    fn read_async() -> impl std::future::Future<Output = Result<String, String>>;
}

/// Async trait for implementing MCP prompts.
///
/// Prompts are templates that can be resolved with arguments to produce messages.
/// This async version allows for non-blocking operations like external API calls, etc.
pub trait AsyncPromptHandler: Sized {
    /// The name of the prompt as it will appear to MCP clients.
    const NAME: &'static str;

    /// Optional description of what the prompt does.
    const DESCRIPTION: Option<&'static str> = None;

    /// The type that defines the prompt's arguments.
    type Arguments: PromptArguments;

    /// Resolves the prompt with the given arguments to produce messages asynchronously.
    fn resolve_async(args: Value) -> impl std::future::Future<Output = Result<Vec<PromptMessage>, String>>;
}

/// Automatic bridging from async to sync for tools
/// This allows async implementations to work with the sync WIT interface
/// The async runtime is managed by the WASM component host (e.g., Spin)
impl<T: AsyncToolHandler> ToolHandler for T {
    const NAME: &'static str = T::NAME;
    const DESCRIPTION: &'static str = T::DESCRIPTION;

    fn input_schema() -> Value {
        T::input_schema()
    }

    fn execute(args: Value) -> Result<String, String> {
        // Use spin_executor::run which is the WASM-compatible way to block on async operations
        spin_executor::run(T::execute_async(args))
    }
}

/// Automatic bridging from async to sync for resources
/// This allows async implementations to work with the sync WIT interface
/// The async runtime is managed by the WASM component host (e.g., Spin)
impl<T: AsyncResourceHandler> ResourceHandler for T {
    const URI: &'static str = T::URI;
    const NAME: &'static str = T::NAME;
    const DESCRIPTION: Option<&'static str> = T::DESCRIPTION;
    const MIME_TYPE: Option<&'static str> = T::MIME_TYPE;

    fn read() -> Result<String, String> {
        // Use spin_executor::run which is the WASM-compatible way to block on async operations
        spin_executor::run(T::read_async())
    }
}

/// Automatic bridging from async to sync for prompts
/// This allows async implementations to work with the sync WIT interface
/// The async runtime is managed by the WASM component host (e.g., Spin)
impl<T: AsyncPromptHandler> PromptHandler for T {
    const NAME: &'static str = T::NAME;
    const DESCRIPTION: Option<&'static str> = T::DESCRIPTION;
    type Arguments = T::Arguments;

    fn resolve(args: Value) -> Result<Vec<PromptMessage>, String> {
        // Use spin_executor::run which is the WASM-compatible way to block on async operations
        spin_executor::run(T::resolve_async(args))
    }
}

/// Trait for defining prompt arguments.
///
/// This allows compile-time validation of prompt parameters.
pub trait PromptArguments {
    /// Returns the schema defining the prompt's arguments.
    fn schema() -> Vec<PromptArgument>;
}

/// Represents a single argument for a prompt.
#[derive(Clone, Debug)]
pub struct PromptArgument {
    /// The name of the argument.
    pub name: &'static str,
    /// Optional description of what the argument is for.
    pub description: Option<&'static str>,
    /// Whether this argument is required or optional.
    pub required: bool,
}

/// Represents a message in a prompt conversation.
#[derive(Clone, Debug)]
pub struct PromptMessage {
    /// The role of the message sender.
    pub role: PromptRole,
    /// The content of the message.
    pub content: String,
}

/// The role of a participant in a prompt conversation.
#[derive(Clone, Copy, Debug)]
pub enum PromptRole {
    /// A user message.
    User,
    /// An assistant/AI message.
    Assistant,
}

/// Macro for generating MCP handler implementations.
///
/// This macro generates the necessary WebAssembly bindings and handler logic
/// with zero runtime overhead.
///
/// # Example
///
/// ```rust,ignore
/// wasmcp::create_handler!(
///     tools: [EchoTool, CalculatorTool],
///     resources: [ConfigResource],
///     prompts: [GreetingPrompt],
/// );
/// ```
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

/// Re-export of `serde_json`'s `json!` macro for convenience.
pub use serde_json::json;
/// Re-export of `serde_json::Value` as `Json` for convenience.
pub use serde_json::Value as Json;

// Derive macro for easy argument schemas (could be in separate crate)
// Example usage:
// #[derive(PromptArgs)]
// struct GreetingArgs {
//     #[arg(description = "Name to greet")]
//     name: String,
//     #[arg(description = "Use formal greeting", required = false)]
//     formal: Option<bool>,
// }
