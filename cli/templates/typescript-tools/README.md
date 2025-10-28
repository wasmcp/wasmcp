# {{project_name}}

MCP tools handler written in TypeScript.

## Prerequisites

- [Node.js](https://nodejs.org/) 18 or later
- npm (comes with Node.js)

## Building

```bash
# Install dependencies
npm install

# Build the component
npm run build
```

Or use make:

```bash
make
```

This will generate `target/{{project_name}}.wasm`.

## Project Structure

```
{{project_name}}/
├── src/
│   ├── index.ts           # Your tool implementations
│   └── generated/         # Generated TypeScript bindings (git-ignored)
├── wit/
│   ├── world.wit          # WIT world definition
│   └── deps.toml          # WIT dependencies
├── package.json           # Node.js dependencies and scripts
├── tsconfig.json          # TypeScript configuration
├── webpack.config.js      # Webpack bundler configuration
└── Makefile               # Build automation
```

## Development

### Adding Tools

1. Define your tool's input schema using Zod:

```typescript
const MyToolSchema = z.object({
  param: z.string().describe('Parameter description'),
});
```

2. Add the tool to `listTools()`:

```typescript
{
  name: 'my-tool',
  inputSchema: JSON.stringify(z.toJSONSchema(MyToolSchema)),
  options: {
    description: 'Tool description',
    title: 'My Tool',
  },
}
```

3. Implement the tool in `callTool()`:

```typescript
case 'my-tool':
  return executeMyTool(request);
```

4. Create the execution function:

```typescript
function executeMyTool(request: protocol.CallToolRequest): protocol.CallToolResult {
  const args = JSON.parse(request.arguments!);
  const parsedArgs = MyToolSchema.parse(args);

  // Your tool logic here
  const result = `Result: ${parsedArgs.param}`;

  return textResult(result);
}
```

### Type Safety

- Generated TypeScript bindings provide full type safety
- Zod schemas validate inputs at runtime
- `z.toJSONSchema()` auto-generates JSON Schema for tools

### Error Handling

Always wrap tool execution in try-catch:

```typescript
try {
  const parsedArgs = MyToolSchema.parse(args);
  // Tool logic
  return textResult(result);
} catch (error) {
  if (error instanceof z.ZodError) {
    return errorResult(`Invalid arguments: ${error.message}`);
  }
  return errorResult(error instanceof Error ? error.message : 'Unknown error');
}
```

## Composing into a Server

After building, compose your handler into an MCP server:

```bash
wasmcp compose server target/{{project_name}}.wasm -o server.wasm
```

Or with multiple handlers:

```bash
wasmcp compose server \
  target/{{project_name}}.wasm \
  other-handler.wasm \
  -o server.wasm
```

## Clean Build

```bash
# Remove all build artifacts and dependencies
make clean

# Fresh build
make
```
