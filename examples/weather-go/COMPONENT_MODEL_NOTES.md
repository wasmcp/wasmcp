# Component Model Technical Notes for Go

This document explains the technical constraints and patterns in the weather-go example.

## Key Terminology

- **Wasm** - WebAssembly (not "WASM")
- **Component Model** - Architecture for interoperable Wasm modules
- **WIT** - WebAssembly Interface Types, the IDL for components
- **Canonical ABI** - Defines how types are represented at the binary level

## Why These Patterns Exist

### Result Types Instead of Multiple Returns

```go
// What we write:
func Initialize(...) cm.Result[Shape, Value, Error]

// What we'd prefer in Go:
func Initialize(...) (Value, error)
```

**Why:** WebAssembly core modules can only return a single value. The WIT `result<T, E>` type encodes success/error as a single return value that the Canonical ABI can handle.

### Option Types Instead of Pointers

```go
// What we write:
cm.Some("value")
cm.None[string]()

// What we'd prefer in Go:
&value
nil
```

**Why:** WIT has explicit `option<T>` types. Go pointers don't translate directly across the component boundary because components can't share memory.

### Shape Types in Results

```go
cm.Result[lifecycle.InitializeResultShape, lifecycletypes.InitializeResult, mcptypes.McpError]
```

**Why:** The Shape types are wit-bindgen-go's internal storage types for variant/result types. They ensure proper memory layout for the Canonical ABI.

## The Concurrency Problem

### Why wasihttp.RequestsConcurrently Exists

```go
// This would NOT be concurrent in TinyGo/Wasm:
for _, url := range urls {
    go func(u string) {
        resp, _ := http.Get(u)  // Blocks entire Wasm module
    }(url)
}

// This IS concurrent:
responses := wasihttp.RequestsConcurrently(requests)
```

**Why:** 
- TinyGo compiles to single-threaded Wasm core modules
- Goroutines provide cooperative concurrency, not parallelism
- Each `http.Get()` blocks the entire module until completion
- `RequestsConcurrently` uses WASI's `poll.Poll()` to let the host handle I/O in parallel

## Transport Override Pattern

```go
http.DefaultTransport = &wasihttp.Transport{}
```

This is idiomatic Go - implementing `http.RoundTripper` to route all HTTP through WASI's outgoing-handler. This allows existing Go code using `net/http` to work in Wasm components.

## Component Lifecycle

Unlike regular Go programs:
1. `main()` never executes in components
2. `init()` runs during component instantiation
3. Functionality is exposed through exports, not a main entry point
4. The host runtime calls exported functions directly

## Further Reading

- [Component Model Specification](https://github.com/WebAssembly/component-model)
- [WIT Language Reference](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md)
- [Canonical ABI Explainer](https://github.com/WebAssembly/component-model/blob/main/design/mvp/CanonicalABI.md)
- [TinyGo Wasm Support](https://tinygo.org/docs/guides/webassembly/)