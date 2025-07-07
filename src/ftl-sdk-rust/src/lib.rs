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

// Handler creation helpers
// 
// Note: To create a component, you'll need to:
// 1. Use cargo-component to set up your project
// 2. Generate bindings with `cargo component bindings`
// 3. Implement the generated Guest trait using these helper types
//
// Example implementation:
//
// ```rust
// mod bindings;
// use bindings::exports::component::mcp::handler::Guest;
// 
// struct Component;
// 
// impl Guest for Component {
//     fn list_tools() -> Vec<Tool> {
//         // Use ftl_sdk types to build your tools
//     }
//     // ... implement other methods
// }
// 
// bindings::export!(Component with_types_in bindings);
// ```

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