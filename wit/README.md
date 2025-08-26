# WIT Interfaces

WebAssembly Interface Types defining the MCP component model.

## Files

- `mcp.wit` - MCP handler interface and `mcp-handler` world

## Interface

```wit
interface handler {
  // Tools
  list-tools: func() -> list<tool>
  call-tool: func(name: string, arguments: string) -> tool-result
  
  // Resources  
  list-resources: func() -> list<resource-info>
  read-resource: func(uri: string) -> resource-result
  
  // Prompts
  list-prompts: func() -> list<prompt>
  get-prompt: func(name: string, arguments: string) -> prompt-result
}

world mcp-handler {
  export handler
}
```

## Component Model

```
┌─────────────┐         ┌──────────────┐
│   Gateway   │ imports │   Handler    │
│  Component  │────────►│  Component   │
│(wasmcp-server)│ handler │ (Your Code)  │
└─────────────┘         └──────────────┘
```

Gateway imports the handler interface. Your component exports it.

## Usage

### Rust
The `wasmcp` crate's proc macro embeds this automatically:
```rust
#[mcp_handler(tools(MyTool))]
mod handler {}
```

### TypeScript
The `wasmcp` npm package bundles this:
```typescript
import { createHandler } from 'wasmcp';
```

### Direct Use
```bash
# Generate bindings
wit-bindgen rust wit/mcp.wit

# Build component
wasm-tools component new module.wasm -o component.wasm --adapt wasi_snapshot_preview1.wasm
```

## Versioning

Interface version: `0.1.0`  
Protocol compatibility: MCP 2025-03-26

## License

Apache-2.0