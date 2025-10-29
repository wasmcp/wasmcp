# Composition Modes

Two composition modes with distinct outputs and use cases.

## Mode Comparison

**Server Mode:** `wasmcp compose server [components...]`
- Output exports: `wasi:http/incoming-handler@0.2.3` or `wasi:cli/run@0.2.3`
- Structure: transport → handlers → method-not-found
- Runnable: Yes (wasmtime serve / wasmtime run)
- Use for: Deployment, production servers, local testing

**Handler Mode:** `wasmcp compose handler [components...]`
- Output exports: `wasmcp:server/handler@{version}`
- Structure: handler₁ → handler₂ → ... → handlerₙ
- Runnable: No (intermediate component)
- Use for: Reusable middleware, multi-layer composition, component libraries

## Component Layering

Three layers in the composition stack:

**Layer 1: Capability Components**
- Created: `wasmcp new --template tools|resources|prompts`
- Exports: `wasmcp:protocol/tools@{v}` or `resources` or `prompts`
- Example: calculator.wasm, weather.wasm, database.wasm
- Role: Business logic implementation

**Layer 2: Handler Components** (compose handler output)
- Created: `wasmcp compose handler cap1.wasm cap2.wasm`
- Exports: `wasmcp:server/handler@{version}`
- Structure: Capability components wrapped with middleware and chained
- Example: geo-tools.wasm (geocoding + distance + math)
- Role: Reusable composition units

**Layer 3: Server Components** (compose server output)
- Created: `wasmcp compose server handlers...`
- Exports: WASI interface (http or cli)
- Structure: transport → handlers → method-not-found
- Example: production-server.wasm
- Role: Runnable MCP servers

## Interface Hierarchy

All components in chain use uniform handler interface:

```wit
interface handler {
  use wasmcp:protocol/mcp@{v}.{client-request, server-response, error-code};
  handle-request: func(request: client-request, request-id: string)
    -> result<server-response, error-code>;
}
```

Contract:
- Each component imports `handler` from downstream
- Each component exports `handler` for upstream
- Unknown methods delegated downstream via imported handler
- Known methods handled locally or merged with downstream results

## Auto-Detection and Wrapping

CLI inspects component exports and auto-wraps capability components:

Detection priority (first match wins):
1. Exports `wasmcp:server/handler` → Use as-is (already middleware)
2. Exports `wasmcp:protocol/tools` → Download and compose with tools-middleware
3. Exports `wasmcp:protocol/resources` → Download and compose with resources-middleware
4. Exports `wasmcp:protocol/prompts` → Download and compose with prompts-middleware

Wrapping process for capability component:
1. Parse WIT exports from component binary
2. Identify capability type (tools/resources/prompts)
3. Download corresponding middleware from ghcr.io/wasmcp
4. Compose: middleware(capability) → wrapped-component
5. Wrapped component exports server-handler interface

Important: Composed handlers contain nested capability components but export server-handler at top level, so detection checks server-handler first to prevent re-wrapping.

## Request Flow Example

```
Client request: {"method":"tools/call","params":{"name":"distance",...}}
  ↓
HTTP Transport (server mode only)
  ↓ calls handle-request()
Distance Handler (composed: distance-calc → math)
  ↓ method=="tools/call" && name=="distance" → executes
  ↓ calls downstream for "square", "add", "sqrt"
Math Middleware (wraps math capability)
  ↓ method=="tools/call" && name in ["square","add","sqrt"] → executes
  ↓ calls math capability functions
Math Capability Component
  ↓ returns result
  ← propagates back up chain
```

For listing (tools/list, resources/list):
- Each middleware calls downstream first
- Merges its capabilities with downstream results
- Returns combined list up the chain
- Final response includes all capabilities in chain

For execution (tools/call, resources/read):
- Each middleware checks if it owns the named item
- If yes: executes and returns (short-circuits chain)
- If no: delegates to downstream via handle-request import
- Method-not-found returns error if no component handles it

## Server Mode Specifics

Command: `wasmcp compose server [opts] components...`

Automatically adds:
- Transport component (--transport http|stdio, default: http)
  - http: Exports wasi:http/incoming-handler, serves on :8080/mcp
  - stdio: Exports wasi:cli/run, reads stdin/writes stdout
- Method-not-found terminal handler (returns JSON-RPC -32601 error)
- HTTP-notifications (http transport only, for server-to-client notifications)

