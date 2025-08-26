# wasmcp

TypeScript SDK for MCP WebAssembly components. WIT files bundled, Zod schemas built-in.

[![npm version](https://img.shields.io/npm/v/wasmcp.svg)](https://www.npmjs.com/package/wasmcp)

## Installation

```bash
npm install wasmcp
```

## Usage

```typescript
import { createTool, createHandler, z } from 'wasmcp';

// Define tools with Zod schemas
const echoTool = createTool({
  name: 'echo',
  description: 'Echo a message back',
  schema: z.object({
    message: z.string().describe('Message to echo')
  }),
  execute: async (args) => {
    // TypeScript knows args is { message: string }
    return `Echo: ${args.message}`;
  }
});

// Weather tool with async fetch
const weatherTool = createTool({
  name: 'weather',
  description: 'Get weather for a location',
  schema: z.object({
    location: z.string().describe('City name')
  }),
  execute: async (args) => {
    const response = await fetch(
      `https://api.weather.com?location=${args.location}`
    );
    const data = await response.json();
    return `Weather in ${args.location}: ${data.temperature}°C`;
  }
});

// Export handler
export const handler = createHandler({
  tools: [echoTool, weatherTool]
});
```

## Features

- **Type-safe**: Full TypeScript with inference from Zod schemas
- **Runtime validation**: Automatic input validation with helpful errors
- **Async support**: Native async/await with fetch API
- **WIT bundled**: All WebAssembly interfaces included in npm package
- **Zero config**: Works out of the box with jco

## Class-based API

For complex tools with internal state:

```typescript
import { Tool, z } from 'wasmcp';

class DatabaseTool extends Tool {
  readonly name = 'db_query';
  readonly description = 'Query database';
  
  readonly schema = z.object({
    query: z.string(),
    limit: z.number().default(10)
  });

  private connection: any;

  async execute(args) {
    if (!this.connection) {
      this.connection = await this.connect();
    }
    return await this.connection.query(args.query, args.limit);
  }
  
  private async connect() {
    // Connection logic
  }
}

export const handler = createHandler({
  tools: [DatabaseTool]
});
```

## Building

```json
{
  "scripts": {
    "build": "jco componentize dist/index.js -w wit/world.wit -o handler.wasm"
  }
}
```

```bash
npm run build
```

The resulting WASM component works with any MCP gateway.

## Zod Integration

Built-in Zod provides:
- **Type inference**: No manual TypeScript types needed
- **Rich validation**: Email, URL, regex, custom refinements
- **Helpful errors**: User-friendly validation messages
- **JSON Schema**: Automatic conversion for MCP protocol

Example with validation:

```typescript
const emailTool = createTool({
  name: 'send_email',
  description: 'Send an email',
  schema: z.object({
    to: z.string().email('Invalid email'),
    subject: z.string().max(100, 'Subject too long'),
    priority: z.enum(['low', 'normal', 'high']).default('normal')
  }),
  execute: async (args) => {
    // args is fully typed and validated
    return `Email sent to ${args.to}`;
  }
});
```

## Resources & Prompts

```typescript
import { Resource, Prompt } from 'wasmcp';

// Resources provide data
class ConfigResource extends Resource {
  readonly uri = 'config://app';
  readonly name = 'App Config';
  
  read() {
    return JSON.stringify({ version: '1.0' });
  }
}

// Prompts generate messages
class GreetingPrompt extends Prompt {
  readonly name = 'greeting';
  
  readonly schema = z.object({
    name: z.string(),
    formal: z.boolean().optional()
  });
  
  resolve(args) {
    return [{
      role: 'assistant',
      content: args.formal ? `Good day, ${args.name}.` : `Hey ${args.name}!`
    }];
  }
}
```

## License

Apache-2.0