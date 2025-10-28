# {{project_name}}

MCP resources capability component in TypeScript.

## Prerequisites

- [Node.js](https://nodejs.org/) 18 or later
- npm (comes with Node.js)

## Build

```bash
npm install
npm run build
```

Or use make:

```bash
make  # Output: target/{{project_name}}.wasm
```

## Compose

```bash
wasmcp compose server target/{{project_name}}.wasm -o server.wasm
```

The CLI automatically detects this is a resources-capability component and wraps it with resources-middleware.

## Run

```bash
# HTTP
wasmtime serve -Scli server.wasm

# Stdio
wasmcp compose server target/{{project_name}}.wasm -t stdio -o server.wasm
wasmtime run server.wasm
```

## Implementation

This component uses the **capability pattern**, implementing three methods from the `resources-capability` interface:

- `list_resources()` - Returns all resources this component provides
- `read_resource()` - Returns resource content by URI, or `null` if not handled
- `list_resource_templates()` - Returns URI templates (empty for static resources)

See `src/index.ts` for a simple text resources implementation demonstrating:
- Resource definitions with URIs and metadata
- Static content serving
- No protocol handling or delegation code

The resources-middleware automatically handles:
- MCP protocol translation
- Merging resources from multiple components
- Request delegation to downstream components
- Error handling and response formatting

## Adding Resources

To add new resources:

1. Add a `Resource` entry to the array in `listResources()`:

```typescript
{
  uri: 'text://my-resource',
  name: 'My Resource',
  mimeType: 'text/plain',
  options: {
    description: 'Description of my resource',
  },
}
```

2. Add a case in `readResource()`:

```typescript
case 'text://my-resource':
  return textResource('My resource content');
```

3. That's it! No need to handle merging, delegation, or protocol details - the middleware does that for you.

## Project Structure

```
{{project_name}}/
├── src/
│   ├── index.ts           # Your resource implementations
│   └── generated/         # Generated TypeScript bindings (git-ignored)
├── wit/
│   ├── world.wit          # WIT world definition
│   └── deps.toml          # WIT dependencies
├── package.json           # Node.js dependencies and scripts
├── tsconfig.json          # TypeScript configuration
├── webpack.config.js      # Webpack bundler configuration
└── Makefile               # Build automation
```

## Clean Build

```bash
# Remove all build artifacts and dependencies
make clean

# Fresh build
make
```
