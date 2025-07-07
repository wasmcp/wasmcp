# FTL TypeScript SDK

SDK for building MCP (Model Context Protocol) handler components in TypeScript/JavaScript.

## Installation

```bash
npm install @fastertools/ftl-sdk
```

## Usage

This SDK provides types and utilities to help you implement MCP handlers that can be compiled to WebAssembly components using `jco`.

### 1. Create a new project

```bash
mkdir my-mcp-handler
cd my-mcp-handler
npm init -y
npm install @fastertools/ftl-sdk
npm install -D @bytecodealliance/jco typescript
```

### 2. Set up your project

Create a `tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ES2022",
    "moduleResolution": "node",
    "outDir": "./dist",
    "strict": true
  }
}
```

Add to `package.json`:

```json
{
  "type": "module",
  "scripts": {
    "build": "npm run build:types && npm run build:js && npm run build:component",
    "build:types": "jco types ./wit/mcp.wit -o generated",
    "build:js": "tsc",
    "build:component": "jco componentize dist/index.js --wit ./wit/mcp.wit --world-name mcp-handler --out handler.wasm"
  }
}
```

### 3. Copy WIT files

Copy the MCP WIT files from the ftl-components repository to your project's `wit` directory.

### 4. Implement your handler

```typescript
import { createHandler, createTool, Tool } from '@fastertools/ftl-sdk';

const helloTool: Tool = createTool({
  name: 'hello',
  description: 'Says hello',
  inputSchema: {
    type: 'object',
    properties: {
      name: { type: 'string' }
    }
  },
  execute: async (args) => {
    const name = args.name || 'World';
    return `Hello, ${name}!`;
  }
});

export const handler = createHandler({
  tools: [helloTool],
  resources: [],
  prompts: []
});

// Export the handler methods for jco
export const {
  listTools,
  callTool,
  listResources,
  readResource,
  listPrompts,
  getPrompt
} = handler;
```

### 5. Build your component

```bash
npm run build
```

This will generate a `handler.wasm` file that can be used with the mcp-http-component gateway.

## Features

The SDK provides:
- TypeScript types: `Tool`, `Resource`, `Prompt`
- Factory functions: `createTool()`, `createResource()`, `createPrompt()`
- The `createHandler()` function to create a handler implementation
- Full TypeScript type safety

## Example

See the [examples](../../examples) directory for complete working examples.