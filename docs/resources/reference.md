# CLI Reference

Quick reference for wasmcp commands, component formats, template types, and configuration.

## CLI Commands

### wasmcp new

Create new component from template.

**Syntax:**
```bash
wasmcp new <name> --language <lang> [--template-type <type>] [--version <ver>]
```

**Options:**
- `--language <rust|python|typescript>` - Language for component (default: rust)
- `--template-type <tools|resources|prompts>` - Component capability type (default: tools)
- `--version <version>` - wasmcp framework version (default: latest)

**Examples:**
```bash
wasmcp new calculator --language rust
wasmcp new my-resources --language python --template-type resources
wasmcp new prompts --language typescript --template-type prompts --version 0.4.0
```

### wasmcp compose

Compose components into MCP server.

**Syntax:**
```bash
wasmcp compose <components...> -o <output> [options]
```

**Options:**
- `-o, --output <path>` - Output file path
- `-t, --transport <http|stdio>` - Transport type (default: http)
- `--force` - Overwrite existing output file
- `--verbose` - Enable verbose logging
- `--version <version>` - Framework version for dependencies
- `--deps-dir <path>` - Custom dependency directory
- `--skip-download` - Skip downloading dependencies
- `--override-transport <path>` - Custom transport component
- `--override-method-not-found <path>` - Custom method-not-found handler

**Examples:**
```bash
wasmcp compose calc strings -o server.wasm
wasmcp compose calc -t stdio -o server.wasm
wasmcp compose calc --force --verbose -o server.wasm
wasmcp compose calc --override-transport custom.wasm -o server.wasm
```

### wasmcp registry component

Manage component aliases.

**Add:**
```bash
wasmcp registry component add <alias> <path-or-oci>
```

**List:**
```bash
wasmcp registry component list
```

**Remove:**
```bash
wasmcp registry component remove <alias>
```

### wasmcp registry profile

Manage composition profiles.

**Add:**
```bash
wasmcp registry profile add <name> <components...> -o <output> [-b <base>]
```

**List:**
```bash
wasmcp registry profile list
```

**Remove:**
```bash
wasmcp registry profile remove <name>
```

### wasmcp registry info

View registry configuration.

**Syntax:**
```bash
wasmcp registry info [--components|-c] [--profiles|-p]
```

**Examples:**
```bash
wasmcp registry info              # Show all
wasmcp registry info --components # Components only
wasmcp registry info -p           # Profiles only
```

### wasmcp mcp serve

Run wasmcp CLI's development MCP server.

**Important:** This is NOT for running composed servers. Use `wasmtime` to run composed servers.

**Syntax:**
```bash
wasmcp mcp serve [options]
```

**Options:**
- `--port <port>` - HTTP port (default: 8085)
- `--stdio` - Use stdio transport instead of HTTP
- `-v, --verbose` - Enable verbose logging

**Examples:**
```bash
wasmcp mcp serve                # HTTP on port 8085
wasmcp mcp serve --port 9000    # Custom port
wasmcp mcp serve --stdio        # Stdio transport
```

## Component Specification Formats

### Local Paths

Detected by: starts with `./`, `../`, `~/`, `/` or contains `/` or `\\` or ends with `.wasm`

**Examples:**
```bash
./component.wasm
../target/handler.wasm
/abs/path/component.wasm
~/projects/handler.wasm
handler.wasm
```

### OCI Packages

Detected by: contains `:` character

**Format:** `namespace:name[@version]`

**Examples:**
```bash
wasmcp:calculator@0.1.0          # With version (recommended)
wasmcp:calculator                # Latest version
namespace:handler@2.0.0          # Custom namespace
```

**Important:** Use `namespace:name@version` format, NOT full registry URLs like `ghcr.io/...`

**Registry mapping:**
- `wasmcp:*` → `ghcr.io/wasmcp/*`
- `namespace:*` → `ghcr.io/namespace/*`

**Cache location:** `~/.config/wasmcp/deps/`

### Aliases

Detected by: registered in `~/.config/wasmcp/config.toml`, no special characters

**Examples:**
```bash
calc
strings
weather
```

### Profiles

Detected by: registered as profile in config file

**Examples:**
```bash
dev
prod
```

### Resolution Order

1. Check if profile exists
2. Check if alias exists
3. Check if local path exists
4. Parse as OCI package (if contains `:`)
5. Error if not found

## Template Types

### tools

**Exports:** `tools-capability`

**Purpose:** Execute actions and return results

**WIT interface:**
```wit
interface tools-capability {
  list-tools: func() -> result<tools-list, error>
  call-tool: func(name: string, arguments: string) -> result<call-result, error>
}
```

**Examples:** Calculator, string manipulation, file operations

**Template locations:**
- `cli/templates/rust-tools/`
- `cli/templates/python-tools/`
- `cli/templates/typescript-tools/`

### resources

**Exports:** `resources-capability`

**Purpose:** Expose data, files, or application state

