# Feature-Flagged Server Architecture Plan

## Overview

Transform the wasmcp-server component from a monolithic implementation to a feature-flagged, composable architecture that:
1. Eliminates the need for multiple null components during composition
2. Moves capability negotiation entirely to the server
3. Provides pre-built variants for common use cases
4. Enables compile-time optimization and smaller binaries

## Core Design Principles

### 1. Server-Managed Capability Discovery
The server becomes the single source of truth for capabilities:
- Automatically detects what handlers are wired based on compile-time features
- Builds capability responses without delegating to handlers
- Prevents capability mismatches between advertisement and implementation

### 2. Simplified Handler Interface
Handlers no longer need to implement the `core` interface:
- Just implement their specific handler interface (tools, resources, etc.)
- No boilerplate for initialize/ping/shutdown
- Focus purely on domain logic

### 3. Feature-Based Composition
Use Cargo features to build specific server variants:
- Compile only what's needed
- Reduce binary size
- Eliminate runtime overhead

## Feature Flags

### Handler Features
```toml
[features]
default = ["tools"]  # Most common case

# Handler capabilities
tools = []
resources = []
prompts = []
sampling = []
elicitation = []
roots = []
completion = []

# Convenience combinations
basic = ["tools", "resources"]
full = ["tools", "resources", "prompts", "sampling", "elicitation", "roots", "completion"]
```

### Transport Features
```toml
# Transport mechanisms
sse = []           # Server-Sent Events for subscriptions/notifications
websocket = []     # WebSocket transport (future)

# State management
sessions = ["dep:wasi-keyvalue"]  # Stateful sessions with WASI KV
```

### Optional Features
```toml
# Additional capabilities
metrics = ["dep:wasi-observe"]    # Telemetry and metrics
auth = []                          # Authentication support
ratelimit = ["dep:wasi-keyvalue"] # Rate limiting
cache = ["dep:wasi-keyvalue"]     # Response caching
```

## Implementation Strategy

### Phase 1: Core Refactoring

#### 1.1 Update Server WIT Worlds
Create multiple world definitions for different feature combinations:

```wit
// components/server/wit/worlds.wit
package wasmcp:server@0.1.0;

// Minimal tool server
world server-tools {
    import tool-handler;
    export wasi:http/incoming-handler@0.2.0;
}

// Tools + Resources
world server-tools-resources {
    import tool-handler;
    import resource-handler;
    export wasi:http/incoming-handler@0.2.0;
}

// Full server with all handlers
world server-full {
    import tool-handler;
    import resource-handler;
    import prompt-handler;
    import sampling-handler;
    import elicitation-handler;
    import roots-handler;
    import completion-handler;
    export wasi:http/incoming-handler@0.2.0;
}
```

#### 1.2 Feature-Gated Imports
```rust
// components/server/src/lib.rs
#[cfg(feature = "tools")]
use bindings::imports::fastertools::mcp::tool_handler;

#[cfg(feature = "resources")]
use bindings::imports::fastertools::mcp::resource_handler;

#[cfg(feature = "prompts")]
use bindings::imports::fastertools::mcp::prompt_handler;

// ... etc
```

