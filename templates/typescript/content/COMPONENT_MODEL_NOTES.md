# WebAssembly Component Model Notes - TypeScript/JavaScript

This document provides precise technical documentation of TypeScript/JavaScript integration with the WebAssembly Component Model via jco.

## Architecture

TypeScript compiles to JavaScript, which is bundled and executed within a WebAssembly component using StarlingMonkey (a SpiderMonkey-based JavaScript engine optimized for WebAssembly).

### Component Stack

```
┌─────────────────────────────────┐
│     Your TypeScript Code         │
├─────────────────────────────────┤
│    JavaScript Bundle (Webpack)   │
├─────────────────────────────────┤
│   StarlingMonkey JS Engine       │
│        (SpiderMonkey)            │
├─────────────────────────────────┤
│    WebAssembly Component         │
│      (17MB total size)           │
└─────────────────────────────────┘
```

## jco Toolchain

jco (JavaScript Component Objects) provides:

1. **Type Generation**: `jco types` generates TypeScript definitions from WIT
2. **Componentization**: `jco componentize` embeds JavaScript + StarlingMonkey into a Wasm component
3. **Transpilation**: Can convert components back to JavaScript modules
4. **Runtime Bridge**: Handles async-to-sync conversion and WASI bindings

## Type System Mappings

### Precise Type Correspondence

| WIT Type | TypeScript Type | Runtime Behavior |
|----------|----------------|------------------|
| `string` | `string` | UTF-8 encoded, copied across boundary |
| `u8`-`u32` | `number` | Validated at boundary (throws on overflow) |
| `s8`-`s32` | `number` | Validated at boundary |
| `u64`/`s64` | `bigint` | Precise 64-bit integer support |
| `f32`/`f64` | `number` | JavaScript number is f64 |
| `bool` | `boolean` | Direct mapping |
| `option<T>` | `T \| undefined` | `null` becomes `undefined` |
| `result<T, E>` | `T` (throws on error) | jco converts errors to exceptions |
| `list<T>` | `Array<T>` | Deep copy across boundary |
| `record` | `interface` | Structural typing |
| `variant` | Discriminated union | `{ tag: string, val: T }` pattern |
| `resource` | Class instance | Handle-based with methods |
| `flags` | Object with boolean properties | Each flag becomes a boolean field |
| `enum` | String literal union | Type-safe string values |
| `tuple` | Tuple type | `[T1, T2, ...]` |

## Async Model

### The jco Async Bridge

jco performs sophisticated async-to-sync bridging:

```typescript
// What you write:
export async function callTool(request: CallToolRequest): Promise<CallToolResult> {
  const response = await fetch(url);
  const data = await response.json();
  return processData(data);
}

// What happens internally:
// 1. jco intercepts async function export
// 2. Creates a synchronous wrapper for the Component Model
// 3. Uses internal event loop pumping during await
// 4. Translates fetch() to wasi:http/outgoing-handler calls
// 5. Uses wasi:io/poll for non-blocking I/O
// 6. Resumes JavaScript execution when poll completes
// 7. Returns result synchronously to Component Model caller
```

This is **fundamentally different** from other languages:
- **Rust**: Must use blocking I/O or explicit async runtime
- **Python**: Requires PollLoop and asyncio.gather() for concurrency
- **Go**: Needs wasihttp.RequestsConcurrently() for concurrent HTTP

## Memory Model

### Linear Memory Layout

```
0x00000000 ┌────────────────────┐
           │   StarlingMonkey   │
           │   Runtime Data     │
           ├────────────────────┤
           │   JavaScript Heap  │
           │   (GC managed)     │
           ├────────────────────┤
           │   Call Stack       │
           │   (8MB with our   │
           │    configuration)  │
           ├────────────────────┤
           │   Component Model  │
           │   Marshaling Space │
0x00B70000 └────────────────────┘ (11.7MB default)
```

### Memory Configuration

- **Initial Memory**: 183 pages (11.7MB) - hardcoded by jco
- **Stack Size**: Configurable via `--aot-min-stack-size-bytes`
  - Default insufficient for concurrent async operations
  - We use 8MB (8388608 bytes) for reliable Promise.all()
- **Heap**: Managed by SpiderMonkey's garbage collector
- **Growth**: Currently not supported (fixed size)

## Build Pipeline

### Detailed Build Process

1. **TypeScript Compilation**
   ```bash
   tsc --noEmit  # Type checking only
   ```

