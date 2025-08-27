# MCP WIT Definitions

This directory contains the WebAssembly Interface Types (WIT) definitions for the Model Context Protocol.

## Structure

```
deps/mcp/
├── types.wit        # Core types: errors, content blocks, metadata
├── session.wit      # Session lifecycle and capability negotiation
├── tools.wit        # Tool discovery and execution
├── resources.wit    # Resource management 
├── prompts.wit      # Prompt templates
├── notifications.wit # Event notifications
└── handler.wit      # Handler interface that implementations must provide
```

## Architecture

The MCP protocol is split between a server component that handles HTTP/JSON-RPC and handler implementations that provide the actual functionality:

```
HTTP Request → Server Component → Handler Implementation
                (JSON-RPC)         (WIT Interface)
```

The server component (`wasmcp-server`) handles:
- HTTP request/response
- JSON-RPC 2.0 protocol
- JSON serialization/deserialization
- Error code mapping

Handler implementations only need to implement the `handler` interface functions.

## Using These Interfaces

### In a Rust SDK

```rust
// The SDK will generate bindings from these WIT files
use bindings::mcp::protocol::handler;

impl handler::Host for MyHandler {
    fn handle_list_tools(request: &ListToolsRequest) -> Result<ListToolsResponse> {
        // Your implementation
    }
}
```

### In Other Languages

Any language with component model support can use these WIT definitions to implement MCP servers. The language toolchain will generate appropriate bindings from these files.

## Compatibility

- WIT Version: Based on component model preview2
- MCP Protocol: Draft specification (2024)
- Dependencies: None (self-contained)

## Type Mappings

| MCP Type | WIT Type | Notes |
|----------|----------|-------|
| JSON values | `json-value` (string) | WIT lacks recursive types, so JSON is passed as strings |
| Binary data | `list<u8>` | Used for images, audio content |
| Metadata | `list<tuple<string, string>>` | Extensible key-value pairs |
| Errors | `variant error-code` | Maps to JSON-RPC error codes |

## Synchronization

These WIT files are the source of truth. Run `make sync-wit` from the repository root to sync them to all SDKs and components.