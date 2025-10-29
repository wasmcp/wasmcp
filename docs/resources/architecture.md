# Architecture

How wasmcp composes WebAssembly components into MCP servers using the capability/middleware pattern and chain of responsibility.

## Core Concept

wasmcp separates business logic from protocol handling:

**Your code:** Implements capabilities (tools, resources, prompts)
**Framework code:** Handles MCP protocol translation
**Composition:** Chains components into request/response pipeline

## Component Model

### WebAssembly Components

**Properties:**
- **Composable:** Link compiled binaries without recompilation
- **Sandboxed:** Isolated execution with explicit interfaces
- **Polyglot:** Write in any language with component tooling
- **Distributable:** Push/pull from OCI registries

**Interface definition:** WIT (WebAssembly Interface Types)

**Why for MCP:**
1. Easy to build (narrow interfaces, generated bindings)
2. Highly composable (binary composition, no configuration)
3. Sandboxed (can't see into other components)
4. Progressive (add features by adding components)

## Capability Pattern

### Capability Components (Your Code)

**What they do:** Implement business logic

**What they export:** Clean capability interfaces from `spec/2025-06-18/wit/` (tools, resources, prompts, completions)

**Example (tools):**
```wit
export tools-capability

interface tools-capability {
  list-tools: func() -> result<tools-list, error>
  call-tool: func(name: string, arguments: string) -> result<call-result, error>
}
```

**What they know:**
- Their business domain (math, strings, weather)
- How to execute their operations
- Their input/output contracts

**What they don't know:**
- MCP protocol details
- JSON-RPC format
- How to merge with other components
- Transport mechanisms

**Code location:** `examples/` or components created with `wasmcp new`

### Middleware Components (Framework Code)

**What they do:** Translate between MCP protocol and capabilities

**What they import:** Capability interface (from your component)

**What they export:** Handler interface (for next component in chain)

**Example (tools-middleware):**
```wit
import tools-capability
export handle: func(request: string) -> result<string, error>
```

**Responsibilities:**
- Receive JSON-RPC MCP requests
- Route to appropriate capability methods
- Convert results to MCP response format
- Merge results from multiple sources
- Pass unhandled requests downstream

**Code location:** `crates/tools-middleware/`, `crates/resources-middleware/`, `crates/prompts-middleware/`

### Why This Split

**Separation of concerns:**
- You write domain logic
- Framework handles protocol complexity

**Automatic merging:**
- Multiple components with same capability compose automatically
- Middleware merges results (e.g., combining tool lists)

**Language agnostic:**
- Same capability interface across languages
- Middleware doesn't care about implementation language

**Progressive enhancement:**
- Add capabilities by adding components
- No configuration needed

## Composition Pipeline

### Linear Chain

**Command:**
```bash
wasmcp compose server calc strings -o server.wasm
```

**Creates:**
```
┌─────────────────────┐
│  HTTP Transport     │  Listens on port 8080
└─────────┬───────────┘
          │ handle(request) -> response
          ↓
┌─────────────────────┐
│  Tools Middleware   │  Wraps calc capability
└─────────┬───────────┘
          │ handle(request) -> response
          ↓
┌─────────────────────┐
│  Tools Middleware   │  Wraps strings capability
└─────────┬───────────┘
          │ handle(request) -> response
          ↓
┌─────────────────────┐
│  Method Not Found   │  Returns error for unhandled
└─────────────────────┘
```

### Request Flow

1. Client sends request → Transport receives JSON-RPC
2. Transport calls `handle` → First middleware receives request
3. Middleware checks method:
   - If `tools/list` or `tools/call` → handles and calls capability
   - Otherwise → calls next `handle` in chain
4. Next middleware repeats step 3
5. Method-not-found returns error if no one handles it

### Response Flow (Merging)

**For `tools/list`:**
1. First middleware (calc) returns `[add, subtract, multiply, divide]`
2. Second middleware (strings) calls next, gets response, merges with its own
3. Returns `[add, subtract, multiply, divide, uppercase, lowercase, reverse]`

Middleware automatically merges results from all sources.

**For `tools/call`:**
1. First middleware (calc) checks if tool name matches
   - If yes → executes and returns
   - If no → calls next
2. Second middleware (strings) checks its tools
   - If yes → executes and returns
   - If no → calls next
3. Method-not-found returns "tool not found" error

Only the middleware that owns the tool executes it.

## Handler Interface

All components in chain use same interface from `spec/2025-06-18/wit/server.wit` (server-handler interface):

```wit
interface handler {
  handle: func(request: string) -> result<string, error>
}
```

**Contract:**
- Input: JSON-RPC request string
- Output: JSON-RPC response string (or error)

**Chain structure:**
- Each component **imports** `handle` from next component
- Each component **exports** `handle` for previous component
- Creates linked chain where each can delegate to next

### Middleware Pseudocode

```rust
// Import from next in chain
import fn handle_next(request: String) -> Result<String>

// Import from wrapped capability
import fn list_tools() -> Result<ToolsList>
import fn call_tool(name: String, args: String) -> Result<CallResult>

// Export for previous in chain
export fn handle(request: String) -> Result<String> {
    let parsed = parse_jsonrpc(request);

    match parsed.method {
        "tools/list" => {
            let our_tools = list_tools()?;
            let downstream = handle_next(request)?;
            let downstream_tools = parse_tools_list(downstream);
            let merged = merge_tools(our_tools, downstream_tools);
            return jsonrpc_response(merged);
        }

        "tools/call" => {
            if our_tools.contains(parsed.params.name) {
                let result = call_tool(parsed.params.name, parsed.params.arguments)?;
                return jsonrpc_response(result);
            } else {
                return handle_next(request);
            }
        }

        _ => {
            return handle_next(request);
        }
    }
}
```

See `crates/tools-middleware/` for actual implementation.

## Transport Components

### HTTP Transport

**Location:** `crates/http-transport/`

**Exports:** Nothing (top of chain)

**Imports:** `handle` from first middleware

**How it works:**
1. Listens on HTTP port (default 8080)
2. Receives POST requests at `/mcp`
3. Reads JSON-RPC from request body
4. Calls `handle(request)`
5. Returns JSON-RPC response

**Usage:**
```bash
wasmcp compose server calc -t http -o server.wasm
wasmtime serve -Scli server.wasm
# Server at http://0.0.0.0:8080/mcp
```

### Stdio Transport

**Location:** `crates/stdio-transport/`

**Exports:** Nothing (top of chain)

**Imports:** `handle` from first middleware

**How it works:**
1. Reads JSON-RPC from stdin (one message per line)
2. Calls `handle(request)`
3. Writes JSON-RPC response to stdout

**Usage:**
```bash
wasmcp compose server calc -t stdio -o server.wasm
wasmtime run server.wasm
```

**Used for:** Local MCP clients (Claude Desktop)

## Auto-Detection

### CLI Intelligence

When you run `wasmcp compose server component.wasm`, CLI:

1. **Inspects component** using `wasm-tools component wit`
2. **Detects exports:**
   - `tools-capability` → wrap with `tools-middleware`
   - `resources-capability` → wrap with `resources-middleware`
   - `prompts-capability` → wrap with `prompts-middleware`
   - `handler` → use as-is (already middleware)
3. **Downloads middleware** from `ghcr.io/wasmcp` if needed
4. **Composes chain:** transport → middleware(s) → method-not-found

You don't specify middleware - CLI figures it out.

### Example

**Given:**
```bash
wasmcp compose server calc.wasm strings.wasm weather.wasm
```

**If all export `tools-capability`, CLI builds:**
```
http-transport
    ↓
tools-middleware (wraps calc)
    ↓
tools-middleware (wraps strings)
    ↓
tools-middleware (wraps weather)
    ↓
method-not-found
```

All automatically wrapped and chained.

## Method Not Found

**Location:** `crates/method-not-found/`

**Exports:** `handle` (end of chain)

**Imports:** Nothing (terminus)

**How it works:**
```rust
export fn handle(request: String) -> Result<String> {
    let parsed = parse_jsonrpc(request);
    return jsonrpc_error(
        -32601,  // Method not found
        format!("Method '{}' not supported", parsed.method)
    );
}
```

**Why needed:**
- Every chain needs a terminus
- Ensures all unhandled methods get proper JSON-RPC errors
- Prevents panics from unhandled methods

## Framework Components

Published to `ghcr.io/wasmcp`, auto-downloaded by CLI:

| Component | Purpose | Position |
|-----------|---------|----------|
| `http-transport` | HTTP server | Top |
| `stdio-transport` | Stdio I/O | Top |
| `tools-middleware` | Tools wrapper | Middle |
| `resources-middleware` | Resources wrapper | Middle |
| `prompts-middleware` | Prompts wrapper | Middle |
| `method-not-found` | Error handler | Bottom |

## Sessions and Notifications

### Sessions

**Interface:** `spec/2025-06-18/wit/sessions.wit`

For middleware needing state across requests:

```wit
interface sessions {
  initialize: func(session-id: string) -> result<_, error>
  terminate: func(session-id: string) -> result<_, error>
}
```

**Use cases:** Connection pooling, authentication state

### Notifications

**Interface:** `spec/2025-06-18/wit/notifications.wit`

For server-to-client notifications:

```wit
interface notifications {
  send-progress: func(progress: progress-notification) -> result<_, error>
  send-log: func(log: log-notification) -> result<_, error>
  send-resource-updated: func(update: resource-update) -> result<_, error>
}
```

**Use cases:** Progress updates, logging, real-time data changes

## Custom Components

### Custom Middleware

**WIT definition:**
```wit
package my-org:my-middleware

interface handler {
  handle: func(request: string) -> result<string, error>
}

world my-middleware {
  import handler  // From next in chain
  export handler  // For previous in chain
}
```

**Implementation:**
1. Parse incoming request
2. Handle methods you care about
3. Pass others to imported `handle`
4. Return response

**Usage:**
```bash
wasmcp compose server calc --override-middleware my-middleware.wasm
```

### Custom Transports

Build WebSocket, gRPC, or other transports:

1. Implement `handler` export
2. Import `handle` from first middleware
3. Implement transport server
4. Call `handle(request)` for each message

```bash
wasmcp compose server calc --override-transport websocket-transport.wasm
```

## Component Distribution

### OCI Registry

**Push component:**
```bash
wkg publish component.wasm --registry ghcr.io/my-org
```

**Use in composition:**
```bash
wasmcp compose server my-org:component@1.0.0
```

**Cache location:** `~/.config/wasmcp/deps/`

### Versioning

**Specify version:**
```bash
wasmcp new calc --version 0.4.0
wasmcp compose server calc --version 0.4.0
```

Ensures middleware and capabilities use compatible interfaces.

## Advanced Topics

### Middleware Order

Components chained in specified order:

```bash
wasmcp compose server logger calc strings
```

Creates:
```
transport → logger → calc → strings → method-not-found
```

Logger sees requests first, can log before passing downstream.

### Error Handling

Each component can:
- Handle request successfully
- Return error (short-circuits chain)
- Pass to next (continues chain)

Errors propagate back up to transport.

### Performance

- **Component overhead:** Minimal (near-native call speed)
- **Chain depth:** Linear time, very fast
- **Binary size:** Complete servers under 1MB
- **Memory:** Components sandboxed, can't access each other's memory

## Design Principles

1. **Separation of concerns:** Business logic separate from protocol
2. **Composition over configuration:** Wire components, don't configure
3. **Convention over complexity:** Auto-detect, auto-wrap
4. **Progressive enhancement:** Add features by adding components
5. **Fail-safe:** Method-not-found ensures no panics

Aligns with MCP design principles and component model philosophy.

## Related Resources

- **Building servers:** See `building-servers` resource for practical workflow
- **CLI reference:** See `reference` resource for command details
- **WIT interfaces:** See WIT resources for interface specifications
