# Component Model Technical Notes for TypeScript

This document explains how TypeScript/JavaScript integrates with the WebAssembly Component Model using jco.

## TypeScript's Component Model Integration

TypeScript uses **jco** (JavaScript Component Objects) as its toolchain for the Component Model, providing:
- Type generation from WIT files
- Component building from JavaScript/TypeScript
- Transpilation of components to JavaScript modules
- Runtime support for WebAssembly components

## jco: The JavaScript/TypeScript Toolchain

### What It Does

`jco` is a comprehensive JavaScript-native toolchain that:
1. Generates TypeScript definitions from WIT interfaces
2. Bundles JavaScript/TypeScript into WebAssembly components
3. Provides runtime execution capabilities
4. Transpiles WebAssembly components back to JavaScript

### The Export Pattern

```typescript
// WIT interfaces map to exported objects
export const lifecycle = {
  initialize: lifecycleImpl.initialize,
  clientInitialized: lifecycleImpl.clientInitialized,
  shutdown: lifecycleImpl.shutdown,
};
```

**Key Pattern:**
- Each WIT interface becomes an exported object
- Object contains all the interface's methods
- jco wires these exports to WebAssembly component exports
- Different from Rust's traits or Python's classes

## Type Mappings

| TypeScript Type | WIT Type | Notes |
|-----------------|----------|-------|
| `T \| undefined` | `option<T>` | Natural optional mapping |
| Return value / throw | `result<T, E>` | Transparent error handling |
| `string` | `string` | Direct mapping |
| `number` | `u32`, `s32`, `f32`, `f64` | Context-dependent |
| `bigint` | `u64`, `s64` | For 64-bit integers |
| `Array<T>` | `list<T>` | Direct mapping |
| `{ tag: string, val: T }` | `variant` | Discriminated unions |

## The Async Advantage

### jco's Async Support

```typescript
// jco transparently handles async functions!
export async function callTool(
  request: CallToolRequest,
  context: AuthContext | undefined
): Promise<CallToolResult> {
  // Can use async/await and fetch directly!
  const weather = await getWeatherForCity(city);
  return result;
}
```

**The Magic:**
1. jco transparently bridges async JavaScript to Component Model exports
2. Native `fetch` API works seamlessly through WASI HTTP
3. `Promise.all()` enables true concurrent requests
4. No special runtime or bridge needed (unlike Python's PollLoop)

### How It Works

jco uses sophisticated async-to-sync bridging internally:
- Handles Promise resolution automatically
- Maps JavaScript's event loop to WebAssembly's poll-based I/O
- Provides seamless integration with browser/Node.js fetch APIs
- Enables natural JavaScript async patterns

This is a **huge advantage** over:
- **Python**: Requires explicit PollLoop bridging
- **Rust**: Needs spin_sdk::http::run() wrapper
- **Go**: Requires special wasihttp.RequestsConcurrently() for concurrency

## Build Process

### Pipeline

```bash
TypeScript → JavaScript → WebAssembly Component → Composed Server
     ↓           ↓                ↓                      ↓
   (tsc)     (webpack)      (jco componentize)     (wac plug)
```

### Build Configuration

**webpack.config.js:**
- Target: `webworker` for WebAssembly compatibility
- Output: ES modules for jco
- No minification for debugging

**tsconfig.json:**
- Target: ES2020 for modern JavaScript features
- Module: ES2022 for top-level await support
- Strict mode for type safety

## Generated Types

### Structure

```
src/generated/
├── interfaces/
│   ├── wasmcp-mcp-lifecycle.d.ts      # Interface exports
│   ├── wasmcp-mcp-lifecycle-types.d.ts # Type definitions
│   └── ...
└── wit.d.ts                            # World exports
```

### Usage

```typescript
import type {
  InitializeRequest,
  InitializeResult,
} from '../generated/interfaces/wasmcp-mcp-lifecycle-types.js';

export function initialize(request: InitializeRequest): InitializeResult {
  // Implementation
}
```

## Variant Types

WIT variants become TypeScript discriminated unions:

```typescript
// WIT: variant content-block { text(text-content), ... }
// TypeScript:
type ContentBlock = {
  tag: 'text',
  val: TextContent
} | {
  tag: 'image',
  val: ImageContent
};
```

This pattern provides type-safe variant handling with TypeScript's union types.

## Error Handling

```typescript
// Successful return
export function listTools(request: ListToolsRequest): ListToolsResult {
  return { tools: [...], nextCursor: undefined };
}

// Error case - throw an exception
export function callTool(request: CallToolRequest): CallToolResult {
  if (error) {
    throw new Error("Tool execution failed");
  }
  return result;
}
```

jco transparently converts:
- Return values → WIT `ok` variant
- Thrown exceptions → WIT `err` variant

## Memory Management

Unlike Python or Go:
- **No garbage collector in WebAssembly** - JavaScript GC doesn't cross boundary
- **Automatic marshaling** - jco handles data copying across boundaries
- **No shared memory** - Component Model ensures isolation

## Component Lifecycle

1. **No main()** - Component Model uses exports only
2. **Stateless exports** - Each function call is independent
3. **No global state** - Cannot maintain state between calls
4. **Host manages lifecycle** - Instantiation/destruction by runtime

## Comparison with Other Languages

### Advantages over Go
- **Native async support** with fetch and Promise.all()
- Natural optional types (undefined)
- Cleaner variant representation
- No special concurrency functions needed

### Advantages over Python
- Static type checking at compile time
- Better IDE support with TypeScript
- **No explicit async bridge needed** (PollLoop)
- Native JavaScript async patterns

### Advantages over Rust
- **Most natural async story** - just use async/await
- No async runtime wrapper needed
- Faster development iteration
- Familiar JavaScript patterns

### Trade-offs
- Larger bundle size than Rust
- Less predictable performance than Rust
- JavaScript bridge adds some overhead
- But excellent developer experience

## TypeScript/jco Strengths

1. **Best async support** among all Component Model languages
2. **Native fetch API** works transparently
3. **Promise.all() concurrency** without special handling
4. **Natural JavaScript patterns** preserved

## Best Practices

1. **Keep exports simple** - Complex logic in internal functions
2. **Type everything** - Leverage TypeScript's type system
3. **Avoid state** - Design for stateless operation
4. **Handle errors explicitly** - Use proper error types
5. **Test type generation** - Verify generated types match expectations

## Future Improvements

- **JSPI Support** - Would solve async challenges
- **Direct Component Support** - Native JavaScript component model
- **Better DevTools** - Component-aware debugging
- **Streaming Compilation** - Faster component loading

## Further Reading

- [jco Documentation](https://github.com/bytecodealliance/jco)
- [ComponentizeJS](https://github.com/bytecodealliance/ComponentizeJS)
- [WebAssembly Component Model](https://component-model.bytecodealliance.org/)
- [JSPI Proposal](https://github.com/WebAssembly/js-promise-integration)