Framework components downloaded from ghcr.io/wasmcp:
- wasmcp:http-transport@{version}
- wasmcp:stdio-transport@{version}
- wasmcp:method-not-found@{version}
- wasmcp:http-notifications@{version}
- wasmcp:tools-middleware@{version}
- wasmcp:resources-middleware@{version}
- wasmcp:prompts-middleware@{version}

Options:
- --override-transport <path>: Use custom transport instead of framework
- --override-method-not-found <path>: Use custom terminal handler
- --version <v>: WIT interface version for framework components
- --deps-dir <path>: Cache location for downloaded components
- --skip-download: Use cached components only

## Handler Mode Specifics

Command: `wasmcp compose handler [opts] components...`

Does NOT include:
- Transport (no HTTP/stdio server)
- Method-not-found terminal
- HTTP-notifications

Only includes:
- Handler chain from user components
- Auto-wrapped capability components

Output can be:
- Composed into servers: `wasmcp compose server handler.wasm other.wasm`
- Composed into other handlers: `wasmcp compose handler handler1.wasm handler2.wasm`
- Published to registry: `wkg publish handler.wasm`

Options (subset of server mode):
- --version <v>: WIT interface version
- --deps-dir <path>: Cache location
- -o, --output <path>: Output path (default: handler.wasm)

No transport-related options (--transport, --override-transport, etc)

## Common Patterns

**Pattern: Reusable Handler Library**
```bash
wasmcp compose handler utils/*.wasm -o utils.wasm
wkg publish utils.wasm --registry ghcr.io/org
wasmcp compose server org:utils@1.0.0 app.wasm
```

**Pattern: Multi-Layer Composition**
```bash
wasmcp compose handler low-level.wasm -o low.wasm
wasmcp compose handler low.wasm mid-level.wasm -o mid.wasm
wasmcp compose server mid.wasm high-level.wasm
```

**Pattern: Environment-Specific Servers**
```bash
wasmcp compose handler core/*.wasm -o core.wasm
# Dev: all features + debug, stdio transport
wasmcp compose server core.wasm premium.wasm debug.wasm --transport stdio -o dev.wasm
# Prod: core only, http transport
wasmcp compose server core.wasm --transport http -o prod.wasm
```

**Pattern: Component Testing**
```bash
# Test individual capability
wasmcp compose server calc.wasm -o test.wasm && wasmtime serve test.wasm

# Test composed handler by wrapping in server
wasmcp compose handler calc.wasm math.wasm -o handler.wasm
wasmcp compose server handler.wasm -o test-server.wasm
wasmtime serve test-server.wasm
```

## Troubleshooting

**Error: "Component X does not export server-handler interface"**
- Cause: Component in handler chain doesn't export required interface
- Check: `wasm-tools component wit component.wasm | grep export`
- Fix: Ensure component exports either capability interface (will be wrapped) or server-handler

**Error: "instance does not have an export named wasmcp:protocol/tools@X.X.X"**
- Cause: Component exports server-handler but was detected as capability (old versions)
- Fix: Upgrade wasmcp CLI to v0.4.3+ (checks server-handler first in detection)

**Handler vs Server Confusion**
- Handler: Cannot run with wasmtime (no WASI exports)
- Server: Can run with wasmtime serve/run (has WASI exports)
- Handler must be composed into server before running
- Test: `wasm-tools component wit X.wasm | grep "export wasi"`
  - Has wasi export = server (runnable)
  - No wasi export = handler/capability (not runnable)

## WIT Interface Versions

All framework components versioned together:
- Default: 0.1.0
- Specify: `--version 0.2.0`
- Affects: wasmcp:protocol/*, wasmcp:server/*, all middleware

Capabilities and middleware must use matching versions:
- Capability exports `wasmcp:protocol/tools@0.1.0`
- Middleware imports `wasmcp:protocol/tools@0.1.0`
- Mismatch causes composition errors

Check version compatibility:
```bash
wasm-tools component wit component.wasm | grep "wasmcp:"
```

## Related Documentation

- architecture: Overall system design, capability pattern, chain of responsibility
- building-servers: Practical workflow from wasmcp new to deployment
- reference: Command flags, component formats, config file structure
- spec/2025-06-18/wit/server.wit: Handler interface specification (server-handler)
- spec/2025-06-18/wit/: Capability interfaces (tools, resources, prompts, completions)
