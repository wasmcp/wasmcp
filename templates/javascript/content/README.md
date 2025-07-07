# {{project-name | kebab_case}}

{{project-description}}

## Structure

This is an FTL tool that implements the Model Context Protocol (MCP) using WebAssembly components.

- `handler/` - The JavaScript implementation of your MCP handler
- `ftl.toml` - FTL configuration file
- `spin.toml` - Spin application manifest

## Development

### Prerequisites

- Node.js >= 20.0.0
- FTL CLI

### Building

```bash
ftl build
# or
make build
```

### Testing

```bash
ftl test
# or
make test
```

### Running Locally

```bash
ftl serve
# or
make serve
```

The tool will be available at `http://localhost:3000/mcp`

### Example Usage

```bash
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "{{project-name | snake_case}}",
      "arguments": {
        "input": "Hello, world!"
      }
    },
    "id": 1
  }'
```

## Deployment

```bash
ftl deploy
# or
make deploy
```

## Implementing Your Tool

Edit `handler/src/index.js` to implement your tool's functionality:

1. Modify `listTools()` to define your tools
2. Implement the tool logic in `callTool()`
3. Optionally implement resources and prompts

## Configuration

Edit `ftl.toml` to configure:
- Allowed external hosts
- Build optimization flags
- Other runtime settings