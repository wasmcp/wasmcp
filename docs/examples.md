# wasmcp Examples

This document provides an overview of the example components in the `examples/` directory. Each demonstrates a different language and capability pattern.

## Available Examples

### Calculator (Rust)

**Location:** `examples/calculator-rs/`
**Language:** Rust
**Capability:** Tools

A basic calculator tool component demonstrating Rust component development.

**Implements:**
- `add` - Add two numbers
- `subtract` - Subtract two numbers
- `multiply` - Multiply two numbers
- `divide` - Divide two numbers (with error handling)

**Demonstrates:**
- Rust WIT bindings with `wit-bindgen`
- Tools capability interface
- Error handling in tool execution
- Building with `cargo component`

**Build:**
```bash
cd examples/calculator-rs
make
# Output: target/wasm32-wasip2/release/calculator_rs.wasm
```

**Run:**
```bash
wasmcp compose ../calculator-rs/target/wasm32-wasip2/release/calculator_rs.wasm -o calc.wasm
wasmtime serve -Scli calc.wasm
```

**What to learn:**
- How Rust components export WIT interfaces
- Tool argument parsing and validation
- Rust error types mapped to MCP errors
- Cargo.toml configuration for components

**Template:** Based on `cli/templates/rust-tools/`

---

### String Tools (Python)

**Location:** `examples/strings-py/`
**Language:** Python
**Capability:** Tools

String manipulation tools demonstrating Python component development.

**Implements:**
- `uppercase` - Convert string to uppercase
- `lowercase` - Convert string to lowercase
- `reverse` - Reverse a string
- `count_chars` - Count characters in a string

**Demonstrates:**
- Python WIT bindings with `componentize-py`
- Tools capability interface in Python
- Python type hints for WIT types
- Building Python components

**Build:**
```bash
cd examples/strings-py
make
# Output: strings_py.wasm (in project root)
```

**Run:**
```bash
wasmcp compose ../strings-py/strings_py.wasm -o strings.wasm
wasmtime serve -Scli strings.wasm
```

**What to learn:**
- How Python components work with WIT
- Python-specific component build process
- Handling JSON arguments in Python
- Dependencies and packaging for components

**Template:** Based on `cli/templates/python-tools/`

---

### Weather (TypeScript)

**Location:** `examples/weather-ts/`
**Language:** TypeScript
**Capability:** Tools

Weather tool component demonstrating TypeScript component development.

**Implements:**
- `get_weather` - Get weather for a location
- `get_forecast` - Get multi-day forecast

**Demonstrates:**
- TypeScript WIT bindings with `jco`
- Tools capability interface in TypeScript
- Async operations in components
- Building TypeScript components with `jco`

**Build:**
```bash
cd examples/weather-ts
make
# Output: dist/weather_ts.wasm
```

**Run:**
```bash
wasmcp compose ../weather-ts/dist/weather_ts.wasm -o weather.wasm
wasmtime serve -Scli weather.wasm
```

**What to learn:**
- How TypeScript components interact with WIT
- JavaScript/TypeScript build tooling for components
- Handling async operations
- Type safety with TypeScript and WIT

**Template:** Based on `cli/templates/typescript-tools/`

---

## Combining Examples

One of wasmcp's key features is composing multiple components into a unified server:

### Compose All Three

```bash
wasmcp compose \
  examples/calculator-rs/target/wasm32-wasip2/release/calculator_rs.wasm \
  examples/strings-py/strings_py.wasm \
  examples/weather-ts/dist/weather_ts.wasm \
  -o all-examples.wasm

wasmtime serve -Scli all-examples.wasm
```

Now a single MCP server provides:
- Calculator tools (from Rust component)
- String tools (from Python component)
- Weather tools (from TypeScript component)

All three components' tools are merged into one unified catalog automatically.

### Using Registry

Register each example:

```bash
wasmcp registry component add calc examples/calculator-rs/target/wasm32-wasip2/release/calculator_rs.wasm
wasmcp registry component add strings examples/strings-py/strings_py.wasm
wasmcp registry component add weather examples/weather-ts/dist/weather_ts.wasm
```

Then compose by alias:

```bash
wasmcp compose calc strings weather -o examples-server.wasm
```

Or save as a profile:

```bash
wasmcp registry profile add examples calc strings weather -o examples.wasm

# Later, rebuild quickly:
wasmcp compose examples
```

## Learning Path

### 1. Start with Your Preferred Language

Pick the example in your favorite language and study it:

- **Rust developers:** Start with `calculator-rs/`
- **Python developers:** Start with `strings-py/`
- **TypeScript developers:** Start with `weather-ts/`

