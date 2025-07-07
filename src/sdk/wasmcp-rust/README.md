# wasmcp - Rust SDK for MCP WebAssembly Components

A high-performance, zero-cost abstraction SDK for building MCP (Model Context Protocol) handlers in Rust.

## Design Philosophy

This SDK prioritizes:
- **Zero runtime overhead** - All dispatch is compile-time
- **Minimal binary size** - No heap allocations for metadata
- **Type safety** - Leverage Rust's type system
- **Ergonomic API** - Idiomatic Rust patterns

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
wasmcp = "0.1.0"
```

## Usage

Define your MCP features as zero-sized types implementing the appropriate traits:

```rust
use wasmcp::{ToolHandler, ResourceHandler, PromptHandler, json};

// Define a tool
struct HelloTool;

impl ToolHandler for HelloTool {
    const NAME: &'static str = "hello";
    const DESCRIPTION: &'static str = "Says hello to someone";
    
    fn input_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "name": { 
                    "type": "string",
                    "description": "Name to greet"
                }
            },
            "required": ["name"]
        })
    }
    
    fn execute(args: serde_json::Value) -> Result<String, String> {
        let name = args["name"]
            .as_str()
            .ok_or("Missing name field")?;
        Ok(format!("Hello, {}!", name))
    }
}

// Define a resource
struct ConfigResource;

impl ResourceHandler for ConfigResource {
    const URI: &'static str = "config://app";
    const NAME: &'static str = "Application Config";
    const DESCRIPTION: Option<&'static str> = Some("Current configuration");
    const MIME_TYPE: Option<&'static str> = Some("application/json");
    
    fn read() -> Result<String, String> {
        Ok(r#"{"version": "1.0.0"}"#.to_string())
    }
}

// Generate the handler
wasmcp::create_handler!(
    tools: [HelloTool],
    resources: [ConfigResource],
);
```

## Performance

The SDK uses several techniques for optimal WASM performance:

1. **Zero-sized types** - Tools, resources, and prompts are just type markers
2. **Const evaluation** - Metadata is stored as `&'static str`
3. **Compile-time dispatch** - No vtables or dynamic dispatch
4. **Monomorphization** - Each handler is specialized at compile time

This results in:
- Smaller WASM binaries
- Faster execution
- No runtime registration overhead
- Dead code elimination of unused features

## Prompts with Arguments

For prompts that need arguments, implement the `PromptArguments` trait:

```rust
use wasmcp::{PromptHandler, PromptArguments, PromptArgument, PromptMessage, PromptRole};

struct GreetingPrompt;

struct GreetingArgs;
impl PromptArguments for GreetingArgs {
    fn schema() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "name",
                description: Some("Name to greet"),
                required: true,
            },
            PromptArgument {
                name: "formal",
                description: Some("Use formal greeting"),
                required: false,
            }
        ]
    }
}

impl PromptHandler for GreetingPrompt {
    const NAME: &'static str = "greeting";
    const DESCRIPTION: Option<&'static str> = Some("Generate a personalized greeting");
    
    type Arguments = GreetingArgs;
    
    fn resolve(args: serde_json::Value) -> Result<Vec<PromptMessage>, String> {
        let name = args["name"].as_str().unwrap_or("Friend");
        let formal = args["formal"].as_bool().unwrap_or(false);
        
        let greeting = if formal {
            format!("Good day, {}. How may I assist you?", name)
        } else {
            format!("Hey {}! What's up?", name)
        };
        
        Ok(vec![
            PromptMessage {
                role: PromptRole::Assistant,
                content: greeting,
            }
        ])
    }
}
```

## Building Components

1. Create a new component project:
   ```bash
   cargo component new my-handler --lib
   ```

2. Add the WIT files to your project

3. Build:
   ```bash
   cargo component build --release
   ```

The resulting WASM component will be optimized for size and performance.