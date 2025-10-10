# Testing Strategy for wasmcp

## Overview

wasmcp uses a two-tier testing approach:

1. **Unit Tests** - Test pure logic on native target (fast, no WASI runtime needed)
2. **Integration Tests** - Test full WASI components with wasmtime (slower, validates full stack)

## Unit Tests

Unit tests validate protocol logic without WASI dependencies:
- Base64 stream encoding
- JSON escaping and formatting
- Message serialization logic
- Utility functions

**Run unit tests:**
```bash
# From workspace root - must override wasm32-wasip2 target
cargo test -p protocol --target x86_64-unknown-linux-gnu

# Or from protocol crate directory (uses host target by default)
cd crates/protocol && cargo test
```

**Status**: ✓ 40 unit tests covering all utils.rs functionality

## Integration Tests

Integration tests validate the full component stack:
- WIT bindings generation
- WASI stream integration
- Component composition
- End-to-end message handling

### Approach: cargo-component Pattern

We follow the cargo-component testing pattern:

1. Build test as wasm32-wasip2 component
2. Run through wasmtime with WASI preview2 support
3. Validate output and behavior

**How cargo-component does it:**
```bash
# 1. Build test (don't run - will be componentized)
cargo test --target wasm32-wasip2 --no-run

# 2. Componentize the test binary
wasm-tools component new test.wasm -o test.component.wasm

# 3. Run with wasmtime (using .cargo/config.toml runner)
wasmtime -S preview2 -S cli test.component.wasm
```

### Our Integration Test Structure

```
tests/
  integration/
    protocol_streaming/    # Test streaming responses
      test.wit             # WIT interface for test
      harness.rs           # Test implementation
      runner.rs            # Wasmtime runner that validates output
```

Each integration test:
- Defines a WIT interface for the test scenario
- Implements the test as a WASI component
- Uses wasmtime to execute and validate

## Running All Tests

```bash
# Unit tests only (fast)
make test-unit

# Integration tests only (requires wasmtime)
make test-integration

# All tests
make test
```

## Current Status

- [x] Unit tests: 40 tests for protocol utilities
- [ ] Integration tests: Design in progress

## Design Decisions

### Why Two Test Types?

**Unit Tests (Native)**:
- Fast iteration (no componentization overhead)
- Easy debugging (native tooling)
- Test pure logic without WASI complexity
- Run in CI without wasmtime

**Integration Tests (Component)**:
- Validate WIT bindings correctness
- Test WASI stream integration
- Verify component composition
- Catch ABI and marshal errors

### Why Not wit-bindgen test?

The `wit-bindgen test` command is internal to the wit-bindgen repository and used for testing the bindings generator itself across multiple languages. For application testing, we use:

1. Cargo's built-in test framework for unit tests
2. Custom integration tests with wasmtime for component tests

This gives us better control and simpler tooling.

## Testing Philosophy

> "Test behavior, not implementation"

- Unit tests validate algorithmic correctness
- Integration tests validate protocol compliance
- Both are necessary for a robust MCP server

## Future Work

- [ ] Add integration test for tool call streaming
- [ ] Add integration test for resource streaming
- [ ] Add integration test for stdio transport framing
- [ ] Add integration test for HTTP transport framing
- [ ] Add benchmarks for streaming performance
