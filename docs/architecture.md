# wasmcp Architecture

This document explains how wasmcp components compose into complete MCP servers using WebAssembly's component model and the chain of responsibility pattern.

## Overview

wasmcp separates concerns into three layers:

1. **Capability Components** - Your business logic (tools, resources, prompts)
2. **Middleware Components** - Protocol translation and merging
3. **Transport Components** - Client communication (HTTP, stdio)

These layers compose into a linear pipeline where each component can:
- Handle requests it understands
- Pass others downstream
- Merge results from multiple sources

## The Component Model

### WebAssembly Components

WebAssembly components are:

- **Composable** - Link compiled binaries without recompilation
- **Sandboxed** - Isolated execution with explicit interfaces
- **Polyglot** - Write in any language with component tooling
- **Distributable** - Push/pull from OCI registries like container images

Components define interfaces in WIT (WebAssembly Interface Types) and export/import functions across language boundaries.

### Why Components for MCP?

MCP's design principles map naturally to the component model:

1. **Easy to build** - Components have narrow interfaces, templates generate bindings
2. **Highly composable** - Binary composition without complex configuration
3. **Sandboxed** - Components can't see into each other, only declared interfaces
4. **Progressive** - Add features by adding components to the chain

See [Component Model Specification](https://component-model.bytecodealliance.org/) for details.

## The Capability Pattern

wasmcp uses a **capability/middleware pattern** that separates concerns:

### Capability Components (Your Code)

**What they do:** Implement business logic for tools, resources, or prompts

**What they export:** Clean, focused capability interfaces from `wit/protocol/features.wit`

**Example (tools):**
```wit
// Your component exports:
export tools-capability

interface tools-capability {
  list-tools: func() -> result<tools-list, error>
  call-tool: func(name: string, arguments: string) -> result<call-result, error>
}
```

**Your component:**
- Knows nothing about MCP protocol
- Doesn't handle JSON-RPC
- Just implements the capability interface
- Returns structured results

**Code location:** `examples/` or your own components created with `wasmcp new`

### Middleware Components (Framework Code)

**What they do:** Translate between MCP protocol and capability interfaces

**What they import:** Capability interface (from your component)

**What they export:** Handler interface (for next component in chain)

**Example (tools-middleware):**
```wit
// Middleware imports your capability:
import tools-capability

// And exports handler for the chain:
export handle: func(request: string) -> result<string, error>
```

**Middleware:**
- Receives JSON-RPC MCP request
- Routes to appropriate capability method
- Converts results to MCP response format
- Passes unhandled requests downstream

**Code location:** `crates/tools-middleware/`, `crates/resources-middleware/`, `crates/prompts-middleware/`

### Why This Split?

**Separation of concerns:**
- You write tools/resources/prompts
- Framework handles protocol complexity

**Automatic merging:**
- Multiple components with same capability compose automatically
- Middleware merges their results (e.g., combining tool lists)

**Language agnostic:**
- Capability interface is the same across languages
- Middleware doesn't care what language implements the capability

**Progressive enhancement:**
- Add capabilities by adding components
- No configuration changes needed

## Composition Pipeline

### Linear Chain

When you run `wasmcp compose calc strings -o server.wasm`, the CLI builds this chain:

```
┌─────────────────────┐
│  HTTP Transport     │  Listens on port 8080
└─────────┬───────────┘
          │ handle(request) -> response
          ↓
┌─────────────────────┐
│  Tools Middleware   │  Wraps calc component
│  (calc capability)  │
└─────────┬───────────┘
          │ handle(request) -> response
          ↓
┌─────────────────────┐
│  Tools Middleware   │  Wraps strings component
│  (strings cap.)     │
└─────────┬───────────┘
          │ handle(request) -> response
          ↓
┌─────────────────────┐
│  Method Not Found   │  Returns error for unhandled methods
└─────────────────────┘
```

### Request Flow

1. **Client sends request** → HTTP transport receives JSON-RPC
2. **Transport calls `handle`** → First middleware receives request
3. **Middleware checks method**:
   - If `tools/list` or `tools/call` → handles and calls capability
   - Otherwise → calls next `handle` in chain
4. **Next middleware** repeats step 3
5. **Method-not-found** returns error if no one handles it

### Response Flow (Merging)

For `tools/list`:

1. **First middleware** (calc) returns `[add, subtract, multiply, divide]`
2. **Second middleware** (strings) calls next, gets response, merges with its own
3. **Returns** `[add, subtract, multiply, divide, uppercase, lowercase, reverse, count_chars]`

The middleware components automatically merge results from all sources.

For `tools/call`:

1. **First middleware** (calc) checks if tool name matches its tools
   - If yes → executes and returns
   - If no → calls next
2. **Second middleware** (strings) checks its tools
   - If yes → executes and returns
   - If no → calls next
3. **Method-not-found** returns "tool not found" error

Only the middleware that owns the tool executes it.

## Handler Interface

All components in the chain use the same handler interface from `wit/server/handler.wit`:

```wit
interface handler {
  // Process an MCP request
  handle: func(request: string) -> result<string, error>
}
```

**Simple contract:**
- Input: JSON-RPC request string
- Output: JSON-RPC response string (or error)

Each component in the chain:
- **Imports** `handle` from the next component
- **Exports** `handle` for the previous component

This creates a linked chain where each can delegate to the next.

### Example: Tools Middleware

Pseudocode for how tools-middleware works:

```rust
// Import from next component
import fn handle_next(request: String) -> Result<String>

// Import from wrapped capability component
import fn list_tools() -> Result<ToolsList>
import fn call_tool(name: String, args: String) -> Result<CallResult>

// Export for previous component
export fn handle(request: String) -> Result<String> {
    let parsed = parse_jsonrpc(request);

    match parsed.method {
        "tools/list" => {
            // Get tools from our capability
            let our_tools = list_tools()?;

            // Get tools from downstream
            let downstream = handle_next(request)?;
            let downstream_tools = parse_tools_list(downstream);

            // Merge and return
            let merged = merge_tools(our_tools, downstream_tools);
            return jsonrpc_response(merged);
        }

        "tools/call" => {
            let tool_name = parsed.params.name;

            // Check if we own this tool
            if our_tools.contains(tool_name) {
                let result = call_tool(tool_name, parsed.params.arguments)?;
                return jsonrpc_response(result);
            } else {
                // Pass to downstream
                return handle_next(request);
            }
        }

        _ => {
            // We don't handle this method, pass downstream
            return handle_next(request);
        }
    }
}
```

See `crates/tools-middleware/` for actual implementation.

## Transport Components

### HTTP Transport

**Location:** `crates/http-transport/`

**Purpose:** Expose MCP server over HTTP

**Exports:** Nothing (it's the top of the chain)

**Imports:** `handle` from first middleware

**How it works:**
1. Listens on HTTP port (default 8080)
2. Receives POST requests at `/mcp`
3. Reads JSON-RPC from request body
4. Calls `handle(request)`
5. Returns JSON-RPC response

**Usage:**
```bash
wasmcp compose calc -t http -o server.wasm
wasmtime serve -Scli server.wasm
# Server at http://0.0.0.0:8080/mcp
```

### Stdio Transport

**Location:** `crates/stdio-transport/`

**Purpose:** Expose MCP server over stdin/stdout

**Exports:** Nothing (top of chain)

**Imports:** `handle` from first middleware

**How it works:**
1. Reads JSON-RPC from stdin (one message per line)
2. Calls `handle(request)`
3. Writes JSON-RPC response to stdout

**Usage:**
```bash
wasmcp compose calc -t stdio -o server.wasm
wasmtime run server.wasm
# Reads stdin, writes stdout
```

**Used for:** Local MCP clients like Claude Desktop

## Auto-Detection and Wrapping

### CLI Intelligence

When you run `wasmcp compose component.wasm`, the CLI:

1. **Inspects the component** using `wasm-tools component wit`
2. **Detects exports**:
   - Exports `tools-capability` → wrap with `tools-middleware`
   - Exports `resources-capability` → wrap with `resources-middleware`
   - Exports `prompts-capability` → wrap with `prompts-middleware`
   - Exports `handler` → it's already middleware, use as-is
3. **Downloads middleware** from `ghcr.io/wasmcp` (if needed)
4. **Composes the chain** with transport → middleware(s) → method-not-found

You don't specify middleware explicitly - the CLI figures it out.

### Example Detection

Given these components:

```bash
wasmcp compose calc.wasm strings.wasm weather.wasm
```

If:
- `calc.wasm` exports `tools-capability`
- `strings.wasm` exports `tools-capability`
- `weather.wasm` exports `tools-capability`

CLI builds:

```
http-transport
    ↓
tools-middleware (wraps calc.wasm)
    ↓
tools-middleware (wraps strings.wasm)
    ↓
tools-middleware (wraps weather.wasm)
    ↓
method-not-found
```

All three are automatically wrapped and chained.

## Method Not Found

**Location:** `crates/method-not-found/`

**Purpose:** Terminal handler that returns errors for unhandled methods

**Exports:** `handle` (end of chain)

**Imports:** Nothing (no downstream)

**How it works:**
```rust
export fn handle(request: String) -> Result<String> {
    let parsed = parse_jsonrpc(request);
    return jsonrpc_error(
        -32601, // Method not found
        format!("Method '{}' not supported", parsed.method)
    );
}
```

**Why needed:**
- Every chain needs a terminus
- Ensures all unhandled methods get proper JSON-RPC error responses
- Without it, unhandled methods would panic

## Framework Components

Summary of published framework components:

| Component | Location | Purpose | In Chain |
|-----------|----------|---------|----------|
| `http-transport` | `crates/http-transport/` | HTTP server | Top |
| `stdio-transport` | `crates/stdio-transport/` | Stdio I/O | Top |
| `tools-middleware` | `crates/tools-middleware/` | Tools capability wrapper | Middle |
| `resources-middleware` | `crates/resources-middleware/` | Resources capability wrapper | Middle |
| `prompts-middleware` | `crates/prompts-middleware/` | Prompts capability wrapper | Middle |
| `method-not-found` | `crates/method-not-found/` | Error handler | Bottom |

Published to `ghcr.io/wasmcp`, auto-downloaded by CLI.

## Custom Middleware

Want to build custom middleware? Use the handler pattern:

```wit
// your-middleware.wit
package my-org:my-middleware

interface handler {
  handle: func(request: string) -> result<string, error>
}

world my-middleware {
  // Import from next in chain
  import handler

  // Export for previous in chain
  export handler
}
```

**Implementation:**
1. Parse incoming request
2. Handle methods you care about
3. Pass others to imported `handle`
4. Return response

**Usage:**
```bash
wasmcp compose calc --override-middleware my-middleware.wasm
```

See `crates/` for reference implementations.

## Sessions and Notifications

### Sessions

**Interface:** `wit/server/sessions.wit`

For middleware that needs to maintain state across requests:

```wit
interface sessions {
  initialize: func(session-id: string) -> result<_, error>
  terminate: func(session-id: string) -> result<_, error>
}
```

Components can track session state using these lifecycle hooks.

**Example use:** Connection pooling, authentication state

### Notifications

**Interface:** `wit/server/notifications.wit`

For sending server-to-client notifications:

```wit
interface notifications {
  send-progress: func(progress: progress-notification) -> result<_, error>
  send-log: func(log: log-notification) -> result<_, error>
  send-resource-updated: func(update: resource-update) -> result<_, error>
}
```

Middleware can import this to send notifications during long-running operations.

**Example use:** Progress updates, logging, real-time data changes

See `wit/server/` for full interface definitions.

## Component Distribution

### Registry

Components can be distributed via OCI registries:

```bash
# Push component
wkg publish my-component.wasm --registry ghcr.io/my-org

# Use in composition
wasmcp compose my-org:my-component@1.0.0
```

The CLI automatically downloads and caches components.

**Cache location:** `~/.config/wasmcp/deps/`

### Versioning

Components declare version compatibility in their WIT:

```bash
wasmcp new my-component --version 0.4.0
wasmcp compose my-component.wasm --version 0.4.0
```

Ensures middleware and capabilities use compatible interfaces.

## Advanced Topics

### Custom Transports

Build a WebSocket transport:

1. Implement `handler` export
2. Import `handle` from first middleware
3. Implement WebSocket server in component
4. Call `handle(request)` for each WebSocket message

```bash
wasmcp compose calc --override-transport my-websocket-transport.wasm
```

### Middleware Order

Components are chained in the order specified:

```bash
wasmcp compose logger calc strings
```

Creates:
```
transport → logger → calc → strings → method-not-found
```

Logger sees requests first, can log before passing downstream.

### Error Handling

Each component in the chain can:
- Handle the request successfully
- Return an error (short-circuits the chain)
- Pass to next (continues the chain)

Errors propagate back up the chain to the transport.

### Performance

**Component overhead:** Minimal - calls are near-native speed

**Chain depth:** Linear time complexity, but very fast

**Binary size:** Complete servers can be under 1MB

**Memory:** Components are sandboxed, can't access each other's memory

## Design Principles

wasmcp's architecture follows these principles:

1. **Separation of concerns** - Business logic separate from protocol
2. **Composition over configuration** - Wire components, don't configure
3. **Convention over complexity** - Auto-detect, auto-wrap
4. **Progressive enhancement** - Add features by adding components
5. **Fail-safe** - Method-not-found ensures no panics

These align with MCP's design principles and the component model's philosophy.

## Further Reading

- **WIT Interfaces:** See `docs/wit-interfaces.md` for interface specifications
- **Examples:** Study working compositions in `docs/examples.md`
- **Getting Started:** Build your first component in `docs/getting-started.md`
- **Component Model:** [Official specification](https://component-model.bytecodealliance.org/)
- **MCP Specification:** [Model Context Protocol](https://spec.modelcontextprotocol.io/)

## Component Sources

Study the actual implementations:

- **Your components:** `examples/` directory
- **Framework middleware:** `crates/tools-middleware/`, `crates/resources-middleware/`, etc.
- **Transport layers:** `crates/http-transport/`, `crates/stdio-transport/`
- **WIT interfaces:** `wit/protocol/` and `wit/server/`

---

**The architecture enables:** Building complex MCP servers from simple, focused components that compose without configuration. Like Unix pipes, but for MCP.
