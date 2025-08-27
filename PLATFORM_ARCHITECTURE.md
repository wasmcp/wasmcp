# WASMCP Platform Architecture: Compositional Null Component Pattern

## Executive Summary

This document describes a novel platform architecture for WebAssembly Component Model MCP implementations that enables maximum flexibility through composition. The architecture separates interface definitions, component implementations, and composition logic, allowing any combination of handlers and servers to work together through the use of null components and WAC-based composition.

## Core Concepts

### 1. Interface Registry Package

The `fastertools:mcp` package published to the registry contains **ONLY interface definitions** - no world definitions, no composition opinions:

```wit
// types.wit - Common type definitions
interface types {
    record mcp-error { ... }
    variant content-block { ... }
    // ... other shared types
}

// handler.wit - Handler interfaces
interface core {
    handle-initialize: func(...) -> result<...>;
    handle-ping: func() -> result<...>;
}

interface tool-handler {
    handle-list-tools: func(...) -> result<...>;
    handle-call-tool: func(...) -> result<...>;
}

interface resource-handler {
    handle-list-resources: func(...) -> result<...>;
    handle-read-resource: func(...) -> result<...>;
}

interface prompt-handler {
    handle-list-prompts: func(...) -> result<...>;
    handle-get-prompt: func(...) -> result<...>;
}
```

**Key principle**: The registry package defines contracts, not composition topology.

### 2. Entity-Owned World Definitions

Each component (handler or server) defines its own world, declaring what it provides or requires:

#### Handler Worlds

```wit
// weather-handler/wit/world.wit
package example:weather@1.0.0;

world weather-tools {
    // This handler only provides tools
    export fastertools:mcp/core@0.1.1;
    export fastertools:mcp/tool-handler@0.1.1;
    // No resource or prompt exports - focused implementation
}
```

```wit
// doc-handler/wit/world.wit  
package acme:docs@1.0.0;

world doc-handler {
    // This handler provides resources and prompts
    export fastertools:mcp/core@0.1.1;
    export fastertools:mcp/resource-handler@0.1.1;
    export fastertools:mcp/prompt-handler@0.1.1;
    // No tools - documentation focused
}
```

#### Server Worlds

```wit
// full-server/wit/world.wit
package wasmcp:server@1.0.0;

world full-server {
    // This server requires all capabilities
    import fastertools:mcp/core@0.1.1;
    import fastertools:mcp/tool-handler@0.1.1;
    import fastertools:mcp/resource-handler@0.1.1;
    import fastertools:mcp/prompt-handler@0.1.1;
}
```

```wit
// minimal-server/wit/world.wit
package edge:server@1.0.0;

world edge-server {
    // Minimal server for IoT/edge - tools only
    import fastertools:mcp/core@0.1.1;
    import fastertools:mcp/tool-handler@0.1.1;
}
```

### 3. Null Component Pattern

Null components provide empty/default implementations of interfaces to satisfy composition requirements:

```wit
// null-resources/wit/world.wit
package fastertools:null-resources@1.0.0;

world null-resources {
    export fastertools:mcp/core@0.1.1;
    export fastertools:mcp/resource-handler@0.1.1;
}
```

Implementation:
```rust
// null-resources/src/lib.rs
struct NullResources;

impl resource_handler::Guest for NullResources {
    fn handle_list_resources(_req: ListResourcesRequest) 
        -> Result<ListResourcesResponse, McpError> {
        Ok(ListResourcesResponse {
            resources: vec![],  // Always empty
            next_cursor: None,
        })
    }
    
    fn handle_read_resource(_req: ReadResourceRequest) 
        -> Result<ReadResourceResponse, McpError> {
        Err(McpError {
            code: ErrorCode::MethodNotFound,
            message: "Resources not available".to_string(),
        })
    }
}
```

