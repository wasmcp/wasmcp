# {{project_name}}

MCP prompts capability component in TypeScript.

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
wasmcp compose target/{{project_name}}.wasm -o server.wasm
```

The CLI automatically detects this is a prompts-capability component and wraps it with prompts-middleware.

## Run

```bash
# HTTP
wasmtime serve -Scli server.wasm

# Stdio
wasmcp compose target/{{project_name}}.wasm -t stdio -o server.wasm
wasmtime run server.wasm
```

## Implementation

This component uses the **capability pattern**, implementing two methods from the `prompts-capability` interface:

- `list_prompts()` - Returns all prompts this component provides
- `get_prompt()` - Returns prompt content by name, or `null` if not handled

See `src/index.ts` for example prompts demonstrating:
- Prompt definitions with names and arguments
- Dynamic prompt generation based on arguments
- No protocol handling or delegation code

The prompts-middleware automatically handles:
- MCP protocol translation
- Merging prompts from multiple components
- Request delegation to downstream components
- Error handling and response formatting

## Adding Prompts

To add new prompts:

1. Add a `Prompt` entry to the array in `listPrompts()`:

```typescript
{
  name: 'my-prompt',
  options: {
    meta: undefined,
    arguments: [
      {
        name: 'arg1',
        description: 'First argument',
        required: true,
        title: 'Argument 1',
      },
    ],
    description: 'Description of my prompt',
    title: 'My Prompt',
  },
}
```

2. Add a case in `getPrompt()`:

```typescript
else if (request.name === 'my-prompt') {
  const args = request.arguments ? JSON.parse(request.arguments) : {};
  const arg1 = args.arg1 || 'default';

  return {
    meta: undefined,
    description: 'My prompt description',
    messages: [
      {
        role: 'user' as Role,
        content: {
          tag: 'text',
          val: {
            text: {
              tag: 'text',
              val: `Your prompt text using ${arg1}`,
            },
            options: undefined,
          },
        } as ContentBlock,
      } as PromptMessage,
    ],
  };
}
```

3. That's it! No need to handle merging, delegation, or protocol details - the middleware does that for you.

## Project Structure

```
{{project_name}}/
├── src/
│   ├── index.ts           # Your prompt implementations
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