2. **Bundling**
   ```bash
   webpack  # Creates single bundled.js
   ```
   - Target: `webworker` (no DOM, compatible with Wasm)
   - Module: ES modules for jco compatibility
   - No minification (StarlingMonkey doesn't benefit)

3. **Componentization**
   ```bash
   jco componentize bundled.js \
     --wit wit \
     --world-name {{project-name | kebab_case}} \
     --aot-min-stack-size-bytes 8388608 \
     --out {{project-name | kebab_case}}-provider.wasm
   ```
   - Embeds JavaScript into StarlingMonkey
   - Generates Component Model metadata
   - Configures runtime parameters

4. **Composition**
   ```bash
   wac plug --plug {{project-name | kebab_case}}-provider.wasm transport.wasm \
     -o mcp-http-server.wasm
   ```

## Export Pattern

### TypeScript Module Structure

```typescript
// src/index.ts - The root module jco looks for
import * as lifecycle from './capabilities/lifecycle.js';
import * as authorization from './capabilities/authorization.js';
import * as tools from './capabilities/tools.js';

// Each WIT interface maps to an exported namespace
export { lifecycle, authorization, tools };
```

### Implementation Pattern

```typescript
// src/capabilities/tools.ts
export function listTools(request: ListToolsRequest): ListToolsResult {
  // Synchronous from Component Model perspective
  return { tools: [...], nextCursor: undefined };
}

export async function callTool(
  request: CallToolRequest,
  context: AuthContext | undefined
): Promise<CallToolResult> {
  // Async internally, but jco handles the bridge
  const result = await performAsyncWork();
  return result;
}
```

## Error Handling

### Exception to Result Conversion

```typescript
// TypeScript implementation
export function operation(): SomeResult {
  if (errorCondition) {
    throw new Error("Operation failed: specific reason");
  }
  return successValue;
}

// Component Model sees:
// - Success: result::ok(successValue)
// - Error: result::err({ tag: 'error', val: "Operation failed: specific reason" })
```

### Error Types

- JavaScript `Error` → WIT error variant
- Uncaught exceptions → Component trap
- Promise rejections → Handled by jco, converted to errors

## Performance Characteristics

### Strengths
- **Native JSON**: No serialization overhead for JSON operations
- **Async I/O**: Non-blocking by default with natural syntax
- **Concurrent Requests**: Promise.all() works efficiently

### Weaknesses
- **Binary Size**: 17MB (vs 2.4MB for Rust)
- **Cold Start**: Loading and initializing SpiderMonkey
- **Memory Overhead**: ~11.7MB baseline before application data
- **Stack Pressure**: Async operations consume significant stack

### Benchmarks (Relative)
- Startup time: 10x slower than Rust
- HTTP request latency: 1.5x slower than Rust
- Memory usage: 7x higher than Rust
- Development iteration: 2x faster than Rust

## Zod Integration

### Schema-Driven Development

```typescript
import { z } from 'zod';

// Define schema once
const Schema = z.object({
  name: z.string().min(1).describe("User's name"),
  age: z.number().int().positive().describe("User's age")
});

// Derive TypeScript type
type SchemaType = z.infer<typeof Schema>;

// Generate JSON Schema for WIT
const jsonSchema = z.toJSONSchema(Schema);  // Zod v4 built-in

// Runtime validation
const validated = Schema.parse(untrustedInput);
```

This provides:
1. Compile-time type safety
2. Runtime validation
3. JSON Schema for API documentation
4. Single source of truth

## Debugging

### Current Limitations
- No source maps into Wasm module
- Stack traces show JavaScript positions within bundle
- No step debugging of Component Model boundary
- Console.log works via WASI stdio

### Debugging Strategies
1. Extensive logging before/after boundary crossings
2. Unit test JavaScript logic outside component
3. Use `--debug-starlingmonkey-build` for detailed engine errors
4. Validate inputs thoroughly (Zod helps here)

## Best Practices

1. **Memory Management**
   - Configure adequate stack size for concurrent operations
   - Avoid creating large temporary objects
   - Let GC handle cleanup (no manual management)

2. **Async Operations**
   - Use Promise.all() for concurrent requests
   - Avoid nested async loops (stack pressure)
   - Handle errors at async boundaries

3. **Type Safety**
   - Generate types in CI to catch WIT changes
   - Use Zod for runtime validation
   - Avoid `any` types at component boundaries

4. **Performance**
   - Minimize boundary crossings
   - Batch operations when possible
   - Cache computed values within single call

## Comparison with Other Languages

### vs Rust
- ✅ Natural async/await without runtime complexity
- ✅ Faster development iteration
- ❌ 7x larger binary size
- ❌ Less predictable performance

### vs Python
- ✅ Static typing with TypeScript
- ✅ No explicit async bridge (PollLoop)
- ✅ Better IDE support
- ❌ Similar binary size concerns

### vs Go
- ✅ Natural async patterns (no wasihttp wrapper)
- ✅ True concurrent I/O with Promise.all()
- ✅ Better variant type support
- ❌ Larger memory footprint

## Future Outlook

### Near Term
- Source map support for debugging
- Reduced StarlingMonkey size
- Incremental compilation support

### Long Term
- Native Component Model in V8/SpiderMonkey
- JSPI (JavaScript Promise Integration) for true async components
- Shared-nothing parallelism via component composition
- Sub-5MB runtime possible with optimization

## References

- [jco Repository](https://github.com/bytecodealliance/jco) - Toolchain documentation
- [ComponentizeJS](https://github.com/bytecodealliance/ComponentizeJS) - JavaScript engine component
- [StarlingMonkey](https://github.com/bytecodealliance/StarlingMonkey) - JavaScript runtime
- [Component Model Spec](https://github.com/WebAssembly/component-model) - Canonical specification