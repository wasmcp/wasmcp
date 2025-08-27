# MCP WIT Definitions

WebAssembly Interface Type (WIT) definitions for the Model Context Protocol (MCP).

## Package

`fastertools:mcp@0.1.1` - Complete MCP protocol definition as a single, cohesive package.

## Structure

```
wit/
├── types.wit          # Core types (JSON, errors, metadata)
├── session.wit        # Session lifecycle and initialization
├── notifications.wit  # Event notification system
├── tools.wit          # Tool discovery and execution
├── resources.wit      # Resource reading and templates
├── prompts.wit        # Prompt templates
├── sampling.wit       # LLM sampling (client capability)
├── elicitation.wit    # User input collection (client capability)
├── roots.wit          # File system access (client capability)
├── completion.wit     # Autocompletion (client capability)
├── handler.wit        # Handler interfaces for implementations
└── world.wit          # Standard world definitions
```

## Architecture

MCP is a **bidirectional protocol** enabling rich interactions:

```
┌─────────────────────────────────────────────────┐
│                   MCP Client                     │
│  (IDE, Editor, Application)                      │
│                                                   │
│  Exports: sampling, elicitation, roots,          │
│           completion, notifications              │
│                                                   │
│  Imports: tools, resources, prompts              │
└─────────────────────────────────────────────────┘
                        ↕ 
            Transport (HTTP/WebSocket)
                        ↕
┌─────────────────────────────────────────────────┐
│                   MCP Server                     │
│  (Transport Bridge Component)                    │
│                                                   │
│  Imports: All handler interfaces                 │
└─────────────────────────────────────────────────┘
                        ↕
┌─────────────────────────────────────────────────┐
│                  MCP Handler                     │
│  (Your Implementation)                           │
│                                                   │
│  Exports: core, tool-handler, resource-handler,  │
│           prompt-handler, etc.                   │
│                                                   │
│  Imports: sampling, elicitation, roots,          │
│           completion (to use client capabilities)│
└─────────────────────────────────────────────────┘
```

## Worlds

### `mcp-handler`
Complete MCP implementation with all capabilities. Use this when building a full-featured MCP service.

### `mcp-tool-handler`  
Minimal tool-only implementation. Perfect for simple cases where you just want to expose functions as tools.

### `mcp-client`
MCP client implementation. Use this when building editors, IDEs, or applications that consume MCP services.

### `mcp-server`
Transport bridge component. Use this when building HTTP, WebSocket, or other transport layers.

### `mcp-test`
Testing world with all imports and exports. Use for validation and testing.

## Usage Examples

### Simple Tool Provider

```wit
// weather-tools/wit/world.wit
package mycompany:weather@1.0.0;
use fastertools:mcp/mcp-tool-handler@0.1.1;
```

```rust
// weather-tools/src/lib.rs
impl tool_handler::Host for WeatherTools {
    fn handle_list_tools(_req: ListToolsRequest) -> Result<ListToolsResponse> {
        Ok(ListToolsResponse {
            tools: vec![weather_tool()],
            next_cursor: None,
        })
    }
    
    fn handle_call_tool(req: CallToolRequest) -> Result<ToolResult> {
        // Implementation
    }
}
```

### Full Handler with Client Capabilities

```wit
// assistant/wit/world.wit
package mycompany:assistant@1.0.0;
use fastertools:mcp/mcp-handler@0.1.1;
```

```rust
// assistant/src/lib.rs
// Can request LLM sampling from client
let response = sampling::create_message(CreateMessageRequest {
    messages: vec![/* ... */],
    model_preferences: Some(ModelPreferences {
        speed_priority: Some(0.8),
        ..Default::default()
    }),
    // ...
})?;
```

## Type System

### JSON Values
```wit
variant json-value {
    null,
    boolean(bool),
    integer(s64),
    number(f64),
    str(string),
    array(string),    // JSON-encoded
    object(string),   // JSON-encoded
}
```

### JSON Schema
```wit
record json-schema {
    schema-type: option<string>,
    properties: option<string>,
    required: option<list<string>>,
    description: option<string>,
    additional: option<string>,
}
```

### Error Handling
```wit
variant error-code {
    // JSON-RPC errors
    parse-error,
    invalid-request,
    method-not-found,
    invalid-params,
    internal-error,
    
    // MCP-specific
    resource-not-found,
    tool-not-found,
    prompt-not-found,
    unauthorized,
    rate-limited,
    timeout,
    cancelled,
    custom-code(s32),
}
```

## Protocol Coverage

~95% of the MCP specification is implemented:

- ✅ **Core Protocol**: Session management, errors, notifications
- ✅ **Server → Client**: Tools, resources, prompts
- ✅ **Client → Server**: LLM sampling, elicitation, roots, completion
- ✅ **Advanced Features**: Progress tracking, URI templates, resource subscriptions
- ✅ **Type Safety**: Structured types instead of strings

## Compatibility

- **WIT Version**: Component Model Preview 2
- **MCP Protocol**: 2024 Specification
- **WebAssembly**: WASI Preview 2
- **Languages**: Any with component model support

## Development

### Validate WIT Files
```bash
wasm-tools component wit wit/ --json > /dev/null && echo "✓ Valid"
```

### Generate Bindings
```bash
# Rust
wit-bindgen rust wit/ --out-dir src/bindings

# Go  
wit-bindgen-go generate wit/ bindings/
```

### Sync to SDKs
```bash
make sync-wit  # Copies WIT files to all SDKs
```

## Migration from Earlier Versions

### Breaking Changes in 0.1.1
- Progress fields changed from `u32` to `f64`
- JSON value is now a variant instead of string
- Added new handler interfaces for client capabilities

See `WIT_UPDATE_COMPLETE.md` for detailed migration guide.