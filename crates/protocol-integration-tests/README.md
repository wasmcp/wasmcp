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

## Implementation Notes

- Tests create unique temp files using atomic counters (`test_stream_N.tmp`)
- File descriptors must stay alive for stream lifetime (returned as `_file_desc`)
- `StreamError::Closed` is treated as normal EOF (not an error)
- Base64 encoding uses 4KB chunks for bounded memory usage
- All assertions use exact byte counts (not inequalities)
