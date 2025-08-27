# Rust Weather Example - Compositional Pattern

This example demonstrates the **compositional pattern** for building MCP handlers in Rust. Instead of implementing all MCP capabilities, handlers focus only on what they need, and composition fills the gaps.

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Weather Handler│     │  Null Resources  │     │  Null Prompts   │
│  (Tools only)   │     │  (Empty impl)    │     │  (Empty impl)   │
└────────┬────────┘     └────────┬─────────┘     └────────┬────────┘
         │                       │                          │
         │ exports:              │ exports:                │ exports:
         │ - core                │ - core                  │ - core
         │ - tool-handler        │ - resource-handler      │ - prompt-handler
         │                       │                          │
         └───────────────────┬───┴──────────────────────────┘
                             │
                    WAC Composition (compose.wac)
                             │
                             ▼
                    ┌─────────────────┐
                    │   MCP Server    │
                    │   (Requires all │
                    │    4 handlers)  │
                    └─────────────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │  Composed WASM  │
                    │  (863KB total)  │
                    └─────────────────┘
```

## Key Benefits

1. **Focused Implementation**: The weather handler only implements tools, not resources or prompts
2. **Clean WIT Definition**: The handler's world only exports what it provides
3. **Type Safety**: Only bindings for implemented interfaces are generated
4. **Composability**: Can be composed with different null components or real implementations
5. **No SDK Required**: Uses raw bindings directly - simple and transparent

## File Structure

```
rust/
├── Cargo.toml          # Standard cargo dependencies
├── wit/
│   └── world.wit       # Exports only tool interfaces
├── src/
│   ├── lib.rs          # Handler implementation (tools only)
│   └── bindings.rs     # Auto-generated from WIT
├── compose.wac         # WAC composition script
├── Makefile            # Build and composition automation
└── server.wasm         # Pre-built MCP server component
```

## The WIT World

```wit
package example:weather@0.1.0;

world weather-tools {
    // This handler only provides tools, not resources or prompts
    export fastertools:mcp/core@0.1.1;
    export fastertools:mcp/tool-handler@0.1.1;
}
```

## The WAC Composition

```wac
package example:weather-app@1.0.0;

// Instantiate components with implicit imports
let weather = new example:weather { ... };
let nullresources = new fastertools:null-resources { ... };
let nullprompts = new fastertools:null-prompts { ... };

// Wire handlers to server
let server = new wasmcp:server {
    "fastertools:mcp/core@0.1.1": weather["fastertools:mcp/core@0.1.1"],
    "fastertools:mcp/tool-handler@0.1.1": weather["fastertools:mcp/tool-handler@0.1.1"],
    "fastertools:mcp/resource-handler@0.1.1": nullresources["fastertools:mcp/resource-handler@0.1.1"],
    "fastertools:mcp/prompt-handler@0.1.1": nullprompts["fastertools:mcp/prompt-handler@0.1.1"],
    ...
};

export server["wasi:http/incoming-handler@0.2.0"];
```

## Building and Running

```bash
# Build everything and compose
make compose

# Run the composed server
make run

# In another terminal, test it
make test-init      # Initialize
make test-tools     # List tools
make test-resources # List resources (empty)
make test-prompts   # List prompts (empty)
make test-echo      # Call echo tool
make test-weather   # Call weather tool
```

## What Happens at Runtime

- **Tools requests** → Handled by weather component
- **Resources requests** → Handled by null-resources (returns empty lists/errors)
- **Prompts requests** → Handled by null-prompts (returns empty lists/errors)
- **Core requests** → Handled by weather component

## Extending This Pattern

To add resources or prompts:

1. **Option A**: Implement them in your handler
   - Update your WIT world to export the additional interfaces
   - Implement the trait in your lib.rs
   - Remove the corresponding null component from composition

2. **Option B**: Create a separate handler
   - Create a new component that exports resource/prompt interfaces
   - Replace the null component with your implementation in compose.wac

3. **Option C**: Use someone else's implementation
   - Find a compatible resource/prompt provider component
   - Wire it in via WAC composition

## Size Comparison

- Weather handler alone: ~114KB
- Null resources: ~106KB  
- Null prompts: ~107KB
- Server: ~400KB
- **Composed total: ~863KB**

Compare to monolithic approach where every handler must implement everything!

## FAQ

**Q: Why not just implement empty methods in the handler?**
A: This approach keeps handlers focused and allows true modularity. Different handlers can be mixed and matched.

**Q: What if I need resources later?**
A: Just implement the resource-handler interface and update the composition - no changes to existing tool code!

**Q: Can I share components between projects?**
A: Yes! Null components, utility handlers, and more can be shared as WASM components.

**Q: Is this production ready?**
A: The component model and WAC are evolving, but this pattern demonstrates the intended usage.