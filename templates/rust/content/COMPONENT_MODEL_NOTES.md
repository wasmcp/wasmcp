# Component Model Technical Notes for Rust

This document explains how Rust integrates with the WebAssembly Component Model.

## Rust's Natural Fit

Rust has the most natural integration with the Component Model:
- **Result<T, E>** maps directly to WIT's `result<T, E>`
- **Option<T>** maps directly to WIT's `option<T>`
- **No runtime needed** - Rust compiles directly to WebAssembly
- **Zero-cost abstractions** - Guest traits compile to direct exports

## cargo-component

### What It Does

`cargo-component` is a cargo subcommand that:
1. Generates Rust bindings from WIT files
2. Implements the Component Model ABI
3. Creates the component binary from a Rust module

### The Guest Trait Pattern

```rust
// WIT interface generates a Guest trait
use exports::wasmcp::mcp::lifecycle::Guest as LifecycleGuest;

// We implement the trait for our Component type
impl LifecycleGuest for Component {
    fn initialize(request: InitializeRequest) -> Result<InitializeResult, McpError> {
        // Direct implementation - no wrappers needed
    }
}
```

**Why Guest traits?**
- Each WIT export generates a corresponding Guest trait
- Implementing the trait automatically creates the WebAssembly export
- The trait methods match WIT function signatures exactly

## Type Mappings

| Rust Type | WIT Type | Notes |
|-----------|----------|-------|
| `Result<T, E>` | `result<T, E>` | Perfect 1:1 mapping |
| `Option<T>` | `option<T>` | Natural mapping |
| `String` | `string` | UTF-8 by default |
| `Vec<T>` | `list<T>` | Direct mapping |
| `structs` | `record` | Field-by-field mapping |

## The Component Struct

```rust
pub struct Component;
```

This zero-sized type (ZST) serves as:
- The implementor of all Guest traits
- A namespace for component functionality
- No runtime state (Component Model is stateless)

## Async in WebAssembly

### The spin_sdk Pattern

```rust
spin_sdk::http::run(async move {
    // Async code here
})
```

**How it works:**
1. Component exports must be synchronous (WIT limitation)
2. `spin_sdk::http::run()` executes an async block to completion
3. Uses WebAssembly's poll-based I/O under the hood
4. Similar to Python's PollLoop but type-safe

### Concurrent HTTP

```rust
// Rust's natural async/await with futures
let futures = cities.iter().map(|city| async move {
    get_weather_for_city(&city).await
});
let results = join_all(futures).await;
```

**Key difference from Go:**
- No special concurrent HTTP function needed
- `futures::join_all` works naturally
- The runtime (spin_sdk) handles WebAssembly poll-based I/O

## Build Configuration

### Cargo.toml Metadata

```toml
[package.metadata.component]
package = "wasmcp:mcp-transport-http"

[package.metadata.component.target]
path = "wit"
world = "{{project-name | kebab_case}}"

[package.metadata.component.bindings]
derives = ["serde::Serialize", "serde::Deserialize", "Clone"]
```

This configures:
- Which WIT world to implement
- Where to find WIT files
- Additional derives for generated types

## HTTP in WebAssembly

### Why spin_sdk?

Standard HTTP crates (reqwest, hyper) don't work in WebAssembly because:
- No access to system sockets
- No tokio runtime in WebAssembly
- Must use Component Model HTTP imports

`spin_sdk::http` provides:
- WebAssembly-compatible HTTP client
- Integration with Component Model imports
- Async runtime for WebAssembly

## Memory Management

Unlike Go or Python:
- **No garbage collector** - Rust's ownership handles memory
- **No separate linear memory concerns** - Component Model handles isolation
- **Predictable performance** - No GC pauses or runtime overhead

## Error Handling

```rust
// WIT: result<tool-result, mcp-error>
fn call_tool() -> Result<CallToolResult, McpError> {
    // ? operator works naturally
    let args: ToolArgs = parse_args(request.arguments)?;
    
    // Error variants map to WIT error cases
    Err(McpError {
        code: ErrorCode::MethodNotFound,
        message: "Unknown tool".to_string(),
        data: None,
    })
}
```

The `?` operator and Result type provide ergonomic error handling that maps directly to WIT's result types.

## Component Lifecycle

1. **No main()** - Component Model doesn't use traditional entry points
2. **Static initialization** - `lazy_static!` or `OnceCell` for one-time setup
3. **Stateless exports** - Each function call is independent
4. **Runtime manages lifecycle** - Component instantiation/destruction handled by host

## Comparison with Other Languages

### Advantages over Go
- Natural Result/Option types (no special wrappers)
- Built-in async that works in WebAssembly
- No runtime overhead

### Advantages over Python
- Type safety at compile time
- No runtime type checking needed
- Direct compilation to WebAssembly (no interpreter)

### Trade-offs
- More verbose than Python
- Requires understanding ownership/borrowing
- Longer compile times than Go

## Further Reading

- [cargo-component Documentation](https://github.com/bytecodealliance/cargo-component)
- [Spin SDK Documentation](https://developer.fermyon.com/spin/rust-components)
- [WebAssembly Component Model Book](https://component-model.bytecodealliance.org/)
- [wit-bindgen Documentation](https://github.com/bytecodealliance/wit-bindgen)