#### 1.3 Automatic Capability Detection
```rust
fn handle_initialize(request: InitializeRequest) -> InitializeResponse {
    let mut capabilities = ServerCapabilities::default();
    
    #[cfg(feature = "tools")]
    {
        capabilities.tools = Some(ToolsCapability {
            list_changed: Some(false),
        });
    }
    
    #[cfg(all(feature = "resources", feature = "sse"))]
    {
        capabilities.resources = Some(ResourcesCapability {
            subscribe: Some(true),  // SSE enables subscriptions
            list_changed: Some(true),
        });
    }
    
    #[cfg(all(feature = "resources", not(feature = "sse")))]  
    {
        capabilities.resources = Some(ResourcesCapability {
            subscribe: None,  // No SSE, no subscriptions
            list_changed: None,
        });
    }
    
    #[cfg(feature = "prompts")]
    {
        capabilities.prompts = Some(PromptsCapability {
            list_changed: Some(false),
        });
    }
    
    // Client capabilities (when server acts as handler)
    #[cfg(feature = "sampling")]
    {
        capabilities.experimental = capabilities.experimental.or_else(|| Some(json!({})));
        capabilities.experimental.as_mut().unwrap()["sampling"] = json!(true);
    }
    
    InitializeResponse {
        protocol_version: "2025-06-18",
        capabilities,
        server_info: ImplementationInfo {
            name: "wasmcp-server",
            version: env!("CARGO_PKG_VERSION"),
            title: Some(format!("WasmCP Server ({})", get_features_string())),
        },
    }
}
```

#### 1.4 Conditional Method Routing
```rust
fn route_request(method: &str, params: Value) -> Result<Value, McpError> {
    match method {
        "initialize" => handle_initialize(params),
        "initialized" => Ok(Value::Null),
        "ping" => Ok(Value::Null),
        "shutdown" => handle_shutdown(),
        
        #[cfg(feature = "tools")]
        "tools/list" => tool_handler::handle_list_tools(params),
        #[cfg(feature = "tools")]
        "tools/call" => tool_handler::handle_call_tool(params),
        
        #[cfg(feature = "resources")]
        "resources/list" => resource_handler::handle_list_resources(params),
        #[cfg(feature = "resources")]
        "resources/read" => resource_handler::handle_read_resource(params),
        
        #[cfg(all(feature = "resources", feature = "sse"))]
        "resources/subscribe" => resource_handler::handle_subscribe(params),
        
        _ => Err(McpError {
            code: ErrorCode::MethodNotFound,
            message: format!("Method '{}' not found or not enabled", method),
            data: None,
        })
    }
}
```

### Phase 2: Transport Features

#### 2.1 SSE Support
```rust
#[cfg(feature = "sse")]
fn handle_sse_endpoint(req: Request) -> Response {
    Response::builder()
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(create_event_stream())
}

#[cfg(feature = "sse")]
fn send_notification(event: Notification) {
    // Send to all connected SSE clients
    for client in sse_clients.iter() {
        client.send_event("notification", &event);
    }
}
```

#### 2.2 Session Management
```rust
#[cfg(feature = "sessions")]
use wasi_keyvalue as kv;

#[cfg(feature = "sessions")]
fn get_or_create_session(session_id: Option<String>) -> Session {
    let id = session_id.unwrap_or_else(generate_session_id);
    
    if let Some(session) = kv::get(&format!("session:{}", id)) {
        return session;
    }
    
    let new_session = Session::new(id.clone());
    kv::set(&format!("session:{}", id), &new_session, Some(3600));
    new_session
}

#[cfg(not(feature = "sessions"))]
fn get_or_create_session(_: Option<String>) -> Session {
    Session::stateless()  // Always return fresh session
}
```

### Phase 3: Build and Publish Variants

#### 3.1 Build Script
```bash
#!/bin/bash
# build-variants.sh

# Minimal variants
cargo component build --release --features tools
cp target/wasm32-wasi/release/server.wasm dist/wasmcp-server-tools.wasm

cargo component build --release --features "tools,resources"
cp target/wasm32-wasi/release/server.wasm dist/wasmcp-server-basic.wasm

# With SSE support
cargo component build --release --features "tools,sse"
cp target/wasm32-wasi/release/server.wasm dist/wasmcp-server-tools-sse.wasm

cargo component build --release --features "tools,resources,sse"
cp target/wasm32-wasi/release/server.wasm dist/wasmcp-server-basic-sse.wasm

# Full variant
cargo component build --release --features "full,sse,sessions"
cp target/wasm32-wasi/release/server.wasm dist/wasmcp-server-full.wasm

# Development variant with everything
cargo component build --release --all-features
cp target/wasm32-wasi/release/server.wasm dist/wasmcp-server-dev.wasm
```

