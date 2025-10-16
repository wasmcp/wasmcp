# Bug Report: Incorrect code generation for `option<borrow<resource>>` in cross-interface records

## Summary

wit-bindgen-go generates incorrect lift functions when a record containing `option<borrow<resource>>` is defined in one interface and used as a function parameter in another interface. The lift function incorrectly returns `cm.Option[cm.Rep]` instead of `cm.Option[ResourceType]`, causing a type mismatch compilation error.

## Environment

- **wit-bindgen-go**: Latest from `go.bytecodealliance.org/cmd/wit-bindgen-go` (as of 2025-10-15)
- **go-modules**: commit `55a8715`
- **Go**: 1.23+
- **TinyGo**: 0.34.0 (wasip2 target)

## Minimal Reproduction

### WIT Definition

```wit
package my:test@0.1.0;

interface streams {
  resource output-stream;
}

interface protocol {
  use streams.{output-stream};

  record client-context {
    output: option<borrow<output-stream>>,
  }
}

interface tools {
  use protocol.{client-context};

  do-something: func(ctx: client-context);
}

world test {
  import streams;
  import protocol;
  export tools;
}
```

### Generated Code (Incorrect)

**File: `my/test/protocol/protocol.wit.go`**
```go
type ClientContext struct {
    _      cm.HostLayout           `json:"-"`
    Output cm.Option[OutputStream] `json:"output"`
}
```

**File: `my/test/tools/abi.go`**
```go
func lift_OptionBorrowOutputStream(f0 uint32, f1 uint32) (v cm.Option[cm.Rep]) {
    if f0 == 0 {
        return
    }
    return (cm.Option[cm.Rep])(cm.Some[cm.Rep](cm.Reinterpret[cm.Rep]((uint32)(f1))))
}

func lift_ClientContext(f0 uint32, f1 uint32) (v protocol.ClientContext) {
    v.Output = lift_OptionBorrowOutputStream(f0, f1) // ❌ Type mismatch!
    return
}
```

### Compilation Error

```
my/test/tools/abi.go:18:13: cannot use lift_OptionBorrowOutputStream(f0, f1)
  (value of struct type cm.Option[cm.Rep]) as cm.Option[protocol.OutputStream]
  value in assignment
```

## Expected Behavior

The lift function should return the concrete resource type, not `cm.Rep`:

```go
func lift_OptionBorrowOutputStream(f0 uint32, f1 uint32) (v cm.Option[protocol.OutputStream]) {
    if f0 == 0 {
        return
    }
    return (cm.Option[protocol.OutputStream])(cm.Some[protocol.OutputStream](
        cm.Reinterpret[protocol.OutputStream]((uint32)(f1))))
}
```

## Analysis

The bug occurs due to cross-interface type resolution:

1. **Within same package** (`option<borrow<resource>>` in same interface): ✅ Generates correctly
2. **Cross-package record fields** (`borrow<resource>` without option): ✅ Generates correctly
3. **Cross-package with option wrapper** (`option<borrow<resource>>`): ❌ Bug triggered

The codegen correctly handles:
- Function parameters with `option<borrow<resource>>` → `cm.Option[cm.Rep]`
- Record fields with `borrow<resource>` → `ResourceType`

But fails when combining both patterns in a cross-interface scenario.

## Real-world Impact

This bug affects the wasmcp MCP framework where `ClientContext` (defined in `wasmcp:mcp/protocol`) contains `output: option<borrow<output-stream>>` and is used across multiple capability interfaces.

**Example:** `wasmcp:mcp/tools-capability` interface uses `client-context` from the protocol interface:

```wit
interface tools-capability {
  use protocol.{client-context, ...};

  list-tools: func(request: list-tools-request, client: client-context) -> list-tools-result;
  call-tool: func(request: call-tool-request, client: client-context) -> option<call-tool-result>;
}
```

This prevents building any Go-based MCP tools capability using wit-bindgen-go.

## Workaround

The only current workaround is to manually patch the generated `abi.go` file after each generation:

```go
-func lift_OptionBorrowOutputStream(f0 uint32, f1 uint32) (v cm.Option[cm.Rep]) {
+func lift_OptionBorrowOutputStream(f0 uint32, f1 uint32) (v cm.Option[protocol.OutputStream]) {
     if f0 == 0 {
         return
     }
-    return (cm.Option[cm.Rep])(cm.Some[cm.Rep](cm.Reinterpret[cm.Rep]((uint32)(f1))))
+    return (cm.Option[protocol.OutputStream])(cm.Some[protocol.OutputStream](
+        cm.Reinterpret[protocol.OutputStream]((uint32)(f1))))
 }
```

This is fragile and breaks on every regeneration.

## Reproduction Repository

Full reproduction available at: https://github.com/ianb/wasmcp/tree/main/examples/hash-go

To reproduce:
```bash
git clone https://github.com/ianb/wasmcp.git
cd wasmcp/examples/hash-go
go generate ./...
tinygo build -target=wasip2 -o hash.wasm .
```

## Related Code Locations

In `go-modules` repository, likely related to:
- Type resolution across interfaces
- Borrow handling in lift function generation
- Option type wrapping for borrowed resources

---

**Note:** This issue blocks Go adoption for WebAssembly Component Model applications that use borrowed resources in cross-interface record types, a common pattern in component composition.
