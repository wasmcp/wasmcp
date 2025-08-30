# weather-js-new

An MCP server written in JavaScript

## Getting Started

This is a JavaScript MCP (Model Context Protocol) server that runs as a WebAssembly component.

### Prerequisites

- Node.js 18+ and npm
- [Spin CLI](https://developer.fermyon.com/spin/install)
- [jco](https://github.com/bytecodealliance/jco) - Install with: `npm install -g @bytecodealliance/jco`
- [wac](https://github.com/bytecodealliance/wac) - Install with: `cargo install wac-cli`
- [wkg](https://github.com/bytecodealliance/wkg-cli) - Install with: `cargo install wkg`

### Setup & Building

```bash
# Check tools and install dependencies
make setup

# Build the composed component
make build

# Or build step by step:
npm install           # Install dependencies
npm run bundle        # Bundle JavaScript modules
npm run build         # Build handler component
make                  # Compose with server
```

### Running

```bash
# Run with Spin
spin up

# Or run with wasmtime
make run

# Or run already built component
make serve
```

### Testing

Test the MCP server endpoints:

```bash
# Test initialization
make test-init

# Test listing tools
make test-tools

# Test calling a tool
make test-call
```

## Adding Tools

To add new tools, edit `index.js`:

```javascript
export const myTool = createTool({
    name: 'my_tool',
    description: 'What the tool does',
    schema: {
        type: 'object',
        properties: {
            param: { type: 'string', description: 'Parameter description' }
        },
        required: ['param']
    },
    execute: async (args) => {
        // Tool implementation
        return `Result: ${args.param}`;
    }
});

// Add to the handler
export const toolHandler = createHandler({
    tools: [echoTool, myTool]
});
```

## Deployment

### Deploy to Spin Cloud

```bash
spin cloud deploy
```

### Run with Wasmtime

```bash
wasmtime serve -Scli composed.wasm
```

## Project Structure

```
.
├── index.js        # Main handler implementation
├── helpers.js      # Helper library for MCP
├── package.json    # Node.js dependencies
├── spin.toml       # Spin configuration
├── Makefile        # Build automation
├── wit/            # WebAssembly Interface Types
│   └── world.wit   # MCP world definition
└── composed.wasm   # Final composed component (after build)
```

## License

This project is built with [wasmcp](https://github.com/fastertools/wasmcp).