#### 3.2 Registry Publication
```bash
# Publish each variant
wkg publish dist/wasmcp-server-tools.wasm --version 0.1.0
wkg publish dist/wasmcp-server-basic.wasm --version 0.1.0
wkg publish dist/wasmcp-server-full.wasm --version 0.1.0
```

### Phase 4: Update Composition Patterns

#### 4.1 Simplified Composition
Before (7+ components):
```wac
let handler = new rust:handler { ... };
let nullresources = new fastertools:null-resources { ... };
let nullprompts = new fastertools:null-prompts { ... };
// ... 5 more null components

let server = new fastertools:wasmcp-server {
    // Wire everything manually
};
```

After (2 components):
```wac
let handler = new rust:handler { ... };
let server = new fastertools:wasmcp-server-tools {
    "fastertools:mcp/tool-handler@0.1.1": handler["fastertools:mcp/tool-handler@0.1.1"],
};
export server["wasi:http/incoming-handler@0.2.0"];
```

#### 4.2 Example Makefiles
```makefile
# Simple tools-only server
compose-simple:
    wac compose -o composed.wasm \
        -d rust:handler=handler.wasm \
        -d fastertools:wasmcp-server-tools=wasmcp-server-tools.wasm \
        compose-simple.wac

# Full server with null components for unused handlers
compose-full:
    wac compose -o composed.wasm \
        -d rust:handler=handler.wasm \
        -d fastertools:null-sampling=null-sampling.wasm \
        -d fastertools:wasmcp-server-full=wasmcp-server-full.wasm \
        compose-full.wac
```

## Benefits

### For Users
1. **Simpler composition** - Usually just handler + server variant
2. **Smaller binaries** - Only include what's needed
3. **Better performance** - No runtime feature detection
4. **Clear capabilities** - Server tells truth about what's available

### For Maintainers
1. **Single codebase** - One server component with features
2. **Type safety** - Compile-time verification
3. **Easy testing** - Test each feature combination
4. **Clear architecture** - Features map to capabilities

### For the Ecosystem
1. **Multiple deployment options** - Choose the right variant
2. **Gradual adoption** - Start simple, add features as needed
3. **Future-proof** - Easy to add new transport mechanisms
4. **Component model showcase** - Demonstrates advanced patterns

## Migration Strategy

1. **Keep null components** - Still useful for testing and advanced patterns
2. **Gradual rollout** - Start with tools-only variant
3. **Documentation** - Clear guides for each variant
4. **Backwards compatibility** - Existing compositions still work

## Testing Plan

1. **Unit tests** - Test each feature in isolation
2. **Integration tests** - Test feature combinations
3. **Composition tests** - Verify WAC compositions work
4. **Capability tests** - Ensure advertised = actual
5. **Binary size tests** - Verify size reductions

## Success Criteria

- [ ] Server variants build successfully with different features
- [ ] Capability detection works correctly
- [ ] Binary sizes reduced by 30-50% for minimal variants
- [ ] Composition simplified to 2-3 components typical case
- [ ] All tests pass for each variant
- [ ] Documentation clear and comprehensive

## Timeline

- **Week 1**: Implement core feature flags and capability detection
- **Week 2**: Add SSE and session features
- **Week 3**: Build and test all variants
- **Week 4**: Documentation and examples
- **Week 5**: Registry publication and ecosystem testing

## Open Questions

1. Should we auto-generate the world definitions for each feature combination?
2. How do we handle feature combinations that don't make sense?
3. Should handlers still be allowed to override capabilities?
4. What's the versioning strategy for variants?

## Conclusion

This architecture provides the best of both worlds:
- **Simplicity by default** with pre-built variants
- **Full flexibility** when needed with null components
- **Production-ready** with proper feature management
- **Future-proof** with extensible feature system

Ready to implement! 🚀