# wasmcp - TypeScript SDK for MCP WebAssembly Components

A TypeScript SDK for building MCP (Model Context Protocol) handlers with first-class Zod integration for type safety and runtime validation.

## Features

- **Type-safe**: Full TypeScript support with automatic type inference from Zod schemas
- **Runtime validation**: Automatic input validation with helpful error messages  
- **JSON Schema generation**: Automatic conversion from Zod schemas to MCP's required JSON Schema format
- **Class-based API**: Clean, idiomatic TypeScript using classes
- **Zod v4**: Uses the latest stable Zod for best performance
- **Performance**: O(1) tool/resource lookups with Map-based dispatch

## Installation

```bash
npm install wasmcp
```

Note: `wasmcp` includes Zod v4 as a dependency.

## Quick Start

```typescript
import { createTool, createHandler, z } from 'wasmcp';

// Define a tool using factory function
const helloTool = createTool({
  name: 'hello',
  description: 'Say hello to someone',
  schema: z.object({
    name: z.string().describe('Name to greet')
  }),
  execute: async (args) => {
    // TypeScript knows args is { name: string }
    return `Hello, ${args.name}!`;
  }
});

// Create and export the handler
export const handler = createHandler({
  tools: [helloTool]
});
```

## API Options

### Factory Functions (Recommended for Simple Tools)

The `createTool` function is the simplest way to define tools:

```typescript
const emailTool = createTool({
  name: 'send_email',
  description: 'Send an email',
  schema: z.object({
    to: z.string().email('Invalid email address'),
    cc: z.array(z.string().email()).optional(),
    subject: z.string().max(100, 'Subject too long'),
    body: z.string(),
    priority: z.enum(['low', 'normal', 'high']).default('normal')
  }),
  execute: async (args) => {
    console.log(`Sending ${args.priority} priority email to ${args.to}`);
    return `Email sent to ${args.to} with subject "${args.subject}"`;
  }
});
```

### Class-based API (Better for Complex Tools)

For more complex tools that need internal state or helper methods, use the class-based API:

```typescript
import { Tool } from 'wasmcp';

class DatabaseQueryTool extends Tool {
  readonly name = 'db_query';
  readonly description = 'Query the database';
  
  readonly schema = z.object({
    query: z.string(),
    limit: z.number().int().positive().default(10)
  });

  // Private helper methods
  private sanitizeQuery(query: string): string {
    // Complex sanitization logic
    return query.replace(/;/g, '');
  }

  private async connectToDb() {
    // Connection logic
  }

  async execute(args) {
    const sanitized = this.sanitizeQuery(args.query);
    await this.connectToDb();
    // Execute query...
    return `Query executed: ${sanitized}`;
  }
}
```

Both approaches work seamlessly with `createHandler`:

```typescript
export const handler = createHandler({
  tools: [emailTool, DatabaseQueryTool]
});
```

## Advanced Examples

### Calculator with Custom Validation

```typescript
const calculatorTool = createTool({
  name: 'calculator',
  description: 'Perform math operations',
  schema: z.object({
    a: z.number(),
    b: z.number(),
    operation: z.enum(['add', 'subtract', 'multiply', 'divide'])
  }).refine(
    (data) => !(data.operation === 'divide' && data.b === 0),
    { message: "Cannot divide by zero" }
  ),
  execute: async (args) => {
    switch (args.operation) {
      case 'add': return String(args.a + args.b);
      case 'subtract': return String(args.a - args.b);
      case 'multiply': return String(args.a * args.b);
      case 'divide': return String(args.a / args.b);
    }
  }
});
```

## Resources and Prompts

While tools are the primary feature used today, the SDK also supports resources and prompts:

```typescript
import { Resource, Prompt, PromptMessage } from 'wasmcp';

// Resources provide read-only data
class ConfigResource extends Resource {
  readonly uri = 'config://app';
  readonly name = 'Application Config';
  readonly description = 'Current configuration';
  readonly mimeType = 'application/json';

  read() {
    return JSON.stringify({ version: '1.0.0' });
  }
}

// Prompts generate conversation templates
class GreetingPrompt extends Prompt {
  readonly name = 'greeting';
  readonly description = 'Generate a greeting';
  
  readonly schema = z.object({
    name: z.string(),
    formal: z.boolean().optional()
  });

  resolve(args): PromptMessage[] {
    const greeting = args.formal 
      ? `Good day, ${args.name}.`
      : `Hey ${args.name}!`;
      
    return [
      { role: 'assistant', content: greeting }
    ];
  }
}

// Include them in your handler
export const handler = createHandler({
  tools: [HelloTool, CalculatorTool],
  resources: [ConfigResource],
  prompts: [GreetingPrompt]
});
```

## Building Components

1. Set up your project with the necessary build tools:
   ```bash
   npm install -D @bytecodealliance/jco typescript esbuild
   ```

2. Configure your build scripts in `package.json`:
   ```json
   {
     "scripts": {
       "build": "tsc && esbuild dist/index.js --bundle --format=esm --platform=node --outfile=dist/bundled.js && jco componentize dist/bundled.js --wit ./wit --world-name mcp-handler --out dist/handler.wasm"
     }
   }
   ```

3. Build your component:
   ```bash
   npm run build
   ```

## Benefits of Zod Integration

1. **Automatic type inference** - No need to manually define TypeScript types
2. **Rich validation** - Email, URL, UUID, regex patterns, and more
3. **Helpful error messages** - Zod's `prettifyError` provides user-friendly errors
4. **Composable schemas** - Build complex schemas from simple ones
5. **Transform support** - Transform and validate in one step
6. **JSON Schema** - Automatic conversion for MCP compatibility

## Error Handling

When validation fails, users get helpful error messages:

```
✖ Invalid arguments:
  ✖ Invalid email address
    → at to
  ✖ Subject too long: expected string with max length 100
    → at subject
```

## Performance

The SDK is designed for performance:
- Class instances are created once at startup
- Tool/resource/prompt lookups use Maps for O(1) access
- Zod v4 provides significant performance improvements
- No runtime overhead from decorators or reflection