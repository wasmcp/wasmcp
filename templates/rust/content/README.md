# {{project-name | kebab_case}}

{{project-description}}

## Structure

This is a Spin application that implements the Model Context Protocol (MCP) using WebAssembly components.

- `handler/` - The Rust implementation of your MCP handler
- `spin.toml` - Spin application manifest
- `Makefile` - Build and development commands

## Development

### Prerequisites

- Rust with `wasm32-wasip1` target
- Spin CLI
- cargo-component (will be installed automatically by Makefile)

### Building

```bash
make build
# or
spin build
```

### Testing

The handler includes comprehensive unit tests for all tools:

```bash
make test
```

Tests cover:
- Tool metadata (name, description)
- Input schema validation
- Successful execution paths
- Error handling for invalid inputs

### Running Locally

```bash
spin up
# or
make up
```

The MCP server will be available at `http://localhost:3000/mcp`

### Example Usage

```bash
# List available tools
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "id": 1
  }'

# Call the echo tool
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "echo",
      "arguments": {
        "message": "Hello, world!"
      }
    },
    "id": 2
  }'
```

## Implementing Your Tools

Edit `handler/src/lib.rs` to add new tools:

1. Define a new zero-sized struct for your tool
2. Implement the `ToolHandler` trait
3. Add your tool to the `create_handler!` macro

Example:
```rust
struct MyTool;

impl ToolHandler for MyTool {
    const NAME: &'static str = "my_tool";
    const DESCRIPTION: &'static str = "Description of my tool";
    
    fn input_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "param": { "type": "string" }
            },
            "required": ["param"]
        })
    }
    
    fn execute(args: serde_json::Value) -> Result<String, String> {
        // Your tool logic here
        Ok("Result".to_string())
    }
}

// Don't forget to add it to the handler
wasmcp::create_handler!(
    tools: [EchoTool, MyTool],
);
```

## Configuration

### Spin Configuration

Edit `spin.toml` to configure:
- Component source and version
- Environment variables
- Build commands

### Cargo Configuration

Edit `handler/Cargo.toml` to:
- Add dependencies
- Configure optimization settings
- Update package metadata