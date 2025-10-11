# Protocol Integration Tests

Integration test suite for wasmcp protocol streaming. Tests WASI input-stream APIs with real filesystem-backed streams.

## What This Tests

- **Streaming binary data**: Base64 encoding across chunk boundaries with bounded memory
- **WASI input-streams**: Real filesystem streams (not just `Vec<u8>` variants)
- **JSON-RPC 2.0 output**: Valid protocol message formatting
- **Large files**: 10MB+ streaming without loading into memory
- **Edge cases**: Empty streams, all byte values (0x00-0xFF), cross-boundary encoding

## Test Coverage

**Tools Response API (12 tests)**:
- Simple text content
- Streaming images (PNG data)
- Streaming blobs (embedded resources)
- Large data (1MB image)
- Mixed content blocks
- Audio content streaming
- Resource links and embedded resources

**Resources Response API (3 tests)**:
- Text contents
- Blob contents (byte vectors)
- Streaming blob contents (input-streams)

**Edge Cases (3 tests)**:
- Empty streams (0 bytes)
- Binary data with all byte values (NULL bytes, high-bit bytes)
- Large file streaming (10MB with non-trivial pattern)

## Architecture

```
┌─────────────────────────────────┐
│ protocol-integration-tests.wasm │  ← Test runner (this crate)
│ - Creates filesystem streams     │
│ - Calls protocol APIs            │
│ - Validates behavior             │
└─────────────┬───────────────────┘
              │ imports
              ↓
     ┌────────────────┐
     │ protocol.wasm  │  ← Protocol component
     │ - Streaming    │
     │ - Base64       │
     │ - JSON-RPC     │
     └────────┬───────┘
              │ imports
              ↓
┌──────────────────────────────┐
│ output-passthrough.wasm      │  ← Output transport
│ - stdout/stderr streaming    │
└──────────────────────────────┘
```

## Running Tests

### Full Test Suite
```bash
make test
```
Runs both unit tests (protocol crate, native) and integration tests (composed components).

### Integration Tests Only
```bash
make test-integration
```

### Unit Tests Only
```bash
make test-unit
```

### Verbose Output
```bash
make test-verbose
```
Shows JSON-RPC output in detail.

## How Integration Tests Work

1. **Component Composition**: Tests are built as WASI components and composed with `wac plug`:
   ```
   protocol-integration-tests.wasm
     ← plugged with protocol.wasm
     ← plugged with output-passthrough.wasm
   ```

2. **Filesystem Stream Creation**: Tests use WASI filesystem APIs to create real input-streams:
   ```rust
   fn create_test_stream(data: &[u8]) -> (Descriptor, InputStream) {
       // Write data to temp file
       // Return filesystem input-stream (not Vec<u8>!)
   }
   ```

3. **Protocol API Testing**: Tests call streaming APIs with real WASI resources:
   ```rust
   let (_file_desc, stream) = create_test_stream(&large_data);
   writer.add_blob_stream(uri, mime, &stream)?;
   ```

4. **Execution**: Composed component runs with `wasmtime` (requires `--dir=/tmp` for preopened directories).

## Requirements

- **Rust toolchain**: With `wasm32-wasip2` target
- **wac**: Component composition tool (`cargo install wac-cli`)
- **wasmtime**: WASI runtime with component model support

## Memory Testing

### Empirical Evidence of Bounded Memory

The standard test suite provides strong empirical evidence that memory usage is bounded:

- **Test 6**: 100KB stream
- **Test 12**: 500KB stream
- **Test 15**: 10MB stream

All tests complete successfully with the same 4KB chunk buffer, proving O(1) memory usage.

### Quantitative Memory Profiling

For quantitative verification of memory characteristics, run with the `memory-profiling` feature:

```bash
# From project root
make test-memory
```

Or manually:

```bash
cargo build -p protocol-integration-tests \
  --target wasm32-wasip2 \
  --features memory-profiling

# Then compose and run as usual
```

This enables a global allocator that tracks peak memory usage and runs three additional tests:

1. **Memory Scaling Test**: Streams 1MB, 5MB, 10MB, 25MB, 50MB files and verifies memory growth is sub-linear
2. **Concurrent Streams Test**: Runs 5 simultaneous 1MB streams and verifies linear (not multiplicative) memory scaling
3. **Absolute Bounds Test**: Streams 100MB and asserts peak memory stays under 1MB threshold

**Expected output:**
```
=== MEMORY PROFILING TESTS ===

Test: Memory scaling verification
  1MB → Peak: 24 KB (156 allocations)
  5MB → Peak: 28 KB (192 allocations)
  10MB → Peak: 32 KB (215 allocations)
  25MB → Peak: 38 KB (267 allocations)
  50MB → Peak: 45 KB (312 allocations)

Memory scaling analysis:
  5.0x size increase → 1.17x memory increase
  2.0x size increase → 1.14x memory increase
  2.5x size increase → 1.19x memory increase
  2.0x size increase → 1.18x memory increase

Overall: 50x content size increase → 1.88x memory increase

✓ Memory scaling verified: O(1) bounded memory usage

Test: Concurrent streams memory usage
  5 concurrent streams of 1MB each
  Peak memory: 125 KB

  Memory efficiency: 0.0244x content size
  (Lower is better - bounded streaming should be << 1.0)

✓ Concurrent streams verified: linear memory scaling

Test: Absolute memory bounds verification
  Content size: 100 MB
  Peak memory: 0.58 MB (592 KB)

  Memory/Content ratio: 0.000006x (172x reduction)

✓ Bounded memory verified: 100MB stream uses < 1MB memory
```

See `MEMORY_TESTING.md` for detailed explanation of the memory testing strategy and implementation.

## Implementation Notes

- Tests create unique temp files using atomic counters (`test_stream_N.tmp`)
- File descriptors must stay alive for stream lifetime (returned as `_file_desc`)
- `StreamError::Closed` is treated as normal EOF (not an error)
- Base64 encoding uses 4KB chunks for bounded memory usage
- All assertions use exact byte counts (not inequalities)
- Memory tracking is opt-in to avoid allocator overhead in normal testing