Read the generated `README.md` in each example directory for language-specific details.

### 2. Understand the Capability Pattern

Each example exports the `tools-capability` interface defined in `wit/protocol/features.wit`.

Look at how each language:
- Generates bindings from WIT
- Implements the capability interface
- Handles tool arguments and errors
- Returns results in MCP format

### 3. Build and Modify

Try modifying an example:

1. Add a new tool to the component
2. Rebuild with `make`
3. Compose into a server
4. Test the new tool

This hands-on experience solidifies understanding.

### 4. Explore Other Capabilities

The examples all use tools, but wasmcp supports:

- **Resources** - Expose data/files (see `cli/templates/*-resources/`)
- **Prompts** - Provide prompt templates (see `cli/templates/*-prompts/`)

Try creating a resource or prompt component in your language.

### 5. Study Composition

Understand how components compose:

1. Build all three examples
2. Compose them together
3. Query `tools/list` from the composed server
4. See how all tools appear in one catalog

Read `docs/architecture.md` to understand the middleware chain.

## Example Use Cases

### Building a Development Server

Combine tool components for your workflow:

```bash
# Calculator for quick math
wasmcp registry component add calc examples/calculator-rs/...

# String tools for text manipulation
wasmcp registry component add strings examples/strings-py/...

# Custom file operations component
wasmcp new file-ops --language rust
# ... implement file tools ...
wasmcp registry component add files file-ops/...

# Compose into dev server
wasmcp compose calc strings files -o dev-server.wasm
```

### Language-Specific Examples

Want to see all Python capabilities?

```bash
# Tools
cd examples/strings-py && make

# Resources
wasmcp new py-resources --language python
# Study cli/templates/python-resources/README.md

# Prompts
wasmcp new py-prompts --language python
# Study cli/templates/python-prompts/README.md
```

## Beyond the Examples

### Template Projects

Each example is based on a template in `cli/templates/`:

**Tools Templates:**
- `cli/templates/rust-tools/` - Rust tools (calculator example)
- `cli/templates/python-tools/` - Python tools (strings example)
- `cli/templates/typescript-tools/` - TypeScript tools (weather example)

**Resource Templates:**
- `cli/templates/rust-resources/`
- `cli/templates/python-resources/`
- `cli/templates/typescript-resources/`

**Prompt Templates:**
- `cli/templates/rust-prompts/`
- `cli/templates/python-prompts/`
- `cli/templates/typescript-prompts/`

When you run `wasmcp new`, you get a copy of these templates with your project name.

### Creating Custom Components

Use the examples as a reference for creating your own components:

1. **Choose a language** based on the example you're most comfortable with
2. **Choose a capability** (tools, resources, or prompts)
3. **Generate with `wasmcp new`** to get template code
4. **Study the example** in that language for patterns
5. **Implement your logic** in place of the example implementation

### Framework Components

Want to see how wasmcp itself works?

Study the framework components in `crates/`:

- `crates/tools-middleware/` - How tools are wrapped
- `crates/resources-middleware/` - How resources are wrapped
- `crates/prompts-middleware/` - How prompts are wrapped
- `crates/http-transport/` - HTTP server transport
- `crates/stdio-transport/` - Stdio transport
- `crates/method-not-found/` - Default error handler

These components use the server interfaces from `wit/server/`.

## Testing Examples

### Manual Testing

```bash
# Start the server
wasmtime serve -Scli examples-server.wasm

# List tools
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "id": 1, "method": "tools/list"}'

# Call calculator
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/call",
    "params": {"name": "add", "arguments": {"a": 5, "b": 3}}
  }'

# Call string tool
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {"name": "uppercase", "arguments": {"text": "hello world"}}
  }'
```

### Integration Testing

See each example's tests:

- Rust: `cargo test` in example directory
- Python: Component-level testing
- TypeScript: `npm test` in example directory

## Contributing Examples

Want to add a new example?

1. Choose a capability and language not yet represented
2. Create it with `wasmcp new`
3. Implement something interesting and educational
4. Add it to `examples/` directory
5. Update this documentation
6. Open a pull request

See `CONTRIBUTING.md` for guidelines.

## Questions?

- **Architecture:** How does composition work? See `docs/architecture.md`
- **WIT Interfaces:** What interfaces do components use? See `docs/wit-interfaces.md`
- **Getting Started:** Step-by-step tutorial in `docs/getting-started.md`
- **CLI Reference:** Detailed commands in `cli/README.md`

---

**Happy composing!** These examples demonstrate the core patterns of wasmcp component development. Use them as a starting point for building your own MCP servers.