**Null Component Variants**:
- `fastertools:null-tools` - No tools available
- `fastertools:null-resources` - No resources available  
- `fastertools:null-prompts` - No prompts available
- `debug:null-logging-tools` - Logs all tool calls
- `test:spy-tools` - Records calls for verification
- `test:mock-resources` - Returns mock data

### 4. WAC-Based Composition

The WebAssembly Compositions (WAC) language enables flexible component assembly:

```wac
// app.wac
package myapp:composed@1.0.0;

// Instantiate components
let handler = new example:weather { ... };
let null_resources = new fastertools:null-resources { ... };
let null_prompts = new fastertools:null-prompts { ... };
let server = new wasmcp:server { ... };

// Wire components together
let app = new wasmcp:server {
    "fastertools:mcp/core": handler["fastertools:mcp/core"],
    "fastertools:mcp/tool-handler": handler["fastertools:mcp/tool-handler"],
    "fastertools:mcp/resource-handler": null_resources["fastertools:mcp/resource-handler"],
    "fastertools:mcp/prompt-handler": null_prompts["fastertools:mcp/prompt-handler"],
};

// Export the final HTTP handler
export app["wasi:http/incoming-handler"];
```

Compile with:
```bash
wac compose app.wac --dep example:weather=./handler.wasm \
                    --dep fastertools:null-resources=./null-resources.wasm \
                    --dep fastertools:null-prompts=./null-prompts.wasm \
                    --dep wasmcp:server=./server.wasm \
                    -o final.wasm
```

## Use Cases Enabled

### 1. Progressive Development

Start with minimal functionality and add capabilities over time:

```wac
// v1.wac - MVP with just tools
let mvp = new wasmcp:server {
    core: my_handler.core,
    tool-handler: my_handler["tool-handler"],
    resource-handler: null_resources["resource-handler"],
    prompt-handler: null_prompts["prompt-handler"],
};

// v2.wac - Added resources (no code changes, just composition)
let v2 = new wasmcp:server {
    core: my_handler.core,
    tool-handler: my_handler["tool-handler"],
    resource-handler: my_resources["resource-handler"],  // Real now!
    prompt-handler: null_prompts["prompt-handler"],
};
```

### 2. Testing via Composition

Test components in isolation using test doubles:

```wac
// test.wac
let handler = new my:handler { ... };
let spy = new test:spy-tools { ... };
let mock = new test:mock-resources { ... };

let test_server = new wasmcp:server {
    core: handler.core,
    tool-handler: spy["tool-handler"],        // Spy to verify calls
    resource-handler: mock["resource-handler"], // Mock for predictable data
    prompt-handler: handler["prompt-handler"],
};
```

### 3. A/B Testing

Deploy different implementations side-by-side:

```wac
// version-a.wac
let server_a = new wasmcp:server {
    tool-handler: algorithm_v1["tool-handler"],
    // ...
};

// version-b.wac  
let server_b = new wasmcp:server {
    tool-handler: algorithm_v2["tool-handler"],
    // ...
};
```

### 4. Environment-Specific Builds

Different compositions for different deployment targets:

```wac
// edge.wac - Minimal for IoT
let edge = new edge:server {
    core: handler.core,
    tool-handler: cached_tools["tool-handler"],  // With caching
};

// cloud.wac - Full featured
let cloud = new wasmcp:server {
    core: handler.core,
    tool-handler: handler["tool-handler"],
    resource-handler: handler["resource-handler"],
    prompt-handler: handler["prompt-handler"],
};
```

## Implementation Plan

### Phase 1: Core Infrastructure
1. **Remove world definitions from registry package** ✅
   - Keep only interface definitions
   - Publish as `fastertools:mcp@0.1.1`

2. **Create null components**
   - `fastertools:null-tools`
   - `fastertools:null-resources`  
   - `fastertools:null-prompts`
   - Each ~1KB of WASM

3. **Update existing components**
   - Weather example defines own world ✅
   - Server component keeps its world definition ✅

