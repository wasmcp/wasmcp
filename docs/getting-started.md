# Getting Started with wasmcp

This guide walks you through creating your first MCP server component with wasmcp, from installation to running a complete server.

## Prerequisites

Before you begin, install:

1. **Rust** (latest stable with `wasm32-wasip2` target)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup target add wasm32-wasip2
   ```

2. **Wasmtime** (WebAssembly runtime)
   ```bash
   curl https://wasmtime.dev/install.sh -sSf | bash
   ```

## Install wasmcp CLI

**From releases** (recommended):
```bash
# Download from https://github.com/wasmcp/wasmcp/releases
# Extract and add to PATH
```

**From source:**
```bash
git clone https://github.com/wasmcp/wasmcp
cd wasmcp/cli
cargo build --release
# Binary at: target/release/wasmcp
```

Verify installation:
```bash
wasmcp --version
```

## Create Your First Component

### 1. Generate a Component

Create a calculator tool component in Rust:

```bash
wasmcp new calculator --language rust
cd calculator
```

This generates a complete project with:
- WIT bindings for the tools capability
- Example calculator tool implementation
- Makefile for building
- README with language-specific details

**Available languages:**
- `--language rust` - Rust (default)
- `--language python` - Python
- `--language typescript` - TypeScript

**Available component types (--template-type flag):**

```bash
# Tools component (default) - For executing actions
wasmcp new my-tools --language rust --template-type tools

# Resources component - For exposing data/files
wasmcp new my-resources --language rust --template-type resources

# Prompts component - For providing prompt templates
wasmcp new my-prompts --language rust --template-type prompts
```

If `--template-type` is omitted, it defaults to `tools`.

**Template locations:**
- Tools: `cli/templates/rust-tools/`, `cli/templates/python-tools/`, `cli/templates/typescript-tools/`
- Resources: `cli/templates/rust-resources/`, `cli/templates/python-resources/`, `cli/templates/typescript-resources/`
- Prompts: `cli/templates/rust-prompts/`, `cli/templates/python-prompts/`, `cli/templates/typescript-prompts/`

### 2. Build the Component

```bash
make
```

The Makefile automatically:
- Installs language-specific dependencies (if needed)
- Generates WIT bindings
- Builds the WebAssembly component

**Output location:**
- Rust: `target/wasm32-wasip2/release/calculator.wasm`
- Python: `calculator.wasm` (in project root)
- TypeScript: `dist/calculator.wasm`

### 3. Register with the CLI

Give your component a short alias for easy composition:

```bash
wasmcp registry component add calc target/wasm32-wasip2/release/calculator.wasm
```

This registers `calc` as an alias pointing to your component's full path.

**Registry location:** `~/.config/wasmcp/config.toml`

View registered components:
```bash
wasmcp registry component list
```

## Compose into an MCP Server

### 4. Compose

Create a complete MCP server from your component. You can compose from:
- **Local paths**: `./component.wasm`, `target/wasm32-wasip2/release/calc.wasm`
- **Registry aliases**: `calc` (registered with `wasmcp registry component add`)
- **OCI packages**: `wasmcp:calculator@0.1.0`, `namespace:component@version`

**Important**: OCI packages use the format `namespace:name@version` (NOT full registry URLs like `ghcr.io/...`)

```bash
# Using a registered alias
wasmcp compose calc -o server.wasm

# Using a local path
wasmcp compose ./target/wasm32-wasip2/release/calculator.wasm -o server.wasm

# Using an OCI package (downloads from registry)
wasmcp compose wasmcp:calculator@0.1.0 -o server.wasm
```

This:
1. Detects your component exports `tools-capability`
2. Wraps it with `tools-middleware` (from `crates/tools-middleware/`)
3. Adds HTTP transport (from `crates/http-transport/`)
4. Adds method-not-found handler (from `crates/method-not-found/`)
5. Composes them into a complete server pipeline

The CLI automatically downloads framework components from the `wasmcp` namespace (which maps to `ghcr.io/wasmcp`) on first use.

**Composition options:**
```bash
# Stdio transport (for local MCP clients)
wasmcp compose calc -t stdio -o server.wasm

# HTTP transport (default, for remote clients)
wasmcp compose calc -t http -o server.wasm

# Force overwrite existing output
wasmcp compose calc -o server.wasm --force

