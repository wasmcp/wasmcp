# Building MCP Servers with wasmcp

Complete workflow for creating components, composing into servers, and running them.

## Create Component

**Command:**
```bash
wasmcp new <name> --language <rust|python|typescript> [--template-type <tools|resources|prompts>]
```

**Template types:**
- `tools` (default) - Execute actions, return results
- `resources` - Expose data, files, or state
- `prompts` - Provide prompt templates

**Languages:**
- `rust` - Rust with cargo component
- `python` - Python with componentize-py
- `typescript` - TypeScript with jco

**Examples:**
```bash
wasmcp new calculator --language rust
wasmcp new my-resources --language python --template-type resources
wasmcp new prompts --language typescript --template-type prompts
```

**Output:** Project directory with WIT bindings, example implementation, Makefile, README

## Build Component

**Command:**
```bash
make
```

**What it does:**
- Installs language dependencies (if needed)
- Generates WIT bindings
- Compiles to WebAssembly component

**Output locations:**
- Rust: `target/wasm32-wasip2/release/<name>.wasm`
- Python: `<name>.wasm` (project root)
- TypeScript: `dist/<name>.wasm`

**Prerequisites:**
- Rust: `rustup target add wasm32-wasip2`
- Python: `componentize-py` available
- TypeScript: `jco` available

## Compose Server

**Command:**
```bash
wasmcp compose server <components...> -o <output.wasm> [-t <transport>] [--force] [--verbose]
```

**Component input formats:**

- **Local paths:** `./component.wasm`, `target/wasm32-wasip2/release/calc.wasm`
- **OCI packages:** `wasmcp:calculator@0.1.0`, `namespace:name@version`
  - Format: `namespace:name@version` (NOT `ghcr.io/...`)
  - The `:` character identifies it as an OCI reference
  - Downloads from registry and caches in `~/.config/wasmcp/deps/`
- **Aliases:** `calc` (registered with `wasmcp registry component add`)
- **Profiles:** `dev` (saved with `wasmcp registry profile add`)

**Transport options:**
- `-t http` - HTTP transport (default), run with `wasmtime serve`
- `-t stdio` - Stdio transport, run with `wasmtime run`

**Examples:**
```bash
# From local path
wasmcp compose server ./calculator.wasm -o server.wasm

# From OCI package
wasmcp compose server wasmcp:calculator@0.1.0 -o server.wasm

# From alias
wasmcp compose server calc -o server.wasm

# Multiple components
wasmcp compose server calc strings weather -o combined.wasm

# Mixed formats
wasmcp compose server calc ./local.wasm wasmcp:remote@1.0 -o server.wasm

# Stdio transport
wasmcp compose server calc -t stdio -o server.wasm

# Force overwrite + verbose
wasmcp compose server calc -o server.wasm --force --verbose
```

**What composition does:**
1. Detects component exports (tools-capability, resources-capability, prompts-capability)
2. Wraps each with appropriate middleware (tools-middleware, resources-middleware, etc.)
3. Downloads framework components from `wasmcp` namespace (maps to `ghcr.io/wasmcp`)
4. Chains middleware into pipeline: transport → middleware(s) → method-not-found
5. Outputs single executable server component

**Registry location:** `~/.config/wasmcp/config.toml`

## Run Server

**Important:** Use `wasmtime` to run composed servers, NOT `wasmcp mcp serve`. The `wasmcp mcp serve` command runs the CLI's own development MCP server.

**HTTP transport:**
```bash
wasmtime serve -Scli server.wasm
```

- Listens on `http://0.0.0.0:8080/mcp`
- Accepts POST requests with JSON-RPC
- Use for remote MCP clients

**Stdio transport:**
```bash
wasmtime run server.wasm
```

- Reads JSON-RPC from stdin
- Writes JSON-RPC to stdout
- Use for local MCP clients (Claude Desktop, etc.)

## Test Server

**List tools:**
```bash
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "id": 1, "method": "tools/list"}'
```

**Call a tool:**
```bash
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/call",
    "params": {
      "name": "add",
      "arguments": {"a": 5, "b": 3}
    }
  }'
```

## Register Components

**Add alias:**
```bash
wasmcp registry component add <alias> <path-or-oci>
```

**Examples:**
```bash
wasmcp registry component add calc ./calculator.wasm
wasmcp registry component add strings wasmcp:string-tools@1.0.0
```

**List aliases:**
```bash
wasmcp registry component list
```

**Use in compose:**
```bash
wasmcp compose server calc strings -o server.wasm
```

## MCP Client Integration

**Claude Desktop (stdio):**
```json
{
  "mcpServers": {
    "my-server": {
      "command": "wasmtime",
      "args": ["run", "/path/to/server.wasm"]
    }
  }
}
```

**HTTP clients:** Connect to `http://localhost:8080/mcp`

## Complete Example

```bash
# Create and build component
wasmcp new calculator --language rust
cd calculator && make && cd ..

# Register it
wasmcp registry component add calc calculator/target/wasm32-wasip2/release/calculator.wasm

# Compose into server
wasmcp compose server calc -o server.wasm

# Run server
wasmtime serve -Scli server.wasm

# Test
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "id": 1, "method": "tools/list"}'
```

## Multiple Components

```bash
# Build multiple components
wasmcp new calc --language rust && cd calc && make && cd ..
wasmcp new strings --language python && cd strings && make && cd ..

# Register both
wasmcp registry component add calc calc/target/wasm32-wasip2/release/calc.wasm
wasmcp registry component add strings strings/strings.wasm

# Compose together
wasmcp compose server calc strings -o combined.wasm
wasmtime serve -Scli combined.wasm
```

Both components' tools/resources/prompts automatically merge into single unified catalog.

## Related Resources

- **Registry management:** See `registry` resource for aliases and profiles
- **CLI reference:** See `reference` resource for detailed command flags and options
- **Architecture:** See `architecture` resource for how composition pipeline works
