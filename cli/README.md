# wasmcp CLI

The wasmcp CLI does two things:

1. **Scaffolding** - Generate Wasmcp component projects with language-specific tooling and build configuration
2. **Composition** - Assemble multiple Wasmcp components into a complete MCP server component

## Installation

### From Source

```bash
cargo install --path .
```

### Prerequisites

- Rust 1.89 or later
- [wkg](https://github.com/bytecodealliance/wasm-pkg-tools) - WebAssembly package manager

Language-specific tooling is required for building components in each supported language:

- **Rust**: [cargo-component](https://github.com/bytecodealliance/cargo-component) 0.14 or later
- **Go**: [TinyGo](https://tinygo.org/) 0.33 or later with [wit-bindgen-go](https://github.com/bytecodealliance/go-modules/tree/main?tab=readme-ov-file#wit-bindgen-go)
- **TypeScript**: [jco](https://github.com/bytecodealliance/jco) for bindings generation
- **Python**: [componentize-py](https://github.com/bytecodealliance/componentize-py) 0.14 or later

## Commands

### `wasmcp new`

Create a new MCP handler component project.

**Syntax:**
```bash
wasmcp new <name> --type <TYPE> --language <LANG> [OPTIONS]
```

**Arguments:**
- `<name>` - Project name (alphanumeric with hyphens and underscores)

**Options:**
- `-t, --type <TYPE>` - Handler type: `middleware`, `tools`, `resources`, `prompts`, `completion`
- `-l, --language <LANG>` - Programming language: `rust`, `go`, `typescript`, `python`
- `--version <VERSION>` - wasmcp version for dependencies (default: `0.3.0-alpha.59`)
- `-o, --output <PATH>` - Output directory (defaults to current directory)
- `--force` - Overwrite existing directory

**Examples:**

```bash
# Create a Rust tools handler
wasmcp new my-tools --type tools --language rust

# Create a Python middleware component
wasmcp new auth-middleware --type middleware --language python

# Create a Go resources handler with specific output location
wasmcp new fs-resources --type resources --language go --output ~/projects
```

The generated project includes:
- WIT interface definitions with correct world imports
- Language-specific source template implementing the handler pattern
- Build configuration (Cargo.toml, go.mod, package.json, etc.)
- Makefile with `build`, `clean`, and `test` targets
- README with usage instructions

### `wasmcp compose`

Compose multiple handler components into a complete MCP server.

**Syntax:**
```bash
wasmcp compose [OPTIONS]
```

**Examples:**

```bash
# Compose with HTTP transport (default)
wasmcp compose \
  --middleware ./logging.wasm \
  --tools ./my-tools.wasm \
  --resources ./my-resources.wasm \
  -o server.wasm

# Compose with stdio transport
wasmcp compose \
  --tools ./my-tools.wasm \
  --transport stdio \
  -o server-stdio.wasm

# Multiple middleware components
wasmcp compose \
  --middleware ./logger.wasm \
  --middleware ./auth.wasm \
  --tools ./tools.wasm
```

**Options:**
- `--middleware <SPEC>` - Middleware component (repeatable)
- `--tools <SPEC>` - Tools handler component (repeatable)
- `--resources <SPEC>` - Resources handler component (repeatable)
- `--prompts <SPEC>` - Prompts handler component (repeatable)
- `--completion <SPEC>` - Completion handler component (repeatable)
- `-t, --transport <TYPE>` - Transport type: `http` or `stdio` (default: `http`)
- `-o, --output <PATH>` - Output file path (default: `mcp-server.wasm`)
- `--version <VERSION>` - wasmcp version for dependencies (default: `0.3.0-alpha.59`)
- `--deps-dir <PATH>` - Dependency download directory (default: `deps`)
- `--skip-download` - Use existing dependencies without downloading
- `--force` - Overwrite existing output file

**Component Specifications:**

A component spec can be:
- **Local file path**: `./target/my-handler.wasm`
- **Registry reference**: `wasmcp:weather-tools@0.1.0` (downloaded via wkg)

**Handler Order and Execution Flow:**

Components are wired in reverse order of specification, creating a forward execution chain. The request flows through components in the order they appear on the command line:

```bash
# Request flows: logger -> auth -> tools -> initialize
wasmcp compose \
  --middleware ./logger.wasm \
  --middleware ./auth.wasm \
  --tools ./tools.wasm
```

**Important:** Handlers terminate requests they handle. Middleware positioned after a handler will not see requests that handler processes. For example:

```bash
# ❌ logger2 will NEVER see tools requests (tools handles and returns)
wasmcp compose \
  --middleware ./logger1.wasm \
  --tools ./tools.wasm \
  --middleware ./logger2.wasm  # Only sees initialize requests

# ✅ Both loggers see all requests
wasmcp compose \
  --middleware ./logger1.wasm \
  --middleware ./logger2.wasm \
  --tools ./tools.wasm
```

Place all middleware that needs to observe requests **before** handler components in the command line order.

## Handler Types

### Middleware

Middleware components intercept all requests without handling specific MCP methods. They both import and export the `incoming-handler` interface.

**Use Cases:**
- Request logging and monitoring
- Authentication and authorization
- Request enrichment (adding headers, context)
- Rate limiting and throttling
- Caching layers
- Metrics collection

**WIT Definition:**
```wit
world middleware {
    import incoming-handler;
    export incoming-handler;
}
```

**Implementation Pattern:**
```rust
fn handle(request: Request, output: OutputStream) {
    // Process request
    log_request(&request);

    // Forward to next handler
    incoming_handler::handle(request, output);
}
```

Middleware does not import writer interfaces since it does not generate MCP responses.

### Tools

Handle `tools/list` and `tools/call` methods for tool execution.

**Responsibilities:**
- List available tools with JSON schema
- Execute tool calls with validated parameters
- Return text, image, or structured content
- Register tools capability during initialization

**WIT Definition:**
```wit
world tools-handler {
    include wasmcp:mcp/tools-handler@<version>;
}
```

The tools-handler world imports `error-result`, `tools-list-result`, and `tools-call-content` for formatting responses.

### Resources

Handle `resources/list`, `resources/templates/list`, and `resources/read` methods for resource access.

**Responsibilities:**
- List available resources
- Provide URI templates for parameterized resources
- Read resource content
- Support MIME types and blob content
- Register resources capability during initialization

**WIT Definition:**
```wit
world resources-handler {
    include wasmcp:mcp/resources-handler@<version>;
}
```

### Prompts

Handle `prompts/list` and `prompts/get` methods for prompt templates.

**Responsibilities:**
- List available prompt templates
- Render prompts with parameter substitution
- Return formatted messages for LLM consumption
- Register prompts capability during initialization

**WIT Definition:**
```wit
world prompts-handler {
    include wasmcp:mcp/prompts-handler@<version>;
}
```

### Completion

Handle `completion/complete` method for argument completion.

**Responsibilities:**
- Provide completion suggestions for partial arguments
- Support context-aware completion
- Register completion capability during initialization

**WIT Definition:**
```wit
world completion-handler {
    include wasmcp:mcp/completion-handler@<version>;
}
```

Note: The world is `completion-handler` (singular) while the method is `completion/complete`.

## Composition Architecture

The `compose` command orchestrates component assembly using the Component Model's composition features:

1. **Dependency Resolution** - Downloads required wasmcp framework components from the registry
2. **Handler Detection** - Identifies handler types by inspecting WIT imports/exports
3. **Graph Construction** - Builds a composition graph with proper import wiring
4. **Instance Creation** - Instantiates components with resolved dependencies
5. **Chain Assembly** - Wires handlers in specified order with shared request context
6. **Encoding** - Produces a single executable component

**Transport Types:**

Servers support two transport types:

- **HTTP** (default) - via `wasmtime serve`
- **Stdio** - via `wasmtime run`

Both transports use the same handler composition, only differing in how they receive requests and send responses.

**Composition Graph:**

```
transport (http or stdio)
    ├─ imports: request, incoming-handler
    └─ exports: wasi:http/incoming-handler (http) or wasi:cli/run (stdio)
         │
         ├─> middleware-1 (if present)
         │     ├─ imports: incoming-handler, request
         │     └─ exports: incoming-handler
         │
         ├─> tools-handler (if present)
         │     ├─ imports: incoming-handler, request
         │     │           error-result, tools-list-result, tools-call-content
         │     └─ exports: incoming-handler
         │
         ├─> resources-handler (if present)
         │     ├─ imports: incoming-handler, request
         │     │           error-result, resources-list-result, resources-read-result
         │     └─ exports: incoming-handler
         │
         └─> initialize-handler (terminus)
               ├─ imports: request, initialize-result
               └─ exports: incoming-handler
```

**Request Flow:**

1. Request arrives at transport (HTTP or newline-delimited JSON-RPC on stdin)
2. Transport parses JSON-RPC, creates request resource
3. Request flows through middleware and handlers in order
4. Each handler either responds or forwards to next
5. Initialize-handler terminates chain
6. Response streams back through transport (HTTP response or newline-terminated JSON to stdout)

**Shared Context:**

All handlers share the same `request` resource instance, enabling:
- Capability registration via `request.needs(capability)`
- Stateless request inspection without copying
- Zero-copy resource passing through the chain

**Streaming Architecture:**

The composition uses direct stream writing for constant memory usage:

```
Client <─ HTTP ─> transport <─ OutputStream ─> handler
```

- Handlers write directly to the output stream (no buffering)
- Backpressure via `check_write()` prevents resource exhaustion
- Enables streaming large responses (MB+) with minimal memory
- Ideal for edge deployment with memory constraints

Response transformation middleware (compression, logging) cannot intercept handler output since the stream is write-only. This design trades flexibility for performance and constant memory usage.

## Registry Integration

Components can reference published packages via wkg-compatible registry URIs:

```bash
wasmcp compose \
  --tools wasmcp:weather-tools@0.1.0 \
  --resources ./local-resources.wasm
```

The CLI automatically:
1. Resolves package reference via `wkg get`
2. Downloads to `deps/` directory
3. Includes in composition

This enables sharing and reusing handlers across projects without manual component management.

## Build Integration

Generated projects include Makefiles with standard targets:

```bash
make build    # Build the component
make clean    # Remove build artifacts
make test     # Run tests (if present)
```

Language-specific build processes:

**Rust:**
```bash
cargo component build --release
# Output: target/wasm32-wasip2/release/<name>.wasm
```

**Go:**
```bash
wit-bindgen-go generate ./wit --out gen
tinygo build -o <name>.wasm -target=wasip2 .
```

**TypeScript:**
```bash
jco componentize app.ts -o <name>.wasm
```

**Python:**
```bash
componentize-py componentize app -o <name>.wasm
```

## Examples

### Create and Compose Multiple Handlers

```bash
# Create handlers
wasmcp new fs-tools --type tools --language rust
wasmcp new db-resources --type resources --language python
wasmcp new logging --type middleware --language go

# Build each
cd fs-tools && make build && cd ..
cd db-resources && make build && cd ..
cd logging && make build && cd ..

# Compose into single server
wasmcp compose \
  --middleware logging/target/logging.wasm \
  --tools fs-tools/target/wasm32-wasip2/release/fs_tools.wasm \
  --resources db-resources/target/db_resources.wasm

# Run
wasmtime serve -Scommon mcp-server.wasm
```

### Middleware Pipeline

```bash
# Create authentication, logging, and rate limiting middleware
wasmcp new auth --type middleware --language rust
wasmcp new logger --type middleware --language rust
wasmcp new ratelimit --type middleware --language rust

# Build and compose in specific order
wasmcp compose \
  --middleware logger/target/wasm32-wasip2/release/logger.wasm \
  --middleware auth/target/wasm32-wasip2/release/auth.wasm \
  --middleware ratelimit/target/wasm32-wasip2/release/ratelimit.wasm \
  --tools tools/target/wasm32-wasip2/release/tools.wasm

# Request flow: HTTP -> logger -> auth -> ratelimit -> tools -> initialize
```

### Registry and Local Composition

```bash
# Mix published and local components
wasmcp compose \
  --tools wasmcp:weather-tools@0.1.0 \
  --resources ./custom-resources/target/resources.wasm \
  --middleware ./logging/target/logging.wasm
```

### Version Compatibility

Component versions must match across the composition. The `--version` flag ensures consistent wasmcp framework versions:

```bash
wasmcp new my-handler --type tools --language rust --version 0.3.0-alpha.59
wasmcp compose --tools my-handler.wasm --version 0.3.0-alpha.59
```

Mismatched versions may cause interface incompatibilities during composition.

### Graph Construction

Composition uses `wac-graph` to build the component graph programmatically:

```rust
let mut graph = CompositionGraph::new();

// Register packages
let request_pkg = graph.register_package(request_package)?;
let handler_pkg = graph.register_package(handler_package)?;

// Instantiate and wire
let request_inst = graph.instantiate(request_pkg);
let handler_inst = graph.instantiate(handler_pkg);

graph.set_instantiation_argument(
    handler_inst,
    "wasmcp:mcp/request@0.3.0-alpha.59",
    graph.alias_instance_export(request_inst, "wasmcp:mcp/request@0.3.0-alpha.59")?,
)?;

// Encode to bytes
let encoded = graph.encode(EncodeOptions::default())?;
```

This provides full control over instance configuration and import wiring.

## Development

### Building the CLI

```bash
cargo build --release
```

### Running Tests

```bash
cargo test
```

### Code Organization

```
cli/
├── src/
│   ├── main.rs          # CLI argument parsing and dispatch
│   ├── compose.rs       # Component composition logic
│   └── scaffold.rs      # Project scaffolding logic
├── templates/           # Language-specific templates
│   ├── rust/
│   ├── go/
│   ├── typescript/
│   └── python/
└── Cargo.toml
```

Template files use Liquid templating with variables:
- `{{ project_name }}` - Project name
- `{{ package_name }}` - Package name (underscores)
- `{{ handler_type }}` - Handler type
- `{{ world_name }}` - WIT world name
- `{{ wasmcp_version }}` - Framework version

## See Also

- [wasmcp Framework Documentation](../README.md)
- [WIT Interface Reference](../wit/README.md)
- [Example Implementations](../examples/)
- [Component Model Specification](https://github.com/WebAssembly/component-model)
- [wac Composition Tool](https://github.com/bytecodealliance/wac)

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../LICENSE) for details.
