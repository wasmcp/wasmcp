# Hash MCP Server (Go)

A simple MCP server that provides cryptographic hash tools, demonstrating how to build MCP capabilities using Go and WebAssembly Components.

## Features

Provides three hash tools:
- **sha256**: Compute SHA-256 hash of text
- **md5**: Compute MD5 hash of text
- **sha1**: Compute SHA-1 hash of text

## Prerequisites

- [Go](https://go.dev/) 1.23 or later
- [TinyGo](https://tinygo.org/) 0.34.0 or later
- Make (optional, but recommended)

## Building

### Using Make (Recommended)

```bash
make build
```

This will:
1. Generate Go bindings from WIT files
2. Apply codegen fix for wit-bindgen-go bug (see below)
3. Build the WebAssembly component

### Manual Build

```bash
# Generate bindings
go generate ./...

# Apply codegen fix
./fix-codegen.sh

# Build component (must specify WIT package and world)
tinygo build -target=wasip2 -wit-package ./wit -wit-world hash -o hash.wasm .
```

## Known Issues

### wit-bindgen-go Code Generation Bug

This example works around a code generation bug in wit-bindgen-go related to `option<borrow<resource>>` types in cross-interface records. See [BUG_REPORT.md](./BUG_REPORT.md) for details.

**The Issue:** When `ClientContext` (defined in `wasmcp:mcp/protocol`) is used in the `tools-capability` interface, the generated lift function has the wrong return type.

**The Fix:** The `fix-codegen.sh` script automatically patches the generated code after running `go generate`. This is handled automatically by the Makefile.

**Status:** Issue submitted to [bytecodealliance/go-modules](https://github.com/bytecodealliance/go-modules). This workaround will be removed once the upstream bug is fixed.

## Project Structure

```
hash-go/
├── main.go              # Implementation of hash tools
├── go.mod               # Go module definition
├── wit/                 # WIT interface definitions
│   ├── world.wit        # World definition
│   └── deps/            # WIT dependencies
├── gen/                 # Generated Go bindings (git-ignored)
├── fix-codegen.sh       # Codegen bug workaround script
├── BUG_REPORT.md        # Detailed bug report
├── Makefile             # Build automation
└── README.md            # This file
```

## Usage

Once built, the `hash.wasm` component can be composed into an MCP server:

```bash
# Using wasmcp CLI (example)
wasmcp serve hash.wasm

# Or compose with other capabilities
wasmcp compose \
  --tools hash.wasm \
  --resources file-system.wasm \
  --output my-server.wasm
```

## Implementation Notes

### Tool Registration

Tools are registered in `init()`:

```go
func init() {
    toolscapability.Exports.ListTools = listTools
    toolscapability.Exports.CallTool = callTool
}
```

### Tool Definition

Each tool provides:
- **Name**: Unique identifier
- **Input Schema**: JSON Schema for arguments
- **Options**: Description, title, etc.

Example:
```go
func createSHA256Tool() protocol.Tool {
    return protocol.Tool{
        Name: "sha256",
        InputSchema: `{
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Text to hash"
                }
            },
            "required": ["text"]
        }`,
        Options: cm.Some(protocol.ToolOptions{
            Description: cm.Some[string]("Compute SHA-256 hash of text"),
            Title: cm.Some[string]("SHA-256 Hash"),
            // ...
        }),
    }
}
```

### Tool Execution

The `callTool` function:
1. Checks if the tool name matches
2. Returns `Some(result)` if handled
3. Returns `None` to delegate to next capability

```go
func callTool(request protocol.CallToolRequest, client protocol.ClientContext) cm.Option[protocol.CallToolResult] {
    switch request.Name {
    case "sha256":
        return cm.Some(executeSHA256(request))
    default:
        return cm.None[protocol.CallToolResult]()
    }
}
```

### Error Handling

Use structured results with `IsError` flag:

```go
func errorResult(message string) protocol.CallToolResult {
    return protocol.CallToolResult{
        Content: cm.ToList([]protocol.ContentBlock{
            protocol.ContentBlockText(protocol.TextContent{
                Text: protocol.TextDataText(message),
            }),
        }),
        IsError: cm.Some(true),
    }
}
```

## Development

### Clean Build

```bash
make clean
make build
```

### Regenerate Bindings

```bash
make generate
```

This runs `go generate` and applies the codegen fix automatically.

## License

See [LICENSE](../../LICENSE) in the repository root.
