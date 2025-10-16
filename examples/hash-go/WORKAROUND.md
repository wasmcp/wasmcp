# wit-bindgen-go Bug Workaround

## Quick Reference

This directory contains a workaround for a code generation bug in wit-bindgen-go. The bug affects `option<borrow<resource>>` types in cross-interface records.

## Usage

### Automated (Recommended)

```bash
make build
```

The Makefile automatically:
1. Generates Go bindings (`go generate`)
2. Applies the codegen fix (`fix-codegen.sh`)
3. Builds the component with correct flags

### Manual

```bash
# Generate bindings
go generate ./...

# Apply fix
./fix-codegen.sh

# Build
tinygo build -target=wasip2 -wit-package ./wit -wit-world hash -o hash.wasm .
```

## What Gets Fixed

**File:** `gen/wasmcp/mcp/tools-capability/abi.go`

**Problem:** Incorrect return type for `lift_OptionBorrowOutputStream`

```diff
-func lift_OptionBorrowOutputStream(f0 uint32, f1 uint32) (v cm.Option[cm.Rep])
+func lift_OptionBorrowOutputStream(f0 uint32, f1 uint32) (v cm.Option[protocol.OutputStream])
```

## Why Is This Needed?

The `ClientContext` record (defined in `wasmcp:mcp/protocol`) contains:
```wit
record client-context {
    output: option<borrow<output-stream>>,
    // ...
}
```

When this record is used in another interface (`tools-capability`), wit-bindgen-go incorrectly generates the lift function with return type `cm.Option[cm.Rep]` instead of `cm.Option[protocol.OutputStream]`, causing a type mismatch.

## When Can This Be Removed?

Once the upstream bug is fixed in bytecodealliance/go-modules, you can:

1. Update wit-bindgen-go to the fixed version
2. Remove `fix-codegen.sh`
3. Update Makefile to remove the fix step
4. Delete this document

## Tracking

- **Bug Report:** [BUG_REPORT.md](./BUG_REPORT.md)
- **Upstream Issue:** [Link when created]
- **Status:** Workaround active (2025-10-15)

## Files

- `fix-codegen.sh` - Shell script that applies the fix
- `BUG_REPORT.md` - Detailed bug analysis for upstream
- `Makefile` - Automated build with fix integrated
- This file - Quick reference

---

**Note:** The fix is idempotent - running `fix-codegen.sh` multiple times is safe.