**WIT interface:**
```wit
interface resources-capability {
  list-resources: func() -> result<resources-list, error>
  read-resource: func(uri: string) -> result<resource-contents, error>
}
```

**Examples:** Configuration files, database schemas, API documentation

**Template locations:**
- `cli/templates/rust-resources/`
- `cli/templates/python-resources/`
- `cli/templates/typescript-resources/`

### prompts

**Exports:** `prompts-capability`

**Purpose:** Provide prompt templates for AI interactions

**WIT interface:**
```wit
interface prompts-capability {
  list-prompts: func() -> result<prompts-list, error>
  get-prompt: func(name: string, arguments: string) -> result<prompt, error>
}
```

**Examples:** Code review templates, documentation generators, analysis frameworks

**Template locations:**
- `cli/templates/rust-prompts/`
- `cli/templates/python-prompts/`
- `cli/templates/typescript-prompts/`

## Transport Options

### HTTP Transport

**Flag:** `-t http` (default)

**Runtime:**
```bash
wasmtime serve -Scli server.wasm
```

**Default endpoint:** `http://0.0.0.0:8080/mcp`

**Use for:** Remote MCP clients, web integrations, multi-client access

**Protocol:** JSON-RPC over HTTP POST

### Stdio Transport

**Flag:** `-t stdio`

**Runtime:**
```bash
wasmtime run server.wasm
```

**Use for:** Local MCP clients (Claude Desktop, etc.), single-client access

**Protocol:** JSON-RPC via stdin/stdout (one message per line)

### Custom Transports

**Override with:**
```bash
wasmcp compose calc --override-transport custom-transport.wasm
```

**Requirements:**
- Export `handle: func(request: string) -> result<string, error>`
- Import `handle` from first middleware component

## Configuration Files

### Registry Config

**Location:** `~/.config/wasmcp/config.toml`

**Format:**
```toml
[components]
calc = "/absolute/path/to/calculator.wasm"
strings = "wasmcp:string-tools@1.0.0"
weather = "calc"  # Alias to alias

[profiles.dev]
components = ["calc", "strings"]
output = "dev-server.wasm"

[profiles.prod]
base = "dev"
components = ["monitor", "logger"]
output = "prod-server.wasm"
```

### MCP Client Config (Claude Desktop)

**Location:** `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS)

**Format (stdio):**
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

**Format (HTTP):**
```json
{
  "mcpServers": {
    "my-server": {
      "type": "http",
      "url": "http://localhost:8080/mcp"
    }
  }
}
```

## Directory Structure

**wasmcp config:**
- `~/.config/wasmcp/config.toml` - Registry configuration
- `~/.config/wasmcp/composed/` - Profile output files
- `~/.config/wasmcp/deps/` - Downloaded OCI packages and framework components

**Component output:**
- Rust: `target/wasm32-wasip2/release/<name>.wasm`
- Python: `<name>.wasm` (project root)
- TypeScript: `dist/<name>.wasm`

## Wasmtime Commands

**Run HTTP server:**
```bash
wasmtime serve -Scli server.wasm
```

**Run stdio server:**
```bash
wasmtime run server.wasm
```

**Check version:**
```bash
wasmtime --version
```

**Installation:**
```bash
curl https://wasmtime.dev/install.sh -sSf | bash
```

## Build Prerequisites

### Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-wasip2
```

### Python
```bash
# componentize-py installed automatically by Makefile
pip install componentize-py
```

### TypeScript
```bash
# jco installed automatically by Makefile
npm install -g @bytecodealliance/jco
```

## Component Detection

CLI automatically detects component type by inspecting exports:

**Exports `tools-capability`:**
- Wraps with `tools-middleware`

**Exports `resources-capability`:**
- Wraps with `resources-middleware`

**Exports `prompts-capability`:**
- Wraps with `prompts-middleware`

**Exports `handler`:**
- Uses as-is (already middleware)

**Multiple capabilities:**
- Wraps with multiple middleware layers

## Version Compatibility

**Specify version when creating:**
```bash
wasmcp new calc --version 0.4.0
```

**Specify version when composing:**
```bash
wasmcp compose calc --version 0.4.0
```

**Version used for:**
- Framework component downloads (transport, middleware, method-not-found)
- WIT interface compatibility
- Component model features

## Error Messages

**Component not found:**
- Verify path exists or alias is registered
- Check `wasmcp registry component list`

**Composition fails:**
- Try `--verbose` flag for detailed output
- Verify framework components in `~/.config/wasmcp/deps/`
- Check wasmtime version is recent

**Server won't start:**
- Verify wasmtime is installed and in PATH
- Check correct transport (HTTP vs stdio)
- Ensure port 8080 is not in use (HTTP)

## Related Resources

- **Building servers:** See `building-servers` resource for complete workflow
- **Registry:** See `registry` resource for aliases and profiles
- **Architecture:** See `architecture` resource for composition pipeline details
