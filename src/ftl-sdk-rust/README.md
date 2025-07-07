# FTL Rust SDK

SDK for building MCP (Model Context Protocol) handler components in Rust.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
ftl-sdk = "0.2.1"
```

## Usage

This SDK provides types and a macro to help you implement MCP handlers. Here's how to create a component:

### 1. Create a new component project

```bash
cargo component new my-mcp-handler --lib
cd my-mcp-handler
```

### 2. Add the MCP WIT files

Copy the WIT files from the ftl-components repository to your project's `wit` directory, or reference them in your `Cargo.toml`:

```toml
[package.metadata.component.target.dependencies]
"component:mcp" = { path = "../path/to/ftl-components/wit" }
```

### 3. Implement your handler

```rust
use ftl_sdk::{create_handler, json, Tool, Resource, Prompt};

fn get_tools() -> Vec<Tool> {
    vec![
        ftl_sdk::create_tool(
            "hello",
            "Says hello",
            json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                }
            }),
            |args| {
                let name = args["name"].as_str().unwrap_or("World");
                Ok(format!("Hello, {}!", name))
            }
        )
    ]
}

fn get_resources() -> Vec<Resource> {
    vec![]
}

fn get_prompts() -> Vec<Prompt> {
    vec![]
}

create_handler!(
    tools: get_tools,
    resources: get_resources,
    prompts: get_prompts
);
```

### 4. Build your component

```bash
cargo component build --release
```

## Features

The SDK provides:
- `Tool`, `Resource`, and `Prompt` types
- Builder functions: `create_tool()`, `create_resource()`, `create_prompt()`
- The `create_handler!` macro to generate component bindings
- Convenience macros: `tool!`, `resource!`, `prompt!`

## Example

See the [examples](../../examples) directory for complete working examples.