### Phase 2: Tooling & Templates
1. **Create WAC composition templates**
   ```bash
   templates/
   ├── minimal.wac      # Tools only
   ├── standard.wac     # Tools + resources
   ├── full.wac         # Everything
   └── test.wac         # Testing setup
   ```

2. **Update project templates**
   - Include `wit/world.wit` in handler templates
   - Provide example WAC files
   - Document composition patterns

3. **Build composition helper CLI**
   ```bash
   wasmcp compose --handler my-handler.wasm \
                  --stubs resources,prompts \
                  --server full-server.wasm \
                  --output app.wasm
   ```

### Phase 3: Advanced Components
1. **Testing components**
   - `test:spy-*` - Record calls
   - `test:mock-*` - Return test data
   - `test:stub-*` - Always fail

2. **Debug components**
   - `debug:logging-*` - Log all calls
   - `debug:trace-*` - Detailed tracing
   - `debug:metrics-*` - Performance metrics

3. **Production components**
   - `prod:cached-*` - With caching
   - `prod:rate-limited-*` - Rate limiting
   - `prod:authenticated-*` - Auth wrappers

## Architecture Benefits

### 1. True Composability
- Any handler works with any server (using appropriate nulls)
- Components can be mixed and matched
- No monolithic implementations required

### 2. Type Safety
- WIT ensures interface compatibility
- Composition fails at build time if incompatible
- No runtime surprises

### 3. Testing Excellence
- Zero-modification testing (production code unchanged)
- Component-level test doubles
- Composition-time dependency injection

### 4. Progressive Enhancement
- Start simple, add capabilities over time
- No big-bang migrations
- Gradual adoption path

### 5. Ecosystem Enablement
- Clear contracts (interfaces)
- Freedom to innovate (worlds)
- Reusable components (nulls)

## Comparison with Alternatives

### vs. Single Monolithic World
❌ Every handler must implement everything
❌ Lots of boilerplate stub code
❌ No flexibility in deployment
✅ Our approach: Compose what you need

### vs. Multiple Server Variants
❌ Need 2^n server variants for n capabilities
❌ Confusion about which server to use
❌ Maintenance burden
✅ Our approach: One server + composition

### vs. Runtime Detection
❌ Complex runtime feature detection
❌ Potential runtime failures
❌ Hidden behavior
✅ Our approach: Explicit composition-time wiring

## Key Insights

1. **Null components are first-class platform infrastructure**, not workarounds. Like Unix's `/dev/null`, they're fundamental primitives.

2. **Entity-owned worlds enable innovation**. Each component declares its shape without constraining others.

3. **Composition replaces configuration**. Instead of runtime flags and detection, use explicit composition.

4. **The pattern has precedent**:
   - Unix: `/dev/null`, `true`, `false` commands
   - Hardware: Pull-up resistors, terminators
   - Software: Null Object Pattern (GoF)

## Success Metrics

1. **Adoption**: Number of handlers using focused worlds
2. **Reusability**: Null components used across projects
3. **Testing**: Projects using composition-based testing
4. **Innovation**: Novel server implementations from community

## Conclusion

This architecture represents a paradigm shift in component composition:
- From monolithic to composable
- From runtime to composition-time
- From configuration to declaration
- From rigid to flexible

By combining entity-owned worlds, null components, and WAC composition, we enable a truly composable platform where any valid combination of components can work together seamlessly.

## Next Steps

1. Implement null components (priority 1)
2. Create example compositions (priority 1)
3. Document patterns in README (priority 2)
4. Build helper tooling (priority 3)
5. Create advanced test components (priority 3)

## References

- [WebAssembly Component Model](https://github.com/WebAssembly/component-model)
- [WIT Language](https://component-model.bytecodealliance.org/design/wit.html)
- [WAC Language](https://github.com/bytecodealliance/wac/blob/main/LANGUAGE.md)
- [Null Object Pattern](https://en.wikipedia.org/wiki/Null_object_pattern)

---

*This architecture enables the platform vision: Not by being prescriptive, but by providing the right primitives and letting the ecosystem innovate.*