# Protocol Integration Tests

Integration test suite for wasmcp protocol streaming. Tests WASI input-stream APIs with real stdin streams.

## What This Tests

- **Streaming binary data from stdin**: Zero allocation, arbitrary sizes
- **WASI input-streams**: Real stdin stream (not heap-allocated buffers)
- **JSON-RPC 2.0 output**: Valid protocol message formatting
- **Bounded memory**: Proven via wasmtime's memory limits
- **Base64 encoding**: Chunk-based streaming across boundaries

## Architecture

```
┌─────────────────────────────────┐
│ protocol-integration-tests.wasm │  ← Test runner (this crate)
│ - Reads stdin via WASI           │
│ - Calls protocol APIs             │
│ - Validates behavior              │
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

### Integration Tests
```bash
make test-integration
```

Runs verification tests:
1. **JSON-RPC structure validation** - Validates output is parseable JSON-RPC 2.0
2. **Base64 encoding verification** - Encodes known 5-byte pattern `[0x00, 0x01, 0x02, 0x03, 0x04]` and verifies output is `AAECAwQ=`
3. **Streaming test** - Pipes 10MB through stdin
4. **Empty stdin handling** - Verifies graceful handling of EOF

### Memory Bounded Tests
```bash
make test-memory
```

Proves bounded memory with wasmtime limits:
1. **100MB in 2MB limit** - Streams 100MB, produces ~140MB JSON-RPC output (base64 expansion), using only 2MB memory (70x output-to-memory ratio)
2. **Minimum viable limit** - Streams 10MB in 1.5MB limit (7x ratio)

Both tests validate:
- Memory limit enforced by wasmtime (`-W max-memory-size`)
- Output is valid JSON-RPC 2.0 (via `jq`)
- Full content processed (output size matches expected base64 expansion)

### Developer Tools - Converting Files to MCP Messages

The test harness includes utilities to convert any file to the corresponding MCP message format with streaming support:

```bash
# Text content (tools/call result with text)
cat myfile.txt | make mcp-text | jq

# Image content (tools/call result with image)
cat image.png | make mcp-image | jq

# Audio content (tools/call result with audio)
cat audio.mp3 | make mcp-audio | jq

# Embedded resource text (tools/call result with embedded resource)
cat config.json | make mcp-resource-text | jq

# Embedded resource blob (tools/call result with embedded resource)
cat binary.dat | make mcp-resource-blob | jq

# Resource text (resources/read result with text)
cat document.md | make mcp-read-text | jq

# Resource blob (resources/read result with blob)
cat image.jpg | make mcp-read-blob | jq
```

**All converters:**
- Stream with bounded memory (2MB for any file size)
- Produce valid JSON-RPC 2.0 messages
- Can be piped to `jq` for pretty printing
- Show how MCP clients receive the content

**Example output:**
```bash
$ echo "Hello, MCP!" | make mcp-text 2>/dev/null | jq
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Hello, MCP!\\n"
      }
    ]
  }
}
```

### Custom Tests
```bash
# Test with 1GB stream (any content type)
dd if=/dev/zero bs=1M count=1024 | make mcp-read-blob >/dev/null

# Test image with memory limit
cat large-image.png | \
  wasmtime run -W max-memory-size=2097152 \
  <(printf '\x03') <(cat large-image.png) \
  target/wasm32-wasip2/debug/test-composed.wasm

# Test with real file
cat /path/to/file.pdf | make mcp-resource-blob | jq '.result.content[0].blob' -r | base64 -d > output.pdf
```

## How It Works

1. **Component Composition**: Tests are built as WASI components and composed with `wac plug`:
   ```
   protocol-integration-tests.wasm
     ← plugged with protocol.wasm
     ← plugged with output-passthrough.wasm
   ```

2. **Stdin Streaming**: Test uses WASI CLI `get-stdin()` to obtain input-stream:
   ```rust
   let stdin = wasi::cli::stdin::get_stdin(); // Returns input-stream
   writer.add_blob_stream("stdin://input", Some("application/octet-stream"), &stdin)?;
   ```

3. **Zero Allocations**: Data flows directly from host stdin → WASI stream → protocol encoder → stdout
   - No `Vec<u8>` allocations for test data
   - No temporary file creation
   - No heap allocation proportional to input size

4. **Execution**: Composed component runs with `wasmtime`, stdin piped from `dd` or other sources.

## Memory Testing

### Proof of Bounded Memory

The test suite proves O(1) memory usage regardless of content size:

```bash
make test-memory  # 100MB in 2MB limit
```

**How it proves bounded memory:**
- WebAssembly linear memory = stack + heap + globals + static data
- `-W max-memory-size=2097152` sets hard 2MB limit
- Test pipes 100MB through stdin
- If test completes → streaming uses < 2MB total

**Why this matters for edge deployment:**
- **Cloudflare Workers**: 128MB limit → 64x headroom
- **Fastly Compute**: 128MB limit → 64x headroom
- **AWS Lambda@Edge**: 128MB limit → 64x headroom

### Testing Larger Sizes in CI

```bash
# CI can test arbitrary sizes with zero overhead
dd if=/dev/zero bs=1M count=1000 | \  # 1GB
  wasmtime run -W max-memory-size=2097152 test-composed.wasm
```

## Requirements

- **Rust toolchain**: With `wasm32-wasip2` target
- **wac**: Component composition tool (`cargo install wac-cli`)
- **wasmtime**: WASI runtime with component model support
- **jq**: JSON processor for validating output structure

## Test Modes

The test harness supports multiple modes via the first byte of stdin:

| Mode | Byte | Description | Use Case |
|------|------|-------------|----------|
| Blob streaming | `0x00` | Stream as base64 blob (resources/read) | Default, large binary files |
| Base64 verify | `0x01` | Verify encoding of known pattern | CI validation |
| Text content | `0x02` | Tools/call result with text | Text responses, logs |
| Image content | `0x03` | Tools/call result with image | PNG, JPEG streaming |
| Audio content | `0x04` | Tools/call result with audio | MP3, WAV streaming |
| Resource text | `0x05` | Embedded resource text (tools/call) | Config files, embedded text |
| Resource blob | `0x06` | Embedded resource blob (tools/call) | Binary resources |
| Read text | `0x07` | Resource text (resources/read) | Text files |
| Read blob | `0x08` | Resource blob (resources/read) | Binary files |

## Implementation Notes

- Stdin accessed via `wasi:cli/stdin::get-stdin()` (WASI 0.2.0)
- Base64 encoding uses 4KB chunks for bounded memory usage
- JSON-RPC output goes to stdout (via output transport)
- Debug messages go to stderr for clean piping
- All modes support arbitrary file sizes with bounded memory
- No test data files required (all via stdin)
