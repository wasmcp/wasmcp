# JavaScript Weather MCP Handler Example

This example demonstrates how to build an MCP handler in JavaScript using `jco` (JavaScript Component Tools).

## Features

This handler implements the same three tools as the Rust example:
- **echo** - Echo a message back to the user
- **get_weather** - Get current weather for a single location
- **multi_weather** - Get weather for multiple cities concurrently

## Prerequisites

- Node.js 18+ 
- `jco` (installed as dev dependency)
- `wac` for composition

## Project Structure

```
javascript/
├── weather-handler.js   # Main handler implementation
├── package.json         # Node.js dependencies and scripts
├── wit/                 # WIT interface definitions
│   ├── world.wit       # Handler world definition
│   └── deps/           # Downloaded MCP dependencies
└── wkg.lock            # Dependency lock file
```

## Building

```bash
# Install dependencies
npm install

# Fetch WIT dependencies (already done)
wkg wit fetch

# Build the component
npm run build
# Creates: weather-handler.wasm (12MB - includes JS runtime)

# Compose with server
npm run compose
# Creates: composed.wasm (ready to run)
```

## Implementation Notes

### JavaScript vs Rust

The JavaScript implementation:
- Uses familiar JavaScript syntax and patterns
- Includes a JavaScript runtime (StarlingMonkey) in the component
- Results in larger binaries (~12MB vs ~850KB for Rust)
- Supports async/await naturally
- Uses standard `fetch` API for HTTP requests

### WIT Bindings

The JavaScript handler exports the `toolHandler` object that implements:
- `handleListTools(request)` - Returns the list of available tools
- `handleCallTool(request)` - Executes the requested tool

The binding between JavaScript and WIT is handled by `jco componentize`.

### Key Differences from Rust

1. **No macros needed** - JavaScript's dynamic nature means no code generation
2. **Larger binary size** - Includes JS runtime overhead  
3. **Standard web APIs** - Uses `fetch` instead of spin-sdk
4. **Dynamic typing** - JSON parsing/stringification instead of serde

## Testing

Run the composed component with Spin:

```bash
spin up --from composed.wasm
```

Then test with the MCP client or direct JSON-RPC calls.

## Development Tips

1. **Type generation**: Run `npm run build:types` to generate TypeScript definitions
2. **Debugging**: The JS runtime in the component supports `console.log` with appropriate WASI imports
3. **Performance**: For production use cases requiring small binaries and high performance, consider Rust

## License

Apache-2.0