# Verbose logging
wasmcp compose calc -o server.wasm --verbose
```

See `cli/README.md` for advanced composition options.

## Run the Server

### 5. Start the Server

**Note**: Composed servers are run with `wasmtime` (the WebAssembly runtime), not `wasmcp`. The `wasmcp mcp serve` command is only for running the wasmcp CLI's own MCP server for development assistance.

**HTTP transport:**
```bash
wasmtime serve -Scli server.wasm
```

Server starts on `http://0.0.0.0:8080/mcp`

**Stdio transport:**
```bash
wasmtime run server.wasm
```

Reads MCP requests from stdin, writes responses to stdout.

### 6. Test the Server

Send a test request:

```bash
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/list"
  }'
```

You should see your calculator tools listed.

Call a tool:

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

## Compose Multiple Components

### 7. Create Another Component

```bash
cd ..
wasmcp new strings --language python
cd strings && make && cd ..
```

### 8. Register It

```bash
wasmcp registry component add strings strings/strings.wasm
```

### 9. Compose Both Together

```bash
wasmcp compose calc strings -o combined-server.wasm
wasmtime serve -Scli combined-server.wasm
```

Now both calculator AND string tools are available in a single server!

**How it works:**
- Both components export `tools-capability`
- CLI wraps each with `tools-middleware`
- Middleware components merge their tool lists automatically
- Single unified catalog presented to clients

This is the power of wasmcp's composition model.

## Use Composition Profiles

### 10. Save a Profile

For frequently-used component combinations:

```bash
wasmcp registry profile add dev calc strings -o dev-server.wasm
```

### 11. Reuse the Profile

```bash
wasmcp compose dev
# Creates: ~/.config/wasmcp/composed/dev-server.wasm

# Or specify output location:
wasmcp compose dev -o ./my-server.wasm
```

**Profile inheritance:**
```bash
# Create prod profile that extends dev
wasmcp registry profile add prod logger monitor -o prod.wasm -b dev
# prod now includes: calc + strings + logger + monitor
```

View profiles:
```bash
wasmcp registry profile list
wasmcp registry info --profiles
```

## Next Steps

### Learn More

- **Architecture:** Understand the composition model in `docs/architecture.md`
- **Examples:** Study working examples in `examples/` directory (see `docs/examples.md`)
- **CLI Reference:** Detailed command documentation in `cli/README.md`
- **WIT Interfaces:** Component interface specifications in `docs/wit-interfaces.md`

### Explore Capabilities

Create components with different capabilities:

```bash
# Resource component (expose data/files)
wasmcp new my-resources --language rust
# Edit to export resources-capability

# Prompt component (provide prompt templates)
wasmcp new my-prompts --language python
# Edit to export prompts-capability
```

See template READMEs in `cli/templates/rust-resources/`, `cli/templates/python-prompts/`, etc.

### Build Framework Components

Want to build custom middleware or transports?

- Study existing framework components in `crates/`
- Reference `wit/server/` interfaces
- See `docs/architecture.md` for the handler pattern

### Integrate with MCP Clients

**Claude Desktop** (`claude_desktop_config.json`):
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

**HTTP Clients:** Point to `http://localhost:8080/mcp`

### Explore Registry Features

```bash
# View all registry info
wasmcp registry info

# Manage components
wasmcp registry component add <alias> <path>
wasmcp registry component remove <alias>
wasmcp registry component list

# Manage profiles
wasmcp registry profile add <name> <components...> -o <output>
wasmcp registry profile remove <name>
wasmcp registry profile list
```

## Troubleshooting

### Component won't build

Check language-specific requirements:
- **Rust:** Ensure `wasm32-wasip2` target is installed
- **Python:** Ensure `componentize-py` is available
- **TypeScript:** Ensure `jco` is available

See generated `README.md` in your component directory.

### Composition fails

- Verify components exist at registered paths
- Try `--verbose` flag to see detailed composition steps
- Check framework components downloaded to `~/.config/wasmcp/deps/`

### Server won't start

- Verify Wasmtime is installed and in PATH
- Check you're using the right transport (HTTP vs stdio)
- Try `wasmtime --version` to ensure it's recent enough

### Can't connect to server

- HTTP: Ensure server is running on expected port (default 8080)
- Stdio: Ensure client is configured for stdio transport
- Check firewall/network settings for HTTP

## Getting Help

- **Documentation:** Browse `docs/` directory
- **Examples:** See working code in `examples/`
- **Issues:** Report bugs at https://github.com/wasmcp/wasmcp/issues
- **Contributing:** See `CONTRIBUTING.md`

---

**Welcome to wasmcp!** You've created, composed, and run your first MCP server. Explore the examples and documentation to learn more about building composable MCP components.
