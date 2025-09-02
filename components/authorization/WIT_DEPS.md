# WIT Dependencies Documentation

## Why We Have Local WIT Files

The `wit/deps/config/` directory contains a local copy of `wasi:config@0.2.0-draft` because:

1. **Runtime Compatibility**: wasmtime (our primary runtime) implements `wasi:config/store`, not `wasi:config/runtime`
2. **No Compatible Registry Package**: At the time of implementation, no published WIT package matched wasmtime's exact interface
3. **Critical for WASI Config**: This interface is how we receive JWT configuration at runtime

## Current Dependencies

```toml
[package.metadata.component.target.dependencies]
"wasi:config" = { path = "wit/deps/config" }
```

## Future Cleanup Opportunities

1. **Monitor wasmtime releases**: If wasmtime updates to use a standard WASI config interface, we can switch to a registry package
2. **Publish our version**: We could publish `wasi:config-store@0.2.0-draft` to help others with the same issue
3. **Runtime detection**: Could potentially support multiple WASI config interfaces based on runtime

## Verification

To verify this is still needed:
```bash
# Try using a registry package
cargo component build --release

# If it fails at runtime with wasmtime, the local copy is still required
```

## Registry Packages We Publish

Our authorization component itself is published as:
- `fastertools:mcp-authorization@0.1.0`

This includes the correct WASI config dependency bundled, so downstream users don't need to worry